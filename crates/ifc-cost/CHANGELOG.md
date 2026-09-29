# Changelog -- ifc-cost

All notable changes to the `ifc-cost` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Added

- IFC2X3 cost schedules can carry their dates (#214): `DateTimeValue`
  carries a date either as IFC4/IFC4X3 `IfcDateTime` text
  (`DateTimeValue::Text`, `From<&str>`) or as one of the IFC2X3
  `IfcDateTimeSelect` records (`Date(CalendarDate)`, `Time(LocalTime)`,
  `DateAndTime(CalendarDate, LocalTime)`). In IFC2X3
  `create_cost_schedule_with_owner_history` stages the `IfcCalendarDate`,
  `IfcLocalTime` and `IfcDateAndTime` records for `SubmittedOn` and
  `UpdateDate` and references them, only once the schedule itself is
  accepted. Record forms are checked against the schema's rules before
  anything is staged (`IfcValidCalendarDate` with `IfcLeapYear`,
  `IfcValidTime`, and the component ranges), refused with `InvalidValue`.
  The types duplicate `ifc-schedule`'s, since sibling domain crates may not
  depend on each other.
- Every draft has a constructor and a setter per optional field, named
  after it: `CostItemDraft::new(global_id)`,
  `CostScheduleDraft::new(global_id)`,
  `NestingDraft::new(global_id, parent, children)`,
  `ScheduleAssignmentDraft::new(global_id, schedule, items)`,
  `QuantityDraft::new(kind, name, value)`; `CostValueDraft` keeps
  `monetary` and `Default` and gains setters.

### Fixed

- `create_cost_schedule_with_owner_history` no longer refuses every dated
  IFC2X3 schedule (#214): a record form is written as the
  `IfcDateTimeSelect` IFC2X3 declares. In IFC4 and IFC4X3 a record form is
  refused with `AuthoringValueType`, as text is in IFC2X3.

### Changed (breaking)

- `create_cost_value`, `create_monetary_unit` and
  `create_currency_relationship` bind the model's declared release and lay
  their records out by attribute name (#213). They wrote the IFC4 layout
  into every model. In IFC2X3 an `IfcCostValue` has eight attributes and
  requires `CostType`, written from `category` (IFC4 renamed `CostType` to
  `Category`); a composed value (`ArithmeticOperator`, `Components`) is
  refused with `AuthoringNotInSchema`. `IfcMonetaryUnit.Currency` is an
  `IfcCurrencyEnum` enumerator in IFC2X3 (a label it does not list is
  `AuthoringValueType`) and an `IfcLabel` from IFC4 on. An IFC2X3
  `IfcCurrencyRelationship` has five attributes and requires `RateDateTime`
  as an `IfcDateAndTime` record. IFC4 and IFC4X3 records are unchanged.
- `create_monetary_unit` takes the `&Model` (`create_monetary_unit(tx,
  model, currency)`), which it needs to bind the release.
- `CostValueDraft::applicable_date` and `fixed_until_date` are
  `Option<DateTimeValue>`, and `create_currency_relationship` takes
  `rate_date_time: Option<DateTimeValue>`: IFC4 and IFC4X3 text as before
  (`"2026-01-01".into()`; the draft setters take `impl Into<DateTimeValue>`),
  or an IFC2X3 record form, which the writer stages. A form the release does
  not declare is refused with `AuthoringValueType`.
- `CostView::schedules` returns `Result<impl Iterator<Item = CostSchedule>,
  CostError>` and every `CostSchedule` accessor reads its attribute by name
  in the model's declared release (#212). The IFC4 positions misread an
  IFC2X3 `IfcCostSchedule`: `PreparedBy` as the predefined type, the
  `SubmittedOn` record as the status and `SubmittedBy` as the
  identification. `identification` reads IFC2X3's `ID`, which IFC4 renamed.
  A header declaring IFC4X1, IFC4X2 or an unknown release is
  `CostError::UnsupportedSchema`, several `CostError::MultipleSchemas`
  (new variants); no header reads as IFC4.
- `CostSchedule::submitted_on` and `update_date` return
  `Option<AuthoredDateTime>`: IFC4/IFC4X3 text or the IFC2X3 date record
  (`AuthoredDateTime::Record`), never `None` for a stated IFC2X3 date.
- `CostSchedule::new(id, entity, release)` takes the release to read
  against and returns `Result`, refusing IFC4X1 and IFC4X2.
- `CostValueDraft`, `CostItemDraft`, `CostScheduleDraft`, `NestingDraft`,
  `ScheduleAssignmentDraft` and `QuantityDraft` are `#[non_exhaustive]`:
  build them with their constructors and setters instead of a struct
  literal. Their fields stay public to read and assign.
- `CostScheduleDraft::submitted_on` and `update_date` are
  `Option<DateTimeValue<'a>>` (were `Option<&'a str>`); text converts with
  `.into()` or through the setters, and IFC4/IFC4X3 output is unchanged.
- `#[non_exhaustive]` on the public enums and result structs a later
  release could extend: `CostItemType`, `CostScheduleType`,
  `CostValueKind`, `QuantityKind`, `ArithmeticOperator`, `UnitBasis` and
  `Consistency`. A `match` outside the crate needs a wildcard arm, and the
  structs can no longer be built outside it.

### Changed

- Depends on `ifc-schema` with its default features named explicitly
  (every bundled release), now that the workspace dependency turns them
  off for the facade's per-release features (#112).
- A model whose header declares `IFC4X1` or `IFC4X2` is refused with the
  existing unsupported-schema error. `ifc-schema` now bundles both
  releases, but no layout here is verified against them, so they are
  never read as IFC4 or IFC4X3.

## [0.2.3] - 2026-09-28

### Added

- `create_cost_item_with_owner_history`,
  `create_cost_schedule_with_owner_history`,
  `nest_cost_items_with_owner_history` and
  `assign_schedule_items_with_owner_history` (#202). Each takes a
  caller-supplied `IfcOwnerHistory` id, which IFC2X3 requires on every
  `IfcRoot`. It must be in the model or staged on the transaction
  (`MissingReference` otherwise) and be an `IfcOwnerHistory`
  (`WrongReferenceType`). None is ever invented. In IFC4 and IFC4X3 the
  record is the plain writer's with the reference in the optional slot.
- `CostAuthoringError::AuthoringValueType` and
  `CostAuthoringError::AuthoringRequired`, appended to the
  `#[non_exhaustive]` enum, so not breaking.

### Changed

- `create_cost_item`, `create_cost_schedule`, `nest_cost_items` and
  `assign_schedule_items` bind the model's declared release and lay their
  records out by attribute name from its table (#202), as quantity
  authoring already did. They wrote the IFC4 layout with `OwnerHistory` `$`
  into every model, which is invalid IFC2X3. **Behaviour change for IFC2X3
  callers:** they now refuse an IFC2X3 model with
  `AuthoringRequired { attribute: "OwnerHistory", .. }` and stage nothing;
  use the `*_with_owner_history` variants there. What IFC2X3 cannot hold
  is refused, never dropped: an `Identification`, `PredefinedType` or
  cost values on a cost item (`AuthoringNotInSchema`), a date string where
  it declares an `IfcDateTimeSelect` (`AuthoringValueType`), and a cost
  schedule without its required `ID` (written from `identification`) or
  `PredefinedType` (`AuthoringRequired`). A header declaring several
  schemas, or one without a bundled table, is refused with
  `MultipleSchemas` or `UnsupportedSchema`. A model without `FILE_SCHEMA`
  binds IFC4 as before. IFC4 and IFC4X3 records are unchanged, record for
  record.

### Fixed

- `assign_cost_quantities` accepts every instantiable subtype of the
  release's own `CostQuantities` declaration (`IfcPhysicalQuantity`), read
  from its table instead of a fixed list (#203). An IFC4X3
  `IfcQuantityNumber` was refused. IFC4 still refuses it, as IFC4 does not
  declare it. The slot is found by name, and an IFC2X3 cost item, which
  has no `CostQuantities`, is refused with `AuthoringNotInSchema` instead
  of written past its five attributes. A header that binds no single
  known release is refused as above.

## [0.2.2] - 2026-09-28

### Added

- `CostAuthoringError::MultipleSchemas`, `UnsupportedSchema`,
  `EntityNotInSchema` and `AuthoringNotInSchema` for release-bound quantity
  authoring (#190), and a re-export of `SchemaVersion`, which they name.
  `CostAuthoringError` is `#[non_exhaustive]`, so this is not breaking. The
  crate now depends on `ifc-schema` for the bundled release tables.

### Fixed

- `mutation::create_quantity` writes the quantity value bare (#190). It
  wrote `IFCQUANTITYAREA('Q',$,$,IFCAREAMEASURE(12.5),$)`. But
  `<Kind>Value` is declared with a defined measure type, not a SELECT, in
  IFC2X3, IFC4 and IFC4X3, and ISO 10303-21 writes a typed parameter only
  in a SELECT slot. It now writes `12.5`, and a count as the integer `4`.
  `CostQuantity::value` reads both forms.
- `mutation::create_quantity` binds the model's declared release, with the
  same rule and output as `ifc-properties`' `create_quantity` (#190). It
  wrote the IFC4 five-attribute layout into every model. Now attributes are
  placed by name from the release's table, so an IFC2X3 quantity has four.
  The signature is unchanged; the refusals are new:
  - a `formula` in IFC2X3 (`AuthoringNotInSchema`);
  - `QuantityKind::Number` outside IFC4X3 (`EntityNotInSchema`);
  - a header declaring several schemas (`MultipleSchemas`);
  - a header declaring one without a bundled table (`UnsupportedSchema`).

  A model without `FILE_SCHEMA` binds IFC4 as before. **Behaviour change:**
  such a model now refuses `QuantityKind::Number`, which IFC4 does not
  declare; declare IFC4X3 to author one. IFC4 and IFC4X3 records are
  otherwise unchanged apart from the bare value, and match `ifc-properties`
  byte for byte in all three releases.

## [0.2.1] - 2026-09-27

### Added

- `nesting_anomalies(model)` and `CostAnomaly::NestedTwice { item, kept,
  rejected, relation }` (#57). A cost item that two `IfcRelNests` place
  under different parents is reported; `Nests` is `SET [0:1]`.

### Fixed

- A cost item nested under two parents is no longer counted twice (#57).
  `parent_of` already returned the first parent, but `children_of` listed
  the item under both. So `descendants_of` and `rolled_up_total` included it
  under each parent, and summing over `roots()` double-counted its value.
  Now only the kept parent (first `IfcRelNests` in file order) lists it,
  and a child listed twice under one parent is listed once. Output changes
  only for files that violate the schema.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-cost-v0.2.3...HEAD
[0.2.3]: https://github.com/openbimrs/ifc/releases/tag/ifc-cost-v0.2.3
[0.2.2]: https://github.com/openbimrs/ifc/releases/tag/ifc-cost-v0.2.2
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-cost-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
