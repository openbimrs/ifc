# openbim-ifc

Feature-gated facade for the pure-Rust OpenBIM.rs IFC crates: the entity
graph, STEP and ifcXML codecs, schema tables, validation, authoring, domain
views and geometry, each behind its own cargo feature.

```bash
cargo add openbim-ifc
cargo add openbim-ifc --features properties,spatial
```

The library target is named `ifc`, so code imports it with `use ifc::...`.
The default feature enables only the model and the STEP codec; every domain
and geometry capability is opt-in, so a build compiles only what it asks for.

- API documentation: [docs.rs/openbim-ifc](https://docs.rs/openbim-ifc)
- Features and the crate each enables: [reference page](https://openbimrs.github.io/ifc/reference/crates/openbim-ifc)
- What is implemented, per capability: [capability matrix](https://openbimrs.github.io/ifc/capabilities)
- Other languages (JavaScript, Python, C): [install guide](https://openbimrs.github.io/ifc/guide/install)

## Design notes

- The facade holds no domain logic of its own. Code that fits in a single
  domain crate belongs there; only workflows that need two sibling domains
  live here, because domain crates never depend on one another.
- Such a cross-domain item is gated on every feature it uses, for example
  `#[cfg(all(feature = "spatial", feature = "geometry-select"))]`, and that
  feature pair is added to the build matrix in `scripts/gate.sh`:
  `--all-features` cannot see a break that appears in one combination only.
  A test-only pair, an integration test gated on several features with no
  facade item behind it, is not added: no shipped item exists only in that
  combination, and the gate's `--all-features` test run covers it.
- The facade never depends on another OpenBIM.rs standard family crate
  (`crates/ifc-model/tests/package_architecture.rs` enforces it); those
  families build on IFC, not the reverse.
