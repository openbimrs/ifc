# idmxml

ISO 29481-3 idmXML (Information Delivery Manual) for Rust.

This crate is a **pure re-export** of [`openbim-idm`](https://crates.io/crates/openbim-idm). It
defines nothing of its own — it exists so the standard is reachable under the
short name practitioners actually use, while there remains exactly one
definition of every type.

```toml
idmxml = "0.1"
# identical to:
openbim-idm = "0.1"
```

Use whichever name reads better in your project. Do not depend on both.

## Status

**Reserved.** See [`openbim-idm`](https://crates.io/crates/openbim-idm) for what is implemented.

## Part of nehirde

A pure-Rust IFC and openBIM toolchain: <https://github.com/GeneralPawz/nehirde>

## License

MIT
