# Install

The IFC stack is published for five languages. They all share one Rust core
(ADR [0013](/adr/0013-language-bindings-wrap-the-facade)), so a file read in
one reads the same in the others.

<!-- INSTALL:TABLE:BEGIN -->

| Language | Package | Latest release | Install | Requires | Reference |
| --- | --- | --- | --- | --- | --- |
| Rust | [`openbim-ifc`](https://crates.io/crates/openbim-ifc) | 0.14.0 (2026-10-04) | `cargo add openbim-ifc` | Rust `1.88.0` | [`openbim-ifc`](/reference/crates/openbim-ifc) |
| JavaScript / TypeScript | [`@openbim/ifc`](https://www.npmjs.com/package/@openbim/ifc) | 0.4.0 (2026-10-04) | `npm install @openbim/ifc` | Node `>=18` | [`openbim-ifc-wasm`](/reference/crates/openbim-ifc-wasm) |
| Python | [`openbim-ifc`](https://pypi.org/project/openbim-ifc/) | 0.3.1 (2026-10-04) | `pip install openbim-ifc` | Python `>=3.9` | [`openbim-ifc-py`](/reference/crates/openbim-ifc-py) |
| C / C++ | [`openbim_ifc`](https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-capi-v0.1.2) (CMake, prebuilt archives) | 0.1.2 (2026-10-04) | `find_package(openbim_ifc)` | a C11 or C++17 compiler and CMake 3.21; Rust to build from source | [`openbim-ifc-capi`](/reference/crates/openbim-ifc-capi) |
| C# / .NET | [`OpenBim.Ifc`](https://www.nuget.org/packages/OpenBim.Ifc) | 0.1.0 (2026-10-04) | `dotnet add package OpenBim.Ifc` | `netstandard2.0` or `net8.0` | [`openbim-ifc-dotnet`](/reference/crates/openbim-ifc-dotnet) |

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

The C ABI is a CMake package, `openbim_ifc`. Unpack a prebuilt archive from
an `openbim-ifc-capi-v*` [GitHub release](https://github.com/openbimrs/ifc/releases)
or install it from a checkout, then:

```cmake
find_package(openbim_ifc 0.1 CONFIG REQUIRED)
target_link_libraries(app PRIVATE openbim_ifc::openbim_ifc)
```

See [C and C++](/bindings/c#install-via-cmake).

## .NET

```bash
dotnet add package OpenBim.Ifc
```

One package for .NET 8 and for .NET Framework 4.6.2 and later (through
`netstandard2.0`), with the native library for Windows, Linux and macOS on
x64 and Arm inside. See [C# and .NET](/bindings/dotnet).
