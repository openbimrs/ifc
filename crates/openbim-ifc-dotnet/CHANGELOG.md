# Changelog -- openbim-ifc-dotnet

All notable changes to the `openbim-ifc-dotnet` crate, which ships as the
`OpenBim.Ifc` NuGet package, are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

## [Unreleased]

### Changed (#423, wire format 1.1)

- The geometry graph payload (#367) is Axiolid's wire format 1.1
  (`axiolid-model` 0.3.9), a minor version of the format (ADR 0021,
  amended): a station on an offset whose length is a quadrature now lowers
  with a seam-snapping window. An envelope carries the lowest wire version
  its content needs: 1.0, or 1.1 when a station carries a seam-snapping
  window; `axiolid-model` 0.3.9 labels every payload 1.1
  (axiolid/kernel#297), and a reader on `axiolid-model` 0.3.8 or older
  refuses 1.1. `ProductGraph.FormatVersion` is `"1.1"`, the newest version
  the package writes.

Semver: minor, as ADR 0021 states for a minor version of the wire format.

## [0.2.0] - 2026-10-10

### Added (#367, geometry Level 2, ADR 0021)

- `IfcModel.ProductGeometry(ids, encoding)` -> `ProductGraph` per product:
  its `ProductGeometry` record (transform, encoding, payload size,
  refusal) and `byte[] Payload`, Axiolid's geometry graph in its wire
  format 1.0 as UTF-8 JSON (`GeometryEncoding.Json`, also `string Json`)
  or CBOR (`GeometryEncoding.Cbor`); `ProductGraph.Format` and
  `FormatVersion`. Needs a native library built with the C ABI's `graph`
  feature (ABI 0.1.9); the packaged one throws `IfcException` with code
  `feature-disabled`.

Semver: additive, a minor release while 0.x.

## [0.1.3] - 2026-10-08

### Added (#358, #342)

- `IfcModel.PropertySetsMany(ids)`: `ObjectPropertySets` (object, sets,
  `PropertyRefusal`) per id, or per object definition when null, resolved
  in one pass; each exactly what `PropertySets` answers.
- `IfcModel.SetAttributeByNamePlain(id, name, value)`: a plain .NET value
  (`string`, integers, `double`, `bool`, a sequence, `EntityHandle`,
  `null`, or a `Value` written exactly) coerced against the attribute's
  declared type; `IfcStatus.TypeMismatch` (32) and `AmbiguousValue` (33).
  Needs the C ABI 0.1.8.

Semver: additive, a patch release.

## [0.1.2] - 2026-10-08

### Added (#328, geometry, ADR 0021)

- `IfcModel.ProductPlacements(ids)`: `ProductPlacement` records with the
  column-major `Transform`, the selected Body `Representation`
  (`SelectedRepresentation`) and a typed `Refusal` (`GeometryRefusal`)
  per product.
- `IfcModel.ProductMeshes(ids)`: a `MeshedProduct` per product, its
  `ProductMesh` record with `float[] Positions` and `uint[] Indices`;
  needs a native library built with the C ABI's `mesh` feature, and the
  packaged one throws `feature-disabled`. Declarations for the C ABI
  0.1.7 exports.

Semver: additive, a patch release.

## [0.1.1] - 2026-10-04

### Added (#330, schema-checked entity creation)

- `IfcModel.Author(ops)`: `AuthorOp`s built with `AuthorOp.Create`,
  `Edit`, `Remove`, `Project`, `Spatial`, `Product`, `TypeObject`,
  `AssignType`, `Contain`, `Aggregate`, `Placement` and `OwnerHistory`,
  applied as one checked transaction; returns per operation the id it
  produced (null for a removal). `IfcModel.Handle(index)` /
  `HandleBase` name the entity an earlier operation produced.
  `CreateEntity(type, attributes)` and `RemoveWithRelationships(id)` are
  one-operation batches. `IfcStatus.MissingAttribute` (30) and
  `StillReferenced` (31). Needs the C ABI 0.1.6. Additive: a patch
  release.

## [0.1.0] - 2026-10-04

### Added (#327)

- The `OpenBim.Ifc` NuGet package: C# over the versioned C ABI of
  `openbim-ifc-capi` (`openbim_ifc_v0_1_*`, ABI 0.1.5 or later), for
  `net8.0` and `netstandard2.0` (.NET Framework 4.6.2 and later), with the
  native library for `win-x64`, `win-arm64`, `linux-x64`, `linux-arm64`,
  `osx-x64` and `osx-arm64`. Build targets copy the Windows libraries into
  a .NET Framework project's output, and the assembly loads them from next
  to itself.
- `IfcModel`, an `IDisposable` over a `SafeHandle`: `Parse`, `Open`
  (owned or memory-mapped), `ParseIfcXml`, `Write`, `WriteIfcXml`,
  `Count`, `Schema`, `Diagnostics`, `Ids`, `IdsOfType`,
  `IdsOfTypeIncludingSubtypes`, `TypeOf`, `Attributes`, `Attribute`,
  `SetAttribute`, `Add`, `Remove` and `DanglingReferences`.
- Attributes by name (#326): `AttributeNames` (`AttributeInfo` records),
  `AttributeByName` and `SetAttributeByName`, with the codes
  `unknown-attribute` and `derived-attribute`.
- The #244 surface: `ParseOptions` (`Lenient`), the `Header` record, read
  and replaced; `Validate` with a `ValidationReport`; ifcXML in the native
  and XSD layouts; `UnreachableProducts`.
- The #123 domain views as C# records: `PropertySets`, `ResolveUnit`,
  `SpatialTree`, `Classifications`, `Material`, `Systems`, `Cost` and
  `Georeferencing`; and writing property sets as one checked transaction
  (`SetProperties` with `PropertyEdit`s, `SetProperty`, `RemoveProperty`),
  checked against the PSD/QTO catalog the native library embeds.
- `Value`, a closed record hierarchy keeping `$` and `*`, `.U.` and
  `.F.`, integer and real, and typed wrappers distinct; `EquatableList<T>`
  so records compare by value.
- `IfcException`, carrying the stable error `Code` every binding shares and
  the C ABI's `IfcStatus`.
- `IfcLibrary.Version` and `IfcLibrary.LiveModels`.
