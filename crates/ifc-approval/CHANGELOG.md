# Changelog -- ifc-approval

All notable changes to the `ifc-approval` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

## [0.2.1] - 2026-09-28

### Added

- `associate_approval_with_owner_history` (#202). It takes the model and a
  caller-supplied `IfcOwnerHistory` id, which IFC2X3 requires on every
  `IfcRoot`. The id must be in the model or staged on the transaction and
  must be an `IfcOwnerHistory`: a missing one is refused with
  `UnknownEntity`, another entity with `AuthoringReferenceType`. None is
  ever invented. In IFC4 and IFC4X3 the reference fills the optional slot.
  This follows `ifc-material`'s `associate_material_with_owner_history`
  (#77) and `ifc-properties`' `*_with_owner_history` writers (#191).
- `ApprovalError::MultipleSchemas`, `ApprovalError::UnsupportedSchema` and
  `ApprovalError::AuthoringRequired { entity, attribute, schema }` (#202).
  `ApprovalError` is `#[non_exhaustive]`, so this is not breaking.

### Fixed

- `associate_approval` no longer writes `IfcRelAssociatesApproval` with
  `OwnerHistory` `$` into an IFC2X3 model, where it is mandatory (#202).
  It binds the model's declared release (a header without `FILE_SCHEMA`
  binds IFC4), lays the record out by attribute name from that release's
  table, and checks `RelatedObjects` against it: `IfcDefinitionSelect` in
  IFC4 and IFC4X3, `IfcRoot` restricted by WR21 (object and property
  definitions) in IFC2X3. IFC4 and IFC4X3 output is unchanged.
  **Behaviour change:** an IFC2X3 model is refused with
  `AuthoringRequired { attribute: "OwnerHistory", .. }` and nothing is
  staged; use `associate_approval_with_owner_history` there. A header that
  declares several schemas, or one without a bundled table, is refused with
  `MultipleSchemas` or `UnsupportedSchema` where it was written before.
  The other writers of this crate carry no `IfcRoot` and are unchanged.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-approval-v0.2.1...HEAD
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-approval-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
