# Changelog -- openbim-ifc-wasm

All notable changes to the `openbim-ifc-wasm` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

## [Unreleased]

### Added

- Browser and bundler builds in the npm package (#40). `@openbim/ifc`
  resolves to the CommonJS build under Node, as before, and to a
  `wasm-bindgen --target bundler` ES module in a bundler;
  `@openbim/ifc/web` is the `--target web` build for a browser without a
  bundler (`await init()` first). Every existing import path still resolves.
- `scripts/build-npm-pkg.sh` (was `build-node-pkg.sh`) builds all three
  targets and checks the packed tarball from Node, a webpack 5 bundle, and
  both browser builds in headless Chrome.

### Changed

- The module is shrunk with `wasm-opt -Oz` (binaryen 132). Measured on the
  default build: 1,337,025 bytes after `wasm-bindgen`, 1,295,241 after
  `wasm-opt` (-3.1%; the code section shrinks 9.1%, from 424,262 to
  385,757 bytes). The compressed size does not fall: 459,437 to 461,953
  bytes with `gzip -9`. The bundled schema data, which wasm-opt cannot
  shrink, is 908,687 of the 1,337,025 bytes.
- The tarball carries one copy of the module per target, so it grows from
  about 0.5 MB to 1.4 MB packed; a consumer loads only one.

## [0.2.0] - 2026-09-29

### Added

- Release features `ifc2x3`, `ifc4`, `ifc4x1`, `ifc4x2`, `ifc4x3` (all
  default, so the npm package is unchanged). A browser build with one
  release, `--no-default-features --features ifc4`, is 699,687 bytes after
  `wasm-bindgen` instead of 1,236,036 (#112).

## [0.1.1] - 2026-09-26

The first npm release built and published by the release workflow, with npm
provenance. The JavaScript API is unchanged.

### Changed

- Reading STEP is faster: the model is now built straight from
  `openbim-step` 0.7.0's borrowed events instead of copying every value
  twice. Real files read with 22-40% fewer instructions, and the parsed
  model is identical (checked on 2,273 files).
- Built against `openbim-ifc` 0.5.0.
- The crate is not published to crates.io (`publish = false`). The package
  ships on npm as `@openbim/ifc` only.

## [0.1.0] - 2026-09-24

Published to npm by hand, before the release workflow existed.

### Added

- The model operations, value encoding and error codes now come from
  `openbim-ifc-binding-core`, shared with the C and Python bindings. The
  JavaScript API is unchanged.

- WebAssembly bindings for the `openbim-ifc` facade (#34, ADR 0013).
  `IfcModel` parses and writes IFC STEP, lists entities, queries by exact
  type or including subtypes (per the file's declared IFC2X3, IFC4 or
  IFC4X3 schema), reads and edits attributes, adds and removes entities,
  and reports dangling references.
- A lossless tagged value encoding with TypeScript declarations
  (`IfcValue`): `$` and `*`, `.U.` and `.F.`, integers and reals, typed
  wrappers, and 64-bit integers as `bigint` all stay distinct.
- Every failure throws an `IfcError` with a stable `code`; a refused edit
  leaves the model unchanged.
- `scripts/build-node-pkg.sh` builds a Node package with the pinned
  `wasm-bindgen` CLI and runs the Node smoke and corpus suites.

[Unreleased]: https://github.com/openbimrs/ifc/compare/openbim-ifc-wasm-v0.2.0...HEAD
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.2.0
[0.1.1]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.1.1
[0.1.0]: https://www.npmjs.com/package/@openbim/ifc/v/0.1.0
