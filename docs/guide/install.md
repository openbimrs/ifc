# Install

The IFC stack is published for four languages. They all share one Rust core
(ADR [0013](/adr/0013-language-bindings-wrap-the-facade)), so a file read in
one reads the same in the others.

<!-- INSTALL:TABLE:BEGIN -->

| Language | Package | Latest release | Install | Requires | Reference |
| --- | --- | --- | --- | --- | --- |
| Rust | [`openbim-ifc`](https://crates.io/crates/openbim-ifc) | 0.7.0 (2026-09-27) | `cargo add openbim-ifc` | Rust `1.88.0` | [`openbim-ifc`](/reference/crates/openbim-ifc) |
| JavaScript / TypeScript | [`@openbim/ifc`](https://www.npmjs.com/package/@openbim/ifc) | 0.1.1 (2026-09-26) | `npm install @openbim/ifc` | Node `>=18` | [`openbim-ifc-wasm`](/reference/crates/openbim-ifc-wasm) |
| Python | [`openbim-ifc`](https://pypi.org/project/openbim-ifc/) | 0.1.0 (2026-09-26) | `pip install openbim-ifc` | Python `>=3.9` | [`openbim-ifc-py`](/reference/crates/openbim-ifc-py) |
| C / C++ | `openbim-ifc-capi` (not published) | not released | build from source | a C11 or C++17 compiler, and Rust to build | [`openbim-ifc-capi`](/reference/crates/openbim-ifc-capi) |

<!-- INSTALL:TABLE:END -->

The table is generated from each package's changelog and manifest, so the
versions and toolchain floors on this page are the ones the packages
declare. The install commands take no version and resolve to the newest
release.

## Rust

Depend on the [`openbim-ifc`](/reference/crates/openbim-ifc) facade and
turn on only the features you need. With no features beyond the default you
get the model and the STEP codec, and nothing else is compiled:

```bash
cargo add openbim-ifc                                  # model + STEP
cargo add openbim-ifc --features properties,spatial    # plus two domains
```

The library is imported as `ifc` (`use ifc::{Codec, StepCodec};`). Every
feature, and the crate each one enables, is listed on the
[facade's reference page](/reference/crates/openbim-ifc#features); the
[crate reference](/reference/) describes each crate.

## JavaScript and TypeScript

```bash
npm install @openbim/ifc
```

See [JavaScript and TypeScript](/bindings/javascript).

## Python

```bash
pip install openbim-ifc
```

See [Python](/bindings/python).

## C and C++

The C ABI is not packaged yet. Build it from a checkout and link the static
or shared library against `openbim-ifc-capi/include/openbim_ifc.h`:

```bash
cargo build -p openbim-ifc-capi --release
```

See [C and C++](/bindings/c).
