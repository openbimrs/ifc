# openbim

Pure-Rust IFC and openBIM infrastructure. No C++ in the dependency graph.

One workspace, many independently published crates. Take a single standard, or
the facade with the features you need — the cost of what you do not use is
zero, because each standard is its own crate rather than a feature of a
monolith.

## Crates

### openBIM standards

| Crate | Standard |
| --- | --- |
| [`openbim`](packages/openbim) | Facade; one feature per standard |
| [`openbim-core`](packages/openbim-core) | Vocabulary shared across standards |
| [`openbim-ids`](packages/openbim-ids) | buildingSMART IDS |
| [`openbim-bcf`](packages/openbim-bcf) | BCF (BIM Collaboration Format) |
| [`openbim-icdd`](packages/openbim-icdd) | ISO 21597 ICDD |
| [`openbim-idm`](packages/openbim-idm) | ISO 29481-3 idmXML |
| [`openbim-loin`](packages/openbim-loin) | ISO 7817-3 / EN 17412-3 LOIN |
| [`openbim-dt`](packages/openbim-dt) | ISO 23387 data templates |

`icdd` and `loin` are also published as alias crates: pure re-exports, so the
standard is reachable under the name practitioners use.

### IFC

`openbim-ifc` is the facade (its lib target is named `ifc`, so call sites read
`use ifc::…`). Beneath it sit the `ifc-*` crates: `ifc-model` is the codec-free
entity graph, `ifc-step` and `ifc-xml` are codecs, and the domain crates are
borrowed projections over the model.

### Substrate

`openbim-codec-xml` and `openbim-codec-zip` carry the encoding substrate. They
sit below both layers, which is what lets the IFC layer and the standards share
XML and ZIP handling without the IFC layer depending on a standard.

## Status

**Reserved.** The structure, boundaries, and gates are in place; the codecs are
not implemented yet. `docs/ROADMAP.md` tracks what is real versus what is
scaffolded — capability claims here are meant to be checkable, not aspirational.

## Design

- [ADR 0015](docs/adr/0015-openbim-standards-as-separate-crates.md) — why one
  crate per standard rather than features of one crate.
- `packages/AGENTS.md` — the layering rules and the one-way dependency rule.

Architecture is enforced by tests, not convention: `scripts/gate.sh` builds
every standard in isolation and proves that enabling one does not drag in
another's dependencies.

## License

MIT
