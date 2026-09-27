# Changelog -- openbim-ifc-binding-core

All notable changes to the `openbim-ifc-binding-core` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

## [Unreleased]

### Added (lazy loading)

- `IfcModel::parse_owned(Vec<u8>)`, `IfcModel::open(path)` and the unsafe
  `IfcModel::open_mapped(path)`. A parsed model keeps its source and
  decodes entities on access (ADR 0015); `parse` copies the input once,
  `parse_owned` and `open` not at all beyond the file read.
- `BindingError::Io`, stable code `io`, for a file that cannot be read.

### Added

- The host-independent half of the language bindings (ADR 0013): `IfcModel`
  operations, the lossless `Tagged` value encoding and `BindingError` with
  stable codes, shared by the WASM, C and Python bindings. Extracted from
  `openbim-ifc-wasm`.
- Non-finite reals (NaN, infinity) are refused for every host; STEP has no
  form for them.
