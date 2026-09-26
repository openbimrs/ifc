# Changelog -- openbim-ifc-capi

All notable changes to the `openbim-ifc-capi` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

## [Unreleased]

### Added

- Opt-in `rusty_alloc` feature (off by default): the library's Rust
  allocations go through the pure-Rust rusty_alloc allocator, pinned to
  exactly 2.2.1; the host's `malloc` is untouched. Reading STEP into a
  model takes 18-35% less CPU time on seven real IFC files, at 1-7% less
  peak memory; the models are identical on 2,273 corpus files. It replaces
  the C `mimalloc` feature, which cost more CPU time than the system
  allocator on a host with transparent huge pages set to `always` (#49).

- Versioned C ABI 0.1 over the IFC facade (#38, ADR 0013), following
  Axiolid's C ABI conventions: `openbim_ifc_v0_1_*` symbols, opaque integer
  handles, caller-owned buffers with a size query, no Rust allocation across
  the boundary, and every panic contained as a status.
- Nested attribute values cross as a pre-order node tape plus one string
  buffer, keeping `$`/`*`, `.U.`/`.F.`, integer/real and typed wrappers
  distinct.
- A cbindgen-generated C11 header (`include/openbim_ifc.h`), checked for
  drift, and a C and C++ smoke test in the gate.
