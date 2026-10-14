# Changelog -- openbim-ifc-wasm

All notable changes to the `openbim-ifc-wasm` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

## [Unreleased]

### Added (#462, INVERSE attributes by name)

- `model.attributeByName(id, "IsDefinedBy")` and every other `INVERSE`
  name, resolved against the file's release: a `list` of `ref`s for a
  `SET` or `BAG` (`SET [0:1]` included), a `ref` or `null` for a
  single-valued inverse.
- `model.inverseAttributeNames(id)` (`InverseAttributeInfo[]`),
  `model.inverseReferences(id)` (`InverseReference[]`, `{ id, index }`)
  and `model.totalInverses(id)`.
- Error code `inverse-attribute` (`IfcErrorCode`) for a write to an
  `INVERSE` name. Reading one no longer throws `unknown-attribute`.

### Added (#480, type propagation on assignment)

- The `assignType` operation takes `mapRepresentations`,
  `mappingTarget`, `materialUsage`, `layerSetDirection`,
  `predefinedType` and `replace`; `contain` and `aggregate` take
  `replace` (declared in `AuthorOp`). Every new field is optional and off
  by default.

Semver: additive, a minor release while 0.x.

## [0.9.0] - 2026-10-13

### Added (#478, cascading delete and purge)

- `model.removeProduct(id, { rebasePlacements?, untypeOccurrences?,
  dryRun? })` and `model.purge({ classes?, includeRoots?,
  includeOwnerHistory?, dryRun? })`, returning a `RemovalReport`
  (`RemovedEntity`, `KeptEntity`; `RemoveProductOptions`, `PurgeOptions`
  in the declarations).

### Deprecated (#478)

- `model.remove(id)`, which leaves references dangling (`@deprecated` in
  the declarations); use `removeWithRelationships` or `removeProduct`. It
  keeps working.

Semver: additive, a minor release while 0.x.

### Added (#450)

- `model.writeIfcZip(inner?, profile?)`, and ifcZIP input to
  `IfcModel.parse`, recognized by the ZIP signature and read within a
  decompression budget (`parse`, `unsupported` or `budget-exceeded`
  otherwise). The `zip` cargo feature, on by default, carries it.

Semver: minor (additive).

### Added (#449)

- `ProductMesh.colour` (`SurfaceColour`: `red`, `green`, `blue`,
  `transparency`, `source` `"item" | "layer" | "material"`,
  `sourceEntity`, `surfaceStyle`) and `ProductMesh.colourRefusal`
  (`ColourRefusal`, code `"ambiguous"` or a geometry code) in the
  `@openbim/ifc/mesh` entry: the colour of the Body's item style, else its
  presentation layer's, else its material's styled representation.
  TypeScript declarations included.

### Added (#463, GlobalIds, types and openings)

- `model.byGuid(globalId)` (`bigint | undefined`), through an index built on
  the first lookup; a `GlobalId` several entities carry throws
  `invalid-model`, and `model.duplicateGlobalIds()` lists them
  (`DuplicateGlobalId`).
- `model.typeObject(id)`, `occurrencesOf`, `openingsOf`, `fillingsOf` and
  `voidedElementOf`, in IFC2X3, IFC4 and IFC4X3.
- `IfcModel.newGuid()`, drawn from the host's `crypto.getRandomValues`
  (browsers, Node 19 and later; `unsupported` on Node 18), and
  `IfcModel.nameBasedGuid(parts)`. The module now links getrandom's
  JavaScript backend.

Semver: additive, a minor release while 0.x.

### Changed

- `IfcModel.georeferencing()` reads IFC2X3 files through their `ePSet_MapConversion` and
  `ePSet_ProjectedCRS` property sets instead of refusing them as
  `unsupported-schema` (#451); such a record's source kind is
  `property-set`. Through `openbim-ifc-binding-core`.

Semver: minor (ADR 0021).

## [0.8.0] - 2026-10-13

### Changed (#433, wire format 1.3)

- The geometry graph payload (#367) may be Axiolid's wire format 1.3
  (`axiolid-model` 0.3.12), a minor version of the format (ADR 0021,
  amended): an `IfcOffsetCurveByDistances` along an offset whose length is
  a quadrature now lowers, continued to the basis's ends the kernel
  measures. An envelope carries the lowest version its content needs, so
  every other payload is unchanged: 1.0, 1.1 or 1.2. A reader on
  `axiolid-model` 0.3.11 or older refuses 1.3.
  `GeometryGraphEnvelope.version` documents the rule.

Semver: minor, as ADR 0021 states for a minor version of the wire format.

## [0.7.0] - 2026-10-12

### Changed (#429, wire format 1.2)

- The geometry graph payload (#367) may be Axiolid's wire format 1.2
  (`axiolid-model` 0.3.11), a minor version of the format (ADR 0021,
  amended): a sectioned solid or surface along an offset whose length is a
  quadrature now lowers, its sections carrying seam-snapping windows.
  An envelope carries the lowest version its content needs, so every other
  payload is unchanged: 1.0, or 1.1 when a station carries a window. A
  reader on `axiolid-model` 0.3.10 or older refuses 1.2. `GeometryGraphEnvelope.version` documents the rule.

Semver: minor, as ADR 0021 states for a minor version of the wire format.

## [0.6.1] - 2026-10-12

### Fixed (axiolid/kernel#297)

- Window-free payloads are labelled 1.0 again, readable by `axiolid-model`
  0.3.8: built with `axiolid-model` 0.3.10, the geometry graph payload's
  envelope carries the lowest wire format version its content needs, 1.0
  unless a station carries a seam-snapping window, then 1.1. `GeometryGraphEnvelope.version` documents the rule.

Semver: patch.

## [0.6.0] - 2026-10-10

### Changed (#423, wire format 1.1)

- The geometry graph payload (#367) is Axiolid's wire format 1.1
  (`axiolid-model` 0.3.9), a minor version of the format (ADR 0021,
  amended): a station on an offset whose length is a quadrature now lowers
  with a seam-snapping window. An envelope carries the lowest wire version
  its content needs: 1.0, or 1.1 when a station carries a seam-snapping
  window; `axiolid-model` 0.3.9 labels every payload 1.1
  (axiolid/kernel#297), and a reader on `axiolid-model` 0.3.8 or older
  refuses 1.1. `GeometryGraphEnvelope.version` documents both.

Semver: minor, as ADR 0021 states for a minor version of the wire format.

## [0.5.0] - 2026-10-10

### Added (#367, geometry Level 2, ADR 0021)

- `IfcModel.productGeometry(ids?, encoding?)` -> `ProductGeometry[]`, each
  product's Body as Axiolid's geometry graph in its wire format 1.0:
  `payload` the JSON text (`"json"`, the default), the parsed
  `GeometryGraphEnvelope` (`"object"`) or the CBOR bytes as a `Uint8Array`
  (`"cbor"`); a typed `refusal` per product. TypeScript types
  `ProductGeometry`, `GeometryGraphEnvelope`, `GeometryGraphNode`,
  `GeometryPayloadEncoding`. Cargo feature `graph`, opt-in: +1.06 MB
  (+341 KB gzip) on the default module, +286 KB (+88 KB gzip) beside
  `mesh`.
- The npm package's mesh entry (`@openbim/ifc/mesh`) is built with
  `mesh,graph` and carries `productGeometry`; the default entry throws
  `feature-disabled` for it.

Semver: additive, a minor release while 0.x (the npm package gains an
API).

## [0.4.3] - 2026-10-08

### Added (#358, #342)

- `IfcModel.propertySetsMany(ids?)`: one `ObjectPropertySets` (object,
  sets, refusal) per id, or per object definition, in one pass; each
  exactly what `propertySets` answers.
- `setAttributeByName` also takes a plain value (`IfcPlainValue`: a
  string, number, bigint, boolean, `null` or array), coerced against the
  attribute's declared type; an `IfcValue` is written exactly as before.
  `IfcErrorCode` gains `type-mismatch` and `ambiguous-value`.

Semver: additive (a plain value used to be refused with `invalid-value`),
a patch release.

## [0.4.2] - 2026-10-08

### Added (#369, the npm mesh entry)

- `@openbim/ifc/mesh`: a second entry of the npm package, the default
  features plus `mesh`, so `productMeshes` works from npm. It mirrors the
  default entry's conditional exports: `@openbim/ifc/mesh` (Node's
  CommonJS build under the `node` condition, the bundler build otherwise),
  `@openbim/ifc/mesh/bundler` and `@openbim/ifc/mesh/web`, in `mesh/`,
  `mesh/bundler/` and `mesh/web/`. It shares the package's PSD/QTO catalog
  files and loader. The default entry is unchanged and still throws
  `feature-disabled` for `productMeshes`.
- `scripts/build-npm-pkg.sh` builds and binds both modules, so the release
  workflow publishes both entries from the same tag in one `npm publish`.
  `tools/check-package.mjs` checks the mesh entry from the packed tarball
  in Node (`require`, `import`, `web`), a webpack bundle and headless
  Chrome, with a `productMeshes` call returning a wall's positions and
  indices, and prints the tarball's sizes.
- `examples/viewer/` imports `@openbim/ifc/mesh/web` through an import map,
  which points at its `build.sh` output in this repository or at an
  installed package.

### Removed

- `scripts/check-mesh.sh`: the mesh entry's Node suite now runs in
  `scripts/build-npm-pkg.sh`.

The mesh module is 5,101,651 bytes (1,753,297 under `gzip -9`) against the
default's 2,940,278; the tarball grows from 3.9 MB to 9.2 MB (11.5 MB to
27.1 MB unpacked). An application that does not import the entry loads
none of it. Semver: additive (a new export path; no existing path, file or
API changes), a patch release.

### Added (#328, geometry, ADR 0021)

- `IfcModel.productPlacements(ids?)` (feature `placements`, default, so in
  the npm package): `ProductPlacement` objects with `transform` (a
  column-major 4x4 in metres, `Matrix4.fromArray`-ready), the selected
  Body `representation` and a typed `refusal` per product. `ids` is a
  `bigint[]` or `BigUint64Array`.
- `IfcModel.productMeshes(ids?)` (feature `mesh`, opt-in; in the npm
  package only from the mesh entry of #369, the default entry throws
  `feature-disabled`): `ProductMesh` objects with
  `positions` (`Float32Array`, relative to `transform`) and `indices`
  (`Uint32Array`). TypeScript declarations for both.
- `examples/viewer/`: a WebGL2 page that draws a file's meshes with no
  build step beyond the module (`build.sh --serve`);
  `tests/js/geometry.mjs` runs against the mesh module.

Placements add 42 KB to the default module (15 KB gzip). Semver:
additive, a patch release.

## [0.4.1] - 2026-10-04

### Added (#330, schema-checked entity creation)

- `IfcModel.author(ops)`: `AuthorOp` objects (`{ op: "product", type,
  container, placement, typeObject, attributes, ... }`) applied as one
  checked transaction, returning `AuthoringResult` (`ids` per operation,
  `created`, `removed`); `IfcModel.handle(index)` names the entity an
  earlier operation produced. `createEntity(typeName, attributes)` and
  `removeWithRelationships(id)` are one-operation batches. Fresh
  `GlobalId`s derive from a seed drawn from `Math.random`. Feature
  `author` (default). `IfcErrorCode` gains `missing-attribute` and
  `still-referenced`. Additive: a patch release.

## [0.4.0] - 2026-10-04

### Added (#326, attributes by name)

- `IfcModel.attributeNames(id)` (`AttributeInfo[]` in slot order),
  `attributeByName(id, name)` and `setAttributeByName(id, name, value)`,
  resolved against the release the header declares; names match
  case-insensitively. `IfcErrorCode` gains `unknown-attribute` and
  `derived-attribute`. Additive; this release is breaking for #318
  anyway.

### Changed (breaking, #318: the catalog is loaded lazily)

- The module no longer embeds the PSD/QTO catalog: default feature
  `property-catalog-runtime` replaces `property-catalog`. The default
  module is 2,648,193 bytes after `wasm-bindgen` (894,392 under
  `gzip -9`), was 6,282,871 (1,855,125).
- The package ships the catalog as `catalog/ifc2x3-tc1.bin` (325,739
  bytes), `catalog/ifc4-add2-tc1.bin` (1,015,315) and
  `catalog/ifc4x3-add2.bin` (1,086,526), with the loader `catalog.mjs`.
- `await IfcModel.loadCatalog(release?, { bytes?, baseUrl? })` loads one
  release's edition (all three without a release): Node reads it from the
  package, a browser or bundle fetches it relative to the module. Each is
  checked against its pinned SHA-256 and kept for the module instance.
  `IfcModel.catalogFile`, `catalogLoaded` and `loadCatalogBytes` are the
  synchronous parts.
- Until a release's catalog is loaded, a write to a `Pset_`/`Qto_` set
  throws the new code `catalog-not-loaded`, never unchecked. A build with
  `property-catalog` embeds the catalog (1.4 MB) and `loadCatalog` is a
  no-op.
- A write to a `Pset_`/`Qto_` set that 0.3.0 checked without a load now
  needs `loadCatalog` first: a breaking change, a minor release under 0.x
  (0.4.0).
- TypeScript: `IfcModel.loadCatalog` (a namespace merged with the class),
  `CatalogLoadOptions`; `IfcErrorCode` gains `template-violation`,
  `missing-property` and `catalog-not-loaded`.

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

[Unreleased]: https://github.com/openbimrs/ifc/compare/openbim-ifc-wasm-v0.9.0...HEAD
[0.9.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.9.0
[0.8.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.8.0
[0.7.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.7.0
[0.6.1]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.6.1
[0.6.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.6.0
[0.5.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.5.0
[0.4.3]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.4.3
[0.4.2]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.4.2
[0.4.1]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.4.1
[0.4.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.4.0
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.3.0
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.2.0
[0.1.1]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-wasm-v0.1.1
[0.1.0]: https://www.npmjs.com/package/@openbim/ifc/v/0.1.0
