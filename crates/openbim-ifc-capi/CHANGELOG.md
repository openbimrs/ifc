# Changelog -- openbim-ifc-capi

All notable changes to the `openbim-ifc-capi` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

## [Unreleased]

### Added (packaging)

- A CMake package, `openbim_ifc` (#41): `find_package(openbim_ifc 0.1 CONFIG)`
  or `add_subdirectory(crates/openbim-ifc-capi)` defines
  `openbim_ifc::openbim_ifc` (shared, or static with
  `OPENBIM_IFC_LINKAGE=STATIC`) and the explicit
  `openbim_ifc::openbim_ifc_shared` / `openbim_ifc::openbim_ifc_static`.
  `cmake --install` lays out the header, both libraries and the config.
- Prebuilt archives of that install tree on each `openbim-ifc-capi-v*` GitHub
  release: Linux and macOS (x86_64, aarch64) and Windows (x86_64, MSVC), with
  `SHA256SUMS`.

### Changed (packaging)

- The shared library records its bare name: ELF `SONAME`
  `libopenbim_ifc_capi.so`, Mach-O install name
  `@rpath/libopenbim_ifc_capi.dylib`. A consumer no longer stores the path the
  library was built at, so the library can be installed and moved.

### Changed

- Links every bundled IFC release explicitly through the binding core's
  new release features; behaviour is unchanged.

### Added (lazy loading)

- `openbim_ifc_v0_1_model_open(path, path_len, ...)`: read a STEP file from
  disk into a model that owns it, one copy less than reading it in the host
  and calling `model_parse`.
- `openbim_ifc_v0_1_model_open_mapped(...)`: the same through a memory
  mapping; the file must stay unchanged until the model is destroyed.
- `OPENBIM_IFC_STATUS_IO` (16) for a file that cannot be read.

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
