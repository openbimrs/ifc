# 0006 — Model/domain and model/serialization separation

- **Status:** Accepted
- **Date:** 2026-08-18
- **Deciders:** GeneralPawz, Hermes
- **Supersedes:** extends 0001, 0005

## Context

Two questions had to be settled before writing any IFC code, because both are
expensive to retrofit:

1. Where does *meaning* live? Does `IfcModel` know what a cost item is?
2. Where does *syntax* live? Is the model "the STEP model"?

IfcOpenShell answers the first by generating a class per entity per schema
version, which is the main source of its weight and forces a recompile per
schema. Many smaller libraries answer the second by making the parser the
model, which makes ifcXML a second parallel stack.

## Decision

**The model stores structure. Domain crates supply meaning. Codecs supply
syntax.**

```
ifc-step  ──┐                            ┌── ifc-cost
ifc-xml   ──┼── Codec ──> ifc-model <────┼── ifc-schedule    (views)
ifc-json  ──┘             (entities)     └── ifc-properties
```

- `ifc-model` holds `Entity { type_name, attributes: Vec<Value> }` and indices.
  It contains **no** `if type_name == "IFCWALL"` anywhere.
- Domain crates borrow `&Model` and interpret. They own no storage.
- `Codec` is a trait **in `ifc-model`**, implemented by codec crates. The model
  depends on no codec.
- The `ifc` facade exposes each domain as a cargo feature.

## Consequences

### Gained

- **Thin builds.** A file-mover compiles 26 crates; the full toolkit compiles
  51. A geometry-free build links no `glam` and no geometry kernel at all.
- **Lossless handling of data we do not understand.** Because entities are
  stored structurally, a file full of cost data parses and re-exports intact in
  a build with **no cost crate compiled**. If the model held a `CostItem`
  struct, dropping the feature would drop the data. This is verified, not
  assumed: `ifc-step/tests/roundtrip.rs::cost_data_survives_without_any_domain_crate_compiled`.
- **Schema-version tolerance.** `IfcBuildingElement` (IFC4) and `IfcBuiltElement`
  (IFC4x3) are just different strings. An entity type from a schema newer than
  this build still parses and round-trips.
- **Cheap new serializations.** ifcXML is a new crate implementing `Codec`,
  with no change to the model or to any consumer. Format conversion is "read
  with one, write with another".

### Paid

- **No compile-time entity typing.** `wall.attribute(2)` is not
  `wall.name()` on a generated struct, so a wrong attribute index is a runtime
  `None` rather than a compile error. Mitigated by putting slot constants in
  one `mod slot` per domain type, next to the schema citation.
- **An extra hop.** Reading a cost value goes through the view rather than
  directly off a struct field. Irrelevant at file scale; measured if it ever
  shows up in a profile.
- **`Value` is a boxed enum.** Owned rather than borrowed from the mmap, so a
  model outlives its source bytes and can be edited. Interning is the
  optimization if memory becomes a constraint; correctness came first.

## Enforcement

Convention would decay. These are tests:

| Test | Enforces |
| --- | --- |
| `ifc/tests/thin_build.rs::thin_build_excludes_every_domain_crate` | features actually gate the dependency graph |
| `...::selecting_cost_does_not_pull_in_unrelated_domains` | domains are independent |
| `...::thin_build_compiles_no_geometry_kernel` | geometry is genuinely optional |
| `...::the_model_does_not_depend_on_any_codec` | the layering is not inverted |
| `ifc-step/tests/roundtrip.rs::cost_data_survives...` | unknown data is preserved |

The dependency-graph tests read `cargo tree` output rather than the manifest,
because a manifest can declare an optional dependency that a feature
accidentally enables. An earlier version of this test read `cargo metadata`'s
package list and passed vacuously — every workspace member is listed there
regardless of features.

## Alternatives rejected

| Option | Why not |
| --- | --- |
| Generate a struct per entity per schema | IfcOpenShell's weight problem; recompile per schema version; 776 + 876 types |
| Put `Value` in `ifc-step` | ifcXML would need a parallel value model; cross-format conversion becomes lossy |
| Make the parser the model | Inverts the layering; a second serialization becomes a second stack |
| Domain structs stored in the model | Dropping a feature would silently drop data on export |
