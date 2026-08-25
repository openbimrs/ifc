# openbim-ifc

Feature-gated facade for the pure-Rust OpenBIM.rs IFC crates.

```toml
[dependencies]
openbim-ifc = { version = "0.1", features = ["schema", "properties"] }
```

The Rust library target is named `ifc`, so consumers import it with `use ifc::...`.
The default feature enables only the STEP codec; domain and geometry capabilities
remain opt-in.

See the [repository README](../README.md) for the capability/status distinction
and standalone build instructions.
