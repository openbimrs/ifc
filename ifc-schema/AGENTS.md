# ifc-schema instructions

Purpose: Parse and query EXPRESS schema metadata; it is metadata, not serialization and not the entity graph.

Follow `../AGENTS.md`. Read `PLAN.md` only when assigned implementation or
roadmap work; record progress and blockers there, not here.

## Boundary

Allowed production dependencies: `openbim-step` for its generic EXPRESS
syntax/model only; no production IFC crate. IFC version/profile integration
happens here and in consumers.

## Module ownership

- `express.rs`: thin adapter/re-export over `openbim_step::express`
- `registry.rs`: IFC-facing wrapper over `openbim_step::schema::SchemaGraph`.
  Supertype chains and Part 21 positional attribute order live in
  `openbim-step` -- none of that is IFC-specific. What stays here is IFC
  version identity, the bundled artifact, and the process-wide cache.
- `version.rs`: which IFC schema a file's `FILE_SCHEMA` token names
- `error.rs`: syntax/source diagnostics
- `artifact.rs` (ifc4 feature): versioned binary codec for a compiled schema
- `bundled.rs` (ifc4 feature): Schema::ifc4(), cached, decoded from data/

## Invariants

- Official EXPRESS files are input evidence, never runtime/build dependencies.
- Preserve source names and version identity; do not guess cross-version equivalence.
- Schema queries are deterministic and do not interpret instance values.
- `data/*.bin` holds compiled structural facts only (entity/attribute/type
  names), never normative EXPRESS source text, comments, or prose. The
  `generation` feature that produces it requires a user-supplied `.exp` file
  and never vendors that file into the crate or its published archive.

Keep `lib.rs` delegating, keep child modules crate-private until they own a real
public contract, and split view/data, traversal, mutation, and validation before
they grow together.

## Verification

Run targeted crate tests and clippy first, then the package architecture/context
gates from `../AGENTS.md`. Record exact exit evidence in `PLAN.md`.
