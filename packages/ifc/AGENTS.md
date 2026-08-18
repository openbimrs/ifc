# AGENTS.md — packages/ifc/

Pure IFC logic. Read `docs/adr/0001` (the split) and `docs/adr/0005` (why these
crates exist) before changing the dependency structure.

The authority for every schema question is `references/ifc-spec/` — the official
EXPRESS schemas. Re-derive facts with `grep` rather than trusting a doc.

## Crates

| Crate | Role | Geometry? |
| --- | --- | --- |
| `ifc-schema` | Schema as data: entity table, supertype chain, attributes | no |
| `ifc-step` | STEP/IFC-SPF reader: mmap, partition, parallel scan | no |
| `ifc-model` | Indexed semantic views: type buckets, spatial tree, rels | no |
| `ifc-properties` | Property sets, quantities, units | no |
| `ifc-material` | Layer sets, profile sets, constituents, usage | no |
| `ifc-classification` | Classification, documents, libraries, external refs | no |
| `ifc-style` | Presentation: styles, colours, textures, layers | no |
| `ifc-structural` | Structural analysis model: members, actions, loads | no |
| `ifc-resource` | Labour, equipment, material, crew resources | no |
| `ifc-systems` | Distribution systems, ports, connectivity graph | no |
| `ifc-cost` | Cost items, schedules, values | no |
| `ifc-schedule` | Tasks, sequencing, work calendars | no |
| `ifc-validate` | WHERE rules, cardinality, GUID + reference integrity | no |
| `ifc-geometry` | Lowers representation items to meshes | **traits only** |
| `ifc-georef` | Map conversion, CRS, local ↔ map transform | **traits only** |
| `ifc-alignment` | IFC4x3 alignments, linear placement, spirals | **traits only** |

Sizing evidence (IFC4 entity counts): presentation/style 48, structural 39,
ports/systems 23, materials 22, resources 21, classification/document 12,
georeferencing 8. IFC4x3 adds 14 alignment entities plus transition spirals.

## Layering — the two separations

`ifc-model` is the centre. It knows **no domain semantics** and **no
serialization**:

```
ifc-step  --+                            +-- ifc-cost
ifc-xml   --+-- Codec --> ifc-model <----+-- ifc-schedule    (views)
ifc-json  --+             (entities)     +-- ifc-properties
```

- **Codecs depend on the model, never the reverse.** `Codec` is a trait *in*
  `ifc-model`; `ifc-step` implements it. Adding ifcXML changes nothing here.
- **Domain crates are views.** They borrow `&Model` and interpret entities.
  They own no storage, so removing one cannot lose data.
- **The model stores structure only.** `Entity { type_name, attributes }`. No
  `if type_name == "IFCWALL"` anywhere in `ifc-model` — enforced by
  `ifc-model/tests/model_invariants.rs`.

The payoff, verified in `ifc-step/tests/roundtrip.rs`: **a file full of cost
data parses and re-exports intact in a build with no cost crate compiled.**
Read `docs/adr/0006` before changing any of this.

## Module layout — modular by default, enforced

Every crate is split into focused modules; `lib.rs` declares and documents them
and holds no behaviour. The intent is that a file is never the place a whole
subsystem lives:

- `ifc-step` — one module per pipeline stage (`lexer`, `partition`, `scan`,
  `resolve`, `escape`, `value`, `header`, `reader`).
- `ifc-schema` — one per schema concern (`entity`, `attribute`, `types`,
  `inheritance`, `registry`, `express`, `version`).
- `ifc-geometry` — **one per representation family** (`swept`, `brep`, `csg`,
  `tessellated`, `mapped`, `profile`, `placement`, `opening`, `units`,
  `context`). This is where a monolith would otherwise form: IFC4 has ~119
  curve/surface entities and 11 swept-solid forms.
- Every crate has an `error` module; failures are named per domain.

Two tests in `ifc-model/tests/no_monolithic_files.rs` enforce this and are
mutation-verified (a 900-line file and a fat `lib.rs` both make them fail):

| Test | Rule |
| --- | --- |
| `no_source_file_is_a_monolith` | no `.rs` file over 800 lines, workspace-wide |
| `lib_rs_delegates_rather_than_implements` | `lib.rs` with modules carries <40 lines of code |

If a file approaches the limit, split it by responsibility rather than raising
`MAX_LINES`. `EXEMPT` exists but is empty on purpose — an entry needs a written
justification.

## The invariant, and how it is enforced

`packages/ifc/` depends on the geometry **contract**, never on an
implementation. Geometry-touching crates are an explicit allowlist
(`MAY_USE_GEOMETRY` in `ifc-geometry/tests/no_backend_dependency.rs`):
`ifc-geometry`, `ifc-georef`, `ifc-alignment`. Everything else must compile with
no geometry stack at all — that is what lets a property/quantity/cost consumer
stay lightweight.

Three tests enforce it, and all three are mutation-verified:

- a non-allowlisted crate gaining `geom-*` → fail
- an allowlisted crate enabling a backend feature → fail
- the allowlist naming a crate that does not exist → fail

If you need geometry in a crate that is not on the list, the design is probably
wrong. If it genuinely belongs, add it to the allowlist **with a reason**.

## Why alignment is separate from geometry

IFC4x3's alignment entities are civil-infrastructure geometry: clothoids, cant,
station-based placement. A consumer working on buildings should not compile
numerical clothoid integration, so it is its own crate rather than part of
`ifc-geometry`.

## Pitfalls

- **Partition boundaries must resync to a record start (`#<digits>=`).**
  Counting paren depth from an arbitrary offset collapses the file to one
  partition, because the depth never returns to zero mid-record. This is a
  validated finding from the sibling `../vendor/solibri` parser.
- **Entity names differ across schema versions.** IFC4x3 renamed
  `IfcBuildingElement` → `IfcBuiltElement` and dropped `IfcProxy`, the
  `*StandardCase` family, `IfcDoorStyle`/`IfcWindowStyle`. Never hardcode a
  single version's name — resolve through `ifc-schema`.
- **A file can parse cleanly and still be invalid.** Parsing and conformance are
  different questions; `ifc-validate` owns the second so the parser hot path
  stays fast.
- **Fixture paths** are `env!("CARGO_MANIFEST_DIR")` joined with
  `../../../test/fixtures` (three levels up from `packages/ifc/<crate>`).

## Status

Scaffold. `ifc-step` recognises STEP headers and schema tokens against the
committed fixtures; `ifc-geometry` carries the `ShapeLowerer` seam generic over
`K: MeshBoolean`. Everything else is documented intent — read the crate's own
module doc before assuming behaviour exists.
