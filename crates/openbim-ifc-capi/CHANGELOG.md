# Changelog -- openbim-ifc-capi

All notable changes to the `openbim-ifc-capi` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

## [Unreleased]

## [0.1.3] - 2026-10-04

### Added (#330, schema-checked entity creation)

- `openbim_ifc_v0_1_model_author`: a value tape `LIST` of operations, each
  `LIST(ENUM op, TEXT field, value, ...)`, applied as one checked
  transaction; one id per operation comes back (0 for a removal), and a
  short id buffer is `BufferTooSmall` before anything is written.
  `openbim_ifc_v0_1_model_create_entity` (a type and a `LIST` of
  `(TEXT name, value)` pairs) and
  `openbim_ifc_v0_1_entity_remove_with_relationships` are one-operation
  batches. `OPENBIM_IFC_HANDLE_BASE` (2^62) plus an operation's position
  names the entity it produced.
- `OPENBIM_IFC_STATUS_MISSING_ATTRIBUTE` (30) and `_STILL_REFERENCED`
  (31).
- The ABI version is 0.1.6: every export above is new, no `v0_1` symbol
  changed. Additive, a patch release.

## [0.1.2] - 2026-10-04

### Added (#326, attributes by name)

- `openbim_ifc_v0_1_entity_attribute_names` (a tape `LIST` of
  `AttributeInfo` records), `_entity_attribute_by_name` and
  `_entity_set_attribute_by_name`, resolved against the release the header
  declares; names are UTF-8, matched case-insensitively.
- `OPENBIM_IFC_STATUS_UNKNOWN_ATTRIBUTE` (28) and `_DERIVED_ATTRIBUTE`
  (29).
- The ABI version is 0.1.5: every export above is new, no `v0_1` symbol
  changed. Additive, a patch release.

### Added (#327, packaging)

- A prebuilt archive for Windows on Arm (`aarch64-pc-windows-msvc`), the
  sixth on each release. The OpenBim.Ifc NuGet package takes its native
  libraries from these archives. The library is unchanged.

### Added (#318)

- `OPENBIM_IFC_STATUS_CATALOG_NOT_LOADED` (27), so every binding code has
  a status. This library embeds the catalog and never returns it. The ABI
  version is 0.1.4; no `v0_1` symbol changed. Additive, a patch release.

### Added (#325, pkg-config)

- `cmake --install` and the Linux and macOS release archives lay out
  `lib/pkgconfig/openbim_ifc.pc` (shared library) and
  `openbim_ifc-static.pc` (static archive plus the system libraries Rust's
  standard library needs, as the CMake static target lists them). Both
  resolve their paths from `${pcfiledir}`, so an unpacked archive works
  anywhere, and carry the crate version. Windows (MSVC) stays CMake-only.
- `scripts/check-cmake.py` builds and runs the C smoke test with
  `pkg-config --cflags --libs` for both modules, against the installed
  tree and the unpacked archive, on Linux and macOS. Additive, a patch
  release.

## [0.1.1] - 2026-10-03

### Added (#123, property sets: write side)

- `openbim_ifc_v0_1_model_set_properties` (a batch as one value tape,
  one id per edit back), `_model_set_property` and
  `_model_remove_property`.
- `OPENBIM_IFC_STATUS_TEMPLATE_VIOLATION` (25) and
  `_MISSING_PROPERTY` (26).
- The ABI version is 0.1.3: every export above is new, no `v0_1` symbol
  changed. Additive, so a patch release.

### Added (#123, domain views: read side)

- `openbim_ifc_v0_1_model_property_sets`, `_model_resolve_unit`,
  `_model_spatial_tree`, `_model_classifications`, `_model_material`,
  `_model_systems`, `_model_cost` and `_model_georeferencing`: each domain
  record as a value tape, a `LIST` of its fields in the shared core's
  order; no Rust allocation crosses the ABI.
- `OPENBIM_IFC_STATUS_INVALID_MODEL` (19), `_MISSING_REFERENCE` (21),
  `_BUDGET_EXCEEDED` (22), `_UNSUPPORTED` (23) and `_WRONG_ENTITY_TYPE`
  (24).
- The ABI version is 0.1.2: every export above is new, no `v0_1` symbol
  changed. Additive, so a patch release.

## [0.1.0] - 2026-10-03

### Added (#244)

- Lenient reads: `openbim_ifc_v0_1_model_parse_with_options`,
  `_model_open_with_options` and `_model_open_mapped_with_options`, taking
  `OPENBIM_IFC_PARSE_*` flag bits (`OPENBIM_IFC_PARSE_LENIENT` preset);
  an unknown bit is `INVALID_VALUE`.
- `openbim_ifc_v0_1_model_header` and `_model_set_header`: the STEP header
  as a value tape of its ten fields in STEP order.
- `openbim_ifc_v0_1_model_validate`: an `OpenbimIfcValidationSummary` and a
  value tape of findings.
- `openbim_ifc_v0_1_model_parse_ifcxml` and `_model_write_ifcxml`: the
  native ifcXML layout, or the `IFC4` / `IFC4X3_ADD2` XSD layout.
- `openbim_ifc_v0_1_model_unreachable_products`: a value tape of products
  no viewer will draw.
- `OPENBIM_IFC_STATUS_UNSUPPORTED_PROFILE` (17) and
  `OPENBIM_IFC_STATUS_FEATURE_DISABLED` (18).
- The ABI version is 0.1.1: every export above is new, no `v0_1` symbol
  changed. Additive, so a patch release.

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
