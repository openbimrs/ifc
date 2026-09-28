# Changelog -- ifc-material

All notable changes to the `ifc-material` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Changed (breaking)

- `MaterialView::profile_set_usages()` also yields
  `IfcMaterialProfileSetUsageTapering`, the schema subtype of
  `IfcMaterialProfileSetUsage` in IFC4 and IFC4X3, in entity-id order, and
  `MaterialProfileSetUsage::try_new` / `try_from_view` accept it (#136). A
  caller that iterated both `profile_set_usages()` and
  `tapering_profile_set_usages()` sees each tapering usage twice; iterate
  `profile_set_usages()` alone and branch on the new
  `MaterialProfileSetUsage::tapering()`, which returns the tapering
  projection (`end_profile_set_id`, `cardinal_end_point`) or `None`.
- The authoring drafts `MaterialDraft`, `LayerDraft`, `LayerSetDraft`,
  `MaterialAssignmentDraft`, `ConstituentDraft` and `ProfileDraft` are
  `#[non_exhaustive]`: build them with `MaterialDraft::new(name)`,
  `LayerDraft::new(thickness)`, `LayerSetDraft::new(layers)`,
  `MaterialAssignmentDraft::new(global_id, related_objects,
  relating_material)`, `ConstituentDraft::new(material)` or
  `ProfileDraft::new(profile)` and a setter named after each optional field
  (`LayerDraft::new(0.2).material(brick).priority(80)`). Fields stay public.
- `ResolvedAssignment` is `#[non_exhaustive]`.

## [0.3.0] - 2026-09-27

### Changed (breaking)

- Views and authoring bind to the release the model's header declares
  (#77). Every slot position comes from that release's bundled
  `ifc-schema` table (IFC2X3 TC1, IFC4 ADD2 TC1 or IFC4X3 ADD2) instead of
  IFC4 constants, so an IFC2X3 model is no longer read with IFC4 slots.
  Behaviour for an IFC4 model, and for an in-memory model whose header
  declares no schema (bound to IFC4 as before), is unchanged, except where
  listed below. Breaking, because the same call now answers differently for
  IFC2X3 and IFC4X3 models:
  - an accessor for an attribute the release does not declare returns
    `MaterialError::NotInSchema` instead of `Ok(None)`. For IFC2X3:
    `Material::{description, category}`,
    `MaterialLayer::{name, description, category, priority}`,
    `MaterialLayerSet::description` and
    `MaterialLayerSetUsage::reference_extent`. A typed absence rather than
    `None`, because `None` claims the file left the value unset;
  - a record the release cannot instantiate -- an
    `IfcMaterialConstituent(Set)`, `IfcMaterialProfile*`,
    `IfcMaterialLayerWithOffsets` or `IfcMaterialRelationship` in an IFC2X3
    model, or IFC2X3's abstract `IfcMaterialProperties` -- is refused with
    `MaterialError::EntityNotInSchema` by its accessors,
    `resolve_material_select` and `assigned_material`;
  - an IFC2X3 `IfcMaterialLayer.LayerThickness` of zero is invalid
    (`IfcPositiveLengthMeasure`; IFC4 relaxed it to
    `IfcNonNegativeLengthMeasure`);
  - the `IfcRelDefinesByType` fallback admits the concrete `IfcTypeObject`
    subtypes of the model's release, so IFC4X3 no longer accepts the
    `IfcDoorStyle`/`IfcWindowStyle` it removed. The generated IFC4 list is
    gone;
  - a header declaring several schemas fails every read and write with
    `MaterialError::MultipleSchemas`, and one declaring an unknown schema
    with `MaterialError::UnsupportedSchema`.
- `create_material` takes the `&Model` it writes into and returns
  `MaterialResult<EntityId>`, like every other authoring function: without
  the model it cannot know the release, and IFC2X3 `IfcMaterial` has no
  `Description` or `Category` slot to write.
- Authoring writes the release's layout: IFC2X3 gets the short
  `IfcMaterial`, `IfcMaterialLayer`, `IfcMaterialLayerSet` and
  `IfcMaterialLayerSetUsage` records. A draft value the release cannot hold
  (an IFC2X3 layer name, category or priority, a layer set description, a
  usage reference extent, a material description or category) is refused
  with `MaterialError::AuthoringNotInSchema` rather than dropped; an entity it
  lacks is refused with `MaterialError::EntityNotInSchema`; and a required
  attribute left unset -- the IFC2X3 `IfcRelAssociatesMaterial.OwnerHistory`
  -- with `MaterialError::AuthoringRequired`, so `associate_material` refuses
  an IFC2X3 model. Nothing is staged on refusal.
- IFC4X3 is read against its own table. Its material entities keep the IFC4
  layouts; the one rename, `IfcMaterialRelationship.Expression` to
  `MaterialExpression` at the same position, is resolved by name.
- `ifc-schema` is now a production dependency.

### Fixed

- `associate_material`, `create_material_properties` and layer-set members
  accept exactly what the release's schema accepts: `RelatingMaterial` any
  instantiable `IfcMaterialSelect` member (an `IfcMaterialLayer` too; IFC2X3
  has no constituent or profile sets), `Material` any
  `IfcMaterialDefinition`, and no `IfcMaterialLayerWithOffsets` in an IFC2X3
  set.

### Added

- `material_schema(model)`, `MaterialView::schema()` and a `schema()`
  accessor on every projection report the bound release; `SchemaVersion` is
  re-exported.
- `try_from_view(view, id)` on every projection looks a record up and binds
  it to the model's release. `try_new` has no header and keeps reading IFC4.
- `associate_material_with_owner_history` takes a caller-supplied
  `IfcOwnerHistory`, which IFC2X3 requires; none is ever invented.
- `MaterialError::{MultipleSchemas, UnsupportedSchema, NotInSchema,
  EntityNotInSchema, AuthoringNotInSchema, AuthoringRequired}`.

- `MaterialView::constituent_fraction_diagnostic`: an opt-in policy check
  that an `IfcMaterialConstituentSet`'s fractions describe one whole
  (#103). It returns a `ConstituentFractionDiagnostic` when every
  constituent states a fraction but the sum is further from 1 than the
  caller's tolerance (`SumNotOne`), or when stated and missing fractions
  are mixed (`PartiallyStated`). IFC4 declares no WHERE rule on the sum,
  so this is never a decode error and never normalises: the accessors
  keep returning the authored fractions.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-material-v0.3.0...HEAD
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-material-v0.3.0
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
