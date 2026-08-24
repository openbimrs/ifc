# openbim-bcf

BCF (BIM Collaboration Format) issue exchange for Rust.

The open issue-exchange format: a ZIP with one directory per topic, each holding the issue XML and optionally a viewpoint and snapshot.

This crate targets **BCF-XML**, the file container. BCF-API — the REST/JSON service specification — is a separate standard sharing the same data model.

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
