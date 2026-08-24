# loin

ISO 7817-3 / EN 17412-3 LOIN (Level of Information Need) for Rust.

This crate is a **pure re-export** of [`openbim-loin`](https://crates.io/crates/openbim-loin). It
defines nothing of its own — it exists so the standard is reachable under the
short name practitioners actually use, while there remains exactly one
definition of every type.

```toml
loin = "0.1"
# identical to:
openbim-loin = "0.1"
```

Use whichever name reads better in your project. Do not depend on both.

## Status

**Reserved.** See [`openbim-loin`](https://crates.io/crates/openbim-loin) for what is implemented.

## Part of nehirde

A pure-Rust IFC and openBIM toolchain: <https://github.com/GeneralPawz/nehirde>

## License

MIT
