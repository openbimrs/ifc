# Changelog -- openbim-ifc-py

All notable changes to the `openbim-ifc-py` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

## [Unreleased]

## [0.1.0] - 2026-09-26

### Added

- Opt-in `rusty_alloc` feature (off by default): the extension's Rust
  allocations go through the pure-Rust rusty_alloc allocator, pinned to
  exactly 2.2.1. Reading STEP into a model takes 18-35% less CPU time on
  seven real IFC files, at 1-7% less peak memory; the models are identical
  on 2,273 corpus files. Build with
  `maturin build --release --features rusty_alloc`. It replaces the C
  `mimalloc` feature, which cost more CPU time than the system allocator on
  a host with transparent huge pages set to `always` (#49).

- Python bindings for the IFC facade (#39, ADR 0013), built with pyo3 and
  maturin as one abi3 wheel for CPython 3.9+. `IfcModel` parses and writes
  IFC STEP, queries by exact type or including subtypes, reads and edits
  attributes, adds and removes entities, and reports dangling references.
- Attribute values are frozen dataclasses (`Null`, `Derived`, `Unknown`,
  `Typed`, ...); bare Python values are refused rather than guessed.
- Every failure raises `IfcError` with the stable `code` shared with the
  C and JavaScript bindings. Parsing releases the GIL.
