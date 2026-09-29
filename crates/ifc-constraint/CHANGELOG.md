# Changelog -- ifc-constraint

All notable changes to the `ifc-constraint` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Added

- `DateTimeInput`: a creation time as IFC4/IFC4X3 `IfcDateTime` text or an
  IFC2X3 `IfcDateTimeSelect` record (`From<&str>`, `From<EntityId>`).
- `ConstraintError::NotInSchema`, `StructuredValue`, `EntityNotInSchema`,
  `AuthoringNotInSchema` and `AuthoringValueType`, and projections'
  `release()`. `SchemaVersion` is re-exported.

### Changed (breaking)

- The constraint views read every attribute by name in the model's
  declared release (#212). They read IFC4 positions, types and SELECTs
  from every file. `ConstraintView` binds the header (IFC2X3, IFC4 or
  IFC4X3; none reads as IFC4) and a lookup refuses IFC4X1, IFC4X2, unknown
  and multiple schemas. An attribute the release does not declare is
  `NotInSchema`: IFC2X3 `IfcMetric.ReferencePath`, and IFC2X3
  `IfcObjective.LogicalAggregator`, whose slot holds `ResultValues`, an
  `IfcMetric`, which was reported as a malformed operator. An IFC2X3
  `CreationTime` record is `StructuredValue` with the record id instead of
  `InvalidValue`; IFC2X3's single `BenchmarkValues` metric is a one-element
  list; `DataValue` is checked against the release's own
  `IfcMetricValueSelect` and is required in IFC2X3. IFC4 and IFC4X3 answers
  are unchanged.
- `create_metric`, `create_objective`, `relate_resource_constraint` and
  `create_reference` bind the declared release and lay their records out by
  name (#212). In IFC2X3 a metric has ten attributes and requires
  `DataValue`, `ReferencePath` is `AuthoringNotInSchema`, an objective takes
  exactly one `IfcMetric` benchmark and no logical aggregator, text
  `CreationTime` is `AuthoringValueType`, and
  `IfcResourceConstraintRelationship` and `IfcReference` are
  `EntityNotInSchema`. Enumeration tokens are checked against the release's
  enumeration (IFC2X3 lacks, for example, `INCLUDES` and `MODELVIEW`). IFC4
  and IFC4X3 records are unchanged.
- `ConstraintBaseDraft::creation_time` is `Option<DateTimeInput>`; the
  setter takes `impl Into<DateTimeInput>`, so `.creation_time("…")` still
  compiles.
- `Metric::try_new`, `Objective::try_new`,
  `ResourceConstraintRelationship::try_new` and
  `ConstraintAssignment::try_new` take the `SchemaVersion` to read against,
  refusing IFC4X1, IFC4X2 and an entity the release does not declare.
  `ConstraintView::new` is no longer `const`. A value for an attribute the
  release does not declare, which `associate_constraint` could not hit, is
  `AuthoringNotInSchema` rather than `AuthoringInvalid`.

- Every public draft is `#[non_exhaustive]`, so struct literals no longer
  compile outside the crate. Each gains a constructor taking its required
  fields and one builder setter per other field, named after the field and
  taking the unwrapped value (`.description("…")` sets `Some`):
  - `ConstraintBaseDraft::new(name, grade)`
  - `MetricDraft::new(base, benchmark)`
  - `ObjectiveDraft::new(base, qualifier)`
  - `ResourceConstraintDraft::new(relating_constraint, related_resources)`
  - `ConstraintAssociationDraft::new(global_id, related_objects, relating_constraint)`
  - `ReferenceDraft::new()`, which now also derives `Default`

### Changed

- Depends on `ifc-schema` with its default features named explicitly
  (every bundled release), now that the workspace dependency turns them
  off for the facade's per-release features (#112).
- A model whose header declares `IFC4X1` or `IFC4X2` is refused with the
  existing unsupported-schema error. `ifc-schema` now bundles both
  releases, but no layout here is verified against them, so they are
  never read as IFC4 or IFC4X3.

## [0.2.1] - 2026-09-28

### Added

- `associate_constraint_with_owner_history` (#202). It takes the model and
  a caller-supplied `IfcOwnerHistory` id, which IFC2X3 requires on every
  `IfcRoot`. The id must be in the model or staged on the transaction and
  must be an `IfcOwnerHistory`: a missing one is refused with
  `UnknownEntity`, another entity with `AuthoringReferenceType`. None is
  ever invented. In IFC4 and IFC4X3 the reference fills the optional slot.
  This follows `ifc-material` (#77) and `ifc-properties` (#191).
- `ConstraintError::MultipleSchemas`, `ConstraintError::UnsupportedSchema`
  and `ConstraintError::AuthoringRequired { entity, attribute, schema }`
  (#202). `ConstraintError` is `#[non_exhaustive]`, so this is not
  breaking.

### Fixed

- `associate_constraint` no longer writes `IfcRelAssociatesConstraint`
  with `OwnerHistory` `$` into an IFC2X3 model, where it is mandatory
  (#202). It binds the model's declared release (a header without
  `FILE_SCHEMA` binds IFC4), lays the record out by attribute name from
  that release's table, and checks `RelatedObjects` against it
  (`IfcDefinitionSelect` in IFC4 and IFC4X3, `IfcRoot` restricted by WR21
  in IFC2X3). IFC4 and IFC4X3 output is unchanged. **Behaviour change:** an
  IFC2X3 model is refused with
  `AuthoringRequired { attribute: "OwnerHistory", .. }` and nothing is
  staged; use `associate_constraint_with_owner_history` there. IFC2X3 also
  requires `Intent`, which IFC4 made optional, so an IFC2X3 association
  without one is refused with `AuthoringRequired` on `Intent`. A header that declares several schemas, or one without a bundled
  table, is refused with `MultipleSchemas` or `UnsupportedSchema`. The
  other writers carry no `IfcRoot` and are unchanged.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-constraint-v0.2.1...HEAD
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-constraint-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
