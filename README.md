# OpenBIM.rs IFC

Pure-Rust IFC infrastructure: schema metadata, entity graph storage, STEP and
ifcXML codecs, typed domain projections, validation scaffolding, and explicit
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

- schema-agnostic entity/value graph with unknown-data round-tripping;
- STEP parsing and writing;
- schema-checked authoring of new entities by attribute name (`ifc-author`);
- EXPRESS schema metadata parsing;
- ifcXML codec foundations;
- borrowed domain projections and geometry-lowering foundations;
- versioned PSD/QTO template catalog data.

Several domain crates intentionally remain architecture scaffolds: their module
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
- `openbim-ifc` is the optional-feature facade.

See `HERMES.md`, `AGENTS.md`, and the nested context files for executable
boundaries and verification commands.

## License

Code is MIT licensed. Embedded catalog data carries the license declared by its
own package and data notices.
