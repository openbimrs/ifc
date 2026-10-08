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

## Read placements and meshes

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
