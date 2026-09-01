# ifc-geometry instructions

Purpose: Interpret all shape-affecting IFC data and lower exact intent into a format-neutral axiolid-model DAG.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

Allowed production dependencies: `ifc-model`, schema metadata, and neutral axiolid
value/representation crates; never `axiolid-kernel`, an algorithm crate, or a backend.

The neutral crates are optional, behind the default-on `lowering` feature. Only
`lower`, `Transform::to_geom`, and the `IfcBooleanOperator` conversion may
name them; everything else reads `ifc-model` slots and must
compile with the feature off. `tests/kernel_free_build.rs` checks the resolved
dependency graph, so a stray unconditional `use axiolid_*` fails the gate
rather than silently relinking the kernel for 2D consumers.

Placement resolution lives in `constraint::placement`, not `lower`. World
coordinates are needed by 2D consumers too, so the function must stay
reachable with `lowering` off; `tests/kernel_free_build.rs` enforces that.

Committed schema-derived artifacts live in `data/`, never `references/` --
that name is reserved for the unredistributable local schema checkout and is
rejected by `scripts/check-leakage.py`. See `data/NOTICE.md`.

## Module ownership

- `resource`, `curve`, `surface`, `solid`, `constraint`: borrowed geometry-resource views
- `select`, `rules`: EXPRESS membership and actionable semantic rules
- `units`, `transform`: source-number interpretation and project-space composition
- `input/` (planned): shape inputs from Profile, Representation, Material,
  Product, and Topology resources
- `lower/`: total IFC-to-`GeometryGraph` translation; no execution

## Invariants

- Preserve exact profiles, curves, surfaces, booleans, mapped instances, and n-gons; never tessellate in this adapter.
- Convert IFC units exactly once at the boundary; basis directions remain dimensionless.
- Output contains no IFC IDs; provenance is an external side table keyed by NodeId.
- Unsupported is typed and names the source entity; no panic or silent substitute.
- Recursive lowering appends to one session-owned graph builder. Family lowerers
  return `NodeId`; they do not freeze isolated child graphs.
- Direction vectors are validated and normalized at a documented boundary;
  never assume IFC direction ratios have unit magnitude.

Keep cross-resource projections attribute-scoped: shared `ifc-model` storage
does not make one feature crate the owner of an IFC entity. Split typed views,
resolution, lowering, mutation, and validation before they grow together.

## Verification

Run targeted tests/clippy, isolated build, and the package architecture/context
gates. Run **both feature columns**: `--no-default-features` must compile,
pass, and link no `axiolid-*` crate. The active-lowering vocabulary gate parses
Rust paths/imports (including root aliases, globs, and macro tokens); do not
replace it with substring scans. Geometry bridges also run declaration/corpus
coverage and the full gate.
