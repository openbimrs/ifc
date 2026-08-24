# openbim-idm

ISO 29481-3 idmXML (Information Delivery Manual) for Rust.

The machine-readable half of the IDM family. Parts 1 and 2 define methodology and BPMN process maps; **part 3** specifies the XML schema for exchange requirements, use cases and business context — the part that can be parsed.

Also published as \`idmxml\`, which is both the standard's own name for the format and an unambiguous way to distinguish part 3 from the process-map halves.

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
