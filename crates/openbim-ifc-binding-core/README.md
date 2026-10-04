# openbim-ifc-binding-core

The host-independent core shared by the `openbim-ifc` language bindings:
the model operations, the lossless tagged value encoding and the stable
error codes, written once and wrapped by the WebAssembly, C and Python
bindings (ADR 0013). It adds no IFC behaviour of its own.

This crate is internal to the bindings and ships only inside them
(`publish = false`); applications use one of the bindings or the
[`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade instead.

- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/openbim-ifc-binding-core)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)

## Design notes

Only what every host shares belongs here: the value encoding, the input
checks that stop the STEP writer emitting a malformed file, and the error
codes. IFC behaviour goes into the facade first; a host crate never becomes a
dependency (`package_architecture` enforces that).

Domain views borrow the model and cannot cross a language boundary, so
each domain operation (#123) returns owned snapshot structs, and the same
data as a `Record`: a named, ordered list of fields that every host
converts with one generic function. The records therefore cannot differ
between JavaScript, Python and C.

Attributes by name (#326) resolve through the facade's
`attribute_slots`, against the release the header declares; this crate
maps the facade's refusals to the shared codes (`unknown-attribute`,
`derived-attribute`, `unsupported-schema`) and carries each slot as an
`AttributeInfo` record, so the three hosts list the same names.

Entity creation (#330) is one checked batch too: each host converts its
operation objects field by field, guided by the core's `OPS` table, into
the tape form `AuthorOp::from_tagged` reads, and the facade's
`apply_authoring` runs them, so no host reads an operation differently.

Property edits go the other way and are read once here too: each host
builds the core's `PropertyEdit` from its own idiom, and the C batch is a
value tape `PropertyEdit::from_tagged` reads, so a host cannot interpret
an edit differently. The edit itself is the facade's
`apply_property_edits`; this crate only maps its refusals to the shared
codes.
