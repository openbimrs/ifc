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
