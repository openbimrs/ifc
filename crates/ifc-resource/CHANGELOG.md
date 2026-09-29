# Changelog -- ifc-resource

All notable changes to the `ifc-resource` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

## [0.3.0] - 2026-09-29

### Changed (breaking)

- `ActorRoleDraft`, `PostalAddressDraft`, `TelecomAddressDraft`,
  `ActorDraft`, `AssetDraft`, `InventoryDraft`, `AppliedValueDraft`,
  `ResourceDraft`, `ResourceTimeDraft`, `AllocationDraft` and the resource
  `NestingDraft` are `#[non_exhaustive]` (#214). Struct literals no longer
  compile outside the crate: build each with `new(...)` and field-named
  setters. New constructors: `ActorRoleDraft::new(role)`,
  `PostalAddressDraft::new()`, `TelecomAddressDraft::new()`,
  `ActorDraft::new(global_id, the_actor)`, `AssetDraft::new(global_id)`,
  `InventoryDraft::new(global_id)` and `AppliedValueDraft::new()`, each
  with a setter per remaining field. The drafts that already had builders
  keep them unchanged. Fields stay public where they were.

### Changed

- Depends on `ifc-schema` with its default features named explicitly
  (every bundled release), now that the workspace dependency turns them
  off for the facade's per-release features (#112).
- A model whose header declares `IFC4X1` or `IFC4X2` is refused with the
  existing unsupported-schema error. `ifc-schema` now bundles both
  releases, but no layout here is verified against them, so they are
  never read as IFC4 or IFC4X3.
- Enumeration checks treat a type declaration form `ifc-schema` adds later
  as not matching; follows `ifc_schema::TypeKind` becoming
  `#[non_exhaustive]`.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-resource-v0.3.0...HEAD
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-resource-v0.3.0
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
