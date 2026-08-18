# AGENTS.md — ifc/

Pure IFC logic. Read `docs/adr/0001` before changing the dependency structure.

## Crates

| Crate | Role | Geometry? |
| --- | --- | --- |
| `ifc-schema` | Schema as data: entity table, supertype chain | none |
| `ifc-parser` | STEP/IFC-SPF reader: mmap, partition, parallel scan | none |
| `ifc-model` | Semantic views: type buckets, spatial tree, properties | none |
| `ifc-shape` | Lowers representation items to meshes | **traits only** |

## The one rule that matters

**No crate here may depend on a geometry backend** (`geom-cpu`, `geom-simd`,
`geom-gpu`, `geom-dispatch`). Only `geom-kernel` (traits) and `geom-core`
(data), and only from `ifc-shape`.

This is what makes the geom package swappable — the project's stated
requirement. It is **enforced**, not trusted: `ifc/shape/tests/no_backend_dependency.rs`
reads these manifests and fails the build on violation. That gate has been
verified to actually fail when a backend dep is added, so do not assume a green
run means the test is asleep.

If you need geometry in `ifc-parser` or `ifc-model`, the design is wrong —
that work belongs in `ifc-shape`.

## Why the separation earns its keep

A consumer doing a property audit, a quantity takeoff, or a schema validation
compiles **no geometry at all**. That is a capability the
IfcOpenShell+OpenCascade stack cannot offer, and it is a selling point worth
protecting.

## Parser pitfall (already paid for once)

Partition boundaries must resync to a record start (`#<digits>=`). Counting
paren depth from an arbitrary offset collapses the whole file into one
partition, because depth never returns to zero when you start mid-record. A
regression test for this is mandatory when the partitioner lands.

## Fixtures

`test/fixtures/` at the workspace root — see its `AGENTS.md`. Reach them with
`env!("CARGO_MANIFEST_DIR")` joined to `../../test/fixtures`; do not hardcode
absolute paths.
