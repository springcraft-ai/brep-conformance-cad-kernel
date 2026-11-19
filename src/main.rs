//! The cad-kernel adapter: answers the B-rep protocol's requests on stdin and stdout, so the
//! conformance suite can drive this kernel.

use std::f64::consts::TAU;
use std::process::ExitCode;

use brep_protocol::{Refusal, model, model::Primitive, profile};
use cad_kernel::affine::Affine;
use cad_kernel::boolean::{Op, combined, selected};
use cad_kernel::build::{
    Profile, ProfileEdge, block, cone, prism, revolve, screwed, sphere, torus,
};
use cad_kernel::geom::{Frame, Rigid, V, tolerance};
use cad_kernel::offset::{Join, Side, grown, normal_sweep, shelled, shrunk};
use cad_kernel::props;
use cad_kernel::query::{Located, Place};
use cad_kernel::rounds::rounded;
use cad_kernel::topo::Brep;
use cad_kernel::util::space::{cross, distance, dot};

/// The contract's `10·ε`, within which a point names an edge or a face.
const ADDRESS: f64 = 1e-6;

/// The mesher's angular deflection, radians: the value cad-kernel's own queries mesh with.
const ANGULAR: f64 = 0.2;

struct CadKernel;

/// The empty solid: cad-kernel says a result is empty by a B-rep with no faces.
fn empty() -> Brep {
    Brep {
        vertices: Vec::new(),
        edges: Vec::new(),
        faces: Vec::new(),
    }
}

fn is_empty(b: &Brep) -> bool {
    b.faces.is_empty()
}

/// cad-kernel's working tolerance for a solid: a part in 10⁹ of its size.
fn tol(b: &Brep) -> f64 {
    tolerance(b.size())
}

/// The motion taking a solid built about the world origin onto `frame`.
fn placement(f: &model::Frame) -> Result<Rigid, Refusal> {
    if dot(cross(f.x, f.y), f.z) < 0.0 {
        return Err(Refusal::invalid("a left-handed frame"));
    }
    Ok(Rigid {
        r: [0, 1, 2].map(|i| [f.x[i], f.y[i], f.z[i]]),
        t: f.origin,
    })
}

/// Where a profile is drawn: its points in space, and the plane's normal.
#[derive(Clone, Copy)]
enum Plane {
    /// The xy plane, facing z.
    Xy,
    /// A revolution's meridian: x out from the z axis, y along it.
    Meridian,
}

impl Plane {
    fn point(self, p: [f64; 2]) -> V {
        match self {
            Plane::Xy => [p[0], p[1], 0.0],
            Plane::Meridian => [p[0], 0.0, p[1]],
        }
    }

    /// The normal a loop counter-clockwise in the profile's own x and y turns about.
    fn normal(self) -> V {
        match self {
            Plane::Xy => [0.0, 0.0, 1.0],
            Plane::Meridian => [0.0, -1.0, 0.0],
        }
    }
}

/// The suite's profile as one of cad-kernel's, drawn in `plane`. cad-kernel reads a loop's edges in
/// any direction, each arc counter-clockwise about its frame's z, so a clockwise arc is the same
/// span read from its other end.
fn sketch(p: &profile::Profile, plane: Plane) -> Result<Profile, Refusal> {
    let edge = |s: &profile::Seg| -> Result<ProfileEdge, Refusal> {
        match *s {
            profile::Seg::Line { a, b } => Ok(ProfileEdge::Line {
                a: plane.point(a),
                b: plane.point(b),
            }),
            profile::Seg::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                let frame =
                    Frame::new(plane.point(center), plane.normal(), plane.point([1.0, 0.0]));
                let span = if sweep.abs() >= TAU - 1e-12 {
                    None
                } else if sweep > 0.0 {
                    Some([start, start + sweep])
                } else {
                    Some([start + sweep, start])
                };
                Ok(ProfileEdge::Arc {
                    frame,
                    r: radius,
                    span,
                })
            }
            profile::Seg::Spline(_) => Err(Refusal::unsupported(
                "the adapter does not yet give a periodic spline as cad-kernel's B-spline",
            )),
            profile::Seg::Conic { .. } => Err(Refusal::unsupported(
                "cad-kernel profiles have no conic segment",
            )),
            profile::Seg::Gn { .. } => Err(Refusal::unsupported(
                "cad-kernel profiles have no blend-section segment",
            )),
        }
    };
    let loops = p
        .loops
        .iter()
        .map(|l| l.iter().map(edge).collect::<Result<Vec<_>, _>>())
        .collect::<Result<Vec<_>, _>>()?;
    if loops.is_empty() {
        return Err(Refusal::invalid("a profile with no loop"));
    }
    Ok(Profile {
        origin: [0.0; 3],
        normal: plane.normal(),
        loops,
        names: Vec::new(),
    })
}

/// Each primitive is built about the world origin and moved onto its frame.
fn make(p: &Primitive) -> Result<Brep, Refusal> {
    let z = [0.0, 0.0, 1.0];
    let o = [0.0; 3];
    let (b, frame) = match p {
        Primitive::Box { frame, size } => (block(o, *size), frame),
        Primitive::Cylinder {
            frame,
            radius,
            height,
        } => (cone(o, z, *radius, *radius, *height), frame),
        Primitive::Sphere { frame, radius } => (sphere(o, z, *radius), frame),
        Primitive::Torus {
            frame,
            major,
            minor,
        } => (torus(o, z, *major, *minor), frame),
        Primitive::Cone {
            frame,
            bottom,
            top,
            height,
        } => (cone(o, z, *bottom, *top, *height), frame),
        Primitive::Extrude {
            frame,
            profile,
            height,
        } => (prism(&sketch(profile, Plane::Xy)?, 0.0, *height), frame),
        Primitive::Revolve {
            frame,
            profile,
            angle,
        } => (
            revolve(&sketch(profile, Plane::Meridian)?, o, z, *angle),
            frame,
        ),
        Primitive::Screw {
            frame,
            profile,
            rise,
            angle,
        } => (
            screwed(
                &sketch(profile, Plane::Meridian)?,
                o,
                z,
                *angle,
                rise * angle,
            ),
            frame,
        ),
        _ => {
            return Err(Refusal::unsupported(
                "cad-kernel has no one call for this constructor",
            ));
        }
    };
    Ok(b.map_err(Refusal::failed)?.moved(&placement(frame)?))
}

/// The boolean of `a` and `b`, where neither is empty.
fn boolean(op: Op, a: &Brep, b: &Brep) -> Result<Brep, Refusal> {
    match (op, is_empty(a), is_empty(b)) {
        (_, false, false) => combined(a, b, op, 0.0).map_err(|e| Refusal::failed(e.msg)),
        (Op::Union, _, true) | (Op::Cut, _, true) => Ok(a.clone()),
        (Op::Union, true, false) => Ok(b.clone()),
        _ => Ok(empty()),
    }
}

/// The edge of `b` each point lies on: the nearest by cad-kernel's own reading of a point onto an
/// edge, which must pass within `10·ε`.
fn edges_at(b: &Brep, points: &[model::Vec3]) -> Result<Vec<usize>, Refusal> {
    let located = Located::new(b, tol(b));
    points
        .iter()
        .map(|&p| {
            (0..b.edges.len())
                .map(|e| (located.nearest(e as u32, p).1, e))
                .filter(|(d, _)| *d <= ADDRESS)
                .min_by(|x, y| x.0.total_cmp(&y.0))
                .map(|(_, e)| e)
                .ok_or_else(|| Refusal::invalid(format!("no edge passes through {p:?}")))
        })
        .collect()
}

/// The face of `b` each point lies on: the one whose surface passes within `10·ε` of it, by the
/// foot cad-kernel reads the point onto, and that holds it. A point several faces hold, on an edge
/// or a vertex, is one the adapter does not choose for.
fn faces_at(b: &Brep, points: &[model::Vec3]) -> Result<Vec<usize>, Refusal> {
    let located = Located::new(b, tol(b));
    points
        .iter()
        .map(|&p| match faces_through(b, &located, p)[..] {
            [] => Err(Refusal::invalid(format!("no face passes through {p:?}"))),
            [fi] => Ok(fi),
            _ => Err(Refusal::unsupported(
                "a point on an edge or a vertex, where several faces meet",
            )),
        })
        .collect()
}

/// The faces of `b` whose surface passes within `10·ε` of `p`, by the foot cad-kernel reads the
/// point onto, and that hold it.
fn faces_through(b: &Brep, located: &Located, p: model::Vec3) -> Vec<usize> {
    (0..b.faces.len())
        .filter(|&fi| {
            let foot = b.faces[fi].surface.point(located.uv(fi, p));
            distance(foot, p) <= ADDRESS && located.face_place(fi, p) != Place::Out
        })
        .collect()
}

/// `p ↦ M·p + t` for the rows `[M | t]`, by cad-kernel's affine map of a whole solid.
fn mapped(s: &Brep, m: [[f64; 4]; 3]) -> Result<Brep, Refusal> {
    let a = Affine::new(m.map(|r| [r[0], r[1], r[2]]), [m[0][3], m[1][3], m[2][3]])
        .map_err(Refusal::invalid)?;
    Ok(s.mapped(&a))
}

impl brep_protocol::Kernel for CadKernel {
    type Solid = Brep;

    fn name(&self) -> String {
        "cad-kernel".into()
    }

    fn version(&self) -> String {
        "0.17.0".into()
    }

    /// cad-kernel's tolerance is relative, a part in 10⁹ of a solid's size; at the 100 mm the
    /// suite's solids stay within, that is 1e-7 mm.
    fn tolerance(&self) -> f64 {
        tolerance(100.0)
    }

    fn configuration(&self) -> String {
        format!("mesh angular deflection {ANGULAR} rad")
    }

    fn make(&self, p: &Primitive) -> Result<Brep, Refusal> {
        make(p)
    }

    fn union(&self, a: &Brep, b: &Brep) -> Result<Brep, Refusal> {
        boolean(Op::Union, a, b)
    }

    fn intersection(&self, a: &Brep, b: &Brep) -> Result<Brep, Refusal> {
        boolean(Op::Common, a, b)
    }

    fn difference(&self, a: &Brep, b: &Brep) -> Result<Brep, Refusal> {
        boolean(Op::Cut, a, b)
    }

    /// cad-kernel's Boolean of many solids at once, each cell kept where any operand holds it.
    fn union_all(&self, solids: &[&Brep]) -> Result<Brep, Refusal> {
        let solids: Vec<&Brep> = solids.iter().copied().filter(|s| !is_empty(s)).collect();
        match solids[..] {
            [] => Ok(empty()),
            [s] => Ok(s.clone()),
            _ => {
                let t = solids.iter().map(|s| tol(s)).fold(0.0, f64::max);
                let keep = |m: &[bool]| m.iter().any(|&x| x);
                Ok(selected(&solids, &keep, t).map_err(Refusal::failed)?.brep)
            }
        }
    }

    /// cad-kernel's Boolean of many solids at once, each cell kept where the target holds it and
    /// no tool does.
    fn difference_all(&self, target: &Brep, tools: &[&Brep]) -> Result<Brep, Refusal> {
        if is_empty(target) {
            return Ok(empty());
        }
        let mut operands = vec![target];
        operands.extend(tools.iter().copied().filter(|s| !is_empty(s)));
        if operands.len() == 1 {
            return Ok(target.clone());
        }
        let t = operands.iter().map(|s| tol(s)).fold(0.0, f64::max);
        let keep = |m: &[bool]| m[0] && !m[1..].iter().any(|&x| x);
        Ok(selected(&operands, &keep, t).map_err(Refusal::failed)?.brep)
    }

    fn volume(&self, s: &Brep) -> Result<f64, Refusal> {
        Ok(props::volume(s))
    }

    /// The boundary's area: cad-kernel measures it face by face.
    fn area(&self, s: &Brep) -> Result<f64, Refusal> {
        Ok(s.faces.iter().map(|f| props::area(s, f)).sum())
    }

    fn check(&self, s: &Brep) -> Result<Vec<String>, Refusal> {
        if is_empty(s) {
            return Ok(Vec::new());
        }
        Ok(match s.check(tol(s)) {
            Ok(()) => Vec::new(),
            Err(e) => vec![format!("{:?}: {}", e.kind, e.msg)],
        })
    }

    fn classify(&self, s: &Brep, points: &[model::Vec3]) -> Result<Vec<model::Side>, Refusal> {
        if is_empty(s) {
            return Ok(vec![model::Side::Out; points.len()]);
        }
        let located = Located::new(s, tol(s));
        Ok(points
            .iter()
            .map(|&p| match located.solid_place(p) {
                Place::In => model::Side::In,
                Place::On => model::Side::On,
                Place::Out => model::Side::Out,
            })
            .collect())
    }

    fn transform(&self, s: &Brep, matrix: [[f64; 4]; 3]) -> Result<Brep, Refusal> {
        mapped(s, matrix)
    }

    fn affine(&self, s: &Brep, matrix: [[f64; 4]; 3]) -> Result<Brep, Refusal> {
        mapped(s, matrix)
    }

    fn fillet(&self, s: &Brep, edges: &[model::Vec3], radius: f64) -> Result<Brep, Refusal> {
        if is_empty(s) {
            return Err(Refusal::invalid("nothing to fillet"));
        }
        let found = edges_at(s, edges)?;
        rounded(s, &found, radius, tol(s)).map_err(Refusal::failed)
    }

    /// Every point within `d` of the solid: cad-kernel's growth with round joins, or its
    /// shrinking where `d` is negative.
    fn offset(&self, s: &Brep, d: f64) -> Result<Brep, Refusal> {
        if is_empty(s) {
            return Ok(empty());
        }
        let r = if d < 0.0 {
            shrunk(s, -d, Join::Round)
        } else {
            grown(s, d, Join::Round)
        };
        r.map_err(Refusal::failed)
    }

    /// The wall every point of the solid within `t` of its other faces: cad-kernel's hollowing
    /// inward, with round joins.
    fn shell(&self, s: &Brep, open: &[model::Vec3], t: f64) -> Result<Brep, Refusal> {
        if is_empty(s) {
            return Err(Refusal::invalid("nothing to shell"));
        }
        let found = faces_at(s, open)?;
        shelled(s, &found, t, Side::In, Join::Round).map_err(Refusal::failed)
    }

    /// cad-kernel sweeps one face along its normal per call: a face swept from 0 to `t`, or from
    /// `t` to 0 where it is negative.
    fn thicken(&self, s: &Brep, faces: &[model::Vec3], t: f64) -> Result<Brep, Refusal> {
        if is_empty(s) {
            return Err(Refusal::invalid("nothing to thicken"));
        }
        let [fi] = faces_at(s, faces)?[..] else {
            return Err(Refusal::unsupported(
                "cad-kernel sweeps one face along its normal per call, not several at once",
            ));
        };
        let span = if t < 0.0 { [t, 0.0] } else { [0.0, t] };
        normal_sweep(s, fi, span).map_err(Refusal::failed)
    }

    /// cad-kernel's outward normal of the one face a point lies on.
    fn normal(
        &self,
        s: &Brep,
        points: &[model::Vec3],
    ) -> Result<Vec<Option<model::Vec3>>, Refusal> {
        if is_empty(s) {
            return Ok(vec![None; points.len()]);
        }
        let located = Located::new(s, tol(s));
        points
            .iter()
            .map(|&p| match faces_through(s, &located, p)[..] {
                [] => Ok(None),
                [fi] => Ok(s.faces[fi].outward(p)),
                _ => Err(Refusal::unsupported(
                    "a point on an edge or a vertex, where several faces meet",
                )),
            })
            .collect()
    }

    fn mesh(&self, s: &Brep, tolerance: f64) -> Result<brep_protocol::Mesh, Refusal> {
        if is_empty(s) {
            return Ok(brep_protocol::Mesh::default());
        }
        let m = cad_kernel::mesh::mesh(s, tolerance, ANGULAR).map_err(Refusal::failed)?;
        Ok(brep_protocol::Mesh {
            positions: m.pts,
            triangles: m.tris,
        })
    }

    fn write_step(&self, s: &Brep) -> Result<String, Refusal> {
        if is_empty(s) {
            return Err(Refusal::invalid("an empty solid has no STEP"));
        }
        cad_kernel::step::write(s, "solid", tol(s)).map_err(Refusal::failed)
    }

    fn unsupported(&self) -> Vec<String> {
        [
            // Operations cad-kernel 0.17 does not have in one call of the protocol's shape.
            "loft",
            "loft_through",
            "sweep",
            "sweep_path",
            "slab",
            "fillet_radii",
            "fillet_variable",
            "fillet_conic",
            "fillet_g2",
            "fillet_setback",
            "fillet_setback_g2",
            "fillet_faces",
            "full_round",
            // cad-kernel's flat along an edge is the piece a chamfer cuts away, not the solid
            // chamfered.
            "chamfer",
            "chamfer_two",
            "chamfer_distances",
            "offset_faces",
            "delete_faces",
            "move_faces",
            "draft",
            "sew",
            "read_step",
            "closest",
            "curvature",
            "centroid",
            "inertia",
            "distance",
            "interference",
            "betti",
            "section_area",
            "section",
            "project",
            "split",
            "display",
            "raycast",
            // cad-kernel's box round a solid is never smaller than it; the protocol asks for the
            // least one.
            "bounding_box",
        ]
        .map(String::from)
        .to_vec()
    }
}

fn main() -> ExitCode {
    match brep_protocol::serve(&CadKernel) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
