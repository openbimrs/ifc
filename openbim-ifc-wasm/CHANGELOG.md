# Changelog -- openbim-ifc-wasm

All notable changes to the `openbim-ifc-wasm` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

## [Unreleased]

### Added

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
