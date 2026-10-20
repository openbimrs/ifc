# C and C++ cookbook

Task-sized recipes for the [C ABI](/bindings/c), `openbim_ifc.h`. Each
recipe is a region of `crates/openbim-ifc-capi/tests/c/cookbook.c`, which
the gate compiles as strict C11 and as C++17, links against the real
library (and against one built with meshes) and runs on every pull
request, checking what each recipe prints; the code below is the code that
ran. The comments show its output for the fixtures it reads:
`test/fixtures/synthetic-properties/synthetic_properties.ifc` (two walls on
one storey, with their property sets) and
`test/fixtures/synthetic-bindings/binding_geometry.ifc` (placed, extruded
products).

Every function, record and status is in the [C API reference](/api/c).

## Set up

Link the CMake package, `openbim_ifc::openbim_ifc`, or use pkg-config, as
the [binding page](/bindings/c#install-via-cmake) shows. The recipes need
`openbim_ifc.h` and the C standard library:

```text
#include "openbim_ifc.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
```

Every call that returns variable-size data writes into caller buffers: call
once with `NULL, 0` for the size, allocate, call again. Records and IFC
values come back as a *value tape*, a flat array of `OpenbimIfcValueNode`
in pre-order plus one string buffer; a record is a `LIST` of its fields in
a fixed order, which the [binding page](/bindings/c#domain-record-tapes)
tables list. The recipes walk tapes with these helpers:

<!-- SNIPPET:cookbook-c-tape -->

```c
/* A value tape the library filled: pre-order nodes plus their strings. */
typedef struct {
  OpenbimIfcValueNode *nodes;
  size_t node_count;
  uint8_t *strings;
  size_t string_len;
} Tape;

/* Allocate a tape for the sizes a first call with NULL buffers reported. */
static Tape tape_alloc(size_t nodes, size_t strings) {
  Tape t;
  t.nodes = (OpenbimIfcValueNode *)calloc(nodes ? nodes : 1, sizeof(OpenbimIfcValueNode));
  t.node_count = nodes;
  t.strings = (uint8_t *)malloc(strings ? strings : 1);
  t.string_len = strings;
  return t;
}

static void tape_free(Tape *t) {
  free(t->nodes);
  free(t->strings);
}

/* The index after the value at `i`: a LIST spans its children, a TYPED its one value. */
static size_t tape_next(const Tape *t, size_t i) {
  if (t->nodes[i].kind == OPENBIM_IFC_KIND_LIST) {
    size_t j = i + 1;
    for (uint32_t c = 0; c < t->nodes[i].child_count; c++) j = tape_next(t, j);
    return j;
  }
  if (t->nodes[i].kind == OPENBIM_IFC_KIND_TYPED) return tape_next(t, i + 1);
  return i + 1;
}

/* Field `n` of the record (a LIST) at `i`. */
static size_t tape_field(const Tape *t, size_t i, uint32_t n) {
  size_t j = i + 1;
  while (n-- > 0) j = tape_next(t, j);
  return j;
}

/* The string of a TEXT, ENUM or TYPED node, as printf's "%.*s" arguments. */
#define TAPE_STR(t, i) (int)(t)->nodes[i].str_len, (const char *)(t)->strings + (t)->nodes[i].str_offset

/* Print the value at `i`: TYPED IFCLABEL(TEXT "F30") prints F30. */
static void tape_print(const Tape *t, size_t i) {
  const OpenbimIfcValueNode *n = &t->nodes[i];
  switch (n->kind) {
    case OPENBIM_IFC_KIND_TYPED: tape_print(t, i + 1); break;
    case OPENBIM_IFC_KIND_TEXT:
    case OPENBIM_IFC_KIND_ENUM: printf("%.*s", TAPE_STR(t, i)); break;
    case OPENBIM_IFC_KIND_BOOL: printf("%s", n->int_value ? "true" : "false"); break;
    case OPENBIM_IFC_KIND_INTEGER: printf("%lld", (long long)n->int_value); break;
    case OPENBIM_IFC_KIND_REAL: printf("%g", n->real_value); break;
    case OPENBIM_IFC_KIND_REF: printf("#%lld", (long long)n->int_value); break;
    case OPENBIM_IFC_KIND_LIST: {
      size_t j = i + 1;
      printf("(");
      for (uint32_t c = 0; c < n->child_count; c++, j = tape_next(t, j)) {
        if (c > 0) printf(", ");
        tape_print(t, j);
      }
      printf(")");
      break;
    }
    default: printf("$"); break;
  }
}

/* The library's message for the last failure on `model`. */
static void print_error(OpenbimIfcModel model) {
  char message[512];
  size_t need = 0;
  openbim_ifc_v0_1_last_error_message(model, (uint8_t *)message, sizeof message, &need);
  fprintf(stderr, "%s\n", message);
}
```

<!-- /SNIPPET -->

## Open a file

<!-- SNIPPET:cookbook-c-open -->

```c
/* Read a file straight from disk; on failure the message lands in `error`. */
OpenbimIfcModel model = 0;
char error[256] = "";
if (openbim_ifc_v0_1_model_open((const uint8_t *)path, strlen(path), &model, (uint8_t *)error,
                                sizeof error) != OPENBIM_IFC_STATUS_OK) {
  fprintf(stderr, "cannot read %s: %s\n", path, error);
  return 1;
}
size_t entities = 0, need = 0;
char schema[32];
openbim_ifc_v0_1_model_len(model, &entities);
openbim_ifc_v0_1_model_schema(model, (uint8_t *)schema, sizeof schema, &need);
printf("%s, %zu entities\n", schema, entities); /* IFC4, 68 entities */
openbim_ifc_v0_1_model_destroy(model);
```

<!-- /SNIPPET -->

`openbim_ifc_v0_1_model_parse` reads bytes you already hold, and
`openbim_ifc_v0_1_model_open_mapped` maps the file instead of reading it.
Every failure is an `OpenbimIfcStatus`; on a model,
`openbim_ifc_v0_1_last_error_code` and `_message` explain the last one
(the `print_error` helper above calls the latter).

## Read property sets

One object's property sets, its own first, then those it inherits from
its type:

<!-- SNIPPET:cookbook-c-property-sets -->

```c
/* Size, allocate, fetch: the property sets of `wall`, its own then its type's. */
size_t sets = 0, nodes = 0, strings = 0;
openbim_ifc_v0_1_model_property_sets(model, wall, &sets, NULL, 0, &nodes, NULL, 0, &strings);
Tape t = tape_alloc(nodes, strings);
if (openbim_ifc_v0_1_model_property_sets(model, wall, &sets, t.nodes, nodes, &nodes, t.strings,
                                         strings, &strings) != OPENBIM_IFC_STATUS_OK) {
  print_error(model);
  tape_free(&t);
  return 1;
}
/* A LIST of PropertySet records: name is field 2, source 4, properties 6.
 * A Property record: name is field 1, value 6. */
for (size_t s = 0, set = 1; s < sets; s++, set = tape_next(&t, set)) {
  size_t name = tape_field(&t, set, 2), source = tape_field(&t, set, 4);
  size_t properties = tape_field(&t, set, 6);
  for (size_t p = 0, prop = properties + 1; p < t.nodes[properties].child_count;
       p++, prop = tape_next(&t, prop)) {
    printf("%.*s.%.*s = ", TAPE_STR(&t, name), TAPE_STR(&t, tape_field(&t, prop, 1)));
    tape_print(&t, tape_field(&t, prop, 6));
    printf(" (%.*s)\n", TAPE_STR(&t, source));
  }
}
tape_free(&t);
/* Pset_WallCommon.IsExternal = false (occurrence)
 * Qto_WallBaseQuantities.Width = 200 (occurrence) ...
 * Pset_WallCommon.FireRating = F30 (type) */
```

<!-- /SNIPPET -->

Many objects in one call, here every wall:

<!-- SNIPPET:cookbook-c-property-sets-many -->

```c
/* Every wall in one pass: the property index is built once, so this
 * stays linear in the model where a loop of the call above is quadratic. */
uint64_t walls[64];
size_t wall_count = 0;
openbim_ifc_v0_1_model_ids_of_type(model, (const uint8_t *)"IfcWall", 7, walls, 64, &wall_count);
size_t answers = 0, nodes = 0, strings = 0;
openbim_ifc_v0_1_model_property_sets_many(model, walls, wall_count, &answers, NULL, 0, &nodes,
                                          NULL, 0, &strings);
Tape t = tape_alloc(nodes, strings);
openbim_ifc_v0_1_model_property_sets_many(model, walls, wall_count, &answers, t.nodes, nodes,
                                          &nodes, t.strings, strings, &strings);
/* An ObjectPropertySets record: object, sets, refusal ([code, message] or NULL). */
for (size_t a = 0, answer = 1; a < answers; a++, answer = tape_next(&t, answer)) {
  size_t sets = tape_field(&t, answer, 1), refusal = tape_field(&t, answer, 2);
  printf("#%lld: ", (long long)t.nodes[tape_field(&t, answer, 0)].int_value);
  if (t.nodes[refusal].kind == OPENBIM_IFC_KIND_LIST) {
    printf("%.*s\n", TAPE_STR(&t, refusal + 1));
    continue;
  }
  printf("%u set(s)\n", t.nodes[sets].child_count);
}
tape_free(&t);
```

<!-- /SNIPPET -->

A `NULL` id array with count 0 answers for every object definition in the
file.

## List storeys and their elements

<!-- SNIPPET:cookbook-c-storeys -->

```c
size_t nodes = 0, strings = 0;
openbim_ifc_v0_1_model_spatial_tree(model, NULL, 0, &nodes, NULL, 0, &strings);
Tape t = tape_alloc(nodes, strings);
openbim_ifc_v0_1_model_spatial_tree(model, t.nodes, nodes, &nodes, t.strings, strings, &strings);
/* A SpatialTree record; its field 2 is a LIST of SpatialNode records:
 * id, global id, name, type name, kind, parent, children, elements, ... */
size_t containers = tape_field(&t, 0, 2);
for (size_t c = 0, node = containers + 1; c < t.nodes[containers].child_count;
     c++, node = tape_next(&t, node)) {
  size_t kind = tape_field(&t, node, 4);
  if (t.nodes[kind].str_len != 6 || memcmp(t.strings + t.nodes[kind].str_offset, "storey", 6))
    continue;
  printf("%.*s\n", TAPE_STR(&t, tape_field(&t, node, 2)));
  size_t elements = tape_field(&t, node, 7);
  for (uint32_t e = 0; e < t.nodes[elements].child_count; e++) {
    uint64_t id = (uint64_t)t.nodes[elements + 1 + e].int_value; /* a REF */
    char type[64];
    size_t need = 0;
    openbim_ifc_v0_1_entity_type(model, id, (uint8_t *)type, sizeof type, &need);
    printf("  %s #%llu\n", type, (unsigned long long)id);
  }
}
tape_free(&t);
/* Level 0
 *   IFCWALL #30
 *   IFCWALL #31 */
```

<!-- /SNIPPET -->

Each node also has its parent (field 5) and children (field 6), so the same
tree gives the project, site and building above the storeys and the
spaces below them.

## Validate

<!-- SNIPPET:cookbook-c-validate -->

```c
/* The counts land in the summary even on the size query, which validates;
 * the findings are a LIST of [severity, rule, entity, attribute index,
 * attribute name, path, message]. */
OpenbimIfcValidationSummary summary;
size_t nodes = 0, strings = 0;
openbim_ifc_v0_1_model_validate(model, 0, &summary, NULL, 0, &nodes, NULL, 0, &strings);
printf("%zu error(s), %zu warning(s)\n", summary.errors, summary.warnings);
Tape t = tape_alloc(nodes, strings);
openbim_ifc_v0_1_model_validate(model, 0, &summary, t.nodes, nodes, &nodes, t.strings, strings,
                                &strings);
for (size_t f = 0, finding = 1; f < summary.finding_count; f++, finding = tape_next(&t, finding)) {
  printf("%.*s %.*s ", TAPE_STR(&t, tape_field(&t, finding, 0)),
         TAPE_STR(&t, tape_field(&t, finding, 1)));
  tape_print(&t, tape_field(&t, finding, 2)); /* #id, or $ for the file */
  printf(": %.*s\n", TAPE_STR(&t, tape_field(&t, finding, 6)));
}
tape_free(&t);
```

<!-- /SNIPPET -->

The recipe ran on the fixture with Wall A's `PredefinedType` set to an
item `IfcWallTypeEnum` does not have. A non-zero `max_findings` caps the
report; `summary.truncated` says when the cap was reached.

## Convert to and from ifcXML

<!-- SNIPPET:cookbook-c-ifcxml -->

```c
/* STEP to ifcXML in this library's lossless layout (profile NULL, 0), and back. */
size_t len = 0;
openbim_ifc_v0_1_model_write_ifcxml(model, NULL, 0, NULL, 0, &len);
uint8_t *xml = (uint8_t *)malloc(len);
openbim_ifc_v0_1_model_write_ifcxml(model, NULL, 0, xml, len, &len);
OpenbimIfcModel back = 0;
char error[256] = "";
if (openbim_ifc_v0_1_model_parse_ifcxml(xml, len, NULL, 0, &back, (uint8_t *)error,
                                        sizeof error) != OPENBIM_IFC_STATUS_OK) {
  fprintf(stderr, "%s\n", error);
}
free(xml);

/* The buildingSMART XSD layout of a release: name it as the profile. */
const char profile[] = "IFC4";
openbim_ifc_v0_1_model_write_ifcxml(model, (const uint8_t *)profile, strlen(profile), NULL, 0,
                                    &len);
```

<!-- /SNIPPET -->

The lossless layout carries everything the STEP file does. An XSD layout
refuses with `OPENBIM_IFC_STATUS_WRITE` what it cannot carry exactly.

## Edit a property set and attributes

<!-- SNIPPET:cookbook-c-edit -->

```c
/* A property value is an exact one-value tape: TYPED IFCLABEL over TEXT
 * "F90". Wall #31 inherits FireRating from its type: this overrides it on
 * the wall. Pset_ and Qto_ sets are checked against the release's PSD/QTO
 * catalog, which the library embeds. */
static const char label[] = "IFCLABELF90"; /* both strings, back to back */
OpenbimIfcValueNode value[2];
memset(value, 0, sizeof value);
value[0].kind = OPENBIM_IFC_KIND_TYPED;
value[0].str_len = 8; /* IFCLABEL */
value[1].kind = OPENBIM_IFC_KIND_TEXT;
value[1].str_offset = 8;
value[1].str_len = 3; /* F90 */
uint64_t holder = 0;
if (openbim_ifc_v0_1_model_set_property(model, wall, (const uint8_t *)"Pset_WallCommon", 15,
                                        (const uint8_t *)"FireRating", 10, value, 2,
                                        (const uint8_t *)label, strlen(label), NULL, 0,
                                        &holder) != OPENBIM_IFC_STATUS_OK) {
  print_error(model); /* a refused edit changes nothing */
}

/* An attribute takes a plain value, coerced against its declared type:
 * TEXT "solidwall" for IfcWallTypeEnum is written .SOLIDWALL. */
OpenbimIfcValueNode plain;
memset(&plain, 0, sizeof plain);
plain.kind = OPENBIM_IFC_KIND_TEXT;
plain.str_len = 9;
openbim_ifc_v0_1_entity_set_attribute_by_name_plain(model, wall, (const uint8_t *)"PredefinedType",
                                                    14, &plain, 1,
                                                    (const uint8_t *)"solidwall", 9);

/* Write the model back to disk. */
size_t len = 0;
openbim_ifc_v0_1_model_write(model, NULL, 0, &len);
uint8_t *step = (uint8_t *)malloc(len);
openbim_ifc_v0_1_model_write(model, step, len, &len);
FILE *file = fopen(path, "wb");
if (file != NULL) {
  fwrite(step, 1, len, file);
  fclose(file);
}
free(step);
```

<!-- /SNIPPET -->

`openbim_ifc_v0_1_model_set_properties` takes several edits as one checked
transaction (a `LIST` of edits, laid out on the
[binding page](/bindings/c#writing-property-sets)): all of them, or none.
A plain attribute value is coerced against the declared type; an
`OPENBIM_IFC_KIND_EXACT` node before a value writes it exactly. A refused
edit changes nothing.

## Read placements, meshes and exact geometry

<!-- SNIPPET:cookbook-c-placements -->

```c
/* ProductPlacement records for every product with a shape (NULL ids,
 * count 0): id, global id, type name, transform (16 REALs, column-major,
 * metres), representation, refusal ([code, entity, message] or NULL). */
size_t products = 0, nodes = 0, strings = 0;
openbim_ifc_v0_1_model_product_placements(model, NULL, 0, &products, NULL, 0, &nodes, NULL, 0,
                                          &strings);
Tape t = tape_alloc(nodes, strings);
openbim_ifc_v0_1_model_product_placements(model, NULL, 0, &products, t.nodes, nodes, &nodes,
                                          t.strings, strings, &strings);
for (size_t p = 0, product = 1; p < products; p++, product = tape_next(&t, product)) {
  size_t refusal = tape_field(&t, product, 5);
  if (t.nodes[refusal].kind == OPENBIM_IFC_KIND_LIST) { /* per product, not a failed call */
    printf("#%lld refused: %.*s\n", (long long)t.nodes[product + 1].int_value,
           TAPE_STR(&t, refusal + 1));
    continue;
  }
  size_t m = tape_field(&t, product, 3) + 1; /* the first of the 16 REALs */
  printf("%.*s at %g %g %g\n", TAPE_STR(&t, tape_field(&t, product, 2)), t.nodes[m + 12].real_value,
         t.nodes[m + 13].real_value, t.nodes[m + 14].real_value);
}
tape_free(&t);
/* IFCWALL at 512002 5.403e+06 3 ... */
```

<!-- /SNIPPET -->

Meshes link a geometry kernel, which the release archives leave out; build
with `cargo build --release -p openbim-ifc-capi --features mesh`, or CMake
with `-DOPENBIM_IFC_CARGO_FEATURES=mesh`:

<!-- SNIPPET:cookbook-c-meshes -->

```c
/* A library built with the `mesh` feature compiles every Body once into
 * a set; the release archives refuse with FEATURE_DISABLED. */
OpenbimIfcMeshes meshes = 0;
OpenbimIfcStatus status = openbim_ifc_v0_1_model_product_meshes(model, NULL, 0, &meshes);
size_t triangles = 0;
if (status == OPENBIM_IFC_STATUS_OK) {
  size_t count = 0, nodes = 0, strings = 0;
  openbim_ifc_v0_1_meshes_records(meshes, &count, NULL, 0, &nodes, NULL, 0, &strings);
  for (size_t i = 0; i < count; i++) {
    size_t need = 0;
    openbim_ifc_v0_1_meshes_positions(meshes, i, NULL, 0, &need);
    float *positions = (float *)malloc((need ? need : 1) * sizeof(float)); /* x y z, metres */
    openbim_ifc_v0_1_meshes_positions(meshes, i, positions, need, &need);
    openbim_ifc_v0_1_meshes_indices(meshes, i, NULL, 0, &need);
    uint32_t *indices = (uint32_t *)malloc((need ? need : 1) * sizeof(uint32_t));
    openbim_ifc_v0_1_meshes_indices(meshes, i, indices, need, &need);
    triangles += need / 3; /* positions are relative to the record's transform */
    free(positions);
    free(indices);
  }
  openbim_ifc_v0_1_meshes_destroy(meshes);
}
```

<!-- /SNIPPET -->

`openbim_ifc_v0_1_meshes_records` gives each mesh's record (id, global id,
type name, transform, vertex and triangle counts, refusal) on a tape.
Positions are `float`s relative to the record's `double` transform, so a
site kilometres from the origin keeps its millimetres.

Curves have no triangles: an `Axis`, a `FootPrint` or an annotation's
curves come as polylines from the same library
([#458](https://github.com/openbimrs/ifc/issues/458)), one position array
per product and a `ProductPolyline` record per curve naming its slice and
the certified bound on how far the exact curve lies from it:

<!-- SNIPPET:cookbook-c-curves -->

```c
/* With the `mesh` feature, the curves of a representation (#458) compile
 * once into a set: one position array per product, and per curve a
 * ProductPolyline record naming its slice and its certified chord bound. */
const char *representation = "Axis"; /* or Plan, FootPrint, Annotation */
OpenbimIfcCurves curves = 0;
OpenbimIfcStatus status = openbim_ifc_v0_1_model_product_curves(
    model, NULL, 0, (const uint8_t *)representation, strlen(representation), &curves);
if (status == OPENBIM_IFC_STATUS_OK) {
  size_t count = 0, nodes = 0, strings = 0;
  openbim_ifc_v0_1_curves_records(curves, &count, NULL, 0, &nodes, NULL, 0, &strings);
  Tape t = tape_alloc(nodes, strings);
  openbim_ifc_v0_1_curves_records(curves, &count, t.nodes, nodes, &nodes, t.strings, strings,
                                  &strings);
  size_t product = 1; /* the first record, inside the outer LIST */
  for (size_t i = 0; i < count; i++, product = tape_next(&t, product)) {
    size_t need = 0;
    openbim_ifc_v0_1_curves_positions(curves, i, NULL, 0, &need);
    float *positions = (float *)malloc((need ? need : 1) * sizeof(float)); /* x y z, metres */
    openbim_ifc_v0_1_curves_positions(curves, i, positions, need, &need);
    size_t polylines = tape_field(&t, product, 5), line = polylines + 1;
    for (uint32_t p = 0; p < t.nodes[polylines].child_count; p++, line = tape_next(&t, line)) {
      /* this curve's vertices: positions[3 * first .. 3 * (first + count)) */
      long long first = t.nodes[tape_field(&t, line, 1)].int_value;
      long long vertices = t.nodes[tape_field(&t, line, 2)].int_value;
      double bound = t.nodes[tape_field(&t, line, 4)].real_value;
      printf("#%lld: %lld vertices from %lld, bound %g\n",
             (long long)t.nodes[tape_field(&t, product, 0)].int_value, vertices, first, bound);
    }
    free(positions);
  }
  tape_free(&t);
  openbim_ifc_v0_1_curves_destroy(curves);
}
```

<!-- /SNIPPET -->

Trimmed, composite, offset and intrinsic curves compile
([#506](https://github.com/openbimrs/ifc/issues/506)). A curve the
reference compiler has no certified polyline for (an implicit curve, an
offset of an offset) is refused with code `unsupported`, its message the
compiler's reason.

The exact representation instead of triangles
([#367](https://github.com/openbimrs/ifc/issues/367)): each Body as
Axiolid's neutral geometry graph in its wire format, for your own
kernel. It is the cargo feature `graph` (`--features graph`, or
`-DOPENBIM_IFC_CARGO_FEATURES=graph`), which the release archives leave
out:

<!-- SNIPPET:cookbook-c-graphs -->

```c
/* A library built with the `graph` feature lowers and encodes every Body
 * once into a set; the release archives refuse with FEATURE_DISABLED.
 * Each payload is Axiolid's wire format, JSON here (or CBOR). */
OpenbimIfcGraphs graphs = 0;
OpenbimIfcStatus status =
    openbim_ifc_v0_1_model_product_geometry(model, NULL, 0, OPENBIM_IFC_GEOMETRY_JSON, &graphs);
size_t written = 0;
if (status == OPENBIM_IFC_STATUS_OK) {
  size_t count = 0, nodes = 0, strings = 0;
  openbim_ifc_v0_1_graphs_records(graphs, &count, NULL, 0, &nodes, NULL, 0, &strings);
  for (size_t i = 0; i < count; i++) {
    size_t need = 0;
    openbim_ifc_v0_1_graphs_payload(graphs, i, NULL, 0, &need);
    if (need == 0) continue; /* no Body, or refused: see the record */
    uint8_t *json = (uint8_t *)malloc(need);
    openbim_ifc_v0_1_graphs_payload(graphs, i, json, need, &need);
    /* hand json[0..need) to your kernel's reader of the wire format */
    written += 1;
    free(json);
  }
  openbim_ifc_v0_1_graphs_destroy(graphs);
}
```

<!-- /SNIPPET -->

`openbim_ifc_v0_1_graphs_records` gives each graph's record (id, global
id, type name, transform, encoding, payload size, refusal). The payload is
the envelope `{"format":"axiolid-geometry-graph","version":"1.0",
"graph":{...}}`, exact, in world coordinates, metres; the
[binding page](/bindings/c#geometry) describes it.

## Find entities by GlobalId, types and openings

An entity by its `GlobalId`, an occurrence's type object and the type's
other occurrences, and a wall's openings with what fills them, read from
`test/fixtures/synthetic-relations/relations_ifc4.ifc`:

<!-- SNIPPET:cookbook-c-global-ids -->

```c
/* The entity a GlobalId names, or NO_VALUE. The first lookup indexes every
 * GlobalId once; the rest reuse the index until the model changes. */
const char *guid = "0LwOtle8vEaxZPGHkxQdYT";
uint64_t wall = 0, wall_type = 0;
if (openbim_ifc_v0_1_model_by_guid(model, (const uint8_t *)guid, strlen(guid), &wall) !=
    OPENBIM_IFC_STATUS_OK) {
  print_error(model); /* none carries it, or several do (INVALID_MODEL) */
  return 1;
}
openbim_ifc_v0_1_entity_type_object(model, wall, &wall_type); /* IfcRelDefinesByType */
printf("wall #%llu, type #%llu\n", (unsigned long long)wall, (unsigned long long)wall_type);

uint64_t openings[16], fillings[16];
size_t count = 0, filled = 0;
openbim_ifc_v0_1_entity_openings(model, wall, openings, 16, &count); /* IfcRelVoidsElement */
for (size_t i = 0; i < count; i++) {
  openbim_ifc_v0_1_entity_fillings(model, openings[i], fillings, 16, &filled);
  printf("opening #%llu: %zu filling(s)\n", (unsigned long long)openings[i], filled);
}
/* wall #17, type #16
 * opening #21: 1 filling(s)
 * opening #23: 0 filling(s) */

char fresh[23], named[23]; /* 22 characters and the NUL */
size_t need = 0;
openbim_ifc_v0_1_new_guid((uint8_t *)fresh, sizeof fresh, &need); /* random, version 4 */
const uint8_t *parts[] = {(const uint8_t *)"my-exporter", (const uint8_t *)"wall-42"};
size_t lengths[] = {11, 7};
openbim_ifc_v0_1_name_based_guid(parts, lengths, 2, (uint8_t *)named, sizeof named, &need);
/* `named` is the same on every run, in every binding */
```

<!-- /SNIPPET -->

A `GlobalId` that several entities carry is `OPENBIM_IFC_STATUS_INVALID_MODEL`,
the last error naming every holder; `openbim_ifc_v0_1_model_duplicate_global_ids`
lists them all as a tape. `openbim_ifc_v0_1_entity_type_object` and
`openbim_ifc_v0_1_entity_voided_element` answer `NO_VALUE` when there is
none. `openbim_ifc_v0_1_new_guid` draws an RFC 9562 version-4 UUID from
the operating system; `openbim_ifc_v0_1_name_based_guid` is version 8, the
same for the same parts in every binding. Both need no model.

## Select entities and read values by path

A [selector](/guide/selectors) picks entities by class, attribute,
property, type, material, classification, location and group; a value path
reads (and writes) one value of an entity. Read from
`test/fixtures/synthetic-relations/selector_ifc4.ifc`:

<!-- SNIPPET:cookbook-c-selectors -->

```c
/* The ids a selector selects, ascending, into a buffer of the caller's. */
const char *query = "IfcWall, Pset_WallCommon.FireRating=F90";
uint64_t ids[16];
size_t count = 0;
if (openbim_ifc_v0_1_model_select(model, (const uint8_t *)query, strlen(query), ids, 16,
                                  &count) != OPENBIM_IFC_STATUS_OK) {
  print_error(model);
  return 1;
}
for (size_t i = 0; i < count; i++) {
  /* A value by path, as a tape: a size query, then the fetch. */
  const char *path = "type.Name";
  size_t need_nodes = 0, need_strings = 0;
  openbim_ifc_v0_1_entity_get_value(model, ids[i], (const uint8_t *)path, strlen(path), NULL,
                                    0, &need_nodes, NULL, 0, &need_strings);
  Tape t = tape_alloc(need_nodes, need_strings);
  openbim_ifc_v0_1_entity_get_value(model, ids[i], (const uint8_t *)path, strlen(path),
                                    t.nodes, t.node_count, &need_nodes, t.strings,
                                    t.string_len, &need_strings);
  printf("#%llu type ", (unsigned long long)ids[i]);
  tape_print(&t, 0);
  printf("\n");
  tape_free(&t);
}
/* #17 type WT-200
 * #18 type WT-200 */

/* A selector that does not parse is INVALID_SELECTOR, with the byte span. */
const char *bad = "IfcWall, colour=red";
size_t start = 0, end = 0;
if (openbim_ifc_v0_1_model_select(model, (const uint8_t *)bad, strlen(bad), ids, 16,
                                  &count) == OPENBIM_IFC_STATUS_INVALID_SELECTOR &&
    openbim_ifc_v0_1_last_error_span(model, &start, &end) == OPENBIM_IFC_STATUS_OK) {
  printf("refused at bytes %zu..%zu\n", start, end); /* refused at bytes 9..15 */
}
```

<!-- /SNIPPET -->

`openbim_ifc_v0_1_model_select_from` restricts a selector to given ids.
`openbim_ifc_v0_1_entity_get_value` writes a value tape: several values as a
`LIST`, a property set as a `LIST` of two-item `LIST`s (the member's name
and its value). `openbim_ifc_v0_1_entity_set_value` takes a one-value tape
of plain values. The grammar version is
`OPENBIM_IFC_SELECTOR_GRAMMAR_MAJOR`.`OPENBIM_IFC_SELECTOR_GRAMMAR_MINOR`.

## Export a table and import it back

The walls of `test/fixtures/synthetic-relations/selector_ifc4.ifc` as a
[table](/guide/tables), one column per value path, keyed by `id`; one cell
edited, the import planned as a dry run, then applied as one checked
transaction:

<!-- SNIPPET:cookbook-c-tables -->

```c
/* The walls as CSV, keyed by id: one column per value path. */
const char *query = "IfcWall";
const char *columns[] = {"Name", "type.Name"};
size_t lengths[] = {strlen(columns[0]), strlen(columns[1])};
size_t need = 0;
openbim_ifc_v0_1_model_export_table(model, (const uint8_t *)query, strlen(query),
                                    (const uint8_t *const *)columns, lengths, 2,
                                    OPENBIM_IFC_TABLE_CSV, OPENBIM_IFC_TABLE_KEY_ID, NULL, 0,
                                    NULL, 0, &need);
char *csv = (char *)malloc(need + 1);
if (openbim_ifc_v0_1_model_export_table(model, (const uint8_t *)query, strlen(query),
                                        (const uint8_t *const *)columns, lengths, 2,
                                        OPENBIM_IFC_TABLE_CSV, OPENBIM_IFC_TABLE_KEY_ID, NULL,
                                        0, (uint8_t *)csv, need, &need) != OPENBIM_IFC_STATUS_OK) {
  print_error(model);
  free(csv);
  return 1;
}
csv[need] = 0;
/* id,Name,type.Name\r\n17,Wall A,WT-200\r\n18,Wall B,WT-200\r\n19,Wall C,\r\n */

/* Rename the shared type in a row, and import the table back: a dry run
 * first. The answer is a TableImport tape: applied, changes, refusals. */
char *renamed = strstr(csv, "17,Wall A,WT-200");
memcpy(renamed + strlen("17,Wall A,"), "WT-250", 6);
size_t nodes = 0, strings = 0;
openbim_ifc_v0_1_model_import_table(model, (const uint8_t *)csv, need, OPENBIM_IFC_TABLE_CSV,
                                    OPENBIM_IFC_TABLE_KEY_AUTO, NULL, 0,
                                    OPENBIM_IFC_TABLE_DRY_RUN, NULL, 0, &nodes, NULL, 0,
                                    &strings);
Tape report = tape_alloc(nodes, strings);
openbim_ifc_v0_1_model_import_table(model, (const uint8_t *)csv, need, OPENBIM_IFC_TABLE_CSV,
                                    OPENBIM_IFC_TABLE_KEY_AUTO, NULL, 0,
                                    OPENBIM_IFC_TABLE_DRY_RUN, report.nodes, nodes, &nodes,
                                    report.strings, strings, &strings);
size_t changes = tape_field(&report, 0, 1), refusals = tape_field(&report, 0, 2);
printf("%u change(s), %u refusal(s)\n", report.nodes[changes].child_count,
       report.nodes[refusals].child_count);
/* The change: TableChange(row, column, key, path, entity, target entity,
 * target, before, after); field 7 is the value now, 8 the value then. */
size_t change = changes + 1;
printf("row %lld: %.*s -> %.*s\n", (long long)report.nodes[tape_field(&report, change, 0)].int_value,
       TAPE_STR(&report, tape_field(&report, change, 7)),
       TAPE_STR(&report, tape_field(&report, change, 8)));
tape_free(&report);
free(csv);
```

<!-- /SNIPPET -->

The export fills a byte buffer as `openbim_ifc_v0_1_model_write` does; the import answers with a `TableImport` tape, whose size query plans without writing. A cell whose text is what its path reads changes nothing;
any refused cell refuses the whole table.

## Remove a product and purge

A door with what it alone owns, a wall whose openings place their fillings,
then what nothing uses any more, from
`test/fixtures/synthetic-relations/cascade_ifc4.ifc`:

<!-- SNIPPET:cookbook-c-remove-product -->

```c
/* Remove door #100 with what it alone owns: placement, representation,
 * its filling and type relationships. A dry run reports and changes
 * nothing; without the flag the same call applies it. A size query
 * never applies a removal. */
size_t nodes = 0, strings = 0;
uint32_t flags = OPENBIM_IFC_REMOVE_DRY_RUN; /* | _REBASE_PLACEMENTS, _UNTYPE_OCCURRENCES */
openbim_ifc_v0_1_model_remove_product(model, 100, flags, NULL, 0, &nodes, NULL, 0, &strings);
Tape report = tape_alloc(nodes, strings);
if (openbim_ifc_v0_1_model_remove_product(model, 100, flags, report.nodes, nodes, &nodes,
                                          report.strings, strings, &strings) !=
    OPENBIM_IFC_STATUS_OK) {
  print_error(model); /* e.g. STILL_REFERENCED: a placement others are relative to */
  tape_free(&report);
  return 1;
}
/* RemovalReport: removed, kept, detached, rebased, applied */
size_t removed = tape_field(&report, 0, 0), kept = tape_field(&report, 0, 1);
printf("removes %u entities\n", report.nodes[removed].child_count);
for (uint32_t k = 0, i = (uint32_t)kept + 1; k < report.nodes[kept].child_count;
     k++, i = (uint32_t)tape_next(&report, i)) {
  /* KeptEntity: id, type name, reason, users */
  if (report.nodes[tape_field(&report, i, 2)].str_len == 6) { /* "shared" */
    tape_print(&report, tape_field(&report, i, 0));
    printf(" %.*s is shared with ", TAPE_STR(&report, tape_field(&report, i, 1)));
    tape_print(&report, tape_field(&report, i, 3));
    printf("\n");
  }
}
tape_free(&report);
/* removes 11 entities
 * #82 IFCLOCALPLACEMENT is shared with (#81)
 * #112 IFCREPRESENTATIONMAP is shared with (#111) */

/* Then purge what nothing uses: no classes means every resource entity
 * no IfcRoot reaches; name classes to purge only those. */
openbim_ifc_v0_1_model_purge(model, NULL, NULL, 0, OPENBIM_IFC_PURGE_DRY_RUN, NULL, 0, &nodes,
                             NULL, 0, &strings);
```

<!-- /SNIPPET -->

The flags combine: `OPENBIM_IFC_REMOVE_REBASE_PLACEMENTS` re-bases
placements relative to a removed one onto its parent, composing the
transforms; `OPENBIM_IFC_REMOVE_UNTYPE_OCCURRENCES` lets a type object
with occurrences go. Without them those removals are
`OPENBIM_IFC_STATUS_STILL_REFERENCED`. A size query never applies a
removal; the same call with buffers large enough does, unless
`OPENBIM_IFC_REMOVE_DRY_RUN`. The
[binding page](/bindings/c#removing-products-and-purging) lists what the
cascade removes and keeps.

## Convert units and the model's length unit

A value between named units, a length in feet and inches, then the whole
millimetre model of `test/fixtures/synthetic-bindings/binding_geometry.ifc`
in metres:

<!-- SNIPPET:cookbook-c-length-unit -->

```c
/* Convert one value between named units; no model needed. */
double feet = 0;
openbim_ifc_v0_1_convert_unit(3000.0, (const uint8_t *)"MILLIMETRE", 10, (const uint8_t *)"foot",
                              4, &feet);
char text[64];
size_t need = 0;
openbim_ifc_v0_1_format_length(3000.0, (const uint8_t *)"MILLIMETRE", 10,
                               OPENBIM_IFC_LENGTH_FORMAT_FEET_INCHES, 16, NULL, 0,
                               OPENBIM_IFC_LENGTH_FORMAT_SUPPRESS_ZERO_INCHES, (uint8_t *)text,
                               sizeof text, &need);
printf("3000 mm = %.4f ft = %s\n", feet, text);

/* Convert the whole model to metres: every length, and areas and
 * volumes with it, as one transaction. A size query plans and applies
 * nothing; the call with the sizes it asked for applies. */
size_t nodes = 0, strings = 0;
uint32_t flags = 0; /* | _KEEP_AREA_AND_VOLUME, _KEEP_REPLACED_UNITS, _DRY_RUN */
const char *target = "METRE";
openbim_ifc_v0_1_model_convert_length_unit(model, (const uint8_t *)target, strlen(target), flags,
                                           NULL, 0, &nodes, NULL, 0, &strings);
Tape report = tape_alloc(nodes, strings);
if (openbim_ifc_v0_1_model_convert_length_unit(model, (const uint8_t *)target, strlen(target),
                                               flags, report.nodes, nodes, &nodes,
                                               report.strings, strings, &strings) !=
    OPENBIM_IFC_STATUS_OK) {
  print_error(model); /* e.g. UNSUPPORTED: a value the conversion cannot carry */
  tape_free(&report);
  return 1;
}
/* LengthUnitReport: from, to, length factor, ..., values converted (12),
 * ..., applied (14) */
printf("%.*s to %.*s: lengths x", TAPE_STR(&report, tape_field(&report, 0, 0)),
       TAPE_STR(&report, tape_field(&report, 0, 1)));
tape_print(&report, tape_field(&report, 0, 2));
printf(", %lld values\n", (long long)report.nodes[tape_field(&report, 0, 12)].int_value);
tape_free(&report);
```

<!-- /SNIPPET -->

`OPENBIM_IFC_LENGTH_UNIT_DRY_RUN` reports without converting. The
[binding page](/bindings/c#units-and-the-model-s-length-unit) lists the
report's fields and the refusals.

## Assign a type and propagate it

Door 1 typed again by its door type, with the type's representation map
mapped onto it, and Wall B moved into the building, from
`test/fixtures/synthetic-relations/cascade_ifc4.ifc`:

<!-- SNIPPET:cookbook-c-assign-type -->

```c
/* Door 1 (#100) typed again by D-90 (#111) with replace: what the old
 * assignment mapped is taken back, the type's representation map is
 * mapped afresh, and PredefinedType is left to the type. Wall B (#40)
 * moves into the building (#13). One batch: both, or neither. */
static Builder b;
memset(&b, 0, sizeof b);
b_list(&b, 2);
b_op(&b, "ASSIGN_TYPE", 5);
b_text(&b, "type_object");
b_ref(&b, 111);
b_text(&b, "objects");
b_list(&b, 1);
b_ref(&b, 100);
b_text(&b, "map_representations"); /* also: material_usage, mapping_target */
b_bool(&b, 1);
b_text(&b, "predefined_type"); /* "ignore" (default), "validate" or "sync" */
b_text(&b, "sync");
b_text(&b, "replace");
b_bool(&b, 1);
b_op(&b, "CONTAIN", 3);
b_text(&b, "structure");
b_ref(&b, 13);
b_text(&b, "elements");
b_list(&b, 1);
b_ref(&b, 40);
b_text(&b, "replace");
b_bool(&b, 1);
uint64_t ids[2];
size_t count = 0;
if (openbim_ifc_v0_1_model_author(model, b.nodes, b.count, (const uint8_t *)b.strings, b.len, ids,
                                  2, &count) != OPENBIM_IFC_STATUS_OK) {
  print_error(model); /* without replace: INVALID_MODEL, both are related already */
  return 1;
}
```

<!-- /SNIPPET -->

The type assignment takes `map_representations`, `mapping_target`, `material_usage`, `layer_set_direction`, `predefined_type` and `replace` (flags are `BOOL` nodes). Every option is off by default: the
operation then writes only the `IfcRelDefinesByType` and refuses an object
that already has a type (`invalid-model`). With `replace` set the previous
type is unassigned and the representations and material usage it
propagated are removed with what only they use; `CONTAIN` and `AGGREGATE` take the
same flag. `material_usage` derives an `IfcMaterialLayerSetUsage` (walls
`AXIS2`, slabs and plates `AXIS3`, `POSITIVE`, offset 0) or an
`IfcMaterialProfileSetUsage` from the type's set; `predefined_type` `"validate"` refuses an
object whose own `PredefinedType` contradicts the type's. The
[binding page](/bindings/c#assigning-types) lists the rules.

## Assemble, assign, copy and remove representations

A wall's body swapped for a copy of another's, and the dry run of removing
a door's body, on the same fixture:

<!-- SNIPPET:cookbook-c-representation -->

```c
/* Give window #130 a copy of wall #40's body #45: the copy gets its own
 * items and profile, and shares the context and the style. Then take
 * wall #20's body #25 off and remove it with what only it used. */
const uint64_t h = OPENBIM_IFC_HANDLE_BASE;
static Builder b;
memset(&b, 0, sizeof b);
b_list(&b, 4);
b_op(&b, "COPY_REPRESENTATION", 1); /* 0 */
b_text(&b, "representation");
b_ref(&b, 45);
b_op(&b, "ASSIGN_REPRESENTATION", 2); /* 1: an IfcProductDefinitionShape */
b_text(&b, "representation");
b_ref(&b, h + 0);
b_text(&b, "to");
b_ref(&b, 130);
b_op(&b, "UNASSIGN_REPRESENTATION", 2); /* 2: #25 stays, unassigned */
b_text(&b, "representation");
b_ref(&b, 25);
b_text(&b, "from");
b_ref(&b, 20);
b_op(&b, "REMOVE_REPRESENTATION", 1); /* 3 */
b_text(&b, "representation");
b_ref(&b, 25);
uint64_t ids[4];
size_t count = 0;
if (openbim_ifc_v0_1_model_author(model, b.nodes, b.count, (const uint8_t *)b.strings, b.len, ids,
                                  4, &count) != OPENBIM_IFC_STATUS_OK) {
  print_error(model); /* a refused batch changes nothing */
  return 1;
}
printf("copy #%llu on #130\n", (unsigned long long)ids[0]);
/* SHAPE_REPRESENTATION assembles one from items instead: fields context,
 * items, and optionally identifier and representation_type. */

/* openbim_ifc_v0_1_model_remove_representation removes one alone and
 * reports what went and what stayed; here, a dry run on the door's body. */
size_t nodes = 0, strings = 0;
openbim_ifc_v0_1_model_remove_representation(model, 105, OPENBIM_IFC_REMOVE_DRY_RUN, NULL, 0,
                                             &nodes, NULL, 0, &strings);
Tape report = tape_alloc(nodes, strings);
if (openbim_ifc_v0_1_model_remove_representation(model, 105, OPENBIM_IFC_REMOVE_DRY_RUN,
                                                  report.nodes, nodes, &nodes, report.strings,
                                                  strings, &strings) != OPENBIM_IFC_STATUS_OK) {
  print_error(model); /* e.g. STILL_REFERENCED: a type's map its occurrences map */
  tape_free(&report);
  return 1;
}
printf("would remove %u entities\n", report.nodes[tape_field(&report, 0, 0)].child_count);
tape_free(&report);
```

<!-- /SNIPPET -->

`SHAPE_REPRESENTATION` checks a given `representation_type` against the
items by the release's `CorrectItemsForType` rule, and refuses a mismatch
with `OPENBIM_IFC_STATUS_INVALID_VALUE`. A representation belongs to one product shape, map or shape
aspect, so assigning one already used is refused (`invalid-model`); on a
type it becomes a new `IfcRepresentationMap`. A copy shares its context,
styles and the type geometry a mapped item maps, and copies everything
else. The [binding page](/bindings/c#shape-representations) has the
rules.

## Create a model

Values passed in are tapes too; this builder writes them:

<!-- SNIPPET:cookbook-c-builder -->

```c
/* A value tape to pass in: pre-order nodes plus one string buffer. */
typedef struct {
  OpenbimIfcValueNode nodes[256];
  size_t count;
  char strings[4096];
  size_t len;
} Builder;

static void b_node(Builder *b, int32_t kind, uint32_t children, int64_t integer, double real,
                   const char *text) {
  OpenbimIfcValueNode *n = &b->nodes[b->count++];
  memset(n, 0, sizeof *n);
  n->kind = kind;
  n->child_count = children;
  n->int_value = integer;
  n->real_value = real;
  if (text != NULL) {
    n->str_offset = b->len;
    n->str_len = strlen(text);
    memcpy(b->strings + b->len, text, n->str_len);
    b->len += n->str_len;
  }
}
static void b_list(Builder *b, uint32_t children) { b_node(b, OPENBIM_IFC_KIND_LIST, children, 0, 0, NULL); }
static void b_text(Builder *b, const char *s) { b_node(b, OPENBIM_IFC_KIND_TEXT, 0, 0, 0, s); }
static void b_enum(Builder *b, const char *s) { b_node(b, OPENBIM_IFC_KIND_ENUM, 0, 0, 0, s); }
static void b_ref(Builder *b, uint64_t id) { b_node(b, OPENBIM_IFC_KIND_REF, 0, (int64_t)id, 0, NULL); }
static void b_real(Builder *b, double v) { b_node(b, OPENBIM_IFC_KIND_REAL, 0, 0, v, NULL); }
static void b_bool(Builder *b, int v) { b_node(b, OPENBIM_IFC_KIND_BOOL, 0, v != 0, 0, NULL); }
/* An authoring operation: a LIST of its ENUM name, then `fields` (TEXT name, value) pairs. */
static void b_op(Builder *b, const char *op, uint32_t fields) {
  b_list(b, 1 + 2 * fields);
  b_enum(b, op);
}
/* A one-attribute `attributes` field: LIST of one LIST(TEXT name, TEXT value). */
static void b_name(Builder *b, const char *name) {
  b_text(b, "attributes");
  b_list(b, 1);
  b_list(b, 2);
  b_text(b, "Name");
  b_text(b, name);
}
```

<!-- /SNIPPET -->

<!-- SNIPPET:cookbook-c-create -->

```c
OpenbimIfcModel model = 0;
openbim_ifc_v0_1_model_create(&model);

/* The header: a LIST of its ten fields in STEP order. Its schema is the
 * release every operation below is checked against. */
static Builder b;
memset(&b, 0, sizeof b);
b_list(&b, 10);
b_list(&b, 1);
b_text(&b, "ViewDefinition [DesignTransferView]"); /* description */
b_text(&b, "2;1");                                 /* implementation level */
b_text(&b, "new.ifc");                             /* name */
b_text(&b, "2026-10-08T00:00:00");                 /* time stamp */
b_list(&b, 0);                                     /* author */
b_list(&b, 0);                                     /* organization */
b_text(&b, "openbim-ifc");                         /* preprocessor version */
b_text(&b, "cookbook");                            /* originating system */
b_text(&b, "");                                    /* authorization */
b_list(&b, 1);
b_text(&b, "IFC4"); /* schema */
openbim_ifc_v0_1_model_set_header(model, b.nodes, b.count, (const uint8_t *)b.strings, b.len);

/* One checked batch: OPENBIM_IFC_HANDLE_BASE + i names operation i's entity. */
const uint64_t h = OPENBIM_IFC_HANDLE_BASE;
memset(&b, 0, sizeof b);
b_list(&b, 7);
b_op(&b, "PROJECT", 1); /* 0 */
b_name(&b, "Demo");
b_op(&b, "PLACEMENT", 0); /* 1: at the origin */
b_op(&b, "SPATIAL", 3);   /* 2 */
b_text(&b, "type");
b_text(&b, "IfcSite");
b_text(&b, "parent");
b_ref(&b, h + 0);
b_text(&b, "placement");
b_ref(&b, h + 1);
b_op(&b, "SPATIAL", 2); /* 3 */
b_text(&b, "type");
b_text(&b, "IfcBuilding");
b_text(&b, "parent");
b_ref(&b, h + 2);
b_op(&b, "SPATIAL", 3); /* 4 */
b_text(&b, "type");
b_text(&b, "IfcBuildingStorey");
b_text(&b, "parent");
b_ref(&b, h + 3);
b_name(&b, "Level 0");
b_op(&b, "PLACEMENT", 2); /* 5: 4 m along x */
b_text(&b, "relative_to");
b_ref(&b, h + 1);
b_text(&b, "location");
b_list(&b, 3);
b_real(&b, 4.0);
b_real(&b, 0.0);
b_real(&b, 0.0);
b_op(&b, "PRODUCT", 4); /* 6 */
b_text(&b, "type");
b_text(&b, "IfcWall");
b_text(&b, "container");
b_ref(&b, h + 4);
b_text(&b, "placement");
b_ref(&b, h + 5);
b_name(&b, "Wall");
uint64_t ids[7];
size_t count = 0;
if (openbim_ifc_v0_1_model_author(model, b.nodes, b.count, (const uint8_t *)b.strings, b.len, ids,
                                  7, &count) != OPENBIM_IFC_STATUS_OK) {
  print_error(model); /* a refused batch changes nothing */
}
uint64_t wall = ids[6]; /* every IfcRoot got a GlobalId */
```

<!-- /SNIPPET -->

Every operation is checked against the release the header declares, and a
batch is one transaction. The [binding page](/bindings/c#creating-entities)
lists every operation and its fields.
