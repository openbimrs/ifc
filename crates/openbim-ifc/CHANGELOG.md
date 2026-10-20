# Changelog -- openbim-ifc

All notable changes to the `openbim-ifc` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

## [0.37.0] - 2026-10-20

### Added (#479, copy and append)

- `AuthorOp::CopyProduct { product, options }` copies an `IfcProduct` or
  `IfcTypeObject` with what `RemoveProduct` would remove with it, by the
  #478 cascade's own reference count, and shares what it would keep. Copied,
  with new `GlobalId`s: the product; its `ObjectPlacement` (relative to the
  same parent; `CopyPlacement::Offset` moves it in the parent's frame,
  `CopyPlacement::At` replaces it); its representation by
  `CopyRepresentation`'s rules (a type's representation maps with their
  origins); the property and quantity sets only it is defined by; its
  material usage; its ports; and, by `CopyProductOptions::openings` and
  `fillings` (both on), its openings and their fillings, placed relative to
  the copied placements. Shared, the copy joining the original's
  relationship: its type assignment, shared property sets, classification,
  document and library references, groups, layers and, by `container` and
  `aggregate` (both on), its spatial container and its whole.
  Relationships that would bind another object (connections, space
  boundaries, parts, a type's occurrences) are skipped and reported.
  Entities a `UNIQUE` rule binds (`IfcPropertyEnumeration`,
  `IfcApplication`) are always shared.
- `AuthorOp::CopyEntity { entity, policy }`: a deep copy along attributes by
  an explicit `CopyPolicy` (classes always copied, always shared; other
  `IfcRoot`s and definitions shared unless asked).
- `CopyReport` (copied, reused, joined, skipped, regenerated, each original
  with its copy) in `AuthoringOutcome::copies` and `StagedAuthoring::copies`.
- `apply_append` and `stage_append` copy entities of another model with
  their closure: what they reference, their property sets, type, material,
  classification, openings, fillings and ports, and what attaches to their
  geometry; relationships to objects not copied are skipped. Equal units,
  contexts, owner history, materials and styles are reused
  (`ReuseOptions`); `GlobalIdMode` keeps, merges by identity (joining the
  target's relationships) or regenerates `GlobalId`s. Attributes move by
  name between releases.
- `AuthoringFailure::UnsupportedCopy`, `IncompatibleRelease` (a type,
  attribute or enumeration constant the target release lacks; never
  dropped) and `UnitMismatch` (differing length or plane angle units; the
  message points at unit conversion, #469).

### Changed (#479)

- The operation options moved from `authoring/op.rs` into their own module
  under the 800-line cap; every public path is unchanged.

Semver: additive (`AuthorOp`, `AuthoringFailure`, `AuthoringOutcome` and
`StagedAuthoring` are `#[non_exhaustive]`): minor.

### Added (#461, #511)

- Feature `validate-express`: `ifc::validate::express`, the EXPRESS rule
  evaluator of `ifc-validate` (every WHERE rule, type rule and global rule
  of a release, from the `.exp` text the caller supplies). In `full`.
- `ifc::validate` is a module re-exporting `ifc-validate` whole. With
  `property-catalog` or `property-catalog-runtime`, its `validate`,
  `validate_with`, `validate_declared` and `validate_with_options` check
  `Pset_`/`Qto_` set names against the declared release's catalog
  (`ReleaseCatalog`): a set the catalog does not define is a
  `property.reserved_set_name` warning. Without a catalog the names are
  reported once as unsupported.

Semver: minor.

### Added (#469)

- `convert_length_unit` and `stage_length_unit_conversion`: rescale every
  length-dimensioned value of a model to a new project length unit (an SI
  `METRE` with any prefix, or `inch`, `foot`, `yard`, `mile`, `US survey
  foot`) as one checked transaction. Which values are lengths, areas and
  volumes comes from the declared release's tables (IFC2X3, IFC4, IFC4X3),
  through every attribute, aggregate and typed `SELECT` value: geometry,
  placements, profiles, quantities and property values. Values with a unit
  of their own keep it. Every `REAL` and `IfcParameterValue` attribute of
  the three releases is reviewed (a test fails on an unreviewed one):
  `IfcGeometricRepresentationContext.Precision` rescales, light
  attenuations rescale inversely, parameter values follow their curve or
  surface (kept on lines, conics, polylines, B-splines, clothoids and
  composite curves of those; rescaled on planes and along a cylinder's
  axis). Georeferencing keeps map coordinates in a `MapUnit` and rescales
  `Scale`; IFC2X3 `ePSet_MapConversion` likewise. Areas and volumes follow
  the length unit by default (`LengthUnitOptions`). Refused, each by
  name, with nothing written (`LengthUnitError::Refused`): parameter
  values on curves whose parameter space IFC leaves undefined (other
  spirals, `IfcCurveSegment`, `IfcOffsetCurveByDistances`), p-curves on
  curved surfaces, `IfcPolynomialCurve` coefficients, texture generator
  parameters, untyped numbers in a `SELECT`, contexts with their own
  units. The `LengthUnitReport` lists what was rescaled and kept.
  Verified over the fixture corpus: a millimetre-metre round trip restores
  every value, compiled geometry agrees in metres after converting to
  feet, and validation findings are unchanged.

Semver: minor (additions).

### Added (#472, #474, values by path as text and tables)

- `ifc::selector::value_text` writes a `SelectedValue` as the text of one
  cell (`TRUE`, the shortest decimal of a real, `#12`, a null text), and
  `SelectorIndex::plan_text_edits` plans many `TextEdit`s (entity, path,
  text) as one transaction: each text read as a value of its target's
  declared type (attributes) or stated type (properties), `IFCLABEL(F90)`
  naming one where nothing states it; a value already held left alone;
  every refusal collected as an `EditRefusal` naming its edit. An
  `EditPlan` lists each `PlannedChange` (target, before, after) and
  commits as one transaction, refused if the model changed since.
  `apply_text_edits` plans and commits.
- `SelectorError::InvalidText` and `SelectorError::ConflictingWrite`
  (the enum is `#[non_exhaustive]`).
- `ifc::sheet` (feature `sheet`, implies `selector`): `export` writes
  entities by value paths as a `Sheet` keyed by `GlobalId` or `id`
  (`KeyColumn`, `SheetOptions` with a null text); `plan_import` reads a
  sheet back by key into an `ImportPlan` of `CellChange`s, or every
  `SheetRefusal` by row and column (header spans, missing, empty,
  unknown, ambiguous and repeated keys, ragged rows, per-cell write
  refusals); `import` commits it as one transaction. An unchanged sheet
  plans no change and commits nothing. `Sheet::read` and `Sheet::write`
  take CSV (RFC 4180) and TSV with escapes; XLSX with feature
  `sheet-xlsx`, through `rust_xlsxwriter` 0.100 and `calamine` 0.37 (pure
  Rust, on the `zip` 9 and `quick-xml` 0.42 already in the tree, no
  default features), every cell written as text. `full` includes
  `sheet-xlsx`.

Semver: minor (additive).

### Changed (#512)

- `product_curves` reads a representation that also holds text or a fill
  area. A dimension `IfcAnnotation` (its lines as `IfcPolyline`s, its
  value as an `IfcTextLiteralWithExtent`) was refused as a whole; its
  lines now come back as polylines, and each `IfcTextLiteral`,
  `IfcTextLiteralWithExtent` and `IfcAnnotationFillArea` of the
  representation is listed in the new `ProductCurves::skipped` (a
  `SkippedItem`: the item and its `NonCurveItem` kind), not drawn and not
  a refusal. Read those items with `ifc_style::StyleView`. Any other item
  that cannot be lowered still refuses the product.
- `SkippedItem` and `NonCurveItem` are re-exported from `ifc-geometry`.

Semver: minor. `ProductCurves` is `#[non_exhaustive]` and gains a field;
products that were refused now return their curves.

### Changed (#467)

- `validate::validate_declared` refuses a file declaring `IFC4X3_TC1` or
  `IFC4X3_ADD1` as `ValidateError::UnpublishedSchema`, as
  `ifc_validate::validate_declared` does: a release buildingSMART
  publishes no EXPRESS for, never validated as IFC4X3 ADD2.

Semver: minor, following `ifc-validate`.

## [0.36.0] - 2026-10-18

### Changed (#506, #509)

- Requires `ifc-geometry` with #506 and #509 (`axiolid-mesh-compile`
  0.3.28). `product_curves` returns certified polylines for trimmed,
  composite, offset and intrinsic curves; a curve the reference compiler
  has no certified polyline for is refused as
  `GeometryError::CompilationRefused` with its reason, naming the product.
  `product_meshes` compiles advanced B-reps whose edges cover part of
  their line, circle or ellipse, with either `SameSense`.
- `geometry::GeometryError::PolylineUnsupported` and
  `geometry::compile::UnpolylinedCurve`, re-exported from `ifc-geometry`,
  are gone.

Semver: minor, following the dependency.

## [0.35.0] - 2026-10-16

### Changed (#515, #519)

- Requires `ifc-geometry` with #515: every `IfcPolygonalBoundedHalfSpace`
  lowers to Axiolid's `BoundedHalfSpaceAlongZ`, its prism along
  `Position`'s Z (axiolid/kernel#321). Oblique clips, such as gable walls
  under sloped roofs, mesh and compile exactly, non-convex and arc
  boundaries included; `product_meshes` cuts small clips over tall
  subjects to their full depth (axiolid/kernel#325, #519).
  `product_graphs` labels a payload with such a clip wire format 1.4
  (`axiolid-model` 0.3.14); every other payload keeps its version and
  bytes.

Semver: minor, following the dependency.

## [0.34.0] - 2026-10-14

### Added (#458, curves and surfaces as output primitives)

Semver: minor (additive; `ProductMesh` is `#[non_exhaustive]`).

- `product_curves(model, products, purpose)` (feature `mesh`): each
  product's curves for a `RepresentationPurpose` as `ProductCurves`
  (world placement, one `ProductPolyline` per curve: the IFC `curve`,
  `f32` positions relative to the placement, `closed`, `chord_bound` and
  `chord_requested`). `Plan` takes the drawing curves (`Plan`, then
  `Annotation`, `FootPrint`, `Axis`), and `Axis`, `FootPrint`,
  `Annotation` one identifier. A trimmed, composite, offset or intrinsic
  curve refuses the product with `GeometryError::PolylineUnsupported`
  until the reference compiler polylines it (#506).
- `product_meshes_for(model, products, purpose)`: the surfaces and solids
  of another representation, such as `Surface`, as one mesh; its curves
  are left to `product_curves`. `product_meshes` is it with `Body`.
- `ProductMesh::closure`: `Solid`, `Surface` (an area, no volume) or
  `Unknown` for an empty mesh. Only a Body mesh resolves a colour; a mesh
  of another representation carries `Ok(None)`.
- `RepresentationPurpose` and `select_product_representation` are
  re-exported at the root.

### Changed (#458)

- A Body whose items mix curves with solids or surfaces now meshes those
  and leaves the curves to `product_curves`; before, the reference backend
  refused the whole Body. A Body with no curves meshes exactly as before.


## [0.33.0] - 2026-10-14

### Changed

- Requires `ifc-author` 0.3.2, whose builder accepts nested aggregates
  that the #481 representation ops write.

### Added (#462, INVERSE attributes by name)

- `attribute_value_by_name(&model, schema, &reverse, id, name)` reads an
  explicit attribute (`AttributeValue::Explicit`) or an `INVERSE` one
  (`AttributeValue::Inverse`): every referrer whose type is the `FOR`
  entity or a subtype in the declared release and whose `FOR` attribute
  (the slot that release gives it) holds the entity, directly or as an
  aggregate member, read from a `ReverseIndex` the caller builds once.
  `InverseValue::Aggregate(ids)` for a `SET` or `BAG`, `SET [0:1]`
  included (zero or one id, never unwrapped), in ascending id order;
  `InverseValue::Single(Option<EntityId>)` for a non-aggregate inverse
  (`VoidsElements`, IFC2X3's `IfcGroup.IsGroupedBy`), refused with
  `NamedAttributeError::InverseCardinality` naming every referrer when
  more than one references it. Both enums are `#[non_exhaustive]`.
- `inverse_slots(schema, type_name)` and `inverse_slot(..)`: every
  `INVERSE` attribute of a type, inherited first, as `InverseSlot` (name,
  `FOR` entity and attribute, aggregation with `lower()`/`upper()`,
  declaring entity); a subtype's redeclaration replaces the inherited one.
  The bundled tables' answers equal the normative EXPRESS's for every
  entity of IFC2X3, IFC4 and IFC4X3 (a spec-backed test).
- `total_inverses(&reverse, id)`: the distinct entities referencing one
  (IfcOpenShell's `get_total_inverses`). `ReverseIndex` and `Referrer`
  are re-exported at the root; `ReverseIndex::referrers` is the
  `get_inverse(.., with_attribute_indices=True)` equivalent.

### Changed (#462)

- `attribute_slot`, `attribute_by_name`, `set_attribute_by_name` and
  `attribute_type` refuse an `INVERSE` name with the new
  `NamedAttributeError::InverseAttribute` instead of `UnknownAttribute`;
  `UnknownAttribute` now reads "has no attribute".

Semver: additive (`NamedAttributeError` is `#[non_exhaustive]`), a minor
release while 0.x.

### Added (#481, shape representations)

- `AuthorOp::ShapeRepresentation { context, items, identifier,
  representation_type }` assembles an `IfcShapeRepresentation` of existing
  items in a geometric representation context. A given
  `RepresentationType` is checked against the items by the declared
  release's `IfcShapeRepresentationTypes` (`CorrectItemsForType`, IFC2X3
  `WR24`), transcribed per release family (IFC2X3; IFC4, IFC4X1, IFC4X2;
  IFC4X3), with `IfcCurve.Dim` derived as `IfcCurveDim` does; without one,
  the most specific listed type admitting every item is inferred. A label
  the release does not list is written as given (its rule returns `?`).
  The identifier defaults to the context's `ContextIdentifier` and is
  required from IFC4 on (`HasRepresentationIdentifier`); `CorrectContext`
  and `NoTopologicalItem` are checked.
- `AuthorOp::AssignRepresentation { representation, to }`: to an
  `IfcProduct`, appended to its `IfcProductDefinitionShape` (created when
  missing; produces it); to an `IfcTypeProduct`, wrapped in a new
  `IfcRepresentationMap` at the origin and appended to `RepresentationMaps`
  (produces the map). A representation already in a product shape,
  representation map or shape aspect is refused (`IfcShapeModel.WR11`).
- `AuthorOp::UnassignRepresentation { representation, from }`: off the
  product shape (an emptied one goes, with its shape aspects, and the
  product's `Representation` is unset) or the type (its map goes with its
  origin); the representation stays, unassigned. Reports what went.
- `AuthorOp::CopyRepresentation { representation }`: a copy of every
  representation item and profile definition it reaches, and of the styled
  items and colour or texture maps attached to them; the context, styles,
  presentation items, the type maps mapped items map and every other
  resource are shared. The copy joins the original's layers.
- `AuthorOp::RemoveRepresentation { representation }`: taken off its uses,
  then removed by the #478 cascade, shared items, profiles, points and
  styles kept and reported. `AuthoringOutcome::reports` and
  `StagedAuthoring::reports` carry the report of an unassignment or a
  representation removal.
- `AuthoringFailure::RepresentationTypeMismatch`, `NoRepresentationType`,
  `UndecidableRepresentationType`, `MissingRepresentationIdentifier`,
  `InvalidRepresentationItem`, `SharedProductShape` and `NotAssigned`.

Semver: additive (`AuthorOp` and `AuthoringFailure` are
`#[non_exhaustive]`): minor.

### Added (#464, selector query language)

- `ifc::selector` (feature `selector`; in `full`): a selector grammar,
  version 1.0 (`GRAMMAR_VERSION`), close to IfcOpenShell's
  `util.selector`: class facets (subtypes included) and `GlobalId` facets
  with `!` negation, explicit attribute facets, property and quantity
  facets (`Pset_WallCommon.FireRating=F90`, names or `/patterns/`),
  `type`, `material`, `classification`, `location` (alias `container`)
  and `group` keywords, `query:` value paths, comparisons `=`, `!=`, `*=`,
  `!*=`, `>`, `>=`, `<`, `<=` against text, numbers, `NULL`, `TRUE`,
  `FALSE` and patterns, `,` facet lists, `+` unions and `/* */` comments.
  `Selector::parse` and `ValuePath::parse` give a typed AST that prints
  back in canonical form; every `ParseError` carries the byte span it
  refuses.
- `SelectorIndex` evaluates a selector (`select`, `select_from`) over the
  model's type index, a `GlobalIdIndex`, the exact resolver's
  `PropertyIndex` (feature `properties`; `FeatureDisabled` without) and
  one scan per relationship kind, each built on first use. A class or
  attribute the release does not declare is refused with its span before
  any entity is read; contradictory or dangling relationships are
  refused, never skipped. Results are in ascending id order.
- Values by path (`SelectorIndex::value`, `get_value`): attributes,
  property sets and properties, `type`, `material(s)`, `container`,
  `parent`, `space`, `storey`, `building`, `site`, `occurrences`,
  `classification`, `group`, `class`, `id`, `predefined_type`, `count`
  and list indices, as `SelectedValue`s.
- Writes by path (`set_value`, `SelectorIndex::stage_set_value`,
  `set_targets`, `stage_attributes`, `property_edits`): an attribute of
  every entity the path reaches, checked against its declared type in the
  release, or a property through the checked property edit, staged on one
  transaction; `SelectorError` names every refusal.
- Depends on `regex-lite` (optional, with `selector`).

Semver: minor (additive).

### Added (#480, type propagation on assignment)

- `AuthorOp::AssignType` takes `options: AssignTypeOptions`. With
  `map_representations`, each of the type's `RepresentationMaps` is mapped
  onto every object as an `IfcShapeRepresentation` (the map's context and
  `RepresentationIdentifier`, `RepresentationType` `MappedRepresentation`)
  holding one `IfcMappedItem`, whose `MappingTarget` is `mapping_target`
  or a fresh identity `IfcCartesianTransformationOperator3D` (`2D` for a
  2D `MappingOrigin`); the representations join the object's
  `IfcProductDefinitionShape`, created when it has none. With
  `material_usage`, an `IfcMaterialLayerSetUsage` (`LayerSetDirection`
  from `layer_set_direction` or the class: `AXIS2` for `IfcWall`, `AXIS3`
  for `IfcSlab` and `IfcPlate`, as the IFC4 ADD2 TC1 documentation fixes
  them; `POSITIVE`; offset 0) or an `IfcMaterialProfileSetUsage` (no
  `CardinalPoint`) is derived from the type's set, one per object in its
  own `IfcRelAssociatesMaterial`; any other material applies through the
  type. `predefined_type: PredefinedTypeMode` is `Ignore` (default),
  `Validate` (refuse an object whose own `PredefinedType` is set, not
  `NOTDEFINED` and differs from the one the type provides; `USERDEFINED`
  compares `ObjectType` with the type's `ElementType`) or `Sync` (unset
  the object's, and its `USERDEFINED` `ObjectType`, where the type
  provides one). `replace` unassigns each object's previous type: it
  leaves its `IfcRelDefinesByType` (removed when emptied), and the
  representations mapped from the previous type's maps and the usage
  derived from its set are removed with what only they use, by the #478
  cascade's ownership rules. `AssignTypeOptions::propagate()` sets the
  three propagations.
- `AuthorOp::Contain` and `AuthorOp::Aggregate` take
  `options: RelateOptions` with `replace`, which moves an object out of
  its previous container or whole instead of refusing it.
- `AssignTypeOptions`, `RelateOptions`, `PredefinedTypeMode`, `LayerAxis`.
- `AuthoringFailure::RepresentationConflict`,
  `UnsupportedRepresentationMap`, `MaterialUsageUndetermined` and
  `PredefinedTypeConflict`; a material on an object that already has one
  is `AlreadyRelated` with `IFCRELASSOCIATESMATERIAL`.

Refusing a second type, container or parent stays the default, and every
option is off by default: a batch that sets none writes what it wrote
before. Semver: minor while 0.x. The three operations gain a field, so a
struct literal of them needs `options` (`AssignTypeOptions::new()`,
`RelateOptions::new()`); new variants of `#[non_exhaustive]` enums and new
`#[non_exhaustive]` types are additive.

## [0.32.0] - 2026-10-13

### Added (#478, cascading delete and purge)

- `AuthorOp::RemoveProduct { product, options }` removes an `IfcProduct` or
  `IfcTypeObject` with everything it solely owns: its `ObjectPlacement`,
  its representation down to the items nothing else uses, the property and
  quantity sets only it is defined by with their properties, its material
  usage (not the material definitions), its openings (each with the same
  cascade) and its ports (nested, or connected in IFC2X3), and every
  relationship left without an end. "Solely owned" is a reference count
  over the batch's view (the base model's `ReverseIndex`, corrected for
  the batch's own changes), taken to a fixed point; a relationship that
  only loses a list member and a presentation layer assignment do not
  count, a styled item and a shape aspect go with what they attach to.
  Shared data is kept: a property set another object uses, a type's
  representation maps, a style or point used elsewhere; so are related
  objects (the type, the container, the fillings of a removed opening) and
  definitions (material definitions, classification references, units,
  contexts, the owner history).
- `RemoveProductOptions` (`#[non_exhaustive]`): `rebase_placements`
  re-bases placements relative to a removed placement onto its nearest
  kept ancestor with the composed `IfcAxis2Placement3D` transform, so
  nothing moves; `untype_occurrences` lets a type object with occurrences
  go, removing its `IfcRelDefinesByType`.
- `AuthorOp::Purge(PurgeOptions)`: without classes, every resource entity
  no `IfcRoot` reaches, nor annotates something reached (styled items,
  layer assignments, shape aspects, resource relationships, coordinate
  operations); with classes, the unreferenced entities of those classes.
  `IfcRoot`s (`include_roots`; `IfcProject` only when named), the owner
  history with its actors (`include_owner_history`) and, by default,
  representation contexts are kept unless asked.
- `RemovalReport` (`removed`, `kept` with a `KeepReason` and the `users`
  of a shared entity, `detached`, `rebased`), `RemovedEntity`,
  `KeptEntity`, `KeepReason` (`Shared`, `RelatedObject`, `Definition`,
  `Protected`): `AuthoringOutcome::reports` and `StagedAuthoring::reports`
  carry one per operation, so staging a batch is the dry run.
- `AuthoringFailure::PlacementHasDependents`, `TypeHasOccurrences` and
  `UnsupportedRebase`.

### Changed (#478)

- The authoring batch reads who references an entity from the base
  model's `ReverseIndex`, built once per batch on the first removal,
  instead of scanning the model per lookup.

Semver: additive (new variants of `#[non_exhaustive]` enums, new fields of
`#[non_exhaustive]` structs), a minor release while 0.x.

### Added (#450)

- ifcZIP, behind the new `zip` feature (on in `full`): `ifc::ifczip`
  reads an archive holding one STEP or ifcXML model at its root
  (buildingSMART implementation agreement CV-2x3-154), the inner format
  and the ifcXML layout detected from the content, through the existing
  codecs (`read`, `extract`, `read_entry`), and writes one Deflate entry
  (`write`, `pack`), reproducibly. Refusals are typed
  (`IfcZipError`): `Malformed`, `NoModel`, `SeveralModels`,
  `UnknownInnerFormat`, `Encrypted`, `UnsupportedCompression` (anything
  but Stored and Deflate), `BudgetExceeded` and `CodecNotCompiled`.
  Decompression is bounded by a `Budget`, absolute (4 GiB) and as a ratio
  to the compressed size (200, above 1 MiB), checked against the declared
  size and again while inflating.
- `ifczip::IfcZipCodec` puts ifcZIP behind the `Codec` interface: `codecs()` lists
  it and `read_path` reads `.ifczip` (an archive is recognized by its ZIP
  signature; without the feature it is refused as `WrongFormat` naming
  the feature).
- `xml_layout::declared_xml_layout` and `DeclaredXmlLayout` (feature
  `ifcxml`): the
  ifcXML layout a document's root declares, and whether its attributes
  are named by position, for readers handed a document without its
  layout.
- Dependencies: `zip` 9 with every default off and Deflate alone, through
  `flate2`'s pure-Rust miniz_oxide backend; only with `zip`.

Semver: minor (additive; new feature and items).

### Added (#449)

- Feature `colour` (`geometry-select`, `style`, `material`):
  `product_colours(model, products)` and `ColourResolver`
  (`product_colour`, `item_colours`) resolve the surface colour of each
  product's Body. Each item takes its own `IfcStyledItem`, then its styled
  presentation layer (a boolean result its `FirstOperand`'s, a mapped item
  its enclosing `IfcMappedItem`'s), then the
  `IfcMaterialDefinitionRepresentation` of the material the product, or
  its type, is made of. An `IfcMaterial`, a layer or profile and its
  material, layer and profile sets and their usages naming one material,
  IFC2X3's `IfcMaterialList` of one material, and constituent sets by
  item (the constituent whose `IfcShapeAspect` names the item) resolve. A
  set of several materials is `ColourError::AmbiguousMaterial`, never its
  first member; Body items that differ are `ColourError::ItemsDiffer`. The
  colour carries its source (`ColourSource`: styled item, layer,
  material).
- `ProductMesh::colour` (`mesh` now implies `colour`): each mesh's colour,
  or its typed `ColourError`, which never refuses the mesh. Positions and
  indices are unchanged.

Semver: additive (`ProductMesh` is `#[non_exhaustive]`), a minor release
while 0.x.

### Added (#463, GlobalIds, types and openings)

- `GlobalIdIndex::build(&model, schema)`: every `IfcRoot` by `GlobalId`,
  read once (O(n), only `IfcRoot`s decoded; the slot looked up by name in
  the release's table). `get(&Guid)` is `Ok(Some(id))`, `Ok(None)`, or
  `Err(DuplicateGlobalId { global_id, entities })` naming every holder in
  file order: a duplicated `GlobalId` is never answered with one of them.
  `duplicates()` lists them all and `invalid()` the `IfcRoot`s whose
  `GlobalId` is unset or malformed. `by_guid(&model, schema, &guid)` answers
  one lookup by a single scan. Behind `schema-api`.
- `type_of(model, schema, occurrence)`, `occurrences_of(.., type_object)`,
  `openings_of(.., element)`, `fillings_of(.., opening)` and
  `voided_element_of(.., opening)`, read forward from
  `IfcRelDefinesByType` (in IFC2X3 too, whose `IsDefinedBy` mixes it with
  property sets), `IfcRelVoidsElement` and `IfcRelFillsElement`, slots by
  attribute name. Each refuses with `ObjectRelationError`
  (`#[non_exhaustive]`): `MissingEntity`, `WrongEntityType`, `Malformed`,
  `MissingReference`, and `Ambiguous` for two types of one object or two
  hosts of one opening, where the schema allows one. Behind `schema-api`.
- `new_guid()`: a random RFC 9562 version-4 `GlobalId` from the operating
  system (`getrandom` 0.3, already linked natively through `ahash`), or
  `GuidError` (`#[non_exhaustive]`). On `wasm32-unknown-unknown` the new
  `js-random` feature links getrandom's JavaScript backend
  (`crypto.getRandomValues`, no `RUSTFLAGS` needed); without it there the
  call refuses with `GuidError::NoRandomSource`. `name_based_guid(parts)`,
  the version-8 name-based `GlobalId` the property edit and authoring
  already derived theirs with, is public; its output is unchanged and
  pinned by a test. `Guid` is re-exported at the root.

Semver: additive, a minor release while 0.x.

### Changed

- `georeferencing` reads IFC2X3 (#451): the `ePSet_MapConversion` and
  `ePSet_ProjectedCRS` property sets of the project (or else of each site)
  resolve with the project length unit to the same `ProjectToMap` as
  IFC4's entities. An IFC2X3 file used to be refused with
  `UnsupportedSchema`; without the sets it now has an empty list, and a
  malformed or incomplete set is refused with the new `GeorefError`
  variants. Property sets of those names in an IFC4 or IFC4X3 file are not
  read. Requires `ifc-georef` with #451.

Semver: minor. A release that was refused now resolves.

## [0.31.0] - 2026-10-13

### Changed

- Requires `ifc-geometry` 0.25: stations on IfcOffsetCurveByDistances
  bases that lower to OffsetsAtStations lower with seam windows, and an
  offset of an offset is refused by its own name (#443, part). Semver:
  minor, following the dependency.

## [0.30.0] - 2026-10-13

### Changed (#433)

- Requires `ifc-geometry` with #433: an `IfcOffsetCurveByDistances` along a
  bounded basis whose length is a quadrature lowers, continued to the
  basis's ends the kernel measures, and `product_graphs` labels such a
  payload wire format 1.3 (`axiolid-model` 0.3.12, axiolid/kernel#303).
  Every other payload keeps its version and bytes.

Semver: minor, following the dependency.

## [0.29.0] - 2026-10-12

### Changed (#429)

- Requires `ifc-geometry` with #429: sectioned solids and surfaces along
  offsets whose length is a quadrature lower with seam-snapping windows on
  their sections, and `product_graphs` labels those payloads wire format
  1.2 (`axiolid-model` 0.3.11, axiolid/kernel#301). Every other payload
  keeps its version and bytes: 1.0, or 1.1 when a station carries a window.

### Changed (#427)

- Requires `ifc-geometry` with #427: with an evaluator, a linear placement
  on an offset whose length is a quadrature (a kerb beside an alignment, an
  offset beside a clothoid) derives, reading a joint within the model's
  precision from the incoming piece.

Semver: minor, following the dependency.

## [0.28.1] - 2026-10-12

### Fixed (axiolid/kernel#297)

- Requires `axiolid-model` 0.3.10 (through `ifc-geometry`'s `wire`
  feature): `product_graphs` payloads carry the lowest wire format version
  their content needs, 1.0 unless a station carries a seam-snapping
  window, then 1.1. Window-free payloads are labelled 1.0 again, readable
  by `axiolid-model` 0.3.8.

Semver: patch.

## [0.28.0] - 2026-10-10

### Changed

- Requires `ifc-geometry` 0.22: stations on offsets whose length is a
  quadrature lower with Axiolid's seam-snapping window (#423), and
  `product_graphs` encodes with `axiolid-model` 0.3.9, wire format 1.1
  (`ifc::geometry::wire::FORMAT_VERSION`). An envelope carries the lowest
  wire version its content needs: 1.0, or 1.1 when a station carries a
  seam-snapping window; `axiolid-model` 0.3.9 labels every payload 1.1
  (axiolid/kernel#297), and a reader on `axiolid-model` 0.3.8 or older
  refuses 1.1.

Semver: minor, following the dependency.

## [0.27.0] - 2026-10-10

### Added (#367, geometry Level 2, ADR 0021)

- Feature `geometry-wire` (`geometry` plus `ifc-geometry/wire`, in `full`):
  `product_graphs(model, ids)` -> `(EntityId, GeometryResult<ProductGraph>)`
  per product, `ProductGraph { world, graph }`: each product's Body lowered
  into Axiolid's neutral `GeometryGraph` exactly as `product_meshes` lowers
  it, in world coordinates (metres), with its world placement alongside
  and `None` for a product with no Body. Encode a graph in Axiolid's wire
  format 1.0 with `GeometryGraph::to_json` / `to_cbor`
  (`ifc::geometry::wire`). `compiled_features()` reports `geometry-wire`.

Semver: additive (a new opt-in feature), a minor release while 0.x, since
the `ifc-geometry` floor rises.

## [0.26.0] - 2026-10-10

### Changed

- Requires `ifc-geometry` 0.20: linear placements on curve-relation bases
  derive through curve paths (#418), and stations lower on offset-curve
  bases (#414), on Axiolid's wave-3 line. Semver: minor, following the
  dependency.

## [0.25.0] - 2026-10-09

### Changed

- Requires `ifc-alignment` 0.8 and `ifc-geometry` 0.19: a Viennese cant
  bend about a held rail lowers to the banked curve (#364), and derived
  linear placements on a seam take the incoming tangent (#409), on
  Axiolid's wave-2 line. Semver: minor, following the dependencies.

## [0.24.0] - 2026-10-09

### Changed

- Requires `ifc-geometry` 0.18: stations on composite, trimmed and
  station-placed curve bases lower with the seam rule at joints (#346),
  and curve segments placed by IfcAxis2PlacementLinear lower to stations
  (#311, part). Semver: minor, following the dependency.

## [0.23.0] - 2026-10-09

### Changed

- Requires `ifc-geometry` 0.17: polygonal half-space boundaries with
  circular-arc segments lower exactly (#398), on Axiolid's 0.3.16 line.
  Semver: minor, following the dependency.

## [0.22.0] - 2026-10-09

### Changed

- Requires `ifc-geometry` 0.16: stations on seams lower with the incoming
  tangent and runs mitre across seams (#346, part 1), on Axiolid's
  0.3.15 line; the backend comparison example (#31). Semver: minor,
  following the dependency.

## [0.21.0] - 2026-10-09

### Changed

- Requires `ifc-geometry` 0.15: collinear `IfcArcIndex` segments lower as
  straight segments everywhere (#396), and geometry where-rules follow the
  declared release, with IFC2X3 `WRnn` names and the release-only rules
  (#397, #400, #402). Semver: minor, following the dependency.

## [0.20.0] - 2026-10-09

### Changed

- Requires `ifc-geometry` 0.14: `IfcPolygonalBoundedHalfSpace` boundaries
  given as `IfcCompositeCurve` or `IfcIndexedPolyCurve` with straight edges
  lower (#393). Semver: minor, following the dependency.

## [0.19.0] - 2026-10-09

### Changed

- Requires `ifc-geometry` 0.13: net bodies (`lower_product_net_with`) are
  subtracted in the host's frame under one world `Instance`, so flush
  openings in georeferenced models mesh (#388). Semver: minor, because the
  re-exported neutral graph changes shape.

## [0.18.0] - 2026-10-08

### Added (#342, declared attribute types)

- Behind `schema-api`: `declared_type(schema, type_text)` resolves an
  EXPRESS type token against a release's tables to a `DeclaredType`:
  `Simple` (`SimpleType`: `INTEGER`, `REAL`, `NUMBER`, `STRING`,
  `BOOLEAN`, `LOGICAL`, `BINARY`, widths dropped), `Entity`,
  `Enumeration` (with its items), `Defined` (with what it aliases, an
  aggregate included: `IfcCompoundPlaneAngleMeasure`), `Select` (every
  member resolved, nested SELECTs included), `Aggregate`, or `Unresolved`
  for a token the tables do not declare; bounded, so a cyclic alias ends
  `Unresolved`. `attribute_type(schema, entity, name)` gives an
  attribute's slot and declared type with its own aggregation levels
  outermost.

Semver: additive, a patch release.

## [0.17.0] - 2026-10-08

### Added (#328, geometry in the bindings, ADR 0021)

- Behind `geometry-select`: `product_placements(model, ids)` -- per product
  with a shape (or per id given), its world placement
  (`products_world_transforms`, metres) and the Body representation
  `select_shape_representation` chooses, as `ProductPlacement { product,
  world, body }`, each half its own `GeometryResult` -- with
  `SelectedRepresentation` (identifier, type, context) and
  `SelectedContext` (context type, derived from the parent for a
  sub-context; identifier; target view literal). `column_major(&Transform)`
  gives the 4x4 column-major matrix graphics APIs read.
- Feature `mesh` (`geometry` plus `ifc-geometry`'s
  `compile-reference-backend`; never default, ADR 0004):
  `product_meshes(model, ids)`, one result per product: `ProductMesh {
  world, positions, indices }`, `f32` positions relative to the world
  placement (mapped back through its inverse in `f64`), `u32` indices,
  compiled with one reference backend at a one-millimetre tolerance; a
  product without a Body has empty arrays, a refused one its
  `GeometryError`. `compiled_features()` reports `mesh`.

### Changed

- `full` now includes `mesh`, and so links the reference compile backend.

Semver of #328: additive; `mesh` needs the `compile` re-exports of the
next `ifc-geometry` (0.11.0).

## [0.16.0] - 2026-10-04

### Changed

- `spatial_properties` and the property-edit planner (`apply_property_edits`,
  `stage_property_edits`) resolve through one `ifc_properties::PropertyIndex`
  of the model instead of calling the exact resolver per element or per
  edited object, so listing every element's properties, or a batch editing
  many objects, is linear in the model (#352). Answers and refusals are
  unchanged. `ifc::properties::PropertyIndex` is reachable through the
  `properties` feature.
- Behind `geometry`, the re-exported `ifc_geometry` gains, under
  `geometry::lower`, `lower_product_net_with` and `NetOptions` (#351): a
  Reference View opening with only a `Reference` representation can be
  taken as already applied to its host's Body, listed in
  `NetLowering::taken_as_applied`. With `ifc-geometry`'s `compile` feature
  it also gains the evaluator-taking linear placement (#353,
  `LoweringSession::with_curve_evaluator`,
  `product_world_transform_with_evaluator`) and the cached-position check
  (#354, `CachedPositionPolicy`), and the derived frame follows IFC4.3
  (#355). When `ifc-geometry` 0.10 is released, the re-export moves to it,
  which makes the next release here a minor one.

Semver of #352 alone: no public signature changes, a patch release.

## [0.15.0] - 2026-10-04

### Changed (breaking)

- Behind `geometry`, the re-exported `ifc_geometry` is 0.9 (was 0.8): IFC4X3
  stations, linear placements, offset curves by distances and sectioned
  solids/surfaces lower exactly (#307), and an `IfcParameterValue`
  distance on an alignment centreline is refused instead of being read as
  plan distance (#347).

### Added (#330, checked authoring)

- Feature `authoring`: `ifc-author` without choosing a release (`author`
  now means `authoring` plus `schema`, as before), and the checked
  authoring batch. `apply_authoring(model, schema, ops, seed)` runs
  `AuthorOp`s in order against the model as the operations before them
  leave it and commits them as one transaction, or refuses with
  `AuthoringError` (the operation's position and an `AuthoringFailure`)
  and changes nothing; `stage_authoring` plans without committing.
- `AuthorOp` (`#[non_exhaustive]`): `Create` an entity by type and named
  attributes, `Edit` named attributes, `Remove` an entity with the
  relationships that reference it, and the builders `Project`, `Spatial`
  (aggregated under its parent), `Product` (placed, contained and typed),
  `TypeObject`, `AssignType`, `Contain`, `Aggregate`, `Placement` (an
  `IfcLocalPlacement` over an `IfcAxis2Placement3D`) and `OwnerHistory`
  (through `ifc-author`'s writers). Every record is built by name through
  `ifc-author`'s `EntityBuilder` against the declared release, then its
  references are resolved (type checked with `Schema::accepts_type`) and
  its aggregates held to their declared bounds; a supplied `GlobalId`
  another entity holds is refused; an object is contained, aggregated or
  typed once, and a model holds one `IfcProject`.
- An `IfcRoot` created without a `GlobalId` gets a name-based one over a
  seed (`fresh_seed()`, from the OS on native targets) and its id; a fixed
  seed reproduces a file. `OwnerHistory` is never invented.
- `authoring_handle(i)` / `HANDLE_BASE` (2^62): an operation names the
  entity an earlier one produced; `AuthoringOutcome::ids` reports the ids.
- Additive: a patch release. The property edit's name-based `GlobalId`
  moved into a shared module unchanged (a test pins its output).

## [0.14.0] - 2026-10-04

### Changed (breaking)

- Behind `property-catalog`, the re-exported `property_catalog` is
  `ifc-template-catalog` 0.4 (was 0.3): one compact snapshot container
  replaces the per-edition bincode files, with new `snapshot` and `runtime`
  modules (#317). The new `property-catalog-runtime` feature checks
  property writes against a catalog supplied at run time (#318).

### Added (#326, attributes by name)

- `attribute_slots(schema, type_name)` and `attribute_slot(schema,
  type_name, name)` (feature `schema-api`): an entity type's explicit
  attribute slots in Part 21 order, inherited first, as `AttributeSlot`
  (index, name, declared type, `optional`, `aggregate`, `derived`,
  `declared_by`). `INVERSE` and new `DERIVE` attributes hold no slot and
  are not listed; an inherited attribute a subtype redeclares as derived
  is marked `derived`. Names match ASCII case-insensitively, as EXPRESS
  identifiers do; answers use the schema's spelling.
- `attribute_by_name(model, schema, id, name)` and
  `set_attribute_by_name(model, schema, id, name, value)`: read and write
  by name against the schema the caller passes. The write refuses a
  derived slot and checks everything before its one write.
  `NamedAttributeError` (`#[non_exhaustive]`): `MissingEntity`,
  `UnknownEntity`, `UnknownAttribute`, `DerivedAttribute`.
- Additive: a patch release.

### Added (#318, runtime PSD/QTO catalog)

- Feature `property-catalog-runtime`: property edits check `Pset_`/`Qto_`
  sets against a catalog the host installs at runtime
  (`property_catalog::runtime::install`) instead of the embedded one; until
  the release's edition is installed such an edit is refused with the new
  `PropertyEditFailure::CatalogNotLoaded { set, edition }` and nothing is
  written. With `property-catalog` too, the embedded catalog is used.
  `property-catalog` now enables `ifc-template-catalog/embedded`
  explicitly. The embedded catalog shrinks from 3.7 MB to 1.4 MB with the
  catalog's compact format (#317).
- Additive (`PropertyEditFailure` is `#[non_exhaustive]`): a patch
  release on its own.

## [0.13.0] - 2026-10-03

### Added (#123, property sets: write side)

- `apply_property_edits` and `stage_property_edits` (feature
  `properties`): a batch of `PropertyEdit`s (set or remove a value,
  addressed by object, set name and property name) planned against the
  model and staged on one `Transaction`, committed all or nothing. An
  inherited value is overridden on the occurrence, never on the type's
  shared set; a shared set or property entity is copied before it
  changes; an emptied set is removed with its relationship. Values are
  checked against the declared release (IFC2X3, IFC4, IFC4X3) and, with
  `property-catalog`, a `Pset_`/`Qto_` set against the release's PSD/QTO
  catalog; without it such a set is refused
  (`PropertyEditFailure::CatalogUnavailable`). New sets and relationships
  take the object's owner history and a name-based `GlobalId`. Types
  `PropertyEdit`, `SetType`, `PropertyEditOutcome`,
  `StagedPropertyEdits`, `PropertyEditError`, `PropertyEditFailure`.
- Additive: on its own a patch release; this release is a minor one for
  #306 anyway.

### Changed (breaking)

- `validate`, `spatial`, `geometry-select`, `properties`,
  `classification`, `material`, `systems`, `cost` and `georef` link only
  the releases the build names (#306): each release feature (`ifc2x3` ...
  `ifc4x3`, and `schema` for all five) now forwards to their crates, which
  no longer pull every release's table in through `ifc-schema`'s
  defaults. An IFC4-only build with all of them carries the IFC4 table
  alone. A build that enabled one of these features without a release
  feature must now add one (or `schema`): `spatial` and `properties` fail
  to compile without one, and the others refuse every file with their
  unsupported- or unbundled-schema error. `full` and `domains` (which
  implies `schema` through `author`) are unchanged. Under 0.x, a minor
  release.

### Added

- `root_identity(model, schema, id)` and `RootIdentity` (behind
  `schema-api`): the `GlobalId` and `Name` of an entity the given schema
  declares an `IfcRoot`, read by attribute name, never guessed from a
  slot (#123).
- `georeferencing(model)` and `GeoreferencingError` (behind
  `georef` + `properties`): every `IfcCoordinateOperation` resolved with
  the project length unit, which `ifc-properties` resolves exactly; the
  join the language bindings carry (#123).
- Re-exports `Budget` from `ifc-model`, which bounded domain traversals
  (a classification hierarchy) take.

## [0.12.0] - 2026-10-03

### Changed (breaking)

- Behind `geometry`, the re-exported `ifc_geometry` is 0.7 (was 0.6). Its
  `select::subtype` carries the IFC4X3 ADD2 supertype chains (#293), so
  `is_a`, select membership and `BodyKind::classify` now answer for
  IFC4X3-only entity names that previously classified as nothing; the new
  `is_a_in`/`supertypes_of_in` answer for one named release.

## [0.11.0] - 2026-10-03

### Changed (breaking)

- Behind `step`, the re-exported `ParseOptions` and `OnMalformed` are
  `openbim-step` 0.11's (were 0.8's), through `ifc-step` (#288). 0.11 adds
  `ParseOptions::accept_real_without_point`, included in
  `ParseOptions::lenient()`: a lenient read keeps a REAL written without
  its decimal point (`1E-05`) with a diagnostic instead of skipping its
  record. Strict reads still refuse it.

## [0.10.0] - 2026-10-02

### Changed (breaking)

- Behind `ifcxml`, the re-exported `XmlCodec` reads strictly by default when
  it has a schema: values are typed from the schema and undeclared names
  are refused (`ifc-xml` #266; see its changelog).
  `.with_reading(SchemaReading::Lenient)` restores the previous read.

### Added

- Behind `ifcxml`, re-exports `SchemaReading` and `XmlLayout` beside
  `XmlCodec` and `XmlProfile`, and with them the buildingSMART XSD layout
  reader `XmlCodec::xsd` (`ifc-xml` #265).

## [0.9.0] - 2026-09-29

### Added

- `tests/stationing_template.rs`: a referent's `Pset_Stationing` authored
  by `alignment` and read by `properties` checks clean against the
  `property-catalog` IFC4X3 ADD2 corrected profile (#216).

### Changed (breaking)

- `PanelPosition` and `Unreachable` are `#[non_exhaustive]`: a match needs a
  wildcard arm.
- `Sector` and `ContainerElements` are `#[non_exhaustive]`; they can no
  longer be built with a struct literal outside the crate.

- Code previously behind `schema` is behind `schema-api`, which `schema`
  and every release feature imply; `schema::for_version` returns a
  `Result` (see `ifc-schema`).

### Changed

- The georeferencing and alignment conformance test gives its placeholder
  `IfcPolyline` two points (`Points` is `LIST [2:?]`, checked since #111).
- Door and window operation reads name the type-object entity per
  verified release (IFC2X3, IFC4, IFC4X3) and refuse any other with
  `ExactPropertyError::UnsupportedSchema`, instead of treating every
  non-IFC2X3 release as IFC4.

### Added

- Per-release schema features (#112): `ifc2x3`, `ifc4`, `ifc4x1`, `ifc4x2`,
  `ifc4x3`, each providing the schema API with that one bundled table, and
  `schema-api` (the API with no table). `schema` keeps its meaning: the API
  with every release. A single-release build refuses the others through
  `schema::for_version` with `schema::NotBundled`. Domain features link the
  releases their crates read, so enabling one brings every release it
  reads.
- `compiled_features()` reports `schema-api` and each release feature.
- `tests/intermediate_releases.rs`: IFC4X1 and IFC4X2 files resolve to
  their own `SchemaVersion`, release id and bundled table (#33).

## [0.8.1] - 2026-09-28

### Changed

- Requires the patch releases published with it: `ifc-properties` 0.5.1
  (bare quantity values, IFC2X3 `*_with_owner_history` authoring, type
  objects in the exact API), `ifc-cost` 0.2.2, `ifc-classification` 0.2.2
  (IFC4X3 binding) and `ifc-systems` 0.2.2.
- `door_operation` and `window_operation` given a type object (an
  `IfcDoorType`, IFC2X3 `IfcDoorStyle`, ...) refuse it with `NotADoor` or
  `NotAWindow`, where they returned `Property(InvalidQueryObject)`: the
  exact property reader now accepts type objects (ifc-properties #193).
  Results and refusals for doors and windows are unchanged.

## [0.8.0] - 2026-09-28

### Changed (breaking)

- `properties` re-exports `ifc-properties` 0.5.0, whose breaking changes
  pass through (see that crate's changelog):
  - `Quantity` and `QuantityKind` are `#[non_exhaustive]`, and gain
    `Quantity::Unresolved` for a quantity without a readable value (#138)
    and `QuantityKind::Number` for IFC4X3 `IfcQuantityNumber`;
  - `create_quantity` / `create_quantity_with` take the `&Model` they write
    into and write its release's layout;
  - `PropertyTemplate` gains fields and is `#[non_exhaustive]` (#108).
- Requires `ifc-spatial` 0.2.3 (`referenced_elements`, release-bound
  container classification) and `ifc-geometry` 0.4.3
  (`BodyItem::item_world`, #185).

### Added

- `spatial_properties(model)` (features `spatial` and `properties`): every
  spatial container in tree order, depth first from the roots, with the
  elements it holds, each with its `exact_properties` list, in IFC2X3, IFC4
  and IFC4X3 (#121). An element is listed as `Contained`
  (`IfcRelContainedInSpatialStructure`), `Referenced`
  (`IfcRelReferencedInSpatialStructure`, so an element spanning several
  storeys appears under each) or `Part` (an `IfcRelAggregates` part, at any
  depth, of a contained element, which the Element Composition concept
  places by its composite's containment), ordered by element id. A nested
  space is its own container, not folded into its storey. A model-level
  refusal (diagnostics, missing or unsupported schema) is the function's
  error; any other `ExactPropertyError` is reported on the element it
  concerns and the other elements are still returned. Properties resolve
  lazily as `ContainerElements::elements` (or `elements_where`, with
  `exact_properties_where` selectors) is iterated. New types:
  `SpatialProperties`, `ContainerElements`, `SpatialContainer`,
  `ContainerName`, `ElementMember`, `ElementProperties` and
  `SpatialMembership`. Needs the next `ifc-spatial` release, which adds
  `SpatialTree::referenced_elements`.

### Changed

- The `spatial` feature classifies spatial containers from the file's
  declared release (#121, via the next `ifc-spatial` release, which now
  links `ifc-schema`): IFC4X3 facilities and facility parts such as
  `IfcRoad`, `IfcRoadPart`, `IfcBridge` and `IfcBridgePart`, and
  `IfcExternalSpatialElement`, are containers, so `SpatialTree`,
  `spatial_properties` and `unreachable_products` see the elements placed
  in them. Containment or reference into a non-container is reported as a
  `SpatialAnomaly`. `unreachable_products` skips containers by the tree's
  classification instead of a name test.

## [0.7.3] - 2026-09-27

### Changed

- The crate README, which is the crates.io page, is rewritten. The published
  one told readers to depend on a Git revision and said the crates were not
  on crates.io. The code is unchanged since 0.7.2.

## [0.7.2] - 2026-09-27

### Added

- `window_operation(model, window)` (features `geometry-select` and
  `properties`), the window counterpart of `door_operation`: each panel of
  a window as a world frame, width, height, hinge side, swing `Sector` for
  a side hinge and tilt `Sector` for a top or bottom hinge, from its
  placement, `OverallWidth`/`OverallHeight`, partitioning
  (`IfcWindowType.PartitioningType`, IFC2X3 `IfcWindowStyle.OperationType`,
  or the occurrence's), `IfcWindowPanelProperties` and the mullion and
  transom offsets of `IfcWindowLiningProperties`, in IFC2X3, IFC4 and
  IFC4X3 (#170). Single, double and triple partitionings with side-hung,
  tilt-and-turn, top- and bottom-hung, sliding, removable and fixed panels
  are derived; `NOTDEFINED` and `USERDEFINED` partitionings, pivot,
  `OTHEROPERATION` and `NOTDEFINED` panels, a window without panel
  properties or overall size, panels that contradict the partitioning, and
  a split without a valid lining offset are refused as
  `WindowOperationError`, never defaulted. Panels tile the placement's XZ
  plane; lining, mullion and transom thicknesses are not applied.
  `Sector` and `Side` are shared with `door_operation`, unchanged.

## [0.7.1] - 2026-09-27

### Added

- `profile_outline` and `ProfileOutline` are re-exported at the root under
  `geometry-select` (#166), beside `describe_profile`: the straight-edged
  outline of an arbitrary closed profile, without the geometry kernel.

### Changed

- Requires `ifc-geometry` 0.4.1, which carries `profile_outline`,
  `product_representation_frame` and the curve-bounded plane fix (#163).

## [0.7.0] - 2026-09-27

### Changed (breaking)

- Re-exports three crates whose breaking releases pass through:
  `validate` is `ifc-validate` 0.3.0 (`Severity::EvaluationError`, renamed
  rule ids, `Budget::max_depth` removed), `material` is `ifc-material` 0.3.0
  (views and authoring bound to the declared release; `create_material`
  takes the model), and `geometry` is `ifc-geometry` 0.4.0 (typed reference
  errors, `IfcBlock` meshes no longer offset by half their extents). See
  each crate's changelog.
- Requires the releases published with it: `ifc-properties` 0.4.1,
  `ifc-spatial` 0.2.2, `ifc-xml` 0.2.1, `ifc-alignment` 0.3.1,
  `ifc-author` 0.2.1, `ifc-control` 0.2.1, `ifc-cost` 0.2.1,
  `ifc-tabular` 0.2.1 and `ifc-template-catalog` 0.2.1.

### Added

- `body_description`, `describe_profile` and their types (`BodyDescription`,
  `BodyItem`, `BodyKind`, `SweptSolid`, `SweepPath`, `ProfileDescription`,
  `ProfileParameters`) are re-exported at the root under `geometry-select`
  (#147), so a rule check reads a body's kind and swept-solid profile
  parameters without linking the geometry kernel.
- `door_operation(model, door)` (features `geometry-select` and
  `properties`): each leaf of a door as a world frame, width, hinge side and
  swing `Sector`, from its placement, `OperationType` and
  `IfcDoorPanelProperties`, in IFC2X3, IFC4 and IFC4X3 (#148). Single and
  double swing, double-acting, sliding, rolling-up and swing-fixed doors are
  derived; `NOTDEFINED`, `USERDEFINED`, revolving, folding, lifting and the
  `DOUBLE_DOOR_SINGLE_SWING_OPPOSITE_*` operations, a door without panel
  properties or `OverallWidth`, and panels that contradict the operation are
  refused as `DoorOperationError`, never defaulted. The leaves lie on the
  placement's x axis; lining offsets across the wall depth are not applied.

### Changed

- The `properties` feature also names `ifc-schema`, which `ifc-properties`
  already links, so the door join reads attributes by name from the bound
  release's table. No crate is added to a build.

## [0.6.0] - 2026-09-26

### Changed (breaking)

- `properties` re-exports `ifc-properties` 0.4.0, whose breaking changes
  (`PropertyAnomaly` is `#[non_exhaustive]`; `template_of_set` returns every
  template of a set) pass through. It also resolves quantity sets in
  `exact_property` instead of reporting them absent (#66).

### Changed

- STEP models load lazily: `from_step_bytes`, `read_path` and every strict
  read validate the whole file but decode each entity on first access
  (ADR 0015, see `ifc-step`). `read_path` hands its buffer to the codec
  instead of letting it copy the file once more.

### Added

- `StepReader`, `ParseOptions` and `OnMalformed` are re-exported, so the
  eager and memory-mapped reads (`StepReader::eager`,
  `StepReader::read_path_mapped`) are reachable through the facade.

### Added

- `Transaction`, `Applied` and `Conflict` are re-exported. `EntityEditor` and
  the domain writers stage into a `Transaction`, which facade users could not
  name, so an editor could be built but never applied.

## [0.5.0] - 2026-09-26

### Added

- `ifc::properties::exact_unit` (ifc-properties 0.3): a measure's effective
  unit resolved exactly to SI, or refused (#53).
- `ifc::spatial::SpatialTree::anomalies` (ifc-spatial 0.2.1): double
  containment and double aggregation are reported (#54).

### Changed

- **Breaking:** re-exports ifc-properties 0.3, whose
  `UnitKind::Si::prefix_exponent` is now `Option<i32>` and whose
  `UnitKind::Conversion` gains an `offset` field.

## [0.4.0] - 2026-09-23

### Added

- Textured tessellated geometry: `ifc::geometry` (ifc-geometry 0.3) lowers
  `IfcIndexedTriangleTextureMap` as a per-corner `uv` channel that survives
  compilation, including multi-item bodies and mirrored `IfcMappedItem`s
  (#30, axiolid/kernel#115).

### Changed

- **Breaking:** re-exports ifc-geometry 0.3, ifc-georef 0.3 and
  ifc-alignment 0.3, which all require Axiolid 0.3.

## [0.3.1] - 2026-09-23

### Added

- `ifc::schema` re-exports `ifc-schema` (feature `schema`), so the bundled
  schemas `ids_of_type_including_subtypes` needs, such as
  `ifc::schema::ifc4()`, are reachable without a direct `ifc-schema`
  dependency. Found by building a crates.io-only consumer of 0.3.0.

## [0.3.0] - 2026-09-23

### Added

- `ids_of_type_including_subtypes` (feature `schema`): every entity of a type
  or any of its subtypes, in file order. `Model::ids_of_type("IfcElement")`
  returns nothing because no instance is declared as the abstract supertype;
  this answers the question that call looks like it should. The caller passes
  the `Schema`, because the tree differs by version (#32).

### Changed

- **Breaking:** requires `ifc-style` 0.3.0, re-exported as `ifc::style`.
  `IndexedTextureMap::maps` there now returns `Vec<EntityId>`.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/openbim-ifc-v0.37.0...HEAD
[0.37.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.37.0
[0.36.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.36.0
[0.35.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.35.0
[0.34.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.34.0
[0.33.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.33.0
[0.32.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.32.0
[0.31.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.31.0
[0.30.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.30.0
[0.29.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.29.0
[0.28.1]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.28.1
[0.28.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.28.0
[0.27.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.27.0
[0.26.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.26.0
[0.25.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.25.0
[0.24.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.24.0
[0.23.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.23.0
[0.22.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.22.0
[0.21.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.21.0
[0.20.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.20.0
[0.19.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.19.0
[0.18.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.18.0
[0.17.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.17.0
[0.16.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.16.0
[0.15.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.15.0
[0.14.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.14.0
[0.13.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.13.0
[0.12.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.12.0
[0.11.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.11.0
[0.10.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.10.0
[0.9.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.9.0
[0.8.1]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.8.1
[0.8.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.8.0
[0.7.3]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.7.3
[0.7.2]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.7.2
[0.7.1]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.7.1
[0.7.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.7.0
[0.6.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.6.0
[0.5.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.5.0
[0.4.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.4.0
[0.3.1]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.3.1
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.3.0
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
