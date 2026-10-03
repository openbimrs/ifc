# Changelog -- openbim-ifc-wasm

All notable changes to the `openbim-ifc-wasm` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

## [Unreleased]

## [0.3.0] - 2026-10-03

### Added (#123, property sets: write side)

- `IfcModel.setProperties(edits)`, `setProperty(object, set, name, value,
  setType?)` and `removeProperty(object, set, name)`, with the TypeScript
  types `PropertyEdit` and `PropertyEditResult`; error codes
  `template-violation` and `missing-property`.
- Default features `properties-write` and `property-catalog`. The default
  module grows from 2,343,181 to 6,282,871 bytes after `wasm-bindgen`
  (796,234 to 1,855,125 under `gzip -9`): the writer about 205 KB, the
  catalog 3.7 MB. A build without the catalog refuses a write to a
  `Pset_`/`Qto_` set with `feature-disabled`.
- Additive: a patch release.

### Changed (#306)

- A build that names fewer releases now carries only their schema
  tables whichever capabilities and domains it enables. IFC4 with every
  capability and domain is 1,750,572 bytes after `wasm-bindgen` (was
  2,342,546, the size of the five-release default); IFC4 with validation
  929,080 (was 1,521,685), with property sets 926,986 (was 1,519,870).
  The default package is unchanged in content: 2,343,181 bytes (+635),
  796,234 under `gzip -9` (-611), 516,930 under brotli (-1,541).

### Added (#303)

- `scripts/bench-opt-level.sh` and `tools/bench-parse.mjs`: build the
  module at `opt-level` 3, `"s"` and `"z"` and time `IfcModel.parse` in
  Node, interleaved, with median and interquartile range. `"z"` cut the
  brotli download by 14.5% but slowed parsing by 85-112%, so the release
  profile stays at 3; the JavaScript guide records the run.

### Added (#123, domain views: read side)

- `model.propertySets(id)`, `model.resolveUnit(measureType, unit?)`,
  `model.spatialTree()`, `model.classifications(id)`, `model.material(id)`,
  `model.systems()`, `model.cost()` and `model.georeferencing()`: snapshot
  objects keyed by `bigint` ids, IFC values in the tagged encoding.
- TypeScript interfaces for every record (`PropertySet`, `Property`,
  `SpatialTree`, `Classification`, `MaterialAssignment`, `Systems`, `Cost`,
  `MapConversion`, ...); `IfcErrorCode` gains `invalid-model`,
  `missing-reference`, `budget-exceeded`, `unsupported` and
  `wrong-entity-type`.
- Default features `properties`, `spatial`, `classification`, `material`,
  `systems`, `cost` and `georef`: the npm package carries every capability.
  A browser build can leave each out; its methods then throw
  `feature-disabled`. Measured after `wasm-bindgen` over a build with every
  release and capability (1,894,584 bytes): properties +155,575, spatial
  +15,512, classification +57,311, material +83,494, systems +56,078, cost
  +36,800, georef +210,702 (properties included); all seven 2,340,188
  bytes (+445,604; +142,672 under `gzip -9`). An IFC4-only build grows
  from 759,820 to 1,517,512 bytes with properties, because the property
  resolver links every release's schema table.
- Additive: a patch release.

## [0.2.1] - 2026-10-03

### Added (#244)

- `IfcModel.parseWithOptions(bytes, options)`: lenient reads
  (`onMalformed: "skip"`, `checkReferences`, `acceptRealWithoutPoint`).
- `model.header()` and `model.setHeader(header)`.
- `model.validate(maxFindings?)`: a `ValidationReport` with sorted
  findings.
- `IfcModel.parseIfcXml(bytes, profile?)` and `model.writeIfcXml(profile?)`.
- `model.unreachableProducts()`.
- TypeScript types `ParseOptions`, `IfcHeader`, `ValidationReport`,
  `ValidationFinding`, `UnreachableProduct`; `IfcErrorCode` gains `io`
  (already thrown, previously missing from the type), `unsupported-profile`
  and `feature-disabled`.
- Default features `ifcxml`, `validate` and `unreachable`; a browser build
  can leave each out, and its methods then throw `feature-disabled`. Their
  cost after `wasm-bindgen`: the default package grows from 1,337,025 to
  1,891,591 bytes; ifcXML adds 314,659, validation 161,630 and the
  reachability lint 65,809. With all three left out the package is
  1,349,493 bytes (744,356 -> 756,824 for an IFC4-only build).
- Additive: a patch release.

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

- `wasm-opt -Oz` was measured, not applied, because it increases the gzip
  and brotli size. With binaryen 132 on the default build the module went
  from 1,337,025 to 1,295,241 bytes raw (-3.1%), but from 459,437 to
  461,953 bytes under `gzip -9` and from 276,835 to 278,749 under brotli.
  The bundled schema data, which wasm-opt cannot shrink, is 908,687 of the
  1,337,025 bytes.
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

[Unreleased]: https://github.com/openbimrs/ifc/compare/openbim-ifc-wasm-v0.3.0...HEAD
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.3.0
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.2.0
[0.1.1]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.1.1
[0.1.0]: https://www.npmjs.com/package/@openbim/ifc/v/0.1.0
