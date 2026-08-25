# OpenBIM.rs IFC

Pure-Rust IFC infrastructure: schema metadata, entity graph storage, STEP and
ifcXML codecs, typed domain projections, validation scaffolding, and explicit
bridges to the format-neutral [Axiolid](https://github.com/axiolid/axiolid-kernel)
geometry contracts.

This is the canonical IFC-family repository for
[OpenBIM.rs](https://github.com/openbimrs/openbim). The integration repository
pins a verified commit here as `packages/ifc`.

## Status

Implemented foundations include:

- schema-agnostic entity/value graph with unknown-data round-tripping;
- STEP parsing and writing;
- EXPRESS schema metadata parsing;
- ifcXML codec foundations;
- borrowed domain projections and geometry-lowering foundations;
- versioned PSD/QTO template catalog data.

Several domain crates intentionally remain architecture scaffolds. Their README,
`AGENTS.md`, and `PLAN.md` files distinguish compiled behavior from reserved
module ownership. No capability should be inferred from a module name alone.

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
