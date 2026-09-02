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
- `export.rs`: deterministic expanded and direct-declaration catalogs for exact
  bundled releases
- `version.rs`: which IFC schema a file's `FILE_SCHEMA` token names
- `error.rs`: syntax/source diagnostics
- `artifact.rs` (ifc4 feature): versioned binary codec for a compiled schema
- `bundled.rs` (ifc4 feature): `ifc2x3()`, `ifc4()`, and `ifc4x3()` are cached,
  decoded from independent artifacts in `data/`. `for_version` maps a parsed
  `FILE_SCHEMA` token only to its exact table; consumers must refuse any future
  recognised-but-unbundled version rather than fall back.

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

## Regenerating an artifact

Requires a normative `.exp`, which is never committed:

```
cargo run -p ifc-schema --features generation --bin ifc-schema-generate -- \
  ifc2x3 references/specs/ifc2x3-tc1/IFC2X3_TC1.exp
cargo run -p ifc-schema --features generation --bin ifc-schema-generate -- \
  ifc4 references/specs/ifc4-add2-tc1/IFC4.exp
cargo run -p ifc-schema --features generation --bin ifc-schema-generate -- \
  ifc4x3 references/specs/ifc4x3-add2/IFC4X3_ADD2.exp
```

The tool refuses a source whose entity/type counts do not match the selected
schema, and a test pins those counts to the committed artifacts.

## Verification

Run targeted crate tests and clippy first, then the package architecture/context
gates from `../AGENTS.md`. Record exact exit evidence in `PLAN.md`.
