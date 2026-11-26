# brep-conformance-cad-kernel

The [brep-conformance](https://github.com/springcraft-ai/brep-conformance) adapter for
[cad-kernel](https://crates.io/crates/cad-kernel): it puts cad-kernel behind the suite's protocol, so
that the suite can score it.

## Who maintains this

brep-conformance scores a B-rep kernel through an adapter: a small program that answers the suite's
requests with the kernel's own calls. Springcraft, which maintains the suite, wrote this adapter and
maintains it.

- **It is not cad-kernel's.** cad-kernel's maintainers did not write it or choose the tests it runs.
- **A failure can be the adapter's.** A case cad-kernel fails in the results may be this adapter
  driving it wrongly. Corrections are welcome: open an issue or a pull request.
- **It adds nothing to the kernel.** The adapter is pass-through, as the suite's
  [contract](https://github.com/springcraft-ai/brep-conformance/blob/main/doc/contract.md#what-an-adapter-may-do)
  asks: one call of the kernel for each function, and anything the kernel has no one call for is
  listed as unsupported. What is scored is cad-kernel, not this code.
- **cad-kernel's maintainers are welcome here.** They know best how to drive cad-kernel, so their
  review and corrections carry the most weight. Ask in an issue to be added as a reviewer.
- **The suite decides what is scored.** An official run uses only a commit of this adapter that the
  suite's maintainers have reviewed as pass-through, whoever wrote it.

## Overview

[`src/main.rs`](src/main.rs) answers each function with cad-kernel's one call for it, as
[the contract](https://github.com/springcraft-ai/brep-conformance/blob/main/doc/contract.md#what-an-adapter-may-do)
asks, and offers only those: `supports` lists them, and the suite scores a case that needs any
other as unsupported. It builds the suite's box, cylinder, sphere, torus and cone
with cad-kernel's own constructors, and its extrusions, revolutions and screws with its prism,
revolution and screw of a profile, each about the origin and moved onto its frame. It answers the
booleans, the booleans of many (cad-kernel's Boolean of any number of solids at once), volume and
area, the validity check, classification, outward normals, rigid and affine maps, fillets,
offsets, shells, meshes, STEP writing and reading, and the deletion of faces with cad-kernel's own,
naming edges and faces by cad-kernel's reading of the point onto them. The centroid is cad-kernel's
first moments of the solid over its volume, all from its one call of the solid's moments. A file of
several solids is not read, since cad-kernel reads each as a solid of its own. A thicken is cad-kernel's sweep of one face along its normal; a
thicken of several faces at once, and a normal at a point several faces meet at, are unsupported.

cad-kernel's tolerance is relative, a part in 10⁹ of a solid's size, and the adapter passes that to
each call that takes one. Its mesher's angular deflection is set to 0.2 rad, the value cad-kernel's
own queries use, and the curves its STEP writer fits are held within the suite's 1e-7 mm; `hello`
says both.

cad-kernel says a result is empty by a B-rep with no faces, and the adapter reads that, and nothing
else, as the empty solid.

Left out of `supports`, since cad-kernel 0.26 has no one call of the protocol's shape for it:

- Lofts: its loft takes the walls' B-spline nets, not the sections.
- Sweeps: it carries a profile along one line, turn or screw; the protocol's paths join several.
- Chamfers: its flat along an edge is the piece a chamfer cuts away, not the solid chamfered.
- Drafts: it drafts a profile as it extrudes it, not a solid's faces.
- Splits: its split takes sheets reaching past the solid, not a plane.
- Sections: its section reads solids of revolution only.
- The least bounding box: its box is never smaller than the solid, but not the least.
- Inertia: its moments are about a point the caller gives, and the centroid about which the
  protocol asks for them takes a call of its own.
- Sewing: it sews named sheets and planar faces it is given, not the loose faces of a STEP file.
- Slabs, blends of unequal, varying or other-shaped section, face offsets and moves, the nearest
  point, curvatures, distance, interference, Betti numbers, projections, displays and ray casts,
  which it does not have.

Profile splines are refused until the adapter gives them as cad-kernel's B-splines.

## Setup

The Rust toolchain [`rust-toolchain.toml`](rust-toolchain.toml) pins.

## Usage

The suite runs the adapter as a separate process. Build the adapter here, then run the suite from a
checkout of [brep-conformance](https://github.com/springcraft-ai/brep-conformance) beside this one:

```bash
cargo build --release
cd ../brep-conformance
cargo build --release -p brep-conformance
target/release/brep-conformance \
    --adapter ../brep-conformance-cad-kernel/target/release/cad-kernel-adapter --out cad-kernel.json
```

## Licences

The adapter in this repository is under either the MIT licence
([LICENSE-MIT](LICENSE-MIT)) or the Apache License, Version 2.0
([LICENSE-APACHE](LICENSE-APACHE)), at your option.

Either licence covers the adapter and nothing else. It does not cover the kernel, and nothing here
relicenses the kernel.

cad-kernel is a separate work under its own licence. Its crate declares `MIT OR Apache-2.0`; its
repository is the authority on that, not this page.

Building the adapter downloads cad-kernel from crates.io and links it in, so a built adapter
contains cad-kernel and is subject to cad-kernel's terms as well as this repository's.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
