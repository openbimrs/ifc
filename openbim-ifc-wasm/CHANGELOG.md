# Changelog -- openbim-ifc-wasm

All notable changes to the `openbim-ifc-wasm` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

## [Unreleased]

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

[Unreleased]: https://github.com/openbimrs/ifc/compare/openbim-ifc-wasm-v0.1.1...HEAD
[0.1.1]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.1.1
[0.1.0]: https://www.npmjs.com/package/@openbim/ifc/v/0.1.0
