# Changelog -- openbim-ifc-capi

All notable changes to the `openbim-ifc-capi` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

## [Unreleased]

### Added (#480, type propagation on assignment)

- `openbim_ifc_v0_1_model_author` reads `BOOL` fields: `ASSIGN_TYPE`
  takes `map_representations`, `material_usage`, `replace`, the id
  `mapping_target` and the texts `layer_set_direction` and
  `predefined_type`; `CONTAIN` and `AGGREGATE` take `replace`. The new
  refusals are `InvalidModel` and `Unsupported`. The header is
  regenerated; only its documentation changed.
- ABI version 0.1.13 (on top of #478's 0.1.12); no `v0_1` symbol changed.

Semver: additive, released on the 0.1 line (the `v0_1` ABI is unchanged:
a tape without the new fields means what it meant).

## [0.1.11] - 2026-10-13

### Added (#478, cascading delete and purge)

- `openbim_ifc_v0_1_model_remove_product(model, product, flags, ...)` and
  `openbim_ifc_v0_1_model_purge(model, classes, class_lengths,
  class_count, flags, ...)`, answering a `RemovalReport` record tape.
  Flags `OPENBIM_IFC_REMOVE_REBASE_PLACEMENTS`,
  `OPENBIM_IFC_REMOVE_UNTYPE_OCCURRENCES`, `OPENBIM_IFC_REMOVE_DRY_RUN`,
  `OPENBIM_IFC_PURGE_INCLUDE_ROOTS`,
  `OPENBIM_IFC_PURGE_INCLUDE_OWNER_HISTORY`, `OPENBIM_IFC_PURGE_DRY_RUN`;
  an unknown flag is `InvalidArgument`. A size query or a short buffer
  plans the removal and answers `BufferTooSmall` without applying it. The
  header is regenerated.
- ABI version 0.1.12 (on top of #449's 0.1.11); every export above is new,
  no `v0_1` symbol changed.

### Deprecated (#478)

- `openbim_ifc_v0_1_entity_remove`, which leaves references dangling (a
  header note); use `openbim_ifc_v0_1_entity_remove_with_relationships` or
  `openbim_ifc_v0_1_model_remove_product`. It keeps working for the life of
  the `v0_1` ABI.

Semver: additive, released on the 0.1 line: the C package keeps 0.1.x while
the `v0_1` ABI is unchanged, because CMake's `find_package(openbim_ifc 0.1)`
does not accept 0.2.

### Added (#450)

- `openbim_ifc_v0_1_model_write_ifczip`, and ifcZIP input to
  `openbim_ifc_v0_1_model_parse` and `_model_open`, recognized by the ZIP
  signature: one STEP or ifcXML model, read within a decompression budget
  (`PARSE`, `UNSUPPORTED` or `BUDGET_EXCEEDED` otherwise). No existing
  symbol changed.

Semver: minor, released on the 0.1 line: the `v0_1` ABI only gains a
symbol.

### Added (#449)

- The `ProductMesh` records of `openbim_ifc_v0_1_meshes_records` gain two
  trailing fields: colour (a `SurfaceColour` record of red, green, blue,
  transparency, source `item`/`layer`/`material`, source entity and
  surface style, or `NULL`) and colour refusal (`GeometryRefusal` or
  `NULL`, code `ambiguous` or a geometry code). No function signature
  changed; the header changes only in its documentation.
- ABI version 0.1.11 (on top of #463's 0.1.10); no `v0_1` symbol changed.

Semver: additive, released on the 0.1 line (the `v0_1` ABI is unchanged
and additive: a reader that walks a record's fields by its child count
reads the new ones; one that hard-codes seven fields reads the first seven
unchanged).

### Added (#463, GlobalIds, types and openings)

- `openbim_ifc_v0_1_model_by_guid` (`out_id`, `NoValue` when absent,
  `InvalidModel` when several entities carry it) and
  `openbim_ifc_v0_1_model_duplicate_global_ids` (a tape of `LIST`s: the
  `GlobalId` and its holders).
- `openbim_ifc_v0_1_entity_type_object`, `_entity_occurrences`,
  `_entity_openings`, `_entity_fillings` and `_entity_voided_element`: an
  optional id to `out_id` or `NoValue`, a list through the buffer protocol.
- `openbim_ifc_v0_1_new_guid` and `openbim_ifc_v0_1_name_based_guid`,
  which need no model. The header is regenerated.
- ABI version 0.1.10; every export above is new, no `v0_1` symbol changed.

Semver: additive, released on the 0.1 line: the C package keeps 0.1.x while
the `v0_1` ABI is unchanged, because CMake's `find_package(openbim_ifc 0.1)`
does not accept 0.2.

### Changed

- `openbim_ifc_v0_1_model_georeferencing` reads IFC2X3 files through their `ePSet_MapConversion` and
  `ePSet_ProjectedCRS` property sets instead of refusing them as
  `unsupported-schema` (#451); such a record's source kind is
  `property-set`. Through `openbim-ifc-binding-core`. No `v0_1` symbol
  changed.

Semver: a behaviour change of an existing export, released on the 0.1
line like the additions above: the `v0_1` ABI is unchanged.

## [0.1.10] - 2026-10-13

### Changed (#433, wire format 1.3)

- The geometry graph payload (#367) may be Axiolid's wire format 1.3
  (`axiolid-model` 0.3.12), a minor version of the format (ADR 0021,
  amended): an `IfcOffsetCurveByDistances` along an offset whose length is
  a quadrature now lowers, continued to the basis's ends the kernel
  measures. An envelope carries the lowest version its content needs, so
  every other payload is unchanged: 1.0, 1.1 or 1.2. A reader on
  `axiolid-model` 0.3.11 or older refuses 1.3. No `v0_1` symbol changed;
  the ABI version is unchanged.

Semver: a minor change in ADR 0021's terms, released on the 0.1 line (as 0.1.7
was): the C package keeps 0.1.x while the `v0_1` ABI is unchanged, because
CMake's `find_package(openbim_ifc 0.1)` does not accept 0.2.

## [0.1.9] - 2026-10-12

### Changed (#429, wire format 1.2)

- The geometry graph payload (#367) may be Axiolid's wire format 1.2
  (`axiolid-model` 0.3.11), a minor version of the format (ADR 0021,
  amended): a sectioned solid or surface along an offset whose length is a
  quadrature now lowers, its sections carrying seam-snapping windows.
  An envelope carries the lowest version its content needs, so every other
  payload is unchanged: 1.0, or 1.1 when a station carries a window. A
  reader on `axiolid-model` 0.3.10 or older refuses 1.2. No `v0_1` symbol changed; the ABI version is unchanged.

Semver: a minor change in ADR 0021's terms, released on the 0.1 line (as 0.1.7
was): the C package keeps 0.1.x while the `v0_1` ABI is unchanged, because
CMake's `find_package(openbim_ifc 0.1)` does not accept 0.2.

## [0.1.8] - 2026-10-12

### Fixed (axiolid/kernel#297)

- Window-free payloads are labelled 1.0 again, readable by `axiolid-model`
  0.3.8: built with `axiolid-model` 0.3.10, the geometry graph payload's
  envelope carries the lowest wire format version its content needs, 1.0
  unless a station carries a seam-snapping window, then 1.1. No `v0_1` symbol changed; the ABI version is unchanged.

Semver: patch.

## [0.1.7] - 2026-10-10

### Changed (#423, wire format 1.1)

- The geometry graph payload (#367) is Axiolid's wire format 1.1
  (`axiolid-model` 0.3.9), a minor version of the format (ADR 0021,
  amended): a station on an offset whose length is a quadrature now lowers
  with a seam-snapping window. An envelope carries the lowest wire version
  its content needs: 1.0, or 1.1 when a station carries a seam-snapping
  window; `axiolid-model` 0.3.9 labels every payload 1.1
  (axiolid/kernel#297), and a reader on `axiolid-model` 0.3.8 or older
  refuses 1.1. No `v0_1` symbol changed; the ABI version is unchanged.

Semver: a minor change in ADR 0021's terms, released as 0.1.7. The C
package keeps its 0.1 line while the `v0_1` ABI is unchanged, because
CMake's `find_package(openbim_ifc 0.1)` does not accept 0.2 (see 0.1.1).

## [0.1.6] - 2026-10-10

### Added (#367, geometry Level 2, ADR 0021)

- Cargo feature `graph` (opt-in; the release archives return
  `FeatureDisabled`): `openbim_ifc_v0_1_model_product_geometry(model, ids,
  id_count, encoding, &graphs)` lowers and encodes each product's Body
  once into an `OpenbimIfcGraphs` handle, read with
  `openbim_ifc_v0_1_graphs_records` (a tape of `ProductGeometry` records:
  id, global id, type name, transform, encoding, payload size, refusal) and
  `openbim_ifc_v0_1_graphs_payload` (the wire payload into a caller byte
  buffer), freed with `openbim_ifc_v0_1_graphs_destroy`; it outlives the
  model. `encoding` is `OPENBIM_IFC_GEOMETRY_JSON` (0) or
  `OPENBIM_IFC_GEOMETRY_CBOR` (1); another value is `InvalidArgument`.
  The payload is Axiolid's wire format 1.0.
- ABI version 0.1.9; no `v0_1` symbol changed. A major version of the wire
  format would be new `v0_2_` exports, not a changed `v0_1_` one.

Semver: additive, a patch release.

## [0.1.5] - 2026-10-08

### Added (#358, #342)

- `openbim_ifc_v0_1_model_property_sets_many(model, ids, id_count, ...)`:
  a tape `LIST` of `ObjectPropertySets` records (object, sets, refusal
  `[code, message]` or `NULL`), one per id, or with null `ids` and count 0
  one per object definition; one pass through one property index, each
  record exactly the per-object call's answer.
- `openbim_ifc_v0_1_entity_set_attribute_by_name_plain`: the value tape
  read as plain host values and coerced against the attribute's declared
  type; `OPENBIM_IFC_KIND_EXACT` (12, valid only there) marks a value
  written exactly.
- `OPENBIM_IFC_STATUS_TYPE_MISMATCH` (32) and `_AMBIGUOUS_VALUE` (33).
- ABI version 0.1.8; no `v0_1` symbol changed.

Semver: additive, a patch release.

## [0.1.4] - 2026-10-08

### Added (#328, geometry, ADR 0021)

- `openbim_ifc_v0_1_model_product_placements(model, ids, id_count, ...)`:
  a tape of `ProductPlacement` records (id, global id, type name,
  transform as 16 `REAL`s or `NULL`, `SelectedRepresentation` or `NULL`,
  `GeometryRefusal` or `NULL`); null `ids` with count 0 selects every
  product with a shape.
- Meshes, cargo feature `mesh` (opt-in; the release archives return
  `FeatureDisabled`): `openbim_ifc_v0_1_model_product_meshes` compiles
  once into an `OpenbimIfcMeshes` handle, read with
  `openbim_ifc_v0_1_meshes_records` (a tape of `ProductMesh` records),
  `_meshes_positions` (`float`), `_meshes_indices` (`uint32_t`) and freed
  with `_meshes_destroy`. The set owns its data and outlives the model.
- ABI version 0.1.7; no `v0_1` symbol changed.

Semver: additive, a patch release.

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
