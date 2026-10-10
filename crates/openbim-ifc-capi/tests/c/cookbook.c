/* The C cookbook (docs/cookbook/c.md, #331): every recipe on that page is a
 * `docs:snippet` region below, run by scripts/check-c.sh against the real
 * library, as C11 and as C++17, and against a library built with the `mesh`
 * and `graph` features, and checked against the fixture it reads.
 *
 *   cookbook <test/fixtures directory> <scratch directory>
 *
 * Exits non-zero with a message on any mismatch.
 */
#include "openbim_ifc.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(cond, what)                                              \
  do {                                                                 \
    if (!(cond)) {                                                     \
      fprintf(stderr, "FAIL %s:%d %s\n", __FILE__, __LINE__, what);    \
      return 1;                                                        \
    }                                                                  \
  } while (0)

#define OK(call) CHECK((call) == OPENBIM_IFC_STATUS_OK, #call)

static char properties_path[4096];
static char geometry_path[4096];
/* What the recipes print, so each test can check it. */
static char out[16384];
static size_t out_len;

#define printf(...) (out_len += (size_t)snprintf(out + out_len, sizeof out - out_len, __VA_ARGS__))

// docs:snippet cookbook-c-tape
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
// docs:end

static int recipe_open(void) {
  const char *path = properties_path;
  // docs:snippet cookbook-c-open
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
  // docs:end
  CHECK(strcmp(out, "IFC4, 68 entities\n") == 0, out);
  return 0;
}

static int recipe_property_sets(OpenbimIfcModel model, uint64_t wall) {
  out_len = 0;
  out[0] = 0;
  // docs:snippet cookbook-c-property-sets
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
  // docs:end
  return 0;
}

static int recipe_property_sets_many(OpenbimIfcModel model) {
  out_len = 0;
  out[0] = 0;
  // docs:snippet cookbook-c-property-sets-many
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
  // docs:end
  CHECK(strcmp(out, "#30: 3 set(s)\n#31: 2 set(s)\n") == 0, out);
  return 0;
}

static int recipe_storeys_of(OpenbimIfcModel model) {
  // docs:snippet cookbook-c-storeys
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
  // docs:end
  return 0;
}

static int recipe_storeys(OpenbimIfcModel model) {
  out_len = 0;
  out[0] = 0;
  if (recipe_storeys_of(model) != 0) return 1;
  CHECK(strcmp(out, "Level 0\n  IFCWALL #30\n  IFCWALL #31\n") == 0, out);
  return 0;
}

static int recipe_validate(OpenbimIfcModel model) {
  out_len = 0;
  out[0] = 0;
  // docs:snippet cookbook-c-validate
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
  // docs:end
  CHECK(summary.conformant == 0 && summary.errors > 0, out);
  CHECK(strstr(out, "\nerror ") != NULL && strstr(out, " #30: ") != NULL, out);
  return 0;
}

/* Fetch a buffer-returning call's bytes: the size query, then the fetch. */
static uint8_t *write_ifcxml(OpenbimIfcModel model, const char *profile, size_t *len) {
  size_t profile_len = profile ? strlen(profile) : 0;
  openbim_ifc_v0_1_model_write_ifcxml(model, (const uint8_t *)profile, profile_len, NULL, 0, len);
  uint8_t *xml = (uint8_t *)malloc(*len);
  if (openbim_ifc_v0_1_model_write_ifcxml(model, (const uint8_t *)profile, profile_len, xml, *len,
                                          len) != OPENBIM_IFC_STATUS_OK) {
    free(xml);
    return NULL;
  }
  return xml;
}

static int recipe_ifcxml(OpenbimIfcModel model) {
  // docs:snippet cookbook-c-ifcxml
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
  // docs:end
  CHECK(back != 0, "the lossless layout reads back");
  size_t a = 0, b = 0;
  OK(openbim_ifc_v0_1_model_len(model, &a));
  OK(openbim_ifc_v0_1_model_len(back, &b));
  CHECK(a == b, "the same entities");
  OK(openbim_ifc_v0_1_model_destroy(back));
  uint8_t *xsd = write_ifcxml(model, profile, &len);
  CHECK(xsd != NULL, "the XSD layout writes");
  OpenbimIfcModel from_xsd = 0;
  OK(openbim_ifc_v0_1_model_parse_ifcxml(xsd, len, (const uint8_t *)profile, strlen(profile),
                                         &from_xsd, NULL, 0));
  free(xsd);
  uint64_t walls[4];
  size_t wall_count = 0;
  OK(openbim_ifc_v0_1_model_ids_of_type(from_xsd, (const uint8_t *)"IfcWall", 7, walls, 4,
                                        &wall_count));
  CHECK(wall_count == 1, "the wall survives the XSD layout");
  OK(openbim_ifc_v0_1_model_destroy(from_xsd));
  return 0;
}

static int recipe_edit(OpenbimIfcModel model, const char *path) {
  uint64_t wall = 31;
  // docs:snippet cookbook-c-edit
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
  // docs:end
  CHECK(holder != 0, "the property was written");
  OpenbimIfcModel again = 0;
  OK(openbim_ifc_v0_1_model_open((const uint8_t *)path, strlen(path), &again, NULL, 0));
  OpenbimIfcValueNode nodes[4];
  uint8_t strings[64];
  size_t node_count = 0, string_len = 0;
  OK(openbim_ifc_v0_1_entity_attribute_by_name(again, wall, (const uint8_t *)"PredefinedType", 14,
                                               nodes, 4, &node_count, strings, sizeof strings,
                                               &string_len));
  CHECK(nodes[0].kind == OPENBIM_IFC_KIND_ENUM && nodes[0].str_len == 9 &&
            memcmp(strings + nodes[0].str_offset, "SOLIDWALL", 9) == 0,
        "the plain text became the enumeration item");
  CHECK(recipe_property_sets(again, wall) == 0, "the edited wall's sets read");
  CHECK(strstr(out, "Pset_WallCommon.FireRating = F90 (occurrence)\n") != NULL, out);
  OK(openbim_ifc_v0_1_model_destroy(again));
  remove(path);
  return 0;
}

static int recipe_placements(OpenbimIfcModel model) {
  out_len = 0;
  out[0] = 0;
  // docs:snippet cookbook-c-placements
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
  // docs:end
  CHECK(strncmp(out, "IFCWALL at 512002 5.403e+06 3\n", 30) == 0, out);
  return 0;
}

static int recipe_meshes(OpenbimIfcModel model) {
  // docs:snippet cookbook-c-meshes
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
  // docs:end
  if (status == OPENBIM_IFC_STATUS_FEATURE_DISABLED) {
    CHECK(meshes == 0, "no set without the feature");
    return 0;
  }
  CHECK(status == OPENBIM_IFC_STATUS_OK, "meshes compile");
  CHECK(triangles == 24, "the wall and the slab: two boxes");
  return 0;
}

static int recipe_graphs(OpenbimIfcModel model) {
  // docs:snippet cookbook-c-graphs
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
  // docs:end
  if (status == OPENBIM_IFC_STATUS_FEATURE_DISABLED) {
    CHECK(graphs == 0, "no set without the feature");
    return 0;
  }
  CHECK(status == OPENBIM_IFC_STATUS_OK, "graphs lower");
  CHECK(written == 2, "the wall and the slab");
  return 0;
}

// docs:snippet cookbook-c-builder
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
// docs:end

static int recipe_create(void) {
  // docs:snippet cookbook-c-create
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
  // docs:end
  CHECK(count == 7, "one id per operation");
  char type[32];
  size_t need = 0;
  OK(openbim_ifc_v0_1_entity_type(model, wall, (uint8_t *)type, sizeof type, &need));
  CHECK(strcmp(type, "IFCWALL") == 0, type);
  out_len = 0;
  out[0] = 0;
  CHECK(recipe_storeys_of(model) == 0, "the wall is on the storey");
  CHECK(strncmp(out, "Level 0\n  IFCWALL #", 19) == 0, out);
  OpenbimIfcValidationSummary summary;
  size_t nodes = 0, strings = 0;
  openbim_ifc_v0_1_model_validate(model, 0, &summary, NULL, 0, &nodes, NULL, 0, &strings);
  CHECK(summary.errors == 0, "the new model is valid");
  OK(openbim_ifc_v0_1_model_destroy(model));
  return 0;
}

int main(int argc, char **argv) {
  if (argc != 3) {
    fprintf(stderr, "usage: cookbook <test/fixtures directory> <scratch directory>\n");
    return 2;
  }
  snprintf(properties_path, sizeof properties_path,
           "%s/synthetic-properties/synthetic_properties.ifc", argv[1]);
  snprintf(geometry_path, sizeof geometry_path, "%s/synthetic-bindings/binding_geometry.ifc",
           argv[1]);

  if (recipe_open() != 0) return 1;

  OpenbimIfcModel model = 0;
  OK(openbim_ifc_v0_1_model_open((const uint8_t *)properties_path, strlen(properties_path), &model,
                                 NULL, 0));
  if (recipe_property_sets(model, 30) != 0) return 1;
  CHECK(strncmp(out, "Pset_WallCommon.IsExternal = false (occurrence)\n", 48) == 0, out);
  CHECK(strstr(out, "Qto_WallBaseQuantities.Width = 200 (occurrence)\n") != NULL, out);
  CHECK(strstr(out, "Pset_WallCommon.FireRating = F30 (type)\n") != NULL, out);
  if (recipe_property_sets_many(model) != 0) return 1;
  if (recipe_storeys(model) != 0) return 1;
  char edited[4200];
  snprintf(edited, sizeof edited, "%s/cookbook-edit.ifc", argv[2]);
  if (recipe_edit(model, edited) != 0) return 1;
  OK(openbim_ifc_v0_1_model_destroy(model));

  /* A file that fails validation: Wall A's PredefinedType is no enum item. */
  FILE *file = fopen(properties_path, "rb");
  CHECK(file != NULL, "the fixture opens");
  static char text[65536];
  size_t len = fread(text, 1, sizeof text - 1, file);
  fclose(file);
  text[len] = 0;
  const char *from = "'Wall A',$,$,$,$,$,$)";
  char *at = strstr(text, from);
  CHECK(at != NULL, "Wall A in the fixture");
  static char broken[65600];
  snprintf(broken, sizeof broken, "%.*s'Wall A',$,$,$,$,$,.NOTANENUM.)%s", (int)(at - text), text,
           at + strlen(from));
  OK(openbim_ifc_v0_1_model_parse((const uint8_t *)broken, strlen(broken), &model, NULL, 0));
  if (recipe_validate(model) != 0) return 1;
  OK(openbim_ifc_v0_1_model_destroy(model));

  OK(openbim_ifc_v0_1_model_open((const uint8_t *)geometry_path, strlen(geometry_path), &model, NULL,
                                 0));
  if (recipe_ifcxml(model) != 0) return 1;
  if (recipe_placements(model) != 0) return 1;
  if (recipe_meshes(model) != 0) return 1;
  if (recipe_graphs(model) != 0) return 1;
  OK(openbim_ifc_v0_1_model_destroy(model));

  if (recipe_create() != 0) return 1;

  size_t live = 1;
  OK(openbim_ifc_v0_1_live_models(&live));
  CHECK(live == 0, "every model destroyed");
  fprintf(stderr, "cookbook: all recipes ok\n");
  return 0;
}
