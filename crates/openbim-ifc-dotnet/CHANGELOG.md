# Changelog -- openbim-ifc-dotnet

All notable changes to the `openbim-ifc-dotnet` crate, which ships as the
`OpenBim.Ifc` NuGet package, are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

## [Unreleased]

### Added (#469)

- `Units.Convert(value, from, to)` converts between units named as text
  (an SI unit with its prefix, or a named IFC4 Table 673 unit with exact
  factors); `Units.FormatLength` writes a length as decimal text or feet
  and inches with fractions (`LengthStyle`); `IfcModel.ConvertUnitValue`
  converts between two unit entities of a model.
- `IfcModel.ConvertLengthUnit(target, areaAndVolume, removeReplacedUnits,
  dryRun)` rescales every length, area and volume value to a new project
  length unit as one checked transaction and returns a `LengthUnitReport`
  (`ConvertedValues`, `KeptValues`). Values it cannot carry, and offset
  units (#110), throw `unsupported`. Needs the native library with C ABI
  0.1.18.

Semver: minor.

## [0.9.1] - 2026-10-18

### Changed (#506, #509)

- `ProductCurves` returns polylines for trimmed, composite, offset and intrinsic
  curves (an `IfcTrimmedCurve` axis, an `IfcCompositeCurve` footprint, an
  IFC4X3 clothoid), which were `unsupported` refusals. A curve the
  reference compiler has no certified polyline for (an implicit curve, an
  offset of an offset) is still `unsupported`, its refusal now naming the
  product rather than the curve. Advanced B-reps whose edges cover part of
  their line, circle or ellipse mesh (#509). No API change.

Semver: patch.

## [0.9.0] - 2026-10-16

### Changed (#515, wire format 1.4)

- The geometry graph payload (#367) may be Axiolid's wire format 1.4
  (`axiolid-model` 0.3.14), a minor version of the format (ADR 0021,
  amended): every `IfcPolygonalBoundedHalfSpace` now lowers to
  `BoundedHalfSpaceAlongZ`, the half space bounded along its `Position`'s
  Z (axiolid/kernel#321), so an oblique clip such as a gable wall's under
  a sloped roof is carried as authored. An envelope carries the lowest
  version its content needs: a payload with such a clip is 1.4 (it was
  1.0), every other is unchanged. A reader on `axiolid-model` 0.3.13 or
  older refuses 1.4.
  `ProductGraph.FormatVersion` is `"1.4"`, the newest version the package
  writes.

Semver: minor, as ADR 0021 states for a minor version of the wire format.

## [0.8.0] - 2026-10-14

### Added (#458, curves and surfaces as output primitives)

Semver: minor (additive).

- `model.ProductCurves(ids, representation = "Plan")`: a `CurvedProduct`
  per product, its `ProductCurves` record (with a `ProductPolyline` per
  curve: `Curve`, `FirstVertex`, `VertexCount`, `Closed`, `ChordBound`,
  `ChordRequested`) and `float[] Positions`. A trimmed, composite, offset
  or intrinsic curve is an `unsupported` refusal (#506).
- `model.ProductMeshes(ids, representation)` meshes a named
  representation, such as `"Surface"`; `ProductMesh.Closure` is `solid`,
  `surface` or `unknown`.
- Requires the C ABI 0.1.17 (`openbim-ifc-capi` with #458).


## [0.7.0] - 2026-10-14

### Added (#462, INVERSE attributes by name)

- `model.AttributeByName(id, "IsDefinedBy")` and every other `INVERSE`
  name: a `Value.List` of `Value.Ref` for a `SET` or `BAG` (`SET [0:1]`
  included), a `Value.Ref` or `Value.Null` for a single-valued inverse.
- `model.InverseAttributeNames(id)` (`InverseAttributeInfo`),
  `model.InverseReferences(id)` (`InverseReference(Id, Index)`) and
  `model.TotalInverses(id)`.
- `IfcStatus.InverseAttribute` (35, `inverse-attribute`) for a write to
  an `INVERSE` name.
- Requires the C ABI 0.1.16 (`openbim-ifc-capi` with #462).

### Added (#481, shape representations)

- `AuthorOp.ShapeRepresentation(context, items, identifier, representationType)`,
  `AuthorOp.AssignRepresentation(representation, to)`,
  `AuthorOp.UnassignRepresentation(representation, from)`,
  `AuthorOp.CopyRepresentation(representation)` and
  `AuthorOp.RemoveRepresentation(representation)` for `IfcModel.Author`.
- `IfcModel.RemoveRepresentation(id, dryRun)`, returning a
  `RemovalReport`.
- Requires the C ABI 0.1.15 (`openbim-ifc-capi` with #481).

### Added (#464, selectors and values by path)

- `model.Select(query)`, `model.Select(query, baseIds)`,
  `model.GetValue(id, path)` (a property set as a `Value.List` of name and
  value pairs) and `model.SetValue(id, path, value)`;
  `IfcModel.SelectorGrammarVersion`.
- `IfcStatus.InvalidSelector` and the code `invalid-selector`, with
  `IfcException.Span`, the byte range an invalid selector refuses.
- Requires the C ABI 0.1.14 (`openbim-ifc-capi` with #464).

Semver: additive, a minor release while 0.x.

### Added (#480, type propagation on assignment)

- `AuthorOp.AssignType(typeObject, objects, ownerHistory,
  mapRepresentations, mappingTarget, materialUsage, layerSetDirection,
  predefinedType, replace)` with the enums `PredefinedTypeMode` and
  `LayerAxis`, and a `replace` parameter on `AuthorOp.Contain` and
  `AuthorOp.Aggregate`. Every new parameter is optional and off by
  default.
- Requires the C ABI 0.1.13 (`openbim-ifc-capi` with #480).

Semver: additive (new optional parameters; source-compatible), a minor
release while 0.x.

## [0.6.0] - 2026-10-13

### Added (#478, cascading delete and purge)

- `model.RemoveProduct(id, rebasePlacements, untypeOccurrences, dryRun)`
  and `model.Purge(classes, includeRoots, includeOwnerHistory, dryRun)`,
  returning a `RemovalReport` (`RemovedEntity`, `KeptEntity`).
- Requires the C ABI 0.1.12 (`openbim-ifc-capi` with #478).

### Deprecated (#478)

- `model.Remove(id)` is `[Obsolete]`: it leaves references dangling. Use
  `RemoveWithRelationships` or `RemoveProduct`. It keeps working; a caller
  building with warnings as errors sees CS0618.

Semver: additive, a minor release while 0.x.

### Added (#450)

- `IfcModel.WriteIfcZip(inner = "step", xsdProfile = null)`, and ifcZIP
  input to `IfcModel.Parse`, recognized by the ZIP signature and read
  within a decompression budget (`parse`, `unsupported` or
  `budget-exceeded` otherwise).

Semver: minor (additive).

### Added (#449)

- `ProductMesh` gains `Colour` (`SurfaceColour`: `Red`, `Green`, `Blue`,
  `Transparency`, `Source` `item`/`layer`/`material`, `SourceEntity`,
  `SurfaceStyle`) and `ColourRefusal` (a `GeometryRefusal`, code
  `ambiguous` or a geometry code) as its last positional parameters, in a
  native library built with `mesh`: the colour of the Body's item style,
  else its presentation layer's, else its material's styled
  representation.

Semver: additive, a minor release while 0.x (a caller that constructs
`ProductMesh` itself passes the two new parameters).

### Added (#463, GlobalIds, types and openings)

- `model.ByGuid(globalId)` (`ulong?`), through an index built on the first
  lookup; a `GlobalId` several entities carry throws `invalid-model`, and
  `model.DuplicateGlobalIds()` lists them (`DuplicateGlobalId`).
- `model.TypeObject(id)`, `OccurrencesOf`, `OpeningsOf`, `FillingsOf` and
  `VoidedElementOf`, in IFC2X3, IFC4 and IFC4X3.
- `IfcModel.NewGuid()` and `IfcModel.NameBasedGuid(params string[])`.
- Requires the C ABI 0.1.10 (`openbim-ifc-capi` with #463).

Semver: additive, a minor release while 0.x.

### Changed

- `IfcModel.Georeferencing()` reads IFC2X3 files through their `ePSet_MapConversion` and
  `ePSet_ProjectedCRS` property sets instead of refusing them as
  `unsupported-schema` (#451); such a record's source kind is
  `property-set`. Through `openbim-ifc-binding-core`.

Semver: minor (ADR 0021).

## [0.5.0] - 2026-10-13

### Changed (#433, wire format 1.3)

- The geometry graph payload (#367) may be Axiolid's wire format 1.3
  (`axiolid-model` 0.3.12), a minor version of the format (ADR 0021,
  amended): an `IfcOffsetCurveByDistances` along an offset whose length is
  a quadrature now lowers, continued to the basis's ends the kernel
  measures. An envelope carries the lowest version its content needs, so
  every other payload is unchanged: 1.0, 1.1 or 1.2. A reader on
  `axiolid-model` 0.3.11 or older refuses 1.3.
  `ProductGraph.FormatVersion` is `"1.3"`, the newest version the package
  writes.

Semver: minor, as ADR 0021 states for a minor version of the wire format.

## [0.4.0] - 2026-10-12

### Changed (#429, wire format 1.2)

- The geometry graph payload (#367) may be Axiolid's wire format 1.2
  (`axiolid-model` 0.3.11), a minor version of the format (ADR 0021,
  amended): a sectioned solid or surface along an offset whose length is a
  quadrature now lowers, its sections carrying seam-snapping windows.
  An envelope carries the lowest version its content needs, so every other
  payload is unchanged: 1.0, or 1.1 when a station carries a window. A
  reader on `axiolid-model` 0.3.10 or older refuses 1.2. `ProductGraph.FormatVersion` is `"1.2"`, the newest version the package writes.

Semver: minor, as ADR 0021 states for a minor version of the wire format.

## [0.3.1] - 2026-10-12

### Fixed (axiolid/kernel#297)

- Window-free payloads are labelled 1.0 again, readable by `axiolid-model`
  0.3.8: built with `axiolid-model` 0.3.10, the geometry graph payload's
  envelope carries the lowest wire format version its content needs, 1.0
  unless a station carries a seam-snapping window, then 1.1. `ProductGraph.FormatVersion` stays `"1.1"`, the newest version the package writes.

Semver: patch.

## [0.3.0] - 2026-10-10

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
