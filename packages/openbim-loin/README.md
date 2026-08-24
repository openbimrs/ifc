# openbim-loin

ISO 7817-3 / EN 17412-3 LOIN (Level of Information Need) for Rust.

A machine-readable statement of *how much* information is required about which objects, for a given purpose, at a given milestone, between a given pair of actors: geometric detail, alphanumeric properties and documentation.

EN 17412-1 defines the concepts in prose; part 3 is the exchange format, and it is what this crate targets. The LOIN namespace is **not yet final**, so namespace migration is a first-class concern here.

## Status

**Reserved.** This release establishes the crate name and its place in the
layering. It does not parse files yet — see the crate documentation for what is
implemented versus reserved.

No ISO/CEN schema is vendored in this crate. Types are written *from* the
schemas; the schema files themselves are referenced out of tree, because
possessing a copy of a standard does not establish the right to redistribute it.

## Part of nehirde

A pure-Rust IFC and openBIM toolchain: <https://github.com/GeneralPawz/nehirde>

Design rationale for the crate layout: `docs/adr/0015`.

## License

MIT
