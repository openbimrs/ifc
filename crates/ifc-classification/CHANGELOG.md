# Changelog -- ifc-classification

All notable changes to the `ifc-classification` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

## [0.3.0] - 2026-09-29

### Changed (breaking)

- `ClassificationHierarchy` and `EffectiveClassifications` are
  `#[non_exhaustive]`.
- Every public draft is `#[non_exhaustive]`, so struct literals no longer
  compile outside the crate. Each gains a constructor taking its required
  fields and one builder setter per other field, named after the field and
  taking the unwrapped value (`.source("NBS")` sets `Some`):
  - `ClassificationDraft::new(name)`
  - `DocumentDraft::new(identification, name)`
  - `LibraryDraft::new(name)`
  - `AssociationDraft::new(global_id, related_objects)`
  - `ExternalReferenceRelationshipDraft::new(relating_reference, related_resources)`
  - `ClassificationReferenceDraft::new()`, `DocumentReferenceDraft::new()` and
    `LibraryReferenceDraft::new()`, which now also derive `Default`

### Changed

- Depends on `ifc-schema` with its default features named explicitly
  (every bundled release), now that the workspace dependency turns them
  off for the facade's per-release features (#112).
- `UnsupportedSchema` reads "a release this crate has no verified layout
  for" instead of "no bundled schema table".
- A model whose header declares `IFC4X1` or `IFC4X2` is refused with the
  existing unsupported-schema error. `ifc-schema` now bundles both
  releases, but no layout here is verified against them, so they are
  never read as IFC4 or IFC4X3.

## [0.2.2] - 2026-09-28

### Changed (breaking behaviour)

The public API only grows (`cargo semver-checks` against 0.2.1 reports no
break), but the same call now answers differently in the cases below, so
the next release is 0.3.0. Each change is required by #194: an IFC4X3
model must be read against its own table, and a header that binds no
single known release must be refused rather than read as IFC4.

- Views and authoring bind the release the header declares (#194), as
  `ifc-material` does since #77. One `IFC2X3`, `IFC4`, or
  `IFC4X3`/`IFC4X3_ADD2` declaration binds that release's bundled table
  (IFC2X3 TC1, IFC4 ADD2 TC1, IFC4X3 ADD2). Before, every header other
  than `IFC2X3` was read against IFC4, so a consumer had to refuse an
  IFC4X3 model's classifications.
  - `classification_schema` answers `SchemaVersion::Ifc4x3` for an IFC4X3
    header, where it answered `Ifc4`.
  - An element that exists only in IFC4X3 (for example `IfcRoad`) is an
    `IfcDefinitionSelect` member there, so its classification resolves;
    read against IFC4 it was a `ReferenceType` error.
  - `IfcResourceObjectSelect` is the IFC4X3 one, which adds
    `IfcShapeAspect`, for `external_reference_relationship(s)`,
    `external_references_for` and `create_external_reference_relationship`.
  - IFC4X3 renamed `IfcClassification.Location` to `Specification`, same
    position and type. `ClassificationSystem::location()` and
    `ClassificationDraft::location` read and write it. No other attribute
    this crate touches differs between IFC4 and IFC4X3
    (`IfcClassificationReference`, `IfcRelAssociatesClassification`,
    `IfcExternalReference.Identification`, documents, libraries and
    `IfcRoot` are unchanged).
- A header declaring several schemas fails every read and write with
  `MultipleSchemas`; before, only a set including IFC2X3 did, and e.g.
  `IFC4`+`IFC4X3` was read as IFC4. One declaration without a bundled table
  (such as `IFC4X1`) fails with the new `UnsupportedSchema`; before, it was
  read as IFC4. A header with no declaration (an in-memory model) still
  binds IFC4, as in 0.2.0.
- Authoring lays each record out by attribute name in the declared
  release's table instead of writing the IFC4 layout into every model.
  IFC2X3 gets its own records (a four-attribute `IfcClassification`,
  `ItemReference`/`DocumentId` for `Identification`, a three-attribute
  `IfcDocumentInformationRelationship`, ...). What IFC2X3 cannot hold is
  refused with a typed error and nothing is staged:
  - a draft value for an attribute it lacks (`IfcClassification`
    `Description`, `Location`, `ReferenceTokens`; a reference's
    `Description` or `Sort`; a document's `Location`; a document
    reference's `Description` or `ReferencedDocument`; a library's
    `Location` or `Description`; a library reference's `Description`,
    `Language` or `ReferencedLibrary`) with `AuthoringNotInSchema`;
  - text for an attribute it types as a record (`EditionDate`,
    `CreationTime`, `LastRevisionTime`, `ElectronicFormat`, `ValidFrom`,
    `ValidUntil`, `VersionDate`) with `AuthoringValueType`;
  - a required attribute left unset (`IfcClassification.Source` and
    `Edition`, and `IfcRoot.OwnerHistory`) with `AuthoringRequired`. So
    `associate_classification`, `associate_document` and
    `associate_library` refuse an IFC2X3 model; use their new
    `*_with_owner_history` variants there;
  - a reference its selects do not admit (an `IfcClassification` as
    `RelatingClassification`, which IFC2X3 types as
    `IfcClassificationNotationSelect`; a reference as `ReferencedSource`;
    a person as library `Publisher`) with `AuthoringReferenceType`;
  - `create_external_reference_relationship` with `EntityNotInSchema`.
  IFC4 output is unchanged, and IFC4X3 output is the IFC4 record at the
  same positions.
- Every reference an authoring call checks is checked against the type the
  release declares, and the error's `expected` label is that type's name
  (the same strings as before for IFC4).

### Added

- `create_classification_in(tx, model, draft)`: `create_classification`
  in the model's release. `create_classification` takes no model, so it
  keeps writing the IFC4 layout (valid for IFC4 and IFC4X3) and is
  documented as such.
- `associate_classification_with_owner_history`,
  `associate_document_with_owner_history` and
  `associate_library_with_owner_history`: take a caller-supplied
  `IfcOwnerHistory`, which IFC2X3 requires. It must be in the model or
  staged, and be an `IfcOwnerHistory`; one is never invented. This follows
  `ifc-material` (#77) and `ifc-properties` (#191).
- `ClassificationError::UnsupportedSchema`, `EntityNotInSchema`,
  `AuthoringNotInSchema`, `AuthoringRequired` and `AuthoringValueType`.
  `ClassificationError` is `#[non_exhaustive]`, so these are additive.

### Fixed

- `DocumentInformation::status()` and `create_document` accept
  `FINALDRAFT`. Every release declares it in `IfcDocumentStatusEnum`, but
  both hard-coded a list without it, so a valid document was
  `InvalidValue`. Both enumerations now come from the release's table.
- `effective_classifications` reads `IfcRelDefinesByType` by attribute
  name instead of fixed positions.

### Verified

- Every attribute, select and enumeration the crate touches is pinned
  against the three bundled tables, and the tables against the normative
  EXPRESS under `references/ifc-spec` (`tests/release_layout.rs`).
- Records authored in IFC2X3, IFC4 and IFC4X3 through every writer are
  written as STEP, read back, re-read through the views, and validated by
  `ifc-validate` with no error finding
  (`openbim-ifc/tests/release_bound_classification_authoring.rs`).

## [0.2.1] - 2026-09-25

### Added

- Views read IFC2X3 models against the IFC2X3 schema (#51). A header
  declaring exactly `IFC2X3` binds the IFC2X3 table; every slot position,
  select, and domain comes from it. Before, every model was read with IFC4
  positions, so IFC2X3 files only worked where the two releases happen to
  share a slot.
  - `IfcRelAssociatesClassification.RelatingClassification` accepts IFC2X3
    `IfcClassificationNotationSelect` (notation or reference), and
    `RelatedObjects` is checked against IFC2X3 `IfcRoot`.
  - IFC2X3 `ItemReference` and `DocumentId` are read by the IFC4-named
    `identification()` accessors, since they keep position and meaning.
  - `IfcClassificationReference.ReferencedSource` follows the declared
    release: `IfcClassification` only in IFC2X3.
- `classification_schema(model)` reports the release reads bind to.
  `SchemaVersion` is re-exported.
- `ClassificationNotation`, `ClassificationView::notations()`,
  `notation()`, and `notation_values()`: an IFC2X3 notation's code is its
  facets' `NotationValue`s, in authored order, with no invented separator.
- `ClassificationError::NotInSchema`: an accessor for an attribute the
  release lacks (for example `description()` on an IFC2X3
  `IfcClassification`, or any `IfcExternalReferenceRelationship` read in an
  IFC2X3 model) fails with this instead of returning `Ok(None)`, which
  would read as "authored as empty".
- `ClassificationError::StructuredValue`: a text accessor that meets an
  attribute the release types as a record, such as an IFC2X3
  `IfcCalendarDate` `EditionDate`, returns the record id instead of
  rejecting a valid value as `InvalidValue`.
- `ClassificationError::MultipleSchemas`: a header declaring several
  schemas including IFC2X3 cannot be bound to one release.

### Unchanged

- IFC4 and undeclared (in-memory) models read exactly as in 0.2.0: every
  existing IFC4 test passes untouched. On two real IFC4 models, all 1,482
  classified objects resolve.
- IFC4X3 headers still read against IFC4, as in 0.2.0. Their own table is
  out of scope here.
- Authoring stays IFC4-only.
- Projections built with `try_new` have no model header, so they keep
  reading against IFC4.

### Verified

- On five real IFC2X3 models (two ArchiCAD, Solibri, a structural model,
  Revit), all 3,672 classified objects resolve their effective
  classifications, and every classification system (5) and reference (62)
  reads, IFC2X3 calendar edition dates included.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-classification-v0.3.0...HEAD
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-classification-v0.3.0
[0.2.2]: https://github.com/openbimrs/ifc/releases/tag/ifc-classification-v0.2.2
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-classification-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
