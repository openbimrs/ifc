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
