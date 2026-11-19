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

The adapter is a single Rust program that answers each function of the suite's protocol with
cad-kernel's one call for it, and lists the rest as unsupported.

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
