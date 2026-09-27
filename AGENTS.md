# openbimrs/ifc

Pure-Rust IFC: schema tables, the entity graph, STEP and ifcXML codecs,
borrowed domain views, schema-checked authoring and validation, bridges into
the format-neutral Axiolid geometry model, and JavaScript, Python and C
bindings. This file is the only `AGENTS.md`; before editing a crate, read its
`README.md` and the `//!` docs of the module you touch.

## Layout

- `crates/`: every workspace crate, one directory each.
- `xtask/`: `cargo run -p xtask -- docs | todo | plans`, the docs generator
  and repository lints.
- `scripts/`: the gate, schema fetch, code generators and release helper.
- `docs/`: the VitePress site; ADRs in `docs/adr/`. Generated regions are
  rewritten by `cargo run -p xtask -- docs`, never by hand.
- `test/fixtures/`: small curated `.ifc` files; read its `README.md` first.
- `tools/`, `benchmarks/`: fixture generators and comparative measurements.
- `references/`: gitignored local checkouts (the normative IFC schemas and
  docs under `references/ifc-spec/`). Read-only, never a build dependency.

## Dependency rule

Dependencies point down: `ifc-model` and `ifc-schema` at the bottom, codecs
and domain views above them, the `openbim-ifc` facade above those, and the
bindings above the facade through `openbim-ifc-binding-core`. Sibling domain
crates never depend on one another; cross-domain work belongs in the facade.
The enforced truth is `crates/ifc-model/tests/package_architecture.rs`; the
generated map is `docs/architecture/crates.md`.

## Geometry boundary

IFC crates resolve units, references, placements and representation choice,
and preserve exact intent in neutral geometry values. They never tessellate,
heal or execute booleans. Only `ifc-geometry`, `ifc-alignment` and
`ifc-georef` may depend on `axiolid-*` crates, and only neutral representation
crates; execution providers are reachable only through `ifc-geometry`'s opt-in
`compile` features (ADR 0004, ADR 0012, ADR 0018).

## Behaviour rules

- Refuse with a typed error: unsupported, invalid, missing-reference and
  budget-exceeded are distinct. Never substitute geometry, a default or a
  guessed value silently.
- Unknown entities and attributes survive a codec round trip.
- Domain views borrow `&Model`; they never copy the entity graph.
- Mutation stages on a transaction: a refused edit leaves the model unchanged.
- Version-specific behaviour is explicit. Validation uses the declared
  release's own tables; IFC4 ADD2 TC1 is the domain-projection baseline unless
  a crate proves another layout.
- Schema claims come from `references/ifc-spec/`, never from memory. The
  schemas are CC BY-ND 4.0: `scripts/fetch-ifc-schemas.sh` fetches and
  checksums them; never vendor them or a trimmed derivative.
- Public types implement `Debug` and `Clone`; extensible public enums and
  errors are `#[non_exhaustive]`.

## Open work

Open work lives in GitHub issues (ADR 0016). A marker in code names its issue
as `TODO(#N)`; `cargo run -p xtask -- todo --check` rejects any other form. Do
not add plans, checklists or progress logs to the repository.

## Gate

Iterate with targeted tests, then run the full gate before a merge:

```bash
scripts/gate.sh            # lint, test, features, bindings; judge by exit code
```

It fetches the schemas and sets `IFC_SPEC_REQUIRED=1`, which turns every
schema-backed test that would skip without `references/ifc-spec/` into a
failure. Set it yourself when running those tests alone. The architecture
tests it runs:

```bash
cargo test -p ifc-model --test package_architecture
cargo test -p ifc-model --test progressive_context
cargo test -p ifc-model --test module_reachability
cargo test -p ifc-model --test no_monolithic_files
cargo test -p ifc-geometry --test declaration_manifest
cargo test -p ifc-geometry --test no_backend_dependency
cargo test -p ifc-geometry --test kernel_free_build
```

Mutation-verify a new architecture or context test before trusting it: break
the rule, watch the test fail, restore. A performance claim needs a committed
benchmark, its baseline environment and a measured comparison; a green gate is
not one.
