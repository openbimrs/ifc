# Changelog -- ifc-properties

All notable changes to the `ifc-properties` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Added

- `*_with_owner_history` variants of the predefined property-set writers
  (#202): `add_door_lining_properties_with_owner_history`,
  `add_window_lining_properties_with_owner_history`,
  `add_door_panel_properties_with_owner_history`,
  `add_window_panel_properties_with_owner_history`,
  `add_permeable_covering_properties_with_owner_history` and
  `add_reinforcement_definition_properties_with_owner_history`. Each takes
  the model and a caller-supplied `IfcOwnerHistory`, which IFC2X3 requires
  on every `IfcRoot`, validated as for #191 (`MissingEntity`,
  `AuthoringInvalid`); none is ever invented. The record is laid out by
  attribute name from the declared release's table, and each value is
  checked against the type that release declares for it. In IFC2X3 that
  refuses `LiningToPanelOffsetX/Y` (and a window's `LiningOffset`) with
  `AuthoringNotInSchema`, and a zero thickness, which IFC2X3 types as
  `IfcPositiveLengthMeasure` where IFC4 has `IfcNonNegativeLengthMeasure`,
  with `AuthoringInvalid`; a token outside the release's enumeration is
  refused too. In IFC4 and IFC4X3 the record is the old writer's with the
  owner history in its optional slot.
- `*_with_owner_history` variants of the template writers (#202):
  `add_property_set_template_with_owner_history`,
  `add_complex_property_template_with_owner_history` and
  `attach_template_with_owner_history`. They bind the model's release, so
  an IFC2X3 model, which declares no templates, is refused with
  `EntityNotInSchema`.

- `ExactValue::Complex` with `ExactComplexValue` and `ExactComplexMember`
  (#208): `exact_property` and the enumerations resolve an
  `IfcComplexProperty` or `IfcPhysicalComplexQuantity` as a present
  composite in IFC2X3, IFC4 and IFC4X3, with no value type, instead of
  refusing it with `UnsupportedProperty`. Members resolve as set members
  do, nested complexes included. New errors `ComplexCycle`,
  `ComplexTooDeep` and `ComplexBudgetExceeded` refuse a cycle, nesting
  past 16 levels and more than 10 000 nested members; a repeated member
  name is `InconsistentValues` where the release forbids it (`WR22`,
  `UniqueQuantityNames`).

### Changed

- The predefined property-set and template writers that take no model
  (`add_door_lining_properties`, ..., `add_reinforcement_definition_properties`,
  `add_property_set_template`, `add_complex_property_template`,
  `attach_template`) are unchanged: they write the IFC4 layout with
  `OwnerHistory` `$`, which is valid IFC4 and IFC4X3 (whose layouts of these
  entities are the same) and never valid IFC2X3. Without a model they
  cannot refuse IFC2X3; their documentation now says so, as #191 did for
  `add_property_set`. They now lay the IFC4 record out by attribute name
  from the IFC4 table instead of fixed slots; the output is identical. The
  lining writers moved to `pset/lining.rs` and `add_complex_property_template`
  to `pset/template_authoring.rs`; public paths are unchanged.

## [0.5.1] - 2026-09-28

### Added

- `add_property_set_with_owner_history`,
  `add_element_quantity_with_owner_history`,
  `attach_property_set_with_owner_history` and
  `attach_type_with_owner_history` (#191). Each takes the model and a
  caller-supplied `IfcOwnerHistory` id, which IFC2X3 requires on every
  `IfcRoot`. The id must be in the model or staged on the transaction and
  must be an `IfcOwnerHistory`; a missing one is refused with
  `MissingEntity`, another entity with `AuthoringInvalid`. None is ever
  invented. The record is laid out by attribute name from the declared
  release's table, and in IFC4 and IFC4X3 the reference is written into
  the optional slot. This follows `ifc-material`'s
  `associate_material_with_owner_history` (#77).
- `PropertyError::AuthoringRequired { entity, attribute, schema }`: the
  model's release requires an attribute the call leaves unset (#191).
  `PropertyError` is `#[non_exhaustive]`, so this is not breaking.

### Changed

- Type objects are no longer refused by the exact API (#193).
  `exact_property`, `exact_properties`, `exact_properties_where`,
  `exact_property_sets_where` and `exact_predefined_sets` answered an
  `IfcTypeObject` with `ExactPropertyError::InvalidQueryObject`, so a
  checker could not test an `IfcWallType` as IDS does. They now resolve
  the type object's own `HasPropertySets`: property sets, quantity sets
  and predefined sets, in IFC2X3, IFC4 and IFC4X3. Every subtype of the
  release's `IfcTypeObject` is accepted, IFC2X3 `IfcDoorStyle` and
  `IfcWindowStyle` included. The list is read and validated by the code
  that already reads an occurrence's inherited type sets. A result carries
  `ExactSource::Type(id)` with the queried object's id, which is what an
  occurrence of that type reports for the same set. `HasPropertySets = $`
  is a proven absence; `()` is refused with `MalformedAggregate`, and
  duplicate names with `DuplicateMatchingSets { source: Type(id), .. }` and
  `DuplicateMatchingProperties`, as for an inherited type. A type object
  named in `IfcRelDefinesByProperties.RelatedObjects` is still refused with
  `InvalidOccurrenceTarget`, also when it is the queried object: IFC2X3
  admits only `IfcObject` there, and IFC4 and IFC4X3 forbid it by
  `NoRelatedTypeObject` ("handled through the direct relationship
  HasPropertySets at IfcTypeObject"). Occurrence results are unchanged. No
  type or signature changes; callers that relied on the refusal now get
  answers, and `InvalidQueryObject` remains for what is neither an
  occurrence nor a type object. In `openbim-ifc`, `door_operation` and
  `window_operation` given a type object now refuse with `NotADoor` or
  `NotAWindow` instead of `Property(InvalidQueryObject)`.

### Fixed

- Quantity values are written bare (#190). `create_quantity`,
  `create_quantity_with` and `set_quantity_value` wrote
  `IFCQUANTITYAREA('A',$,$,IFCAREAMEASURE(12.5),$)`. But `<Kind>Value` is
  declared with a defined measure type (`IfcAreaMeasure`, ...), not a
  SELECT, in IFC2X3, IFC4 and IFC4X3, and ISO 10303-21 writes a typed
  parameter only in a SELECT slot. They now write `12.5` (a whole count as
  the integer `4`). `set_quantity_value` replaces a typed value it finds
  with the bare one. The readers (`quantity_set`, `exact_property`) still
  accept both forms. Code that inspects the written `Value` sees
  `Value::Real`/`Value::Integer` where it saw `Value::Typed`.
- `IfcRoot.OwnerHistory` is no longer written as `$` into an IFC2X3 model
  by `attach_property_set` and `attach_type` (#191). It is mandatory in
  IFC2X3 (`OPTIONAL` from IFC4 on), so those records were invalid.
  **Behaviour change for IFC2X3 callers:** both now refuse an IFC2X3 model
  with `PropertyError::AuthoringRequired { attribute: "OwnerHistory", .. }`
  and stage nothing; use the `*_with_owner_history` variants there.
  Both now bind the model's declared release the way quantity authoring
  does. So a model whose header declares several schemas, or one without a
  bundled table, is refused with `MultipleSchemas` or `UnsupportedSchema`
  where it used to be written in the IFC4 layout. `NoRelatedTypeObject`
  and the `IfcTypeObject` checks use the declared release's inheritance
  instead of IFC4's. IFC4 and IFC4X3 output is unchanged.
  `add_property_set` and `add_element_quantity` take no model, so they
  cannot see the release. They still write `$` and are documented as
  IFC4/IFC4X3 only. In IFC2X3, `attach_property_set_with_owner_history`
  refuses to attach a definition whose `OwnerHistory` is unset. Not
  changed: the template writers, whose entities IFC2X3 does not declare,
  and the predefined property-set writers, which write the IFC4 layout
  without a model.

## [0.5.0] - 2026-09-28

### Changed (breaking)

- `quantity_set`, `quantity_sets` and complex quantities list a simple
  quantity whose value is `$`, cut off by a truncated record, or not a
  number, as the new `Quantity::Unresolved` (#138). It sits in file order
  beside the others and carries the quantity's id, `Name`, `Description`,
  kind, stated unit and `Formula`, and a `reason`: the new
  `UnresolvedValue::Missing` or `UnresolvedValue::NotNumeric { found }`.
  It has no value field, so nothing can read it as 0. Since #107 such a
  quantity was reported but left out of `quantities`, so a caller listing a
  set could not see that it existed. `PropertyAnomaly::QuantityValueMissing`
  and `QuantityValueNotNumeric` are still reported, once each.
- `Quantity` is `#[non_exhaustive]`. Exhaustive matches need an arm for
  `Unresolved` and a wildcard arm. `UnresolvedValue` is `#[non_exhaustive]`
  too.
- `compare` returns `Comparison::NotComparable` for an unresolved quantity,
  and `stated_unit` returns its stated unit.
- A quantity without a readable value is now checked against `WR21`: a
  stated unit of the wrong kind is reported as
  `PropertyAnomaly::QuantityUnitMismatch`, as it is for a valued quantity.
- New `QuantityKind::Number` for IFC4X3 `IfcQuantityNumber` (#138), and
  `QuantityKind` is `#[non_exhaustive]`. Exhaustive matches need an arm
  for `Number` and a wildcard arm. In a model whose declared release is
  IFC4X3, the permissive readers now resolve `IfcQuantityNumber` as a
  `Simple` quantity, where they returned `Quantity::Unsupported`.
  `NumberValue` and `Formula` are located by name in that release's table.
  A `$` or non-numeric value is `Quantity::Unresolved`, and the anomalies
  are reported as for the other kinds. Its measure is `IfcNumericMeasure`.
  It has no WHERE rule, so there is no unit-kind check (`required_unit` is
  `None`) and a negative number is not a `NegativeQuantity`. `compare`
  compares it only against a computed `Number`, and `stated_unit` returns
  whatever unit it states. IFC2X3 and IFC4 do not declare the entity. In
  their models, and in a model without one known declared release, it
  stays `Quantity::Unsupported`.
- Quantity authoring binds to the model's declared release (#138).
  `create_quantity(tx, model, kind, name, value)` and
  `create_quantity_with(tx, model, kind, name, value, extras)` take the
  `&Model` they write into and return `Result<EntityId, PropertyError>`.
  They used to write the IFC4 five-attribute record into every model, so
  an IFC2X3 quantity got a `Formula` slot IFC2X3 does not declare.
  Attributes are now placed by name in the release's table: IFC2X3
  quantities have four attributes, IFC4 and IFC4X3 ones five. The binding
  is the one `ifc-material` uses. One recognised `FILE_SCHEMA` binds that
  release, and a model with no `FILE_SCHEMA` binds IFC4. Several
  declarations are refused with the new `PropertyError::MultipleSchemas`,
  and an unknown one with `UnsupportedSchema`. Also refused:
  - `QuantityKind::Number` outside IFC4X3, with `EntityNotInSchema`;
  - a `Formula` for an IFC2X3 model, with `AuthoringNotInSchema`;
  - a non-finite value, with `AuthoringInvalid`.

  A count is no longer truncated. A whole count is still written as an
  integer. A fractional one is written as a real where `IfcCountMeasure`
  is `NUMBER` (IFC2X3, IFC4), and refused where it is `INTEGER` (IFC4X3).
  `set_quantity_value` binds the same way. It writes the value slot named
  in the release, refuses a record whose attribute count is not the
  release's with `MalformedEntitySlots`, and repairs a `$` value (a
  `Quantity::Unresolved`) into a simple quantity. Nothing is staged on a
  refusal. The new `PropertyError` variants are additive, since the enum is
  `#[non_exhaustive]`; the signatures are the breaking part.
  `add_quantity_to_set`, `add_element_quantity` and
  `add_physical_complex_quantity` are unchanged, because `IfcElementQuantity`
  and `IfcPhysicalComplexQuantity` have the same attributes in all three
  releases.

The exact API is unchanged and agrees: `exact_property` refuses a `$` value
with `MissingValueSlot`, a non-numeric one with `UnsupportedValue` and a
truncated record with `MalformedEntitySlots`, in IFC2X3, IFC4 and IFC4X3.

- `PropertyTemplate` reads every attribute of both template entities by
  name from the declared release's table (#108) and gains `kind`
  (`PropertyTemplateKind::Simple` or `Complex`), `enumerators`,
  `secondary_unit`, `expression`, `access_state`, `usage_name` and
  `templates` (the nested `HasPropertyTemplates` of a complex template,
  read through the same bounded, cycle-aware traversal as complex
  properties, with `template(name)` to look one up). It is now
  `#[non_exhaustive]`, so code that builds it with a struct literal or
  destructures it exhaustively must change; later fields will not break
  callers again.

### Added

- `exact_property_sets_where(model, object, select_set)` and
  `ExactPropertySetEntry { name, set_id, source, members }`: the property
  sets and quantity sets an object carries whose names the selector picks,
  empty ones included (#186). `exact_properties_where` lists properties, so a
  matching set without any left no trace; an IDS property facet must fail
  on exactly that set. The traversal, type-over-occurrence order and
  refusals are those of `exact_properties_where`, except that a
  `HasProperties` or `Quantities` of `()` or `$` (invalid, both are
  `SET [1:?]`) is listed with `members == 0` instead of refused: the question
  is whether the set exists. A type set with an occurrence set's name is
  listed as well, since overriding works per property.

- `template_deviations(model)` compares every property set that an
  `IfcRelDefinesByTemplate` links to an `IfcPropertySetTemplate` with that
  template (#109), reading the model and its templates only, bound to the
  declared release (IFC4 or IFC4X3; IFC2X3 has no templates and is refused
  with `TemplateError::NoTemplates`). It returns a `TemplateReport` of
  `TemplateFinding`s, each naming the set, the template and the concrete
  mismatch: `MissingProperty`, `UnexpectedProperty` (matched by `Name`, as
  the IFC4 `IfcPropertySetTemplate` documentation states), `WrongForm` (the
  property's entity against the template's `TemplateType`, e.g. a single
  value where `P_ENUMERATEDVALUE` is prescribed), `WrongMeasureType`
  (`PrimaryMeasureType`, and `SecondaryMeasureType` for bounded and table
  values, against the declared type of each value or the referenced
  entity), `WrongSetKind` (a `QTO_*` template on an `IfcPropertySet`, a
  `PSET_*` one on an `IfcElementQuantity`), `WrongAttachment` (a
  `*_TYPEDRIVENONLY` set on an occurrence, an `*_OCCURRENCEDRIVEN` set on a
  type) and `OutsideApplicableEntity` (`IfcEntity[/PREDEFINEDTYPE]`
  entries, comma separated). Complex properties are compared with complex
  templates member by member. Quantity sets are checked the same way with
  `Q_*` templates. What the documentation leaves open is
  `TemplateFinding::Undecided` with an `UndecidedReason`, never guessed:
  an unknown or undocumented template type (IFC4X3 `Q_NUMBER`), an unknown
  measure type or `ApplicableEntity` entry, an object whose predefined
  type is unstated, a `[PerformanceHistory]` entry, a predefined property
  set. Malformed facts met on the way are `PropertyAnomaly`s in the same
  report. New types: `TemplateReport`, `TemplateFinding`, `MeasureRole`,
  `UndecidedReason`, `TemplateError`, all `#[non_exhaustive]`.
- `property_template_checked` and `property_set_template_checked` read a
  template bound to the declared release and report every malformed fact
  met (#108), refusing a model without a single supported release, an
  IFC2X3 model (`TemplateError::NoTemplates`), an absent entity and a
  non-template with `TemplateError`.
- `PropertyAnomaly::NotATemplate`, `PropertyAnomaly::SlotCountMismatch` and
  `PropertyAnomaly::MalformedAttribute` (#108, #109): a template member or
  `RelatingTemplate` that is no template, a record whose attribute count
  is not the release's, and an attribute value its declared type does not
  admit (including an enumeration constant the release does not define).
  `PropertyAnomaly` is `#[non_exhaustive]`, so this is not breaking.

### Fixed

- `property_template` read every template with the
  `IfcSimplePropertyTemplate` layout (#108), so an
  `IfcComplexPropertyTemplate`, including one written by
  `add_complex_property_template`, reported its `UsageName` as its
  `TemplateType` and never exposed its nested templates; and any entity at
  all read as a template. It now returns `None` for an entity that is not a
  simple or complex template in the declared release (the IFC4 table when
  the header names none it bundles, and nothing in an IFC2X3 file).
  `property_set_template`, `property_set_templates` and `template_of_set`
  read their attributes by name the same way.


## [0.4.1] - 2026-09-27

### Added

- `exact_property`, `exact_properties` and `exact_properties_where` resolve
  the attributes of predefined property sets (#149): `IfcDoorLiningProperties`,
  `IfcDoorPanelProperties`, `IfcWindowLiningProperties`,
  `IfcWindowPanelProperties` and every other `IfcPropertySetDefinition`
  that is neither a property set nor a quantity set, in IFC2X3, IFC4 and
  IFC4X3. A set's members are the attributes its entity declares below
  `IfcPropertySetDefinition`, named and typed as the declared release's
  table has them (`LiningThickness` is `IFCPOSITIVELENGTHMEASURE` in IFC2X3,
  `IFCNONNEGATIVELENGTHMEASURE` in IFC4). Values keep the provenance of a
  single value (`property_id` is the set's id) with no explicit unit, so a
  length resolves to the project unit through `exact_unit`; ratios are
  their ratio measure. New `ExactValue::Enum` carries an enumeration
  constant checked against the release's members, and `ExactValue::Entity`
  an entity reference (`ShapeAspectStyle`) checked but not followed. An
  unset optional attribute is `Present` with `ExactValue::Null` and its
  declared type; an unset required one is `MissingValueSlot`. A set that
  states no `Name` is found under its entity name
  (`IfcDoorLiningProperties`); such sets of one entity (a door's panel set
  per leaf) are ambiguous only for a member they share. New
  `exact_predefined_sets(model, object, entity)` returns every assigned set
  of that entity with all its attributes (`ExactPredefinedSet`, with
  `attribute(name)`), occurrence sets first, no override applied, for door
  operation geometry (#148); a name that is not a predefined set in the
  release is the new `ExactPropertyError::NotAPredefinedSet`. What cannot
  be read exactly is still refused with `UnsupportedDefinition`: a
  selected aggregate attribute (`ReinforcementSectionDefinitions`), and,
  as before, a set without `Name` that the set name does not select but
  that has an attribute of the requested name (#66). A predefined set's
  `Name` that is neither text nor `$` is now `MalformedName` rather than
  treated as unnamed.
- `exact_property`, `exact_properties` and `exact_properties_where` resolve
  `IfcPropertyEnumeratedValue`, `IfcPropertyListValue`,
  `IfcPropertyBoundedValue`, `IfcPropertyTableValue` and
  `IfcPropertyReferenceValue` (#150), which they refused with
  `UnsupportedProperty`. New `ExactValue` variants `Enumerated`, `List`,
  `Bounded`, `Table` and `Reference` carry them, with the new types
  `ExactTypedValue` (one `IfcValue` and its declared type),
  `ExactEnumeratedValue`/`ExactEnumeration` (selected values and the
  referenced `IfcPropertyEnumeration`), `ExactBoundedValue` (lower, upper,
  set point), `ExactTableValue`/`ExactTableRow` (rows, expression, defining
  and defined unit, interpolation), `ExactReferenceValue` and
  `ExactEntityRef` (usage name and target). `ExactProperty.value_type` is
  `None` for these; `unit_id` is the list's, the bounded value's or the
  enumeration's `Unit`. Every attribute is read by name from the declared
  release's table: IFC2X3 requires the value lists and the reference target
  and has no `SetPointValue` or `CurveInterpolation`; IFC4/IFC4X3 make them
  optional. A malformed kind is refused as a single value is, and the
  WHERE rules that decide how the values read (one type per list and
  between bounds, equal table columns, selected values drawn from the
  referenced enumeration) and `LIST OF UNIQUE` are enforced with the new
  `ExactPropertyError::InconsistentValues { entity, rule }`, where `rule` is
  the release's own label (`WR21` in IFC4, `WR1` in IFC2X3). Both enums are
  `#[non_exhaustive]`; callers that relied on the refusal now get answers.
  An `IfcComplexProperty` is still refused.
- `exact_properties(model, object)` and
  `exact_properties_where(model, object, select_set, select_property)`:
  exact enumeration of an object's properties and simple quantities (#78),
  for checks such as IDS property facets that name sets and properties by
  pattern. Each `ExactPropertyEntry` carries the member's name and the same
  `ExactProperty` (provenance, set, value type, unit, value) that
  `exact_property` reports. The traversal, model and assignment validation
  and refusals are those of `exact_property`, and for one set name and one
  property name the result equals its answer (`[x]` for `Present(x)`, empty
  for `Absent`, the same error otherwise). An inherited property is left
  out when an occurrence set of the same name selects a property of the
  same name. An empty result is a proven absence. Only selected members
  must resolve: an unselected `IfcPropertyEnumeratedValue` or complex
  quantity does not refuse the answer, but every member of a selected set
  must be well formed. Additive; nothing existing changes.
- `exact_property`, `exact_schema` and `exact_unit` resolve IFC4X3 ADD2
  models (#76), bound to the bundled IFC4X3 ADD2 table. They were refused
  with `ExactPropertyError::UnsupportedSchema`. The header tokens are those
  `ifc_schema::SchemaVersion::from_header_token` maps to that release
  (`IFC4X3_ADD2` and `IFC4X3`). Tests pin the IFC4X3 differences:
  `IfcRelDefinesByProperties.RelatedObjects` admits any non-type
  `IfcObjectDefinition` (an `IfcProject` too), `IfcPropertySetDefinitionSet`
  is traversed under its `SET [1:?]` rules, `IfcTypeObject.HasPropertySets`
  may be `$` but not empty, `IfcQuantityNumber` resolves as
  `IFCNUMERICMEASURE` only in IFC4X3, and `IfcDerivedUnit` has IFC4X3's four
  attributes. `IfcDimensionsForSIUnit`/`IfcCorrectDimensions` for IFC4X3 are
  checked against the EXPRESS source, so the farad keeps IFC4's
  dimensions. Callers that relied on IFC4X3 being refused now get answers;
  the result type is unchanged.
- `PropertyAnomaly::MemberNotReference` and `PropertyAnomaly::DuplicateMember`
  (#137). `property_set_checked`, `property_checked`, `quantity_set`,
  `quantity_sets`, `property_sets_by_object` and `resolved_properties`
  report an item of `HasProperties`, `Quantities` or `HasQuantities` that is
  not an entity reference, and a member listed twice, instead of skipping
  the one and reading the other twice. `PropertyAnomaly` is
  `#[non_exhaustive]`, so this is not breaking.

- `property_checked` and `property_set_checked`: the values of `property`
  and `property_set`, together with a `PropertyAnomaly` for every member
  they could not resolve (#107). `property_sets_by_object` and
  `resolved_properties` now include these anomalies, once per set.
- `PropertyAnomaly` variants for what nested and malformed members used
  to lose silently (#107). `PropertyAnomaly` is `#[non_exhaustive]`, so
  this is not breaking:
  - `ComplexCycle`: a complex property or complex quantity reaches itself
    again through its members, at any cycle length.
  - `ComplexTooDeep`: complex nesting deeper than 16 levels.
  - `ComplexBudgetExceeded`: one read followed more than 10,000 nested
    member references.
  - `MissingMember`: a set or complex entity lists an id absent from the
    file.
  - `QuantityValueMissing` / `QuantityValueNotNumeric`: a simple quantity
    whose value attribute is `$`, absent, or not a number.

### Fixed

- A member listed twice in a property set, quantity set, complex property
  or complex quantity was resolved twice (#137). It is now resolved once,
  at its first position, and reported as `DuplicateMember`; a property set
  therefore no longer reports such a repeat as a `DuplicatePropertyName`
  of itself.
- Complex properties and complex quantities guarded nesting only against a
  DIRECT self-member and a bare depth of 16 (#107). A longer cycle, or
  nesting past the depth, was silently truncated to an empty member list,
  and a densely cyclic file cost exponential work (a clique of 8 complex
  properties followed 7^16 paths) before the depth stopped it. Members are
  now followed along a tracked path with a depth bound and a member
  budget, and every cut is reported. A member shared by two complex
  properties (legal: `IfcProperty.PartOfComplex` is `SET [0:?]`) is still
  resolved under both and is not a cycle.
- `quantity_set` and `quantity_sets` no longer drop, without a word, a
  simple quantity with a missing or non-numeric value, or a member id
  absent from the file (#107). Such members are still left out of
  `quantities` (a `Quantity::Simple` needs a number), but each is now
  reported.
- The `property` documentation claimed over-deep nesting yields
  `PropertyValue::Unsupported`; it never did. It now states what happens.

## [0.4.0] - 2026-09-26

### Added

- `exact_unit` maps `IFCSECTIONALAREAINTEGRALMEASURE` to
  `SECTIONAREAINTEGRALUNIT`, whose name differs from the measure's.
  Before, it was refused as unmapped. The pairing follows the measure's
  definition (m^5) and the IFC4 annex E structural example.
- The permissive views report the duplicates they resolve (#58).
  `exact_property` already refuses these files; the permissive views now
  keep a documented winner and say so:
  - `PropertyAnomaly::TypedTwice`: an object with two `IfcRelDefinesByType`
    (IFC4 `IsTypedBy` is `SET [0:1]`; IFC2X3 `IfcObject` WR1).
  - `PropertyAnomaly::DuplicateSetName`: two same-named property sets on one
    occurrence or one type (IFC4 `UniquePropertySetNames`). It is reported
    once per owner, including for a type that no occurrence uses.
  - `PropertyAnomaly::DuplicatePropertyName`: two same-named properties in
    one set (IFC4 `UniquePropertyNames`, IFC2X3 `WR32`), reported by
    `property_sets_by_object`.

### Changed (breaking)

- `PropertyAnomaly` is `#[non_exhaustive]`, so future checks can add
  variants. Exhaustive matches need a wildcard arm.
- `template_of_set` returns `BTreeMap<EntityId, Vec<EntityId>>`: every
  template defining a set, ascending by id and without repeats (#60).
  `IfcPropertySetDefinition.IsDefinedBy` is `SET [0:?] OF
  IfcRelDefinesByTemplate`, so several templates are legal, but the old
  `BTreeMap<EntityId, EntityId>` kept only the last and silently dropped the
  others, on valid files. A caller that wants one template must now choose,
  and is not handed an arbitrary one.

### Fixed

- An object typed twice kept the **last** `IfcRelDefinesByType` in file
  order and said nothing. It now keeps the first by relationship id, and
  inherits only that type's sets.
- Two same-named property sets on one owner made the later one win. For
  occurrences, the earlier one was misreported as `shadowed`: shadowing
  means an occurrence set overriding a type set. The set with the lower id
  now wins within each route. An occurrence set still overrides a
  same-named type set, and that is not reported.

### Fixed

- `exact_property` no longer reports `Absent` for a quantity in a same-named
  `IfcElementQuantity` (#66). Quantity sets were skipped, so a checker
  asking for `Qto_WallBaseQuantities.Length` (IDS treats quantities as
  properties) got a confident "missing".
  - Simple quantities now resolve exactly. `value_type` is the release's
    declared measure of the value attribute (`IFCLENGTHMEASURE`), and
    `unit_id` is the quantity's `IfcNamedUnit`.
  - Complex, duplicated or malformed quantities are refused, as are a
    same-named property set and quantity set.
  - A predefined property set (`IfcDoorLiningProperties`, …) whose own
    attribute carries the requested name is refused with
    `UnsupportedDefinition` instead of being skipped into `Absent`.
  - A quantity set with no `Name` is refused (`MalformedName`), because it
    could be the set asked for.
  - Results change from `Absent` to `Present` or an error only where the
    old answer was unproven.

## [0.3.0] - 2026-09-26

### Added

- `exact_unit(model, measure_type, explicit_unit)` resolves a measure's
  effective unit to an exact SI conversion (#53). It uses the explicit unit,
  otherwise the project default, and returns
  `value_si = value * scale + offset` with the SI dimensional exponents.
  Conversion-based chains, derived units, the gram, and degrees Celsius are
  resolved. Count and ratio measures get a dimensionless answer.
- It refuses rather than guesses (`ExactUnitError`):
  - unknown prefixes, unit names or measure types
  - duplicate or missing project units
  - cyclic or over-deep conversion chains
  - a unit whose name, declared `Dimensions` or factor unit contradicts its
    type
  - an explicit unit of the wrong type
  - offset units
- It binds to the declared release like `exact_property` and reads that
  release's `IfcDimensionsForSiUnit`/`IfcCorrectDimensions`. They differ: IFC2X3
  TC1's farad fails its own capacitance check, and it is reported as such.
- `UnitKind::Conversion::offset` keeps
  `IfcConversionBasedUnitWithOffset.ConversionOffset`, which was dropped.

### Changed (breaking)

- `UnitKind::Si::prefix_exponent` is now `Option<i32>`. An unrecognised
  prefix is `None` instead of `0`, which read `.BOGUS.` as "unprefixed" and
  scaled by 1.0.
- `UnitKind::si_scale` raises the prefix to the unit's power:
  `MILLI SQUARE_METRE` is 1e-6 (was 1e-3), and `MILLI CUBIC_METRE` is 1e-9.
  It is `None` for an unknown prefix.
- `UnitKind::Conversion` gains the `offset` field, so exhaustive patterns
  must name it or use `..`.

## [0.2.1] - 2026-09-25

### Added

- `exact_property` resolves IFC2X3 models (#48). It binds to the release the
  single `FILE_SCHEMA` token declares, IFC2X3 TC1 or IFC4 ADD2 TC1, and reads
  every structural fact from that release's bundled table: slot counts,
  entity domains (IFC2X3 `RelatedObjects` is `IfcObject`, IFC4's is
  `IfcObjectDefinition`), type objects (IFC2X3 `IfcDoorStyle` and
  `IfcWindowStyle` carry inherited properties like any other type), and the
  `IfcValue`/`IfcUnit` selects. An 8-slot IFC2X3 `IfcWall` resolves under an
  IFC2X3 header and is a slot mismatch under IFC4, and the reverse.
- `exact_schema(model)` reports the release a resolution binds to, so a
  consumer can bind vocabulary per release without re-parsing the header. It
  fails exactly where `exact_property` fails at model level.
  `SchemaVersion` is re-exported.
- `ExactPropertyError::NotInSchema` names a construct the declared release
  does not define, such as an `IfcDoorType`, an `IfcPropertySetDefinitionSet`,
  or an `IFCBINARY` value in an IFC2X3 file. A file that mixes releases fails
  closed instead of being read with the wrong table.
- `ExactPropertyError::UnsupportedRelationship`: a proper subtype of
  `IfcRelDefinesByProperties` or `IfcRelDefinesByType` that relates the
  queried object, such as IFC2X3 `IfcRelOverridesProperties`, is refused.
  Before, it was skipped as if absent, and the overridden value was returned.

### Changed

- IFC4X3 headers stay `UnsupportedSchema`: the table is bundled, but exact
  semantics for it are not verified yet (#33 tracks IFC4X1/IFC4X2).
- On five real IFC2X3 models, every sampled lookup failed with
  `UnsupportedSchema` before. Now 715 of 726 sampled lookups resolve; the other
  11 are `DuplicateMatchingSets`, where a property asked for by name alone is
  carried by two sets assigned to the same object. The same refusal occurs on
  the IFC4 control models.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-properties-v0.5.1...HEAD
[0.5.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-properties-v0.5.1
[0.5.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-properties-v0.5.0
[0.4.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-properties-v0.4.1
[0.4.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-properties-v0.4.0
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-properties-v0.3.0
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-properties-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
