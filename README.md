# OpenBIM.rs IFC

Pure-Rust IFC infrastructure: schema metadata, entity graph storage, STEP and
ifcXML codecs, typed domain projections, schema-aware validation, and explicit
bridges to the format-neutral [Axiolid](https://github.com/axiolid/axiolid-kernel)
geometry contracts.

This is the canonical IFC-family repository for
[OpenBIM.rs](https://github.com/openbimrs/openbim). The integration repository
pins a verified commit here as `packages/ifc`.

**📖 [Documentation](https://openbimrs.github.io/ifc/)** —
[capabilities and status](https://openbimrs.github.io/ifc/capabilities) ·
[architecture](https://openbimrs.github.io/ifc/architecture/) ·
[roadmap](https://openbimrs.github.io/ifc/project/roadmap) ·
[API on docs.rs](https://docs.rs/openbim-ifc)

## Status

Read the [capability matrix](https://openbimrs.github.io/ifc/capabilities)
before planning work against this repository. It states, per capability,
whether behaviour is implemented, partial, scaffold, or absent, with the file
that proves it.

Implemented foundations include:

- schema-agnostic entity/value graph with unknown-data round-tripping and
  transactional structural edits;
- STEP and ifcXML parsing/writing;
- schema-checked authoring of new entities by attribute name (`ifc-author`);
- bundled IFC2x3 TC1 and IFC4 ADD2 TC1 structural schema metadata;
- declared-schema validation with explicit unsupported-rule reporting;
- borrowed property, quantity, material, schedule, systems, and spatial views;
- material and quantity authoring staged through caller-owned transactions;
- geometry selection/lowering foundations and versioned PSD/QTO catalogs.

Some domain crates intentionally remain architecture scaffolds: their module
trees declare ownership of a schema area without implementing it. Their README,
`AGENTS.md`, and `PLAN.md` files distinguish compiled behavior from reserved
module ownership, and the capability matrix counts the stubs per crate. **No
capability should be inferred from a module or crate name alone.**

## Use

```toml
[dependencies]
openbim-ifc = "0.1"
```

The library target is named `ifc`:

```rust
use ifc::{Model, StepCodec};
```

The default facade feature enables STEP only. Add domain and geometry features
explicitly.

Geometry comes in two sizes, because a drawing consumer should not compile a
solid modeller:

| Feature | Gets you | Links a kernel |
| --- | --- | --- |
| `geometry-select` | representation contexts, plan/body selection, placements, units | no |
| `geometry` | the above plus lowering into the neutral geometry DAG | yes |

Using `ifc-geometry` directly, `default-features = false` is the same split.
Both are checked against the resolved dependency graph, not the manifest.

## Develop

Requires Rust 1.88 or newer.

```bash
git clone https://github.com/openbimrs/ifc.git
cd ifc
./scripts/gate.sh
```

The official IFC schema artifacts are not vendored. Tests which need locally
checked-out normative references skip honestly when those references are absent;
committed generated manifests and redistributable regression fixtures remain
self-contained.

## Architecture

- `ifc-model` owns the serialization-independent record graph.
- `ifc-schema` interprets EXPRESS schema metadata.
- `ifc-step` and `ifc-xml` are codecs over the model.
- domain crates expose borrowed projections rather than duplicate object graphs.
- only `ifc-geometry`, `ifc-georef`, and `ifc-alignment` may depend on neutral
  Axiolid representation crates; no IFC crate selects a CPU/GPU backend.
- `ifc-geometry` keeps those crates optional: representation selection is slot
  reading over `ifc-model` and links no geometry code at all.
- `openbim-ifc` is the optional-feature facade.

See `HERMES.md`, `AGENTS.md`, and the nested context files for executable
boundaries and verification commands.

## License

Code is AGPL-3.0-or-later licensed. Embedded catalog data carries the license declared by its
own package and data notices.
