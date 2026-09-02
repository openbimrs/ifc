# openbim-ifc

Feature-gated facade for the pure-Rust OpenBIM.rs IFC crates.

```toml
[dependencies]
openbim-ifc = { git = "https://github.com/openbimrs/ifc.git", rev = "a7c4949bb941504ce874bdec13bd81d33491b5cb", features = ["schema", "properties"] }
```

The workspace crates are not published on crates.io yet; Cargo records this
immutable Git revision in `Cargo.lock`.

The Rust library target is named `ifc`, so consumers import it with `use ifc::...`.
The default feature enables only the STEP codec; domain and geometry capabilities
remain opt-in.

See the [repository README](../README.md) for the capability/status distinction
and standalone build instructions.
