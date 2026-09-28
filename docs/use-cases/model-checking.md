# Checking a model

This scenario takes an IFC file somebody else wrote and answers two questions
a checker has to answer before anything else: *is this file legal IFC for the
schema it declares?* and *what exactly does it assert about this element?* It
uses `ifc-validate` for the first and the exact lookups in `ifc-properties`
for the second.

Neither step fixes the file, and neither claims more than it checked: an
unchecked rule is reported as unchecked, a capped report says it was capped,
and a lookup that cannot prove its answer refuses instead of guessing.

## What the crate provides

Enable the facade features:

```bash
cargo add openbim-ifc --features validate,properties,schema
```

`schema` is needed only for choosing tables yourself (the findings-cap example
below); `validate_declared` picks them from the file.

## Declared-schema validation

Reading is permissive on purpose, so validation is a separate, explicit pass.
`validate_declared` resolves the file's own `FILE_SCHEMA` token to the bundled
IFC2x3 TC1, IFC4 ADD2 TC1, IFC4X1 FINAL, IFC4X2 FINAL or IFC4X3 ADD2 tables
and checks the model against exactly that release. No WHERE rule is registered
for IFC4X1 or IFC4X2 yet; their reports carry one `where.release` finding
saying so. A file that declares no schema, an unrecognised token,
or a release this build bundles no tables for is a `ValidateError`, never a
validation against a guessed release.

<!-- SNIPPET:model-checking-validate -->

```rust
use ifc::validate::{validate_declared, Severity};
use ifc::{Codec, StepCodec};

let model = StepCodec.read_bytes(bytes)?;
// IFC2X3, IFC4 or IFC4X3 tables, chosen by the file's FILE_SCHEMA.
// A file that declares none, or an unknown token, is refused.
let report = validate_declared(&model)?;

println!("{}", report.summary()); // "0 errors, 0 evaluation errors, 0 warnings, 2 unsupported"
for finding in report.sorted() {
    if finding.severity == Severity::Error {
        println!("{} at {}: {}", finding.rule, finding.path, finding.message);
    }
}
let clean = report.is_conformant() && !report.is_truncated();
```

<!-- /SNIPPET -->

Every finding carries a stable rule id (`structure.reference.dangling`,
`global.UniqueGlobalId`, …), a path to the entity or attribute, a severity and
a one-sentence message. `report.sorted()` orders findings by severity, rule
and path, so two runs over the same file produce identical output and a report
can be diffed in CI.

The four severities are not a scale:

| Severity | Meaning | Affects `is_conformant()` |
| --- | --- | --- |
| `Error` | The file breaks a schema requirement: a dangling reference, a missing required attribute, a wrong type, a duplicated `GlobalId` | yes |
| `EvaluationError` | A registered rule applies to an instance but could not be decided for it (an operand it cannot read, a missing target); the verdict is unknown, so it is never a pass | yes |
| `Warning` | Legal, but very likely a mistake | no |
| `Unsupported` | A rule *this validator* did not evaluate; a statement about the tool, not the file | no |

What is checked natively: references (dangling, and of the wrong kind in
entity slots, in SELECT slots and inside aggregates), required slots,
aggregate shape (a scalar where a `LIST` is declared and vice versa), entity,
select, defined-type and enumeration compatibility of each value and of each
aggregate member, abstract instantiation, scalar forms,
`STRING(n) FIXED` widths, unique `GlobalId`s and a single `IfcProject`, plus
the registered WHERE-rule predicates listed on the
[coverage page](/coverage#validation).

## The findings cap

Validation runs on files nobody on your side wrote. A pathological file with a
million dangling references must not turn a CI job into an out-of-memory
kill, so every run has a `Budget`. `validate` and `validate_declared` use
`Budget::DEFAULT`; `validate_with` takes an explicit one.

<!-- SNIPPET:model-checking-budget -->

```rust
use ifc::schema::ifc4;
use ifc::validate::{validate_with, Budget};

// Budget::DEFAULT caps stored findings; `validate` and
// `validate_declared` use it. Tighten it for a quick CI verdict.
let budget = Budget { max_findings: 1 };
let report = validate_with(&model, ifc4(), budget);

if report.is_truncated() {
    // Counts are now lower bounds: "1 error" means "at least 1".
    println!("stopped early: {} (at least)", report.summary());
}
```

<!-- /SNIPPET -->

The cap is a reporting limit, not a correctness shortcut. When it is hit the
report is marked truncated, and every count in `summary()` becomes a lower
bound. A gate that treats "conformant" as "passed" should also require
`!report.is_truncated()`, as the first example does.

## What is reported as unsupported

`ifc-validate` has no EXPRESS expression evaluator. Rather than let a clean
report mean "the rules we did not implement passed", each WHERE rule it knows
about is registered with an explicit state and the releases that declare it,
and the unimplemented ones are reported with severity `Unsupported` whenever
the file's release declares them and the file contains an instance of the
entity they constrain or of a subtype (global rules are always reported).

<!-- SNIPPET:model-checking-unsupported -->

```rust
use ifc::validate::where_rule::{Support, RULES};
use ifc::validate::Severity;

// What this run could not check, because the file uses what it constrains.
for finding in report.findings() {
    if finding.severity == Severity::Unsupported {
        println!("unchecked {}: {}", finding.rule, finding.message);
    }
}

// The whole registry, independent of any file.
for rule in RULES {
    if let Support::Unsupported(reason) = rule.support {
        println!("{} is never evaluated: {reason}", rule.id);
    }
}
```

<!-- /SNIPPET -->

The unsupported categories, each with a registered example on the
[coverage page](/coverage#validation):

| Category | Why it is not evaluated |
| --- | --- |
| Arbitrary EXPRESS `WHERE` expressions | There is no expression evaluator; only predicates provable from direct structure and scalars are implemented natively. |
| Aggregate bounds (`LIST [3:?]`) | The schema parser keeps whether an attribute is an aggregate, but not its bounds. |
| `INVERSE` semantics | Validation does not derive inverse relationships, so a rule whose form in some release depends on one is unsupported in every release. |
| Geometric consistency | Validation does not evaluate geometry. |

The registry is deliberately **not** exhaustive over every rule the bundled
schemas declare. A rule that is not registered is neither evaluated nor
reported, so even a report with zero errors and zero unsupported findings does
not mean full EXPRESS conformance. The [capability matrix](/capabilities)
states the same boundary.

## Exact property and quantity lookup

A checker asks questions such as "is `Pset_WallCommon.FireRating` set on this
wall, and to what?". The permissive property views in `ifc-properties` read
what they can; `exact_property` is the fail-closed counterpart built for
checking. It searches the property sets and quantity sets assigned to the
occurrence and, through `IfcRelDefinesByType`, to its type, with occurrence
values overriding inherited ones.

<!-- SNIPPET:model-checking-exact -->

```rust
use ifc::properties::{exact_property, ExactResolution, ExactValue};

// Pset_WallCommon.FireRating on this wall (occurrence, else its type).
match exact_property(&model, wall, Some("Pset_WallCommon"), "FireRating")? {
    ExactResolution::Present(property) => {
        println!(
            "{} = {:?} ({:?})",
            property.property_set, property.value, property.source
        );
    }
    ExactResolution::Absent => println!("proven absent"),
    _ => {}
}

// Quantities resolve the same way: a quantity is a property to a checker.
if let ExactResolution::Present(length) =
    exact_property(&model, wall, Some("Qto_WallBaseQuantities"), "Length")?
{
    if let ExactValue::Real(value) = length.value {
        println!(
            "{value} {:?} in unit {:?}",
            length.value_type, length.unit_id
        );
    }
}
```

<!-- /SNIPPET -->

The three outcomes are distinct on purpose:

- `Present` carries the value, its declared IFC type (a quantity reports its
  measure, such as `IFCLENGTHMEASURE`), an explicit unit if the file states one,
  the owning set and entity ids, and whether it came from the occurrence or a
  type. Enumerated, list, bounded, table and reference values arrive as
  composite values (`ExactValue::Enumerated`, `List`, `Bounded`, `Table`,
  `Reference`) whose members carry their own declared types.
- `Absent` is a *proven* absence: every assigned set was read completely and
  none held the name.
- An `ExactPropertyError` means the evidence did not allow an answer: a
  dangling reference, a malformed aggregate, an ambiguous duplicate set, more
  than one type assignment, a complex property, values that contradict the
  rules deciding how they read (a list of mixed types, table columns of
  unequal length), an unnamed predefined property set outside the named set
  that could hold the name, STEP
  diagnostics on the model, or a schema other than IFC2X3, IFC4 or IFC4X3
  ADD2. Treat it as "cannot tell", never as "absent".

A check that names its set or property by pattern, as an IDS facet may,
needs every match rather than one. `exact_properties_where(model, object,
select_set, select_property)` enumerates them with the same traversal and
refusals, and `exact_properties` lists everything. For one set name and one
property name the enumeration answers exactly what `exact_property` answers,
and an empty list is a proven absence.

Predefined property sets such as `IfcDoorLiningProperties` and
`IfcDoorPanelProperties` keep their values in entity attributes rather than
named properties. Both lookups read them by the attribute's schema name
(`LiningDepth`, `PanelOperation`), typed by the declaring release: lengths
as their measure type in the project unit, ratios as
`IFCNORMALISEDRATIOMEASURE`, enumerations as `ExactValue::Enum`, and an
unset optional attribute as `Null`. A set that states no `Name` is found
under its entity name. A door carries one panel set per leaf, which a
lookup by name cannot tell apart, so `exact_predefined_sets(model, door,
"IfcDoorPanelProperties")` lists every such set with its source instead.

The value is the file's. WHERE rules on the value, such as a non-negative
length, are not evaluated here; that is validation's job, and the rule may be
[unsupported](#what-is-reported-as-unsupported). Only the rules that decide
how a composite value reads are enforced, since without them there is no one
reading. Resolving `unit_id` to a
project unit is a separate call (`exact_unit`). Quantities are authored
assertions and are never recomputed from geometry.

## IDS lives elsewhere

buildingSMART Information Delivery Specifications — requirement files that say
which properties, classifications and materials an element must carry — are a
separate standard with their own repository,
[openbimrs/ids](https://github.com/openbimrs/ids). An IDS checker is built on
top of the lookups above; this repository provides the model, the validator and
the exact property resolution, not IDS parsing or IDS reporting.

## What remains application work

- Deciding which findings block a delivery, and suppressing known ones by rule
  id.
- Presenting findings to people: grouping, BCF issues, reports. Findings are
  values; printing policy is yours.
- Requirement checking against IDS or project rules, including the
  requirements this validator reports as unsupported.
- Geometry-derived checks, such as comparing an asserted area with a computed
  one (`ifc_properties::compare` reports the agreement; computing the area is a
  geometry service's job).

## Evidence

- `crates/openbim-ifc/tests/docs_model_checking.rs` runs every snippet on this page,
  including a truncated report, a refused file without a declared schema, and
  a refused lookup on a dangling property relationship.
- `crates/ifc-validate/tests/` covers the native rules and a corpus run;
  `crates/ifc-validate/src/where_rule/registry.rs` is the registry the coverage table
  is generated from.
- `crates/ifc-properties/tests/exact.rs`, `exact_quantities.rs`, `exact_values.rs`,
  `exact_predefined.rs`, `exact_predefined_forms.rs` and `exact_ifc2x3.rs`
  cover exact resolution per release.
