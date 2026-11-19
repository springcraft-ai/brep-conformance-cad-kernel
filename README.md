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
asks, and lists the rest as unsupported. It builds the suite's box, cylinder, sphere, torus and cone
with cad-kernel's own constructors, and its extrusions, revolutions and screws with its prism,
revolution and screw of a profile, each about the origin and moved onto its frame. It answers the
booleans, the booleans of many (cad-kernel's Boolean of any number of solids at once), volume and
area, the validity check, classification, rigid and affine maps, fillets, offsets, shells, meshes
and STEP with cad-kernel's own, naming edges and faces by cad-kernel's reading of the point onto
them.

cad-kernel's tolerance is relative, a part in 10⁹ of a solid's size, and the adapter passes that to
each call that takes one. Its mesher's angular deflection is set to 0.2 rad, the value cad-kernel's
own queries use, and `hello` says so.

cad-kernel says a result is empty by a B-rep with no faces, and the adapter reads that, and nothing
else, as the empty solid.

Listed as unsupported, since cad-kernel 0.17 has no one call of the protocol's shape for it: lofts
(its loft takes B-spline nets, not profiles), sweeps, slabs, chamfers (its flat along an edge is
the piece a chamfer cuts away, not the solid chamfered), blends of unequal, varying or other-shaped
section, face offsets, deletes, moves, thickens and drafts, STEP reading and sewing, the nearest
point, normals, curvatures, mass properties beyond volume, distance, interference, Betti numbers,
sections, projections, splits, displays, ray casts, and the least bounding box (cad-kernel's box is
never smaller than the solid, but not the least). Profile splines are refused until the adapter
gives them as cad-kernel's B-splines.

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

The adapter in this repository is under the MIT licence: see [LICENSE](LICENSE).

That licence covers the adapter and nothing else. It does not cover the kernel, and nothing here
relicenses the kernel.

cad-kernel is a separate work under its own licence. Its crate declares `MIT OR Apache-2.0`; its
repository is the authority on that, not this page.

Building the adapter downloads cad-kernel from crates.io and links it in, so a built adapter
contains cad-kernel and is subject to cad-kernel's terms as well as this repository's.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
