# C API reference

Every type, constant, function and status code of
`crates/openbim-ifc-capi/include/openbim_ifc.h`, the header the C ABI ships.
cbindgen generates the header from the Rust exports, a test fails when the
committed copy is stale, and `cargo run -p xtask -- docs` writes this page
from it, so the page is the published surface.

For task-sized examples see the [C cookbook](/cookbook/c); for the
ownership and buffer protocol, value tapes and record layouts, the
[binding page](/bindings/c).

<!-- API:C:BEGIN -->

## Types

### `OpenbimIfcCurves`

```c
typedef uint64_t OpenbimIfcCurves;
```

Opaque handle to a compiled polyline set. Zero is never a valid handle.

### `OpenbimIfcValueNode`

One node of a value tape. Fields a kind does not use are zero.

| Field | Type | Meaning |
| --- | --- | --- |
| `kind` | `int32_t` | One of the `OPENBIM_IFC_KIND_*` codes. |
| `child_count` | `uint32_t` | Children following a `list` node; 0 otherwise. |
| `int_value` | `int64_t` | Payload of `bool` (0/1), `integer` and `ref`. |
| `real_value` | `double` | Payload of `real`. |
| `str_offset` | `uint64_t` | Byte offset of this node's string in the string buffer. |
| `str_len` | `uint64_t` | Byte length of this node's string (no NUL terminator). |

### `OpenbimIfcModel`

```c
typedef uint64_t OpenbimIfcModel;
```

Opaque model handle. Zero is never a valid handle.

### `OpenbimIfcGraphs`

```c
typedef uint64_t OpenbimIfcGraphs;
```

Opaque handle to an encoded graph set. Zero is never a valid handle.

### `OpenbimIfcMeshes`

```c
typedef uint64_t OpenbimIfcMeshes;
```

Opaque handle to a compiled mesh set. Zero is never a valid handle.

### `OpenbimIfcValidationSummary`

Counts from one validation run.

| Field | Type | Meaning |
| --- | --- | --- |
| `finding_count` | `size_t` | Number of findings on the tape. |
| `errors` | `size_t` | Schema violations. |
| `evaluation_errors` | `size_t` | Implemented rules that could not be decided for an instance. |
| `warnings` | `size_t` | Legal but suspicious conditions. |
| `unsupported` | `size_t` | Rules this validator does not evaluate. |
| `conformant` | `uint32_t` | 1 when there are no errors and no evaluation errors, else 0. |
| `truncated` | `uint32_t` | 1 when the run hit `max_findings`: the counts are lower bounds. |

### `OpenbimIfcVersion`

ABI and crate versions, reported separately: the ABI version changes only
when the C surface does, the crate version on every release.

| Field | Type | Meaning |
| --- | --- | --- |
| `abi_major` | `uint16_t` | ABI major; also the `v0_1` in every symbol name. |
| `abi_minor` | `uint16_t` | ABI minor. |
| `abi_patch` | `uint16_t` | ABI patch. |
| `crate_major` | `uint16_t` | Crate major. |
| `crate_minor` | `uint16_t` | Crate minor. |
| `crate_patch` | `uint16_t` | Crate patch. |

## Constants

| Constant | Value | Meaning |
| --- | ---: | --- |
| `OPENBIM_IFC_GEOMETRY_CBOR` | 1 | `encoding` for `openbim_ifc_v0_1_model_product_geometry`: CBOR bytes (RFC 8949). |
| `OPENBIM_IFC_GEOMETRY_JSON` | 0 | `encoding` for `openbim_ifc_v0_1_model_product_geometry`: JSON text (RFC 8259), UTF-8, not NUL-terminated. |
| `OPENBIM_IFC_HANDLE_BASE` | 4611686018427387904 | The first id of the handle range (2^62): `OPENBIM_IFC_HANDLE_BASE + i` names the entity operation `i` of a batch produced. |
| `OPENBIM_IFC_KIND_BINARY` | 7 | Binary digits in the string range. |
| `OPENBIM_IFC_KIND_BOOL` | 2 | `.T.`/`.F.`; `int_value` is 1 or 0. |
| `OPENBIM_IFC_KIND_DERIVED` | 1 | `*`. |
| `OPENBIM_IFC_KIND_ENUM` | 8 | Enumeration name in the string range. |
| `OPENBIM_IFC_KIND_EXACT` | 12 | Only in a plain value (`openbim_ifc_v0_1_entity_set_attribute_by_name_plain`, #342): one child follows and is written exactly as given, never coerced. Refused in every other tape. |
| `OPENBIM_IFC_KIND_INTEGER` | 4 | Integer in `int_value`. |
| `OPENBIM_IFC_KIND_LIST` | 10 | Aggregate; `child_count` children follow. |
| `OPENBIM_IFC_KIND_NULL` | 0 | Kind codes. Plain integers, not a Rust enum, so an unknown value from C is rejected rather than read as an invalid discriminant. |
| `OPENBIM_IFC_KIND_REAL` | 5 | Finite real in `real_value`. |
| `OPENBIM_IFC_KIND_REF` | 9 | Entity id in `int_value` (non-negative). |
| `OPENBIM_IFC_KIND_TEXT` | 6 | Text in the string range. |
| `OPENBIM_IFC_KIND_TYPED` | 11 | Typed wrapper; type name in the string range, one child follows. |
| `OPENBIM_IFC_KIND_UNKNOWN` | 3 | `.U.`. |
| `OPENBIM_IFC_LENGTH_FORMAT_DECIMAL` | 0 | `openbim_ifc_v0_1_format_length` style: decimal, in `display_unit` (default the value's unit) with `places` digits after the point. |
| `OPENBIM_IFC_LENGTH_FORMAT_FEET_INCHES` | 1 | `openbim_ifc_v0_1_format_length` style: feet and inches, `5' - 6 1/2"`, rounded to `1/places` of an inch. |
| `OPENBIM_IFC_LENGTH_FORMAT_INCHES` | 2 | `openbim_ifc_v0_1_format_length` style: inches, `66 1/2"`, rounded to `1/places` of an inch. |
| `OPENBIM_IFC_LENGTH_FORMAT_SUPPRESS_ZERO_INCHES` | 1 | `openbim_ifc_v0_1_format_length` flag: write a whole number of feet as `5'` rather than `5' - 0"`. |
| `OPENBIM_IFC_LENGTH_UNIT_DRY_RUN` | 4 | `openbim_ifc_v0_1_model_convert_length_unit` flag: report only. |
| `OPENBIM_IFC_LENGTH_UNIT_KEEP_AREA_AND_VOLUME` | 1 | `openbim_ifc_v0_1_model_convert_length_unit` flag: keep the area and volume units and their values instead of replacing them by the square and cube of the new length unit. |
| `OPENBIM_IFC_LENGTH_UNIT_KEEP_REPLACED_UNITS` | 2 | `openbim_ifc_v0_1_model_convert_length_unit` flag: keep the replaced unit entities even when nothing else refers to them. |
| `OPENBIM_IFC_PARSE_ACCEPT_REAL_WITHOUT_POINT` | 4 | Read a real written without its decimal point (`1E-05`) as that real, with a diagnostic. |
| `OPENBIM_IFC_PARSE_CHECK_REFERENCES` | 2 | Report duplicate instance ids and references to undefined ids as diagnostics. Nothing is dropped. |
| `OPENBIM_IFC_PARSE_LENIENT` | 5 | The lenient preset: `SKIP_MALFORMED \| ACCEPT_REAL_WITHOUT_POINT`. |
| `OPENBIM_IFC_PARSE_SKIP_MALFORMED` | 1 | Skip a data record that cannot be parsed and report it as a diagnostic, instead of failing the read. |
| `OPENBIM_IFC_PURGE_DRY_RUN` | 4 | `openbim_ifc_v0_1_model_purge` flag: report only. |
| `OPENBIM_IFC_PURGE_INCLUDE_OWNER_HISTORY` | 2 | `openbim_ifc_v0_1_model_purge` flag: also purge an unreferenced owner history with its actors. |
| `OPENBIM_IFC_PURGE_INCLUDE_ROOTS` | 1 | `openbim_ifc_v0_1_model_purge` flag: also purge unreferenced `IfcRoot`s of the classes. |
| `OPENBIM_IFC_REMOVE_DRY_RUN` | 4 | `openbim_ifc_v0_1_model_remove_product` flag: report only. |
| `OPENBIM_IFC_REMOVE_REBASE_PLACEMENTS` | 1 | `openbim_ifc_v0_1_model_remove_product` flag: re-base placements relative to a removed placement instead of refusing. |
| `OPENBIM_IFC_REMOVE_UNTYPE_OCCURRENCES` | 2 | `openbim_ifc_v0_1_model_remove_product` flag: untype a type object's occurrences instead of refusing. |
| `OPENBIM_IFC_SELECTOR_GRAMMAR_MAJOR` | 1 | The selector grammar's major version (#464); a selector written for one major version parses the same in every build of it. |
| `OPENBIM_IFC_SELECTOR_GRAMMAR_MINOR` | 0 | The selector grammar's minor version: it grows when the grammar accepts more. |

## Functions

### `openbim_ifc_v0_1_convert_unit`

```c
OpenbimIfcStatus openbim_ifc_v0_1_convert_unit(double value, const uint8_t *from, size_t from_len, const uint8_t *to, size_t to_len, double *out_value);
```

Convert `value` from the unit named `from` (`from_len` UTF-8 bytes) to
the unit named `to`, written to `out_value`. Needs no model.
`InvalidValue` for text that names no unit; `TypeMismatch` for units of
different dimensions; `Unsupported` for a unit with an offset (degree
Fahrenheit, #110); `InvalidArgument` for text that is not UTF-8.

**Safety.**
`from` and `to` valid for their lengths; `out_value` valid for one
write.

### `openbim_ifc_v0_1_curves_destroy`

```c
OpenbimIfcStatus openbim_ifc_v0_1_curves_destroy(OpenbimIfcCurves curves);
```

Destroy a polyline set. A stale or repeated handle is `InvalidHandle`.

### `openbim_ifc_v0_1_curves_positions`

```c
OpenbimIfcStatus openbim_ifc_v0_1_curves_positions(OpenbimIfcCurves curves, size_t index, float *buffer, size_t capacity, size_t *out_required);
```

Copy the positions of product `index` -- `x, y, z` per vertex, metres,
relative to its record's transform, every polyline's vertices in turn
-- into `buffer` (`capacity` floats), after writing the count needed
(3 x vertex count) to `out_required`. `OutOfRange` for an index past
the set.

**Safety.**
As for `openbim_ifc_v0_1_meshes_positions`.

### `openbim_ifc_v0_1_curves_records`

```c
OpenbimIfcStatus openbim_ifc_v0_1_curves_records(OpenbimIfcCurves curves, size_t *out_count, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

The set's `ProductCurves` records as a tape, in the order compiled;
`out_count` gets their number. `ProductCurves`: id (`REF`), global id,
type name, transform (`LIST` of 16 `REAL`s, column-major, metres, or
`NULL`), vertex count (`INTEGER`), polylines (`LIST` of
`ProductPolyline`), refusal (`GeometryRefusal` or `NULL`).
`ProductPolyline`: curve (`REF` of the IFC curve, or `NULL`), first
vertex and vertex count (`INTEGER`s: its slice of the positions),
closed (`BOOL`: its last vertex repeats its first), chord bound and
chord requested (`REAL`s, metres: the certified bound on how far the
exact curve lies from the polyline, `0` for a polyline the file
authored, and the budget). No polylines and no refusal is a product
without the representation, or with no curves.

**Safety.**
`out_count` valid for one write; otherwise as for
`openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_entity_add`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_add(OpenbimIfcModel model, const uint8_t *type_name, size_t type_len, size_t attribute_count, const OpenbimIfcValueNode *nodes, size_t node_count, const uint8_t *strings, size_t string_len, uint64_t *out_id);
```

Append an entity of `type_name` whose `attribute_count` attributes are
on the tape back to back, and write its new id to `out_id`.

**Safety.**
As for `openbim_ifc_v0_1_entity_set_attribute`; `type_name` valid for
`type_len` reads; `out_id` valid for one write.

### `openbim_ifc_v0_1_entity_attribute`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_attribute(OpenbimIfcModel model, uint64_t id, size_t index, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

Attribute `index` of entity `id` as a value tape (`$` past the end).

**Safety.**
As for `fill_tape`: each buffer null with capacity 0, or valid for its
capacity; both `out_*_required` valid for one write.

### `openbim_ifc_v0_1_entity_attribute_by_name`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_attribute_by_name(OpenbimIfcModel model, uint64_t id, const uint8_t *name, size_t name_len, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

Attribute `name` (UTF-8, `name_len` bytes, any case, e.g. `Name`) of
entity `id` as a value tape; `NULL` when the record stops before its
slot.

An `INVERSE` name (`IsDefinedBy`, #462) reads the entities referencing
`id` through the declared release's `FOR` clause: a `LIST` of `REF`s
in ascending id order for a `SET` or `BAG` (`SET 0:1` included: zero
or one element), a `REF` or `NULL` for a single-valued inverse, which
two referrers refuse with `InvalidModel`.

`MissingEntity`, `UnknownAttribute`, `UnsupportedSchema` as for
`openbim_ifc_v0_1_entity_attribute_names`.

**Safety.**
`name` valid for `name_len` reads; otherwise as for
`openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_entity_attribute_names`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_attribute_names(OpenbimIfcModel model, uint64_t id, size_t *out_count, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

Every explicit attribute of entity `id` in slot order, inherited first:
a `LIST` of `AttributeInfo` records, their number in `out_count`.
`AttributeInfo`: name (`TEXT`, the schema's spelling), index
(`INTEGER`, the slot), type name (`TEXT`), optional (`BOOL`), aggregate
(`BOOL`), derived (`BOOL`: written `*`, not writable), declared by
(`TEXT`, the entity introducing it). `INVERSE` attributes hold no slot
and are listed by `openbim_ifc_v0_1_entity_inverse_attribute_names`.

`MissingEntity`; `UnsupportedSchema` when the header names no bundled
release or the release does not declare the entity's type.

**Safety.**
`out_count` valid for one write; otherwise as for
`openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_entity_attributes`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_attributes(OpenbimIfcModel model, uint64_t id, size_t *out_count, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

Every attribute of entity `id`, back to back on one tape, plus how many
top-level values it holds.

**Safety.**
As for `openbim_ifc_v0_1_entity_attribute`; `out_count` valid for one
write.

### `openbim_ifc_v0_1_entity_fillings`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_fillings(OpenbimIfcModel model, uint64_t id, uint64_t *buffer, size_t capacity, size_t *out_required);
```

Every element filling opening `id` (`IfcRelFillsElement`), in file
order. Refusals as for `openbim_ifc_v0_1_entity_type_object`
(`WrongEntityType`: not an `IfcOpeningElement`).

**Safety.**
As for `openbim_ifc_v0_1_model_ids`.

### `openbim_ifc_v0_1_entity_get_value`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_get_value(OpenbimIfcModel model, uint64_t id, const uint8_t *path, size_t path_len, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

The value `path` (UTF-8, e.g. `type.Name`, `Pset_WallCommon.FireRating`)
reads from entity `id`, as a value tape: a scalar or reference as its
node, several values as a `LIST`, a property set as a `LIST` of
two-item `LIST`s, each the member's name (`TEXT`) and value.

`InvalidSelector` for a path that does not parse, `MissingEntity`, and
as for `openbim_ifc_v0_1_model_select`.

**Safety.**
`path` valid for `path_len` reads; otherwise as for
`openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_entity_inverse_attribute_names`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_inverse_attribute_names(OpenbimIfcModel model, uint64_t id, size_t *out_count, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

Every `INVERSE` attribute of entity `id`, inherited ones first: a
`LIST` of `InverseAttributeInfo` records, their number in `out_count`.
`InverseAttributeInfo`: name (`TEXT`, the schema's spelling), entity
(`TEXT`, the referencing type before `FOR`), for attribute (`TEXT`, its
attribute after `FOR`), aggregate (`BOOL`: a `SET` or `BAG`, read as a
`LIST`), lower (`INTEGER`), upper (`INTEGER`, or `NULL` for `?`),
declared by (`TEXT`, the entity introducing it).

`MissingEntity`; `UnsupportedSchema` as for
`openbim_ifc_v0_1_entity_attribute_names`.

**Safety.**
As for `openbim_ifc_v0_1_entity_attribute_names`.

### `openbim_ifc_v0_1_entity_inverse_references`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_inverse_references(OpenbimIfcModel model, uint64_t id, size_t *out_count, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

Every reference to entity `id`, in ascending order: a `LIST` of
`InverseReference` records, their number in `out_count`, one per
referencing entity and attribute. `InverseReference`: id (`REF`, the
referencing entity), index (`INTEGER`, its attribute holding the
reference, directly or inside an aggregate). Needs no schema.

`MissingEntity`.

**Safety.**
As for `openbim_ifc_v0_1_entity_attribute_names`.

### `openbim_ifc_v0_1_entity_occurrences`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_occurrences(OpenbimIfcModel model, uint64_t id, uint64_t *buffer, size_t capacity, size_t *out_required);
```

Every object type object `id` types, in file order. Refusals as for
`openbim_ifc_v0_1_entity_type_object` (`WrongEntityType`: not an
`IfcTypeObject`).

**Safety.**
As for `openbim_ifc_v0_1_model_ids`.

### `openbim_ifc_v0_1_entity_openings`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_openings(OpenbimIfcModel model, uint64_t id, uint64_t *buffer, size_t capacity, size_t *out_required);
```

Every opening voiding element `id` (`IfcRelVoidsElement`), in file
order. Refusals as for `openbim_ifc_v0_1_entity_type_object`
(`WrongEntityType`: not an `IfcElement`).

**Safety.**
As for `openbim_ifc_v0_1_model_ids`.

### `openbim_ifc_v0_1_entity_remove`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_remove(OpenbimIfcModel model, uint64_t id);
```

Remove entity `id`; references to it are left dangling.

Deprecated (#478): a file with dangling references parses and is wrong.
Use `openbim_ifc_v0_1_entity_remove_with_relationships` for one entity,
or `openbim_ifc_v0_1_model_remove_product` for a product with what it
solely owns. It keeps working for the life of the `v0_1` ABI.

### `openbim_ifc_v0_1_entity_remove_with_relationships`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_remove_with_relationships(OpenbimIfcModel model, uint64_t id);
```

Remove entity `id` with its relationships: it is taken out of every
relationship holding it, and a relationship left without an end goes
too. Unlike `openbim_ifc_v0_1_entity_remove`, nothing is left dangling:
`StillReferenced` while an entity other than a relationship needs `id`,
`MissingEntity` when there is none.

### `openbim_ifc_v0_1_entity_set_attribute`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_set_attribute(OpenbimIfcModel model, uint64_t id, size_t index, const OpenbimIfcValueNode *nodes, size_t node_count, const uint8_t *strings, size_t string_len);
```

Set attribute `index` of entity `id` from a one-value tape. Writing past
the end pads the gap with `$`.

**Safety.**
`nodes` must be valid for `node_count` reads and `strings` for
`string_len` reads (either may be null when its length is 0).

### `openbim_ifc_v0_1_entity_set_attribute_by_name`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_set_attribute_by_name(OpenbimIfcModel model, uint64_t id, const uint8_t *name, size_t name_len, const OpenbimIfcValueNode *nodes, size_t node_count, const uint8_t *strings, size_t string_len);
```

Set attribute `name` (UTF-8, any case) of entity `id` from a one-value
tape. Every check runs before the write, so a refusal changes nothing.

`DerivedAttribute` for a slot the entity's type derives (written `*`);
`InverseAttribute` for an `INVERSE` name, which holds no slot; otherwise as for `openbim_ifc_v0_1_entity_attribute_by_name` and
`openbim_ifc_v0_1_entity_set_attribute`.

**Safety.**
`name` valid for `name_len` reads; the tape as for
`openbim_ifc_v0_1_entity_set_attribute`.

### `openbim_ifc_v0_1_entity_set_attribute_by_name_plain`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_set_attribute_by_name_plain(OpenbimIfcModel model, uint64_t id, const uint8_t *name, size_t name_len, const OpenbimIfcValueNode *nodes, size_t node_count, const uint8_t *strings, size_t string_len);
```

Set attribute `name` (UTF-8, any case) of entity `id` from a one-value
tape of plain values, coerced against the attribute's declared type in
the declared release (#342); returns as
`openbim_ifc_v0_1_entity_set_attribute_by_name` does.

`TEXT` is a string, `INTEGER` an integer, `REAL` a float, `BOOL` a
boolean, `REF` an entity handle, `LIST` a sequence of plain values,
`NULL` unset. A string becomes a label (bare) or the enumeration item it
names; an integer an `INTEGER` or `REAL` as declared; a reference is
checked to exist and fit; in a SELECT, the one member that takes the
value is written as a typed parameter. An `EXACT` node's child, and
`DERIVED`, `UNKNOWN`, `BINARY`, `ENUM` and `TYPED` nodes, are written as
given.

`TypeMismatch` for a value that does not fit, `AmbiguousValue` for one
several SELECT members take, `MissingReference` for a reference to a
missing entity, `Unsupported` for a declared type the tables do not
resolve; otherwise as for
`openbim_ifc_v0_1_entity_set_attribute_by_name`. A refusal changes
nothing.

**Safety.**
As for `openbim_ifc_v0_1_entity_set_attribute_by_name`.

### `openbim_ifc_v0_1_entity_set_value`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_set_value(OpenbimIfcModel model, uint64_t id, const uint8_t *path, size_t path_len, const OpenbimIfcValueNode *nodes, size_t node_count, const uint8_t *strings, size_t string_len);
```

Write a one-value tape of plain values at `path` (UTF-8) of entity
`id`, as one transaction: an attribute (`Name`, `type.Name`) coerced
as `openbim_ifc_v0_1_entity_set_attribute_by_name_plain` coerces it,
or a property (`Pset_WallCommon.FireRating`) from a `TYPED` value, or a
plain one when the property already states its type. A refusal
changes nothing.

`InvalidValue` for a path that names no single value (a pattern,
`count`, a prefix reaching no entity), `UnknownAttribute`,
`DerivedAttribute`, `TypeMismatch` (also for a plain value no stated
property types), `AmbiguousValue`, and the property edit's refusals.

**Safety.**
`path` valid for `path_len` reads; the tape as for
`openbim_ifc_v0_1_entity_set_attribute_by_name_plain`.

### `openbim_ifc_v0_1_entity_total_inverses`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_total_inverses(OpenbimIfcModel model, uint64_t id, size_t *out_count);
```

How many distinct entities reference entity `id`, through any
attribute, written to `out_count`. Needs no schema. `MissingEntity`.

**Safety.**
`out_count` must be null or valid for one write.

### `openbim_ifc_v0_1_entity_type`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_type(OpenbimIfcModel model, uint64_t id, uint8_t *buffer, size_t capacity, size_t *out_required);
```

The type name of entity `id`, upper-case, NUL-terminated.

**Safety.**
As for `openbim_ifc_v0_1_model_write`.

### `openbim_ifc_v0_1_entity_type_object`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_type_object(OpenbimIfcModel model, uint64_t id, uint64_t *out_id);
```

The type object of occurrence `id` (`IfcRelDefinesByType`), written to
`out_id`; `NoValue` when it has none. `MissingEntity`,
`WrongEntityType` (not an `IfcObject`), `InvalidModel` (two types, or a
malformed relationship), `MissingReference`, `UnsupportedSchema`.

**Safety.**
`out_id` valid for one write.

### `openbim_ifc_v0_1_entity_voided_element`

```c
OpenbimIfcStatus openbim_ifc_v0_1_entity_voided_element(OpenbimIfcModel model, uint64_t id, uint64_t *out_id);
```

The element opening `id` voids (`IfcRelVoidsElement`), written to
`out_id`; `NoValue` when it voids none. Refusals as for
`openbim_ifc_v0_1_entity_type_object` (`WrongEntityType`: not an
`IfcFeatureElementSubtraction`; `InvalidModel`: two hosts).

**Safety.**
`out_id` valid for one write.

### `openbim_ifc_v0_1_format_length`

```c
OpenbimIfcStatus openbim_ifc_v0_1_format_length(double value, const uint8_t *unit, size_t unit_len, uint32_t style, uint32_t places, const uint8_t *display_unit, size_t display_unit_len, uint32_t flags, uint8_t *buffer, size_t capacity, size_t *out_required);
```

Write `value`, a length in the unit named `unit`, as NUL-terminated
text. `style` is `OPENBIM_IFC_LENGTH_FORMAT_DECIMAL` (in `display_unit`,
or the value's unit when `display_unit` is null, with `places` decimals),
`OPENBIM_IFC_LENGTH_FORMAT_FEET_INCHES` or
`OPENBIM_IFC_LENGTH_FORMAT_INCHES` (rounded to `1/places` of an inch);
`flags`: `OPENBIM_IFC_LENGTH_FORMAT_SUPPRESS_ZERO_INCHES`. Needs no
model. `InvalidValue` for a unit name, a zero denominator or a length
too large; `TypeMismatch` for a unit that is not a length;
`OutOfRange` for more than 255 decimals; `InvalidArgument` for an
unknown style or flag, or text that is not UTF-8.

**Safety.**
`unit` valid for `unit_len`; `display_unit` null or valid for
`display_unit_len`; `buffer` and `out_required` as for
`openbim_ifc_v0_1_new_guid`.

### `openbim_ifc_v0_1_graphs_destroy`

```c
OpenbimIfcStatus openbim_ifc_v0_1_graphs_destroy(OpenbimIfcGraphs graphs);
```

Destroy a graph set. A stale or repeated handle is `InvalidHandle`.

### `openbim_ifc_v0_1_graphs_payload`

```c
OpenbimIfcStatus openbim_ifc_v0_1_graphs_payload(OpenbimIfcGraphs graphs, size_t index, uint8_t *buffer, size_t capacity, size_t *out_required);
```

Copy the wire payload of graph `index` -- JSON text without a trailing
NUL, or CBOR bytes -- into `buffer` (`capacity` bytes), after writing
the size needed (its record's payload size) to `out_required`; 0 for a
product with no graph. `OutOfRange` for an index past the set.

**Safety.**
As for the other buffer calls: `buffer` valid for `capacity` writes
when non-null, `out_required` for one.

### `openbim_ifc_v0_1_graphs_records`

```c
OpenbimIfcStatus openbim_ifc_v0_1_graphs_records(OpenbimIfcGraphs graphs, size_t *out_count, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

The set's `ProductGeometry` records as a tape, in the order lowered;
`out_count` gets their number. `ProductGeometry`: id (`REF`), global
id, type name, transform (`LIST` of 16 `REAL`s, column-major, metres,
already applied to the graph, or `NULL`), encoding (`json` or `cbor`),
payload size (`INTEGER`, bytes), refusal (`GeometryRefusal` or `NULL`).
A payload size of 0 and no refusal is a product with no Body
representation.

**Safety.**
`out_count` valid for one write; otherwise as for
`openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_last_error_code`

```c
OpenbimIfcStatus openbim_ifc_v0_1_last_error_code(OpenbimIfcModel model, uint8_t *buffer, size_t capacity, size_t *out_required);
```

The last error on `model` as its stable code (`parse`, `missing-entity`,
...), NUL-terminated; `NoValue` if the last call succeeded.

**Safety.**
`buffer` must be null (with `capacity` 0) or valid for `capacity`
writes; `out_required` valid for one write.

### `openbim_ifc_v0_1_last_error_message`

```c
OpenbimIfcStatus openbim_ifc_v0_1_last_error_message(OpenbimIfcModel model, uint8_t *buffer, size_t capacity, size_t *out_required);
```

The last error on `model` as a human message, NUL-terminated; `NoValue`
if the last call succeeded.

**Safety.**
As for `openbim_ifc_v0_1_last_error_code`.

### `openbim_ifc_v0_1_last_error_span`

```c
OpenbimIfcStatus openbim_ifc_v0_1_last_error_span(OpenbimIfcModel model, size_t *out_start, size_t *out_end);
```

The byte range in the query the last error on `model` refuses, written
to `out_start` and `out_end`; `NoValue` when the last call succeeded or
its error names no span (only `InvalidSelector` does).

**Safety.**
`out_start` and `out_end` valid for one write each.

### `openbim_ifc_v0_1_live_models`

```c
OpenbimIfcStatus openbim_ifc_v0_1_live_models(size_t *out_count);
```

Number of live models, for leak checks.

**Safety.**
`out_count` must be null or valid for one write.

### `openbim_ifc_v0_1_meshes_destroy`

```c
OpenbimIfcStatus openbim_ifc_v0_1_meshes_destroy(OpenbimIfcMeshes meshes);
```

Destroy a mesh set. A stale or repeated handle is `InvalidHandle`.

### `openbim_ifc_v0_1_meshes_indices`

```c
OpenbimIfcStatus openbim_ifc_v0_1_meshes_indices(OpenbimIfcMeshes meshes, size_t index, uint32_t *buffer, size_t capacity, size_t *out_required);
```

Copy the triangle indices of mesh `index` -- three vertex indices per
triangle -- into `buffer` (`capacity` values), after writing the count
needed (3 x triangle count) to `out_required`. `OutOfRange` for an
index past the set.

**Safety.**
As for `openbim_ifc_v0_1_meshes_positions`.

### `openbim_ifc_v0_1_meshes_positions`

```c
OpenbimIfcStatus openbim_ifc_v0_1_meshes_positions(OpenbimIfcMeshes meshes, size_t index, float *buffer, size_t capacity, size_t *out_required);
```

Copy the positions of mesh `index` -- `x, y, z` per vertex, metres,
relative to its record's transform -- into `buffer` (`capacity`
floats), after writing the count needed (3 x vertex count) to
`out_required`. `OutOfRange` for an index past the set.

**Safety.**
As for the other buffer calls: `buffer` valid for `capacity` writes
when non-null, `out_required` for one.

### `openbim_ifc_v0_1_meshes_records`

```c
OpenbimIfcStatus openbim_ifc_v0_1_meshes_records(OpenbimIfcMeshes meshes, size_t *out_count, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

The set's `ProductMesh` records as a tape, in the order compiled;
`out_count` gets their number. `ProductMesh`: id (`REF`), global id,
type name, transform (`LIST` of 16 `REAL`s, column-major, metres, or
`NULL`), vertex count, triangle count (`INTEGER`s), refusal
(`GeometryRefusal` or `NULL`), colour (`SurfaceColour` or `NULL`),
colour refusal (`GeometryRefusal` or `NULL`), closure (`TEXT`, #458:
`solid`, `surface` for an area with no volume, or `unknown` for an
empty or refused mesh). Empty arrays and no refusal is a product with no
Body representation.

`SurfaceColour` (#449): red, green, blue (`REAL`s in 0, 1),
transparency (`REAL`, 0 opaque, or `NULL` when the file states none),
source (`item`, `layer` or `material`, in that precedence), source
entity (`REF`: the `IfcStyledItem`, the layer assignment or the
`IfcMaterial`), surface style (`REF`). The colour is the one the Body's
items agree on, `NULL` when nothing colours the product; the colour
refusal's code is `ambiguous` (two styled items or colours at one
level, a layer or profile set of several materials, a constituent set
no shape aspect resolves, or Body items of different colours) or a
shared geometry code, and never refuses the mesh.

**Safety.**
`out_count` valid for one write; otherwise as for
`openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_model_author`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_author(OpenbimIfcModel model, const OpenbimIfcValueNode *nodes, size_t node_count, const uint8_t *strings, size_t string_len, uint64_t *out_ids, size_t ids_capacity, size_t *out_count);
```

Apply the operations on the tape as one checked transaction against the
release the header declares: all of them, in order, or none, and the
model unchanged.

`out_count` gets the number of operations. `out_ids` gets, per
operation, the id of the entity it produced, or 0 for a removal; with
`ids_capacity` below the operation count the call returns
`BufferTooSmall` and applies nothing.

`UnsupportedSchema` for an unbundled release or an undeclared type;
`UnknownAttribute`, `DerivedAttribute`, `MissingAttribute`;
`InvalidValue` for a value of the wrong type, form or cardinality, a
duplicate `GlobalId`, a malformed tape or handle, or a placement the
schema cannot hold; `WrongEntityType` for an abstract type, a builder's
type of the wrong kind or a reference the attribute does not accept;
`MissingEntity`, `MissingReference`; `InvalidModel` for a second
containment, decomposition, typing, material or `IfcProject` (unless
`replace`), a mapped representation that clashes with one the object
has, a `PredefinedType` that contradicts the type's, a shape
representation that already has its one use, a product definition shape
other products share, or a representation not assigned where it is
unassigned from; `InvalidValue` also for a representation type its items
do not fit, items no type fits, or a missing representation identifier;
`Unsupported` for a representation map that cannot be mapped, a material
usage whose defaults cannot be determined, or a representation type check
needing a curve dimension that cannot be derived; `StillReferenced` for a
removal an entity other than a relationship still needs.

**Safety.**
`nodes` valid for `node_count` reads and `strings` for `string_len`
(either null when its length is 0); `out_ids` null with capacity 0, or
valid for `ids_capacity` writes; `out_count` valid for one write.

### `openbim_ifc_v0_1_model_by_guid`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_by_guid(OpenbimIfcModel model, const uint8_t *global_id, size_t global_len, uint64_t *out_id);
```

The entity whose `GlobalId` is `global_id` (UTF-8, `global_len` bytes),
written to `out_id`; `NoValue` when none carries it.

The first lookup indexes every `GlobalId` once; later ones reuse the
index until the model changes. `InvalidValue` for text that is not a
`GlobalId`, `InvalidModel` when several entities carry it (the last
error names them), `UnsupportedSchema`.

**Safety.**
`global_id` valid for `global_len` reads; `out_id` valid for one write.

### `openbim_ifc_v0_1_model_classifications`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_classifications(OpenbimIfcModel model, uint64_t object, size_t *out_count, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

The classifications of `object`: a `LIST` of `Classification` records,
its own then its type object's; `out_count` gets their number.
`Classification`: relationship (`REF`), global id, source, type object,
target (`REF`), kind (`reference`, `system` or `notation`),
identification, name, location, notation (`LIST` of `TEXT`), parents
(`LIST` of `REF`), system (`id, name, source, edition` or `NULL`).

`MissingEntity`, `UnsupportedSchema`, `InvalidModel`,
`MissingReference`, `BudgetExceeded`, `FeatureDisabled`.

**Safety.**
As for `openbim_ifc_v0_1_model_property_sets`.

### `openbim_ifc_v0_1_model_convert_length_unit`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_convert_length_unit(OpenbimIfcModel model, const uint8_t *target, size_t target_len, uint32_t flags, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

Convert the model's length unit to the unit named `target`
(`target_len` UTF-8 bytes): an SI `METRE` with any prefix, or `inch`,
`foot`, `yard`, `mile`, `US survey foot`. Every length, area and volume
value is rescaled as one checked transaction. `flags`:
`OPENBIM_IFC_LENGTH_UNIT_KEEP_AREA_AND_VOLUME`,
`OPENBIM_IFC_LENGTH_UNIT_KEEP_REPLACED_UNITS`,
`OPENBIM_IFC_LENGTH_UNIT_DRY_RUN`. The answer is the `LengthUnitReport`
tape of the module docs; a size query or a short buffer plans the
conversion and answers `BufferTooSmall` without applying it.

`InvalidValue` for a target that is not a length unit;
`Unsupported` naming the values the conversion cannot carry (a
parameter on a curve whose parameter space IFC leaves undefined, a
p-curve off a plane, polynomial coefficients ...) and for an offset
unit (#110); `InvalidModel` when the project length unit does not
resolve; `UnsupportedSchema`; `InvalidArgument` for an unknown flag or
text that is not UTF-8; `FeatureDisabled`.

**Safety.**
`target` valid for `target_len`; the buffers as for
`openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_model_convert_unit_value`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_convert_unit_value(OpenbimIfcModel model, double value, uint64_t from, uint64_t to, double *out_value);
```

Convert `value` from unit entity `from` to unit entity `to` of the model
(an `IfcSIUnit`, `IfcConversionBasedUnit` or `IfcDerivedUnit`), written
to `out_value`. `MissingEntity`; `TypeMismatch` for units of different
dimensions; `Unsupported` for a monetary, context-dependent or offset
unit; `InvalidModel` for a unit that does not resolve;
`UnsupportedSchema`; `FeatureDisabled`.

**Safety.**
`out_value` valid for one write.

### `openbim_ifc_v0_1_model_cost`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_cost(OpenbimIfcModel model, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

Every cost schedule and cost item as one `Cost` record: schedules
(`LIST` of `CostSchedule`), items (`LIST` of `CostItem`, each with its
`CostValue` tree, applied values tagged), anomalies.

`UnsupportedSchema`, `MissingReference`, `BudgetExceeded`,
`FeatureDisabled`.

**Safety.**
As for `openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_model_create`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_create(OpenbimIfcModel *out_model);
```

Create an empty model and write its handle to `out_model`.

**Safety.**
`out_model` must be null or valid for one write.

### `openbim_ifc_v0_1_model_create_entity`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_create_entity(OpenbimIfcModel model, const uint8_t *type_name, size_t type_len, const OpenbimIfcValueNode *nodes, size_t node_count, const uint8_t *strings, size_t string_len, uint64_t *out_id);
```

Create one entity of `type_name` (UTF-8, any case) from named
attributes: a batch of one `CREATE`. The tape is one `LIST` of
`LIST(TEXT name, value)` pairs (an empty `LIST` for none). An `IfcRoot`
without a `GlobalId` gets a fresh one. `out_id` gets the new id.

Refusals as for `openbim_ifc_v0_1_model_author`.

**Safety.**
`type_name` valid for `type_len` reads; the tape as for
`openbim_ifc_v0_1_model_author`; `out_id` valid for one write.

### `openbim_ifc_v0_1_model_dangling_references`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_dangling_references(OpenbimIfcModel model, uint64_t *buffer, size_t capacity, size_t *out_required);
```

Every dangling reference as `(from, to)` pairs, flattened: element
`2k` is a referencing id, `2k+1` the missing id it points to.

**Safety.**
As for `openbim_ifc_v0_1_model_ids`; sizes count `u64`s, not pairs.

### `openbim_ifc_v0_1_model_destroy`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_destroy(OpenbimIfcModel model);
```

Destroy a model. A stale or repeated handle is `InvalidHandle`.

### `openbim_ifc_v0_1_model_diagnostic`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_diagnostic(OpenbimIfcModel model, size_t index, uint8_t *buffer, size_t capacity, size_t *out_required);
```

Diagnostic `index` as a NUL-terminated string; `OutOfRange` past the end.

**Safety.**
As for `openbim_ifc_v0_1_model_write`.

### `openbim_ifc_v0_1_model_diagnostic_count`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_diagnostic_count(OpenbimIfcModel model, size_t *out_count);
```

Number of non-fatal parse diagnostics.

**Safety.**
`out_count` must be null or valid for one write.

### `openbim_ifc_v0_1_model_duplicate_global_ids`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_duplicate_global_ids(OpenbimIfcModel model, size_t *out_count, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

Every `GlobalId` several entities carry, in `GlobalId` order, and how
many. The tape is a `LIST`; each item a `LIST` of the `GlobalId`
(`TEXT`) and its holders in file order (`LIST` of `REF`). Empty for a
conformant file. `UnsupportedSchema`.

**Safety.**
`out_count` valid for one write; otherwise as for
`openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_model_georeferencing`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_georeferencing(OpenbimIfcModel model, size_t *out_count, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

Every coordinate operation, resolved: a `LIST` of `MapConversion`
records, empty when the model has none; `out_count` gets their number.
`MapConversion`: operation, kind, source, source kind, target CRS
(`ProjectedCrs` record), eastings, northings, orthogonal height, x axis
(two `REAL`s), scale, factors (three `REAL`s or `NULL`), project unit
and map unit (`name, metres per unit`), map unit declared (`BOOL`),
linear (three columns of three `REAL`s), translation (three `REAL`s).

IFC2X3 is read from the `ePSet_MapConversion` and `ePSet_ProjectedCRS`
property sets: source kind `property-set`, source the `IfcProject` or
`IfcSite` carrying them.

`UnsupportedSchema` (anything but IFC2X3, IFC4 and IFC4X3),
`Unsupported`, `InvalidModel`, `MissingReference`, `FeatureDisabled`.

**Safety.**
As for `openbim_ifc_v0_1_model_property_sets`.

### `openbim_ifc_v0_1_model_header`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_header(OpenbimIfcModel model, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

The file header as a value tape: one `LIST` of ten values, in STEP
header order -- description (`LIST` of `TEXT`), implementation level,
name, time stamp, author (`LIST`), organization (`LIST`), preprocessor
version, originating system, authorization, schema (`LIST`).

**Safety.**
As for `openbim_ifc_v0_1_entity_attribute`: each buffer null with
capacity 0, or valid for its capacity; both `out_*_required` valid for
one write.

### `openbim_ifc_v0_1_model_ids`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_ids(OpenbimIfcModel model, uint64_t *buffer, size_t capacity, size_t *out_required);
```

Every entity id, in file order.

**Safety.**
`buffer` must be null (with `capacity` 0) or valid for `capacity` `u64`
writes; `out_required` valid for one write.

### `openbim_ifc_v0_1_model_ids_of_type`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_ids_of_type(OpenbimIfcModel model, const uint8_t *type_name, size_t type_len, uint64_t *buffer, size_t capacity, size_t *out_required);
```

Ids of every entity of exactly `type_name` (UTF-8, `type_len` bytes,
case-insensitive). Subtypes are not included.

**Safety.**
`type_name` must be valid for `type_len` reads; otherwise as for
`openbim_ifc_v0_1_model_ids`.

### `openbim_ifc_v0_1_model_ids_of_type_including_subtypes`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_ids_of_type_including_subtypes(OpenbimIfcModel model, const uint8_t *type_name, size_t type_len, uint64_t *buffer, size_t capacity, size_t *out_required);
```

Ids of every entity of `type_name` or any of its subtypes, using the
schema the file's header declares. `UnsupportedSchema` if none is bundled.

**Safety.**
As for `openbim_ifc_v0_1_model_ids_of_type`.

### `openbim_ifc_v0_1_model_len`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_len(OpenbimIfcModel model, size_t *out_count);
```

Number of entities.

**Safety.**
`out_count` must be null or valid for one write.

### `openbim_ifc_v0_1_model_material`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_material(OpenbimIfcModel model, uint64_t object, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

The material association of `object`, its own or its type object's, as
one `MaterialAssignment` record, or a `NULL` tape when there is none:
relationship, global id, source, type object, target, type name, kind,
set, name, materials, layers, profiles, constituents (each a `LIST` of
records), usage (record or `NULL`).

`MissingEntity`, `UnsupportedSchema`, `InvalidModel`,
`MissingReference`, `FeatureDisabled`.

**Safety.**
As for `openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_model_open`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_open(const uint8_t *path, size_t path_len, OpenbimIfcModel *out_model, uint8_t *error_buffer, size_t capacity);
```

Read the STEP file at `path` (UTF-8, `path_len` bytes, no NUL needed)
into a model that owns its bytes, and write the new model's handle. An
ifcZIP archive is read as `openbim_ifc_v0_1_model_parse` reads one.

Errors as `openbim_ifc_v0_1_model_parse`, plus `Io` when the file cannot
be opened or read.

**Safety.**
`path` must be valid for `path_len` reads; `out_model` for one write;
`error_buffer`, if non-null, for `capacity` writes.

### `openbim_ifc_v0_1_model_open_mapped`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_open_mapped(const uint8_t *path, size_t path_len, OpenbimIfcModel *out_model, uint8_t *error_buffer, size_t capacity);
```

Read the STEP file at `path` through a memory mapping, and write the new
model's handle. No copy of the file is made, and its pages belong to the
page cache rather than the process heap.

**Safety.**
As `openbim_ifc_v0_1_model_open`, and additionally: the file must not
be modified or truncated until the model is destroyed. The model decodes
entities from the mapping on access; a changed file makes that fail
(reported as `Panic`), end the process (`SIGBUS` on truncation), or read
other content.

### `openbim_ifc_v0_1_model_open_mapped_with_options`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_open_mapped_with_options(const uint8_t *path, size_t path_len, uint32_t flags, OpenbimIfcModel *out_model, uint8_t *error_buffer, size_t capacity);
```

As `openbim_ifc_v0_1_model_open_mapped`, under the
`OPENBIM_IFC_PARSE_*` `flags`.

**Safety.**
As `openbim_ifc_v0_1_model_open_mapped`: the file must not be modified
or truncated until the model is destroyed.

### `openbim_ifc_v0_1_model_open_with_options`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_open_with_options(const uint8_t *path, size_t path_len, uint32_t flags, OpenbimIfcModel *out_model, uint8_t *error_buffer, size_t capacity);
```

As `openbim_ifc_v0_1_model_open`, under the `OPENBIM_IFC_PARSE_*`
`flags`.

**Safety.**
As `openbim_ifc_v0_1_model_open`.

### `openbim_ifc_v0_1_model_parse`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_parse(const uint8_t *data, size_t len, OpenbimIfcModel *out_model, uint8_t *error_buffer, size_t capacity);
```

Parse `len` bytes of STEP and write the new model's handle.

An ifcZIP archive, recognized by its ZIP signature, is read too: its one
STEP or ifcXML model, within a decompression budget. `Parse` for an
archive with no model or several, `Unsupported` for an encrypted entry
or a compression other than Stored and Deflate, `BudgetExceeded` for an
entry that inflates past the budget.

A parse failure has no model to hold its error, so the message is written
to the optional `error_buffer` (NUL-terminated, truncated to `capacity`).

**Safety.**
`data` must be valid for `len` reads; `out_model` for one write;
`error_buffer`, if non-null, for `capacity` writes.

### `openbim_ifc_v0_1_model_parse_ifcxml`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_parse_ifcxml(const uint8_t *data, size_t len, const uint8_t *profile, size_t profile_len, OpenbimIfcModel *out_model, uint8_t *error_buffer, size_t capacity);
```

Parse `len` bytes of ifcXML and write the new model's handle.

`profile` (UTF-8, `profile_len` bytes) is null with length 0 for the
native layout, else `IFC4` or `IFC4X3_ADD2` (any case) for that
release's XSD configuration. `UnsupportedProfile` for any other name,
`UnsupportedSchema` when the release is not bundled, `Parse` when the
document does not read, `FeatureDisabled` without the `ifcxml` feature.
The message goes to the optional `error_buffer` as for
`openbim_ifc_v0_1_model_parse`.

**Safety.**
As `openbim_ifc_v0_1_model_parse`; `profile`, if non-null, valid for
`profile_len` reads.

### `openbim_ifc_v0_1_model_parse_with_options`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_parse_with_options(const uint8_t *data, size_t len, uint32_t flags, OpenbimIfcModel *out_model, uint8_t *error_buffer, size_t capacity);
```

As `openbim_ifc_v0_1_model_parse`, under the `OPENBIM_IFC_PARSE_*`
`flags` (0 is the strict read). What a lenient read recovers from is
listed by `openbim_ifc_v0_1_model_diagnostic`.

**Safety.**
As `openbim_ifc_v0_1_model_parse`.

### `openbim_ifc_v0_1_model_product_curves`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_product_curves(OpenbimIfcModel model, const uint64_t *ids, size_t id_count, const uint8_t *representation, size_t representation_len, OpenbimIfcCurves *out_curves);
```

Compile the curves of each product's `representation` as polylines
(#458), for `ids` (`id_count` of them) or, with `ids` null and
`id_count` 0, every product with a shape, and write the new set's
handle to `out_curves`. `representation` is `representation_len` bytes
of UTF-8: `Plan` (the best drawing curves: `Plan`, `Annotation`,
`FootPrint`, then `Axis`, preferring a `PLAN_VIEW` context), `Axis`,
`FootPrint`, `Annotation`, `Body` or `Surface`. Read the set with
`openbim_ifc_v0_1_curves_records` and `_curves_positions`; destroy it
with `openbim_ifc_v0_1_curves_destroy`. The chord budget is one
millimetre.

A product whose curves cannot be compiled is a record with a refusal,
not a failed call: a curve the reference compiler has no certified
polyline for (an implicit curve, an offset of an offset) is
`unsupported`, naming the product; trimmed, composite, offset and
intrinsic curves compile (#506). `InvalidValue` for another
representation name,
`InvalidArgument` for text that is not UTF-8, `UnsupportedSchema`;
`FeatureDisabled` in a library built without the `mesh` feature.

**Safety.**
`ids` valid for `id_count` reads when non-null; `representation` valid
for `representation_len` reads; `out_curves` valid for one write.

### `openbim_ifc_v0_1_model_product_geometry`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_product_geometry(OpenbimIfcModel model, const uint64_t *ids, size_t id_count, uint32_t encoding, OpenbimIfcGraphs *out_graphs);
```

Lower the Body of each of `ids` (`id_count` of them) or, with `ids` null
and `id_count` 0, of every product with a shape, encode each graph in
Axiolid's wire format as `encoding` (`OPENBIM_IFC_GEOMETRY_JSON` or
`_CBOR`), and write the new set's handle to `out_graphs`. Read it with
`openbim_ifc_v0_1_graphs_records` and `_graphs_payload`; destroy it with
`openbim_ifc_v0_1_graphs_destroy`.

The payload is the envelope `{"format":"axiolid-geometry-graph",
"version":"1.0","graph":{"nodes":...,"roots":...}}`, in world
coordinates, metres; its version is the lowest the content needs,
"1.0"; "1.1" when a station carries a seam-snapping window, "1.2" when a run's sections do, "1.3" when an offset run continues to a basis end the kernel measures, "1.4" when a polygonal bounded half-space clips. A product that cannot be lowered is a record with
a refusal, not a failed call. `InvalidArgument` for another `encoding`;
`UnsupportedSchema`; `FeatureDisabled` in a library built without the
`graph` feature.

**Safety.**
`ids` valid for `id_count` reads when non-null; `out_graphs` valid for
one write.

### `openbim_ifc_v0_1_model_product_meshes`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_product_meshes(OpenbimIfcModel model, const uint64_t *ids, size_t id_count, OpenbimIfcMeshes *out_meshes);
```

Compile the Body mesh of each of `ids` (`id_count` of them) or, with
`ids` null and `id_count` 0, of every product with a shape, and write
the new set's handle to `out_meshes`. Read it with
`openbim_ifc_v0_1_meshes_records`, `_meshes_positions` and
`_meshes_indices`; destroy it with `openbim_ifc_v0_1_meshes_destroy`.

A product that cannot be meshed is a record with a refusal, not a
failed call. `UnsupportedSchema`; `FeatureDisabled` in a library built
without the `mesh` feature (the default build).

**Safety.**
`ids` valid for `id_count` reads when non-null; `out_meshes` valid for
one write.

### `openbim_ifc_v0_1_model_product_meshes_for`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_product_meshes_for(OpenbimIfcModel model, const uint64_t *ids, size_t id_count, const uint8_t *representation, size_t representation_len, OpenbimIfcMeshes *out_meshes);
```

As `openbim_ifc_v0_1_model_product_meshes`, meshing each product's
`representation` (#458): `representation_len` bytes of UTF-8 at
`representation`, one of `Body` (as the call above), `Plan`, `Axis`,
`FootPrint`, `Annotation` or `Surface`. Its surfaces and solids are one
mesh, whose record's closure says `solid` or `surface`; its curves are
left to `openbim_ifc_v0_1_model_product_curves`. A product without the
representation, or with curves only, has an empty mesh and no refusal.
Only a Body mesh carries a colour.

`InvalidValue` for another name, `InvalidArgument` for text that is not
UTF-8; otherwise as the call above.

**Safety.**
`representation` valid for `representation_len` reads; otherwise as
for `openbim_ifc_v0_1_model_product_meshes`.

### `openbim_ifc_v0_1_model_product_placements`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_product_placements(OpenbimIfcModel model, const uint64_t *ids, size_t id_count, size_t *out_count, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

Each product's world placement and selected Body representation, for
`ids` (`id_count` of them) or, with `ids` null and `id_count` 0, every
product with a shape, in id order. The tape is a `LIST` of
`ProductPlacement` records; `out_count` gets their number.
`ProductPlacement`: id (`REF`), global id, type name, transform (`LIST`
of 16 `REAL`s, a column-major 4x4 in metres, or `NULL`), representation
(`SelectedRepresentation` or `NULL`), refusal (`GeometryRefusal` or
`NULL`). `SelectedRepresentation`: id, identifier, representation type,
context (`REF` or `NULL`), context type, context identifier, target
view. `GeometryRefusal`: code (`unsupported`, `invalid-model`,
`missing-reference` or `budget-exceeded`), entity (`REF` or `NULL`),
message.

A product that cannot be placed is a record with a refusal, not a
failed call. `UnsupportedSchema`, `FeatureDisabled`.

**Safety.**
`ids` valid for `id_count` reads when non-null; `out_count` valid for
one write; otherwise as for `openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_model_property_sets`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_property_sets(OpenbimIfcModel model, uint64_t object, size_t *out_count, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

The property sets of `object`: a `LIST` of `PropertySet` records, its
own sets first, then those its type object holds; `out_count` gets their
number. `PropertySet`: id (`REF`), global id, name, type name, source
(`occurrence` or `type`), source id (`REF` or `NULL`), properties
(`LIST` of `Property`). `Property`: id, name, type name, kind, value
type, unit (`REF` or `NULL`), value (tagged, typed), enumeration,
bounds, table, usage, discrimination, quality, members (`LIST` of
`Property`).

`MissingEntity`, `WrongEntityType`, `UnsupportedSchema` (a release other
than IFC2X3, IFC4 or IFC4X3), `InvalidModel`, `FeatureDisabled`.

**Safety.**
`out_count` valid for one write; otherwise as for
`openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_model_property_sets_many`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_property_sets_many(OpenbimIfcModel model, const uint64_t *ids, size_t id_count, size_t *out_count, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

The property sets of each of `ids` (`id_count` of them), in that
order, or, with `ids` null and `id_count` 0, of every object definition
(`IfcObjectDefinition` and its subtypes) in file order, resolved in one
pass through one property index (#358): a `LIST` of
`ObjectPropertySets` records, their number in `out_count`.
`ObjectPropertySets`: object (`REF`), sets (`LIST` of `PropertySet`, as
for `openbim_ifc_v0_1_model_property_sets`; empty when refused),
refusal (`PropertyRefusal` or `NULL`: code (`TEXT`, the shared binding
code), message (`TEXT`)). Each record is exactly what the per-object
call answers for that id, its refusal included.

Refused as a whole only for the model: `UnsupportedSchema`,
`InvalidModel` (a lenient read with skipped records),
`FeatureDisabled`; `NullPointer` for null `ids` with a non-zero count.

**Safety.**
`ids` valid for `id_count` reads when non-null; `out_count` valid for
one write; otherwise as for `openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_model_purge`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_purge(OpenbimIfcModel model, const uint8_t *const *classes, const size_t *class_lengths, size_t class_count, uint32_t flags, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

Purge unreferenced entities as one checked transaction. With
`class_count` 0, every resource entity no `IfcRoot` reaches (nor
annotates something reached); otherwise entities of the `class_count`
classes (UTF-8 names of `class_lengthsi` bytes, any case, subtypes
included) that nothing references. `flags`:
`OPENBIM_IFC_PURGE_INCLUDE_ROOTS`,
`OPENBIM_IFC_PURGE_INCLUDE_OWNER_HISTORY`, `OPENBIM_IFC_PURGE_DRY_RUN`.
The answer is a `RemovalReport` tape: removed (`LIST` of
`RemovedEntity`: id `REF`, type name `TEXT`), kept (`LIST` of
`KeptEntity`: id, type name, reason `TEXT` -- `shared`,
`related-object`, `definition`, `protected` -- users `LIST` of `REF`),
detached and rebased (`LIST`s of `REF`), applied (`BOOL`, false for a
dry run). A size query or a short buffer plans the removal and answers
`BufferTooSmall` without applying it.

`UnsupportedSchema` for a class the release does not declare;
`InvalidArgument` for a class that is not UTF-8 or an unknown flag;
`FeatureDisabled`.

**Safety.**
`classes` and `class_lengths` null (with `class_count` 0) or valid for
`class_count` reads, each class valid for its length; the buffers as
for `openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_model_remove_product`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_remove_product(OpenbimIfcModel model, uint64_t product, uint32_t flags, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

Remove product (or type object) `product` with everything it solely
owns -- its placement, representation, own property and quantity sets,
material usage, openings and ports, and relationships left without an
end -- keeping what is shared, as one checked transaction. `flags`:
`OPENBIM_IFC_REMOVE_REBASE_PLACEMENTS`,
`OPENBIM_IFC_REMOVE_UNTYPE_OCCURRENCES`, `OPENBIM_IFC_REMOVE_DRY_RUN`.
The answer is a `RemovalReport` tape: removed (`LIST` of
`RemovedEntity`: id `REF`, type name `TEXT`), kept (`LIST` of
`KeptEntity`: id, type name, reason `TEXT` -- `shared`,
`related-object`, `definition`, `protected` -- users `LIST` of `REF`),
detached and rebased (`LIST`s of `REF`), applied (`BOOL`, false for a
dry run). A size query or a short buffer plans the removal and answers
`BufferTooSmall` without applying it.

`MissingEntity`; `WrongEntityType` for an entity that is neither an
`IfcProduct` nor an `IfcTypeObject`; `StillReferenced` for a placement
other placements are relative to, a type that still types objects, or
an entity that stays and needs something removed; `Unsupported` for a
placement that cannot be re-based; `InvalidArgument` for an unknown
flag; `FeatureDisabled`.

**Safety.**
As for `openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_model_remove_property`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_remove_property(OpenbimIfcModel model, uint64_t object, const uint8_t *set, size_t set_len, const uint8_t *name, size_t name_len);
```

Remove one property from `object`'s own set:
`openbim_ifc_v0_1_model_set_properties` with one `REMOVE` edit.

**Safety.**
`set` and `name` valid for their lengths (null only for 0).

### `openbim_ifc_v0_1_model_remove_representation`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_remove_representation(OpenbimIfcModel model, uint64_t representation, uint32_t flags, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

Remove shape representation `representation` with what only it uses, as
one checked transaction: taken off its product definition shape (which
goes when left empty), representation map or shape aspects, then removed
with its items down to what nothing else uses; shared items, profiles,
points and styles stay. `flags`: `OPENBIM_IFC_REMOVE_DRY_RUN` alone. The
answer is the `RemovalReport` tape of
`openbim_ifc_v0_1_model_remove_product`; a size query or a short buffer
plans the removal and answers `BufferTooSmall` without applying it.

`MissingEntity`; `WrongEntityType` for an entity that is not an
`IfcShapeModel`; `StillReferenced` for a representation map an
`IfcMappedItem` still maps; `InvalidArgument` for another flag;
`FeatureDisabled`.

**Safety.**
As for `openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_model_resolve_unit`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_resolve_unit(OpenbimIfcModel model, const uint8_t *measure_type, size_t measure_len, uint64_t unit, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

The effective unit of a `measure_type` value (`IFCAREAMEASURE`, UTF-8,
`measure_len` bytes): `unit` when non-zero (a property's stated unit),
otherwise the project default. The tape is one `ResolvedUnit`: unit
(`REF` or `NULL`), from project (`BOOL`), dimensions (`LIST` of seven
`INTEGER`s, SI exponents L M T I Θ N J), scale (`REAL`), offset (`REAL`).

`InvalidValue` for a type that is no measure, `Unsupported`,
`InvalidModel`, `FeatureDisabled`.

**Safety.**
`measure_type` valid for `measure_len` reads; otherwise as for
`openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_model_schema`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_schema(OpenbimIfcModel model, uint8_t *buffer, size_t capacity, size_t *out_required);
```

The first `FILE_SCHEMA` token as a NUL-terminated string, or `NoValue`.

**Safety.**
As for `openbim_ifc_v0_1_model_write`.

### `openbim_ifc_v0_1_model_select`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_select(OpenbimIfcModel model, const uint8_t *query, size_t query_len, uint64_t *buffer, size_t capacity, size_t *out_required);
```

The ids selector `query` (UTF-8, `query_len` bytes) selects, ascending,
filling `buffer` as `openbim_ifc_v0_1_model_ids` does.

Class facets (subtypes included) add entities, `!` removes them;
attribute, property and `type=`, `material=`, `classification=`,
`location=` and `group=` facets filter; `,` chains facets and `+`
unites lists. `InvalidSelector` (span through
`openbim_ifc_v0_1_last_error_span`), `UnsupportedSchema`,
`InvalidModel` for an ambiguous or malformed relationship or a
duplicated `GlobalId` an instance facet names, `MissingReference`, and
the property refusals of `openbim_ifc_v0_1_entity_property_sets`.

**Safety.**
`query` valid for `query_len` reads; otherwise as for
`openbim_ifc_v0_1_model_ids`.

### `openbim_ifc_v0_1_model_select_from`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_select_from(OpenbimIfcModel model, const uint8_t *query, size_t query_len, const uint64_t *base, size_t base_count, uint64_t *buffer, size_t capacity, size_t *out_required);
```

As `openbim_ifc_v0_1_model_select`, selecting from the `base_count`
ids at `base` alone; `MissingEntity` for one the model lacks.

**Safety.**
`base` valid for `base_count` reads; otherwise as for
`openbim_ifc_v0_1_model_select`.

### `openbim_ifc_v0_1_model_set_header`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_set_header(OpenbimIfcModel model, const OpenbimIfcValueNode *nodes, size_t node_count, const uint8_t *strings, size_t string_len);
```

Replace the file header with the one on the tape. A tape of any other
shape is `InvalidValue` and leaves the header unchanged.

**Safety.**
As for `openbim_ifc_v0_1_entity_set_attribute`.

### `openbim_ifc_v0_1_model_set_properties`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_set_properties(OpenbimIfcModel model, const OpenbimIfcValueNode *nodes, size_t node_count, const uint8_t *strings, size_t string_len, uint64_t *out_properties, size_t properties_capacity, size_t *out_count);
```

Apply the edits on the tape as one checked transaction: all of them, in
order, or none, and the model unchanged.

`out_count` gets the number of edits. `out_properties` gets, per edit,
the id of the entity holding the property afterwards, or 0 when the
batch leaves none; with `properties_capacity` below the edit count the
call returns `BufferTooSmall` and applies nothing.

`InvalidArgument` or `InvalidValue` for a malformed tape;
`MissingEntity`, `WrongEntityType`, `UnsupportedSchema`, `InvalidValue`,
`TemplateViolation`, `MissingProperty`, `Unsupported`, `InvalidModel`,
`FeatureDisabled` as the shared codes say.

**Safety.**
`nodes` valid for `node_count` reads and `strings` for `string_len`
(either null when its length is 0); `out_properties` null with capacity
0, or valid for `properties_capacity` writes; `out_count` valid for one
write.

### `openbim_ifc_v0_1_model_set_property`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_set_property(OpenbimIfcModel model, uint64_t object, const uint8_t *set, size_t set_len, const uint8_t *name, size_t name_len, const OpenbimIfcValueNode *nodes, size_t node_count, const uint8_t *strings, size_t string_len, const uint8_t *set_type, size_t set_type_len, uint64_t *out_id);
```

Write one value: `openbim_ifc_v0_1_model_set_properties` with one
`SET` edit. `set` and `name` are UTF-8; the value is a one-value tape;
`set_type` (`IfcPropertySet` or `IfcElementQuantity`) may be null with
length 0. `out_id` gets the entity holding the property.

**Safety.**
`set`, `name` and `set_type` valid for their lengths (null only for 0);
the tape as for `openbim_ifc_v0_1_entity_set_attribute`; `out_id` valid
for one write.

### `openbim_ifc_v0_1_model_spatial_tree`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_spatial_tree(OpenbimIfcModel model, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

The spatial containment tree as one `SpatialTree` record: release
(`TEXT` or `NULL`), roots (`LIST` of `REF`), nodes (`LIST` of
`SpatialNode`: id, global id, name, type name, kind, parent, children,
elements, referenced), orphans, dangling (`LIST` of `[relation,
target]`), anomalies (`LIST` of `kind, relation, subject, kept`).

`UnsupportedSchema` when the header names no bundled release;
`FeatureDisabled`.

**Safety.**
As for `openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_model_systems`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_systems(OpenbimIfcModel model, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

Every system as one `Systems` record: systems (`LIST` of `System`: id,
global id, type name, name, long name, predefined type, members,
serviced buildings, serviced facilities) and anomalies (`LIST` of
`kind, subject, other, message`).

`UnsupportedSchema`, `FeatureDisabled`.

**Safety.**
As for `openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_model_unreachable_products`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_unreachable_products(OpenbimIfcModel model, size_t *out_count, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

Products no viewer will draw, in id order, and how many.

The tape is a `LIST` of products; each is a `LIST` of four values: the
product (`REF`), the reason (`TEXT`: `not-contained-in-spatial-structure`,
`no-representation-in-model-context` or `representation-without-context`),
the target views its geometry was found in instead (`LIST` of `TEXT`,
empty unless the reason is the second), and a one-line message (`TEXT`).
`FeatureDisabled` in a build without the `unreachable` feature.

**Safety.**
`out_count` valid for one write; otherwise as for
`openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_model_validate`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_validate(OpenbimIfcModel model, size_t max_findings, OpenbimIfcValidationSummary *out_summary, OpenbimIfcValueNode *nodes, size_t node_capacity, size_t *out_nodes_required, uint8_t *strings, size_t string_capacity, size_t *out_strings_required);
```

Validate against the schema the header declares.

`out_summary` gets the counts, also when the tape does not fit. The tape
is a `LIST` of findings, sorted by severity, rule, entity and slot; each
finding is a `LIST` of seven values: severity (`TEXT`: `error`,
`evaluation-error`, `warning` or `unsupported`), rule id (`TEXT`),
entity (`REF`, or `NULL` for the file), attribute index (`INTEGER` or
`NULL`), attribute name (`TEXT` or `NULL`), path (`TEXT`) and message
(`TEXT`). `max_findings` 0 is the validator's default budget (10,000).
`UnsupportedSchema` when the declared schema is not bundled;
`FeatureDisabled` in a build without the `validate` feature.

**Safety.**
`out_summary` valid for one write; otherwise as for
`openbim_ifc_v0_1_entity_attribute`.

### `openbim_ifc_v0_1_model_write`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_write(OpenbimIfcModel model, uint8_t *buffer, size_t capacity, size_t *out_required);
```

Serialize as STEP into a caller buffer; `out_required` gets the size.

**Safety.**
`buffer` must be null (with `capacity` 0) or valid for `capacity`
writes; `out_required` valid for one write.

### `openbim_ifc_v0_1_model_write_ifcxml`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_write_ifcxml(OpenbimIfcModel model, const uint8_t *profile, size_t profile_len, uint8_t *buffer, size_t capacity, size_t *out_required);
```

Serialize as ifcXML into a caller buffer; `out_required` gets the size.
`profile` as for `openbim_ifc_v0_1_model_parse_ifcxml`. An XSD-layout
write needs the header to declare the profile's schema, and refuses
with `Write` what the configuration cannot carry.

**Safety.**
As `openbim_ifc_v0_1_model_write`; `profile`, if non-null, valid for
`profile_len` reads.

### `openbim_ifc_v0_1_model_write_ifczip`

```c
OpenbimIfcStatus openbim_ifc_v0_1_model_write_ifczip(OpenbimIfcModel model, const uint8_t *inner, size_t inner_len, const uint8_t *profile, size_t profile_len, uint8_t *buffer, size_t capacity, size_t *out_required);
```

Serialize as an ifcZIP archive into a caller buffer; `out_required` gets
the size.

`inner` (UTF-8, `inner_len` bytes) is null with length 0 or `step` for
STEP inside (`model.ifc`), or `ifcxml` (any case) for ifcXML
(`model.ifcXML`) in the layout `profile` names as for
`openbim_ifc_v0_1_model_write_ifcxml`. `InvalidValue` for another
`inner` or a `profile` with STEP inside; otherwise as the inner write,
and `FeatureDisabled` without the `zip` feature.

**Safety.**
As `openbim_ifc_v0_1_model_write`; `inner` and `profile`, if non-null,
valid for `inner_len` and `profile_len` reads.

### `openbim_ifc_v0_1_name_based_guid`

```c
OpenbimIfcStatus openbim_ifc_v0_1_name_based_guid(const uint8_t *const *parts, const size_t *part_lengths, size_t part_count, uint8_t *buffer, size_t capacity, size_t *out_required);
```

The `GlobalId` named by `parts` (RFC 9562 version 8), NUL-terminated:
the same parts give the same `GlobalId` in every binding and release.
`parts` holds `part_count` UTF-8 strings of `part_lengthsi` bytes
each. Needs no model; `InvalidArgument` for a part that is not UTF-8.

**Safety.**
`parts` and `part_lengths` null (with `part_count` 0) or valid for
`part_count` reads, each part valid for its length; `buffer` and
`out_required` as for `openbim_ifc_v0_1_new_guid`.

### `openbim_ifc_v0_1_new_guid`

```c
OpenbimIfcStatus openbim_ifc_v0_1_new_guid(uint8_t *buffer, size_t capacity, size_t *out_required);
```

A new random `GlobalId` (RFC 9562 version 4, from the operating
system's generator) in IFC's 22-character form, NUL-terminated: 23
bytes. Needs no model. `Unsupported` when the system has no random
source.

**Safety.**
`buffer` null (with `capacity` 0) or valid for `capacity` writes;
`out_required` valid for one write.

### `openbim_ifc_v0_1_version`

```c
OpenbimIfcStatus openbim_ifc_v0_1_version(OpenbimIfcVersion *out_version);
```

Write the ABI and crate versions.

**Safety.**
`out_version` must be null or valid for one write.

## Status codes

| Status | Value | Meaning |
| --- | ---: | --- |
| `OPENBIM_IFC_STATUS_OK` | 0 | Success. |
| `OPENBIM_IFC_STATUS_NULL_POINTER` | 1 | A required pointer was null. |
| `OPENBIM_IFC_STATUS_INVALID_ARGUMENT` | 2 | An argument was malformed (bad UTF-8, a malformed value tape, ...). |
| `OPENBIM_IFC_STATUS_INVALID_HANDLE` | 3 | The model handle is zero, stale, or already destroyed. |
| `OPENBIM_IFC_STATUS_BUFFER_TOO_SMALL` | 4 | The output buffer is smaller than `*out_required`. |
| `OPENBIM_IFC_STATUS_PARSE` | 10 | The STEP input could not be parsed (`parse`). |
| `OPENBIM_IFC_STATUS_WRITE` | 11 | The model could not be serialized (`write`). |
| `OPENBIM_IFC_STATUS_MISSING_ENTITY` | 12 | No entity has the given id (`missing-entity`). |
| `OPENBIM_IFC_STATUS_INVALID_VALUE` | 13 | A value did not follow the encoding (`invalid-value`). |
| `OPENBIM_IFC_STATUS_OUT_OF_RANGE` | 14 | An id or index is outside the representable range (`out-of-range`). |
| `OPENBIM_IFC_STATUS_UNSUPPORTED_SCHEMA` | 15 | The file's schema is not bundled, or a domain view does not read it (`unsupported-schema`). |
| `OPENBIM_IFC_STATUS_IO` | 16 | A file could not be opened or read (`io`). |
| `OPENBIM_IFC_STATUS_UNSUPPORTED_PROFILE` | 17 | No ifcXML XSD profile has that name (`unsupported-profile`). |
| `OPENBIM_IFC_STATUS_FEATURE_DISABLED` | 18 | This build leaves out the feature the call needs (`feature-disabled`). |
| `OPENBIM_IFC_STATUS_INVALID_MODEL` | 19 | A domain view refused the file's data as malformed, ambiguous or unprovable (`invalid-model`). |
| `OPENBIM_IFC_STATUS_NO_VALUE` | 20 | The requested value does not exist (no schema token, no error, ...). |
| `OPENBIM_IFC_STATUS_MISSING_REFERENCE` | 21 | A domain view followed a reference to an entity the file lacks (`missing-reference`). |
| `OPENBIM_IFC_STATUS_BUDGET_EXCEEDED` | 22 | A domain view stopped at a cycle or its depth budget (`budget-exceeded`). |
| `OPENBIM_IFC_STATUS_UNSUPPORTED` | 23 | A domain view met a construct it does not interpret (`unsupported`). |
| `OPENBIM_IFC_STATUS_WRONG_ENTITY_TYPE` | 24 | A domain query named an entity of a type it does not accept, or a property edit a set type the set does not have (`wrong-entity-type`). |
| `OPENBIM_IFC_STATUS_TEMPLATE_VIOLATION` | 25 | A property edit wrote a value its PSD/QTO template or property enumeration refuses (`template-violation`). |
| `OPENBIM_IFC_STATUS_MISSING_PROPERTY` | 26 | A property edit removed a property the object does not state (`missing-property`). |
| `OPENBIM_IFC_STATUS_CATALOG_NOT_LOADED` | 27 | A property edit wrote to a `Pset_`/`Qto_` set before its release's catalog was loaded (`catalog-not-loaded`). This library embeds the catalog, so it never returns this; the value is reserved so every binding code has one. |
| `OPENBIM_IFC_STATUS_UNKNOWN_ATTRIBUTE` | 28 | A by-name attribute access named no attribute of the entity's type in the declared release, explicit or (for a read) `INVERSE` (`unknown-attribute`). |
| `OPENBIM_IFC_STATUS_DERIVED_ATTRIBUTE` | 29 | A by-name attribute write named a slot the entity's type derives, written `*` (`derived-attribute`). |
| `OPENBIM_IFC_STATUS_MISSING_ATTRIBUTE` | 30 | An authoring batch left a required attribute of the declared release unset (`missing-attribute`). |
| `OPENBIM_IFC_STATUS_STILL_REFERENCED` | 31 | An authoring batch removed an entity that an entity other than a relationship still references (`still-referenced`). |
| `OPENBIM_IFC_STATUS_TYPE_MISMATCH` | 32 | A plain value written by name does not fit the attribute's declared type (`type-mismatch`). |
| `OPENBIM_IFC_STATUS_AMBIGUOUS_VALUE` | 33 | A plain value written by name fits several members of the attribute's SELECT; write it exactly instead (`ambiguous-value`). |
| `OPENBIM_IFC_STATUS_INVALID_SELECTOR` | 34 | A selector or value path does not follow the grammar, or names a class or attribute the declared release does not declare (`invalid-selector`); `openbim_ifc_v0_1_last_error_span` gives the byte range it refuses. |
| `OPENBIM_IFC_STATUS_INVERSE_ATTRIBUTE` | 35 | A by-name attribute write named an `INVERSE` attribute, which holds no slot (`inverse-attribute`). |
| `OPENBIM_IFC_STATUS_PANIC` | 255 | A Rust panic was contained at the boundary. Report it as a bug. |

<!-- API:C:END -->
