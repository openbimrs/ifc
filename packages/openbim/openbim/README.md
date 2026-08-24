# openbim

A facade over the openBIM standards, in pure Rust.

```toml
openbim = { version = "0.1", features = ["ids"] }
```

## Available features

| Feature | Crate | Standard |
| --- | --- | --- |
| `dt` | `openbim-dt` | ISO 23387 data templates |
| `ids` | `openbim-ids` | buildingSMART IDS |
| `bcf` | `openbim-bcf` | BCF (BIM Collaboration Format) |
| `icdd` | `openbim-icdd` | ISO 21597 ICDD |
| `idm` | `openbim-idm` | ISO 29481-3 idmXML |
| `loin` | `openbim-loin` | ISO 7817-3 / EN 17412-3 LOIN |
| `full` | all of the above | |

`loin` implies `dt`, because the LOIN schema imports ISO 23387.

**No feature is on by default.** Depending on `openbim` costs only the shared
vocabulary — outcomes, element references, version detection.

## Why the standards are separate crates

Cargo features are additive across the whole dependency graph. If every
standard were a feature of a single crate, any dependency anywhere enabling
`icdd` would make *everyone* compile an RDF stack — including a consumer that
only wanted to read a `.ids` file.

Separate packages make that structurally impossible: a crate not named in your
feature set is never built. This facade exists so that one dependency line is
still available when you want it, and it re-exports only.

The claim is enforced, not asserted: the repository's gate builds each crate in
isolation and each facade feature separately.

## Status

**Scaffold.** The shared vocabulary is real and tested; the standard crates are
name reservations that do not parse files yet. Nothing here silently pretends
to validate a model — the distinction between *applicable and passed*,
*applicable and failed*, and *not applicable* is built into the core type,
because an audit that treats missing data as compliance is worse than no audit.

No ISO/CEN schema is vendored. Types are written *from* the schemas; the schema
files are referenced out of tree, because possessing a copy of a standard does
not establish the right to redistribute it.

## Part of nehirde

A pure-Rust IFC and openBIM toolchain: <https://github.com/GeneralPawz/nehirde>

Design rationale: `docs/adr/0015`.

## License

MIT
