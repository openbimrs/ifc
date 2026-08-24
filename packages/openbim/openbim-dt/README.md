# openbim-dt

ISO 23387 data templates for Rust.

The concept vocabulary that describes *properties themselves*: property definitions, groups of properties, quantity kinds, dimensions, units and object types, plus the reference machinery binding them to external dictionaries such as bSDD.

It is a separate crate because LOIN does not own it — the ISO 7817-3 schema *imports* this namespace, and a bSDD client needs the same vocabulary without depending on LOIN.

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
