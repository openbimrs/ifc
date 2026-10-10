/* C smoke test for the openbim-ifc C ABI (#38).
 *
 * Parses a fixture, reads an entity, edits it, adds one, writes the file,
 * re-parses it and checks the edit survived -- all through the header, the
 * way a native host would. Exits non-zero with a message on any mismatch.
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

static const char FILE_TEXT[] =
    "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n"
    "FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\n"
    "DATA;\n#1=IFCWALL('0abc',$,'Wall',*,$,$,$,$,.STANDARD.);\n"
    "#2=IFCPROPERTYSINGLEVALUE('P',$,IFCLOGICAL(.U.),$);\nENDSEC;\n"
    "END-ISO-10303-21;\n";

/* Read attribute `index` of `id` into caller buffers (size query first). */
static OpenbimIfcStatus read_attribute(OpenbimIfcModel model, uint64_t id,
                                       size_t index,
                                       OpenbimIfcValueNode *nodes,
                                       size_t node_cap, size_t *node_count,
                                       uint8_t *strings, size_t string_cap,
                                       size_t *string_len) {
  return openbim_ifc_v0_1_entity_attribute(model, id, index, nodes, node_cap,
                                           node_count, strings, string_cap,
                                           string_len);
}

/* The example published on the docs site's C page. */
static int documented_example(void) {
  const uint8_t *data = (const uint8_t *)FILE_TEXT;
  size_t len = strlen(FILE_TEXT);
  // docs:snippet c-read-type
  OpenbimIfcModel model = 0;
  if (openbim_ifc_v0_1_model_parse(data, len, &model, NULL, 0) != OPENBIM_IFC_STATUS_OK) {
    return 1;
  }

  /* Every call that returns a buffer: ask for the size, then fetch. */
  size_t need = 0;
  openbim_ifc_v0_1_entity_type(model, 1, NULL, 0, &need);
  uint8_t *type = (uint8_t *)malloc(need);
  openbim_ifc_v0_1_entity_type(model, 1, type, need, &need);
  printf("#1 is %s\n", (const char *)type); /* #1 is IFCWALL */
  free(type);

  openbim_ifc_v0_1_model_destroy(model);
  // docs:end
  return 0;
}

/* The capabilities example published on the docs site's C page. */
static int documented_capabilities(const uint8_t *data, size_t len) {
  // docs:snippet c-beyond-records
  /* A damaged export: skip what cannot be read; each skip is a diagnostic. */
  OpenbimIfcModel model = 0;
  if (openbim_ifc_v0_1_model_parse_with_options(data, len, OPENBIM_IFC_PARSE_LENIENT,
                                                &model, NULL, 0) != OPENBIM_IFC_STATUS_OK) {
    return 1;
  }

  /* Validation: the counts land in a summary even on a size query; the
   * findings come as a value tape, fetched like any other. */
  OpenbimIfcValidationSummary summary;
  memset(&summary, 0, sizeof summary);
  size_t nodes_needed = 0, strings_needed = 0;
  openbim_ifc_v0_1_model_validate(model, 0, &summary, NULL, 0, &nodes_needed,
                                  NULL, 0, &strings_needed);
  printf("%zu errors, conformant: %u\n", summary.errors, (unsigned)summary.conformant);

  openbim_ifc_v0_1_model_destroy(model);
  // docs:end
  return summary.errors > 0 ? 0 : 1;
}

/* The #244 surface: lenient reads, the header, validation, ifcXML and the
 * reachability lint. Returns 0 on success. */
static int capabilities(void) {
  static const char DAMAGED[] =
      "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n"
      "FILE_NAME('d.ifc','',('Ann'),(''),'','','');\nFILE_SCHEMA(('IFC4'));\n"
      "ENDSEC;\nDATA;\n#1=IFCWALL('0abc',$,'Wall',$,$,$,$,$,.STANDARD.);\n"
      "#2=IFCWALL('x',,;\n"
      "#3=IFCRELDEFINESBYPROPERTIES('0def',$,$,$,(#1),#9);\nENDSEC;\n"
      "END-ISO-10303-21;\n";
  OpenbimIfcModel model = 0;
  char message[256];
  CHECK(openbim_ifc_v0_1_model_parse((const uint8_t *)DAMAGED, strlen(DAMAGED),
                                     &model, NULL, 0) == OPENBIM_IFC_STATUS_PARSE,
        "a strict read refuses a damaged file");
  CHECK(openbim_ifc_v0_1_model_parse_with_options(
            (const uint8_t *)DAMAGED, strlen(DAMAGED), 1u << 9, &model,
            (uint8_t *)message, sizeof message) == OPENBIM_IFC_STATUS_INVALID_VALUE,
        "an unknown parse flag is invalid-value");
  OK(openbim_ifc_v0_1_model_parse_with_options(
      (const uint8_t *)DAMAGED, strlen(DAMAGED), OPENBIM_IFC_PARSE_LENIENT, &model,
      NULL, 0));
  size_t count = 0;
  OK(openbim_ifc_v0_1_model_diagnostic_count(model, &count));
  CHECK(count == 1, "the skipped record is reported");

  /* The header: a LIST of ten values; field 2 is the name. */
  OpenbimIfcValueNode nodes[64];
  uint8_t strings[2048];
  size_t node_count = 0, string_len = 0;
  OK(openbim_ifc_v0_1_model_header(model, nodes, 64, &node_count, strings,
                                   sizeof strings, &string_len));
  CHECK(nodes[0].kind == OPENBIM_IFC_KIND_LIST && nodes[0].child_count == 10,
        "header tape is a list of ten fields");
  /* nodes[1] is the description list (one item), nodes[2] its text,
   * nodes[3] the implementation level, nodes[4] the name. */
  CHECK(nodes[4].kind == OPENBIM_IFC_KIND_TEXT && nodes[4].str_len == 5 &&
            memcmp(strings + nodes[4].str_offset, "d.ifc", 5) == 0,
        "header name");
  /* Write the same tape back: a no-op replacement is accepted. */
  OK(openbim_ifc_v0_1_model_set_header(model, nodes, node_count, strings,
                                       string_len));

  /* Validation: #3 references the missing #9. */
  OpenbimIfcValidationSummary summary;
  memset(&summary, 0, sizeof summary);
  OK(openbim_ifc_v0_1_model_validate(model, 0, &summary, nodes, 64, &node_count,
                                     strings, sizeof strings, &string_len));
  CHECK(summary.conformant == 0 && summary.errors >= 1, "a dangling reference is an error");
  CHECK(nodes[0].kind == OPENBIM_IFC_KIND_LIST &&
            nodes[0].child_count == summary.finding_count,
        "one tape record per finding");
  CHECK(nodes[1].kind == OPENBIM_IFC_KIND_LIST && nodes[1].child_count == 7,
        "a finding has seven fields");

  /* ifcXML: native layout out and back; an unknown profile is refused. */
  size_t need = 0;
  CHECK(openbim_ifc_v0_1_model_write_ifcxml(model, NULL, 0, NULL, 0, &need) ==
            OPENBIM_IFC_STATUS_BUFFER_TOO_SMALL,
        "ifcXML size query");
  uint8_t *xml = (uint8_t *)malloc(need);
  OK(openbim_ifc_v0_1_model_write_ifcxml(model, NULL, 0, xml, need, &need));
  OpenbimIfcModel from_xml = 0;
  OK(openbim_ifc_v0_1_model_parse_ifcxml(xml, need, NULL, 0, &from_xml, NULL, 0));
  free(xml);
  OK(openbim_ifc_v0_1_model_len(from_xml, &count));
  CHECK(count == 2, "ifcXML round trip keeps both entities");
  OK(openbim_ifc_v0_1_model_destroy(from_xml));
  const char profile[] = "IFC2X3";
  CHECK(openbim_ifc_v0_1_model_write_ifcxml(model, (const uint8_t *)profile,
                                            strlen(profile), NULL, 0, &need) ==
            OPENBIM_IFC_STATUS_UNSUPPORTED_PROFILE,
        "no XSD profile for IFC2X3");
  char code[32];
  OK(openbim_ifc_v0_1_last_error_code(model, (uint8_t *)code, sizeof code, &need));
  CHECK(strcmp(code, "unsupported-profile") == 0, "stable profile error code");

  /* The wall has no representation: nothing is unreachable. */
  OK(openbim_ifc_v0_1_model_unreachable_products(model, &count, nodes, 64,
                                                  &node_count, strings,
                                                  sizeof strings, &string_len));
  CHECK(count == 0 && nodes[0].kind == OPENBIM_IFC_KIND_LIST &&
            nodes[0].child_count == 0,
        "no unreachable products");

  OK(openbim_ifc_v0_1_model_destroy(model));
  CHECK(documented_capabilities((const uint8_t *)DAMAGED, strlen(DAMAGED)) == 0,
        "the documented capabilities example runs");
  return 0;
}

/* A wall whose type holds a property set and a layer set (#123). */
static const char DOMAIN_TEXT[] =
    "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n"
    "FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n"
    "#2=IFCWALLTYPE('1YvctVUKr0kugbFTf53O9L',$,'WT',$,$,(#30),$,$,$,.SOLIDWALL.);\n"
    "#3=IFCWALL('2YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,$,$,$,.STANDARD.);\n"
    "#4=IFCRELDEFINESBYTYPE('3YvctVUKr0kugbFTf53O9L',$,$,$,(#3),#2);\n"
    "#20=IFCMATERIAL('Concrete',$,$);\n"
    "#22=IFCMATERIALLAYER(#20,0.2,.U.,'Core',$,$,$);\n"
    "#24=IFCMATERIALLAYERSET((#22),'WT-200',$);\n"
    "#26=IFCRELASSOCIATESMATERIAL('2ZvctVUKr0kugbFTf53O9L',$,$,$,(#2),#24);\n"
    "#30=IFCPROPERTYSET('3ZvctVUKr0kugbFTf53O9L',$,'Pset_WallCommon',$,(#31));\n"
    "#31=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$);\n"
    "ENDSEC;\nEND-ISO-10303-21;\n";

/* The domain example published on the docs site's C page. */
static int documented_domains(const uint8_t *data, size_t len) {
  // docs:snippet c-domain-views
  OpenbimIfcModel model = 0;
  if (openbim_ifc_v0_1_model_parse(data, len, &model, NULL, 0) != OPENBIM_IFC_STATUS_OK) {
    return 1;
  }

  /* The property sets of wall #3, inherited ones included: a LIST of
   * PropertySet records, each a LIST of its fields (see the table below). */
  size_t sets = 0, nodes_needed = 0, strings_needed = 0;
  openbim_ifc_v0_1_model_property_sets(model, 3, &sets, NULL, 0, &nodes_needed, NULL, 0,
                                       &strings_needed);
  OpenbimIfcValueNode *nodes =
      (OpenbimIfcValueNode *)malloc(nodes_needed * sizeof(OpenbimIfcValueNode));
  uint8_t *strings = (uint8_t *)malloc(strings_needed);
  openbim_ifc_v0_1_model_property_sets(model, 3, &sets, nodes, nodes_needed, &nodes_needed,
                                       strings, strings_needed, &strings_needed);
  /* nodes[4] is the first set's name, nodes[6] its source ("type"). */
  printf("%zu set(s); first: %.*s\n", sets, (int)nodes[4].str_len,
         (const char *)strings + nodes[4].str_offset);
  free(nodes);
  free(strings);
  openbim_ifc_v0_1_model_destroy(model);
  // docs:end
  return sets == 1 ? 0 : 1;
}

/* The domain views (#123): records cross as tapes with the shared codes. */
static int domains(void) {
  OpenbimIfcModel model = 0;
  OK(openbim_ifc_v0_1_model_parse((const uint8_t *)DOMAIN_TEXT, strlen(DOMAIN_TEXT), &model,
                                  NULL, 0));
  OpenbimIfcValueNode nodes[256];
  uint8_t strings[4096];
  size_t count = 0, node_count = 0, string_len = 0;
  OK(openbim_ifc_v0_1_model_property_sets(model, 3, &count, nodes, 256, &node_count,
                                          strings, sizeof strings, &string_len));
  CHECK(count == 1 && nodes[0].kind == OPENBIM_IFC_KIND_LIST, "one inherited set");
  CHECK(nodes[1].kind == OPENBIM_IFC_KIND_LIST && nodes[1].child_count == 7,
        "a PropertySet record has seven fields");
  CHECK(nodes[2].kind == OPENBIM_IFC_KIND_REF && nodes[2].int_value == 30, "set id");
  CHECK(nodes[6].str_len == 4 && memcmp(strings + nodes[6].str_offset, "type", 4) == 0,
        "inherited from the type");
  CHECK(nodes[7].kind == OPENBIM_IFC_KIND_REF && nodes[7].int_value == 2, "type object");
  /* nodes[9] is the Property record; its value (field 6) is IFCBOOLEAN(.T.). */
  CHECK(nodes[9].child_count == 14, "a Property record has fourteen fields");
  CHECK(nodes[16].kind == OPENBIM_IFC_KIND_TYPED && nodes[17].kind == OPENBIM_IFC_KIND_BOOL &&
            nodes[17].int_value == 1,
        "the value keeps its type");

  /* The material: one record; a layer set inherited from the type. */
  OK(openbim_ifc_v0_1_model_material(model, 3, nodes, 256, &node_count, strings,
                                     sizeof strings, &string_len));
  CHECK(nodes[0].kind == OPENBIM_IFC_KIND_LIST && nodes[0].child_count == 14,
        "a MaterialAssignment record");
  OK(openbim_ifc_v0_1_model_material(model, 20, nodes, 256, &node_count, strings,
                                     sizeof strings, &string_len));
  CHECK(nodes[0].kind == OPENBIM_IFC_KIND_NULL, "no association is a NULL tape");

  OK(openbim_ifc_v0_1_model_spatial_tree(model, nodes, 256, &node_count, strings,
                                         sizeof strings, &string_len));
  CHECK(nodes[0].kind == OPENBIM_IFC_KIND_LIST && nodes[0].child_count == 6,
        "a SpatialTree record");

  /* Many objects in one pass (#358): the wall's one inherited set, a
   * refusal for the material, which carries no property sets. */
  const uint64_t objects[2] = {3, 20};
  OK(openbim_ifc_v0_1_model_property_sets_many(model, objects, 2, &count, nodes, 256,
                                               &node_count, strings, sizeof strings,
                                               &string_len));
  CHECK(count == 2 && nodes[0].child_count == 2, "one record per object");
  CHECK(nodes[1].kind == OPENBIM_IFC_KIND_LIST && nodes[1].child_count == 3,
        "an ObjectPropertySets record has three fields");
  CHECK(nodes[2].kind == OPENBIM_IFC_KIND_REF && nodes[2].int_value == 3, "the object");
  CHECK(nodes[3].kind == OPENBIM_IFC_KIND_LIST && nodes[3].child_count == 1, "one set");
  /* Every object definition when ids is NULL and the count 0. */
  OK(openbim_ifc_v0_1_model_property_sets_many(model, NULL, 0, &count, nodes, 256,
                                               &node_count, strings, sizeof strings,
                                               &string_len));
  CHECK(count == 2, "the wall type and the wall");
  OK(openbim_ifc_v0_1_model_destroy(model));

  /* The same refusal as every host: IFC2X3 has no georeferencing. */
  static const char IFC2X3[] =
      "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n"
      "FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC2X3'));\nENDSEC;\nDATA;\n"
      "#1=IFCWALL('0abc',$,'Wall',$,$,$,$,$);\nENDSEC;\nEND-ISO-10303-21;\n";
  OK(openbim_ifc_v0_1_model_parse((const uint8_t *)IFC2X3, strlen(IFC2X3), &model, NULL, 0));
  CHECK(openbim_ifc_v0_1_model_georeferencing(model, &count, nodes, 256, &node_count,
                                              strings, sizeof strings, &string_len) ==
            OPENBIM_IFC_STATUS_UNSUPPORTED_SCHEMA,
        "georeferencing refuses IFC2X3");
  char code[32];
  size_t need = 0;
  OK(openbim_ifc_v0_1_last_error_code(model, (uint8_t *)code, sizeof code, &need));
  CHECK(strcmp(code, "unsupported-schema") == 0, "the shared code");
  CHECK(openbim_ifc_v0_1_model_property_sets(model, 99, &count, nodes, 256, &node_count,
                                             strings, sizeof strings, &string_len) ==
            OPENBIM_IFC_STATUS_MISSING_ENTITY,
        "a missing object");
  OK(openbim_ifc_v0_1_model_destroy(model));

  return documented_domains((const uint8_t *)DOMAIN_TEXT, strlen(DOMAIN_TEXT));
}

/* One tape node; strings index into the caller's buffer. */
static OpenbimIfcValueNode node(int32_t kind, uint32_t children, int64_t integer,
                                uint64_t offset, uint64_t len) {
  OpenbimIfcValueNode n;
  memset(&n, 0, sizeof n);
  n.kind = kind;
  n.child_count = children;
  n.int_value = integer;
  n.str_offset = offset;
  n.str_len = len;
  return n;
}

/* The property-writing example published on the docs site's C page. */
static int documented_writing(OpenbimIfcModel model) {
  // docs:snippet c-domain-write
  /* Wall #3 inherits IsExternal from its type: writing it on the wall
   * creates an occurrence override; the type's shared set is unchanged.
   * The value is a one-value tape: IFCBOOLEAN(.F.). */
  const char *wrapper = "IFCBOOLEAN";
  OpenbimIfcValueNode value[2];
  memset(value, 0, sizeof value);
  value[0].kind = OPENBIM_IFC_KIND_TYPED;
  value[0].str_len = strlen(wrapper);
  value[1].kind = OPENBIM_IFC_KIND_BOOL;
  value[1].int_value = 0;
  uint64_t property = 0;
  OpenbimIfcStatus status = openbim_ifc_v0_1_model_set_property(
      model, 3, (const uint8_t *)"Pset_WallCommon", 15, (const uint8_t *)"IsExternal", 10,
      value, 2, (const uint8_t *)wrapper, strlen(wrapper), NULL, 0, &property);
  // docs:end
  return status == OPENBIM_IFC_STATUS_OK && property != 0 ? 0 : 1;
}

/* Writing property sets (#123): one checked transaction, shared codes. */
static int property_edits(void) {
  OpenbimIfcModel model = 0;
  OK(openbim_ifc_v0_1_model_parse((const uint8_t *)DOMAIN_TEXT, strlen(DOMAIN_TEXT), &model,
                                  NULL, 0));
  CHECK(documented_writing(model) == 0, "the documented writing example runs");

  /* A batch: SET Custom.Note = IFCLABEL('x'), then REMOVE Custom.Missing,
   * which the wall does not state: the whole batch is refused. */
  static const char STRINGS[] = "SETCustomNoteIFCLABELxREMOVECustomMissing";
  OpenbimIfcValueNode batch[14];
  batch[0] = node(OPENBIM_IFC_KIND_LIST, 2, 0, 0, 0);
  batch[1] = node(OPENBIM_IFC_KIND_LIST, 6, 0, 0, 0);
  batch[2] = node(OPENBIM_IFC_KIND_ENUM, 0, 0, 0, 3);
  batch[3] = node(OPENBIM_IFC_KIND_REF, 0, 3, 0, 0);
  batch[4] = node(OPENBIM_IFC_KIND_TEXT, 0, 0, 3, 6);
  batch[5] = node(OPENBIM_IFC_KIND_TEXT, 0, 0, 9, 4);
  batch[6] = node(OPENBIM_IFC_KIND_TYPED, 0, 0, 13, 8);
  batch[7] = node(OPENBIM_IFC_KIND_TEXT, 0, 0, 21, 1);
  batch[8] = node(OPENBIM_IFC_KIND_NULL, 0, 0, 0, 0);
  batch[9] = node(OPENBIM_IFC_KIND_LIST, 4, 0, 0, 0);
  batch[10] = node(OPENBIM_IFC_KIND_ENUM, 0, 0, 22, 6);
  batch[11] = node(OPENBIM_IFC_KIND_REF, 0, 3, 0, 0);
  batch[12] = node(OPENBIM_IFC_KIND_TEXT, 0, 0, 28, 6);
  batch[13] = node(OPENBIM_IFC_KIND_TEXT, 0, 0, 34, 7);
  size_t strings_len = strlen(STRINGS);

  size_t before_len = 0, after_len = 0;
  openbim_ifc_v0_1_model_write(model, NULL, 0, &before_len);
  uint8_t *before = (uint8_t *)malloc(before_len);
  OK(openbim_ifc_v0_1_model_write(model, before, before_len, &before_len));

  uint64_t properties[2] = {0, 0};
  size_t count = 0;
  CHECK(openbim_ifc_v0_1_model_set_properties(model, batch, 14, (const uint8_t *)STRINGS,
                                              strings_len, properties, 1, &count) ==
            OPENBIM_IFC_STATUS_BUFFER_TOO_SMALL,
        "a short id buffer is refused before anything is written");
  CHECK(count == 2, "two edits");
  CHECK(openbim_ifc_v0_1_model_set_properties(model, batch, 14, (const uint8_t *)STRINGS,
                                              strings_len, properties, 2, &count) ==
            OPENBIM_IFC_STATUS_MISSING_PROPERTY,
        "the refused removal refuses the batch");
  char code[32];
  size_t need = 0;
  OK(openbim_ifc_v0_1_last_error_code(model, (uint8_t *)code, sizeof code, &need));
  CHECK(strcmp(code, "missing-property") == 0, "the shared code");
  uint8_t *after = (uint8_t *)malloc(before_len + 1);
  OK(openbim_ifc_v0_1_model_write(model, after, before_len + 1, &after_len));
  CHECK(after_len == before_len && memcmp(before, after, before_len) == 0,
        "a refused batch leaves the model unchanged");
  free(before);
  free(after);

  /* The SET edit alone applies; its id comes back. */
  batch[0].child_count = 1;
  OK(openbim_ifc_v0_1_model_set_properties(model, batch, 9, (const uint8_t *)STRINGS, 22,
                                           properties, 2, &count));
  CHECK(count == 1 && properties[0] != 0, "the written property's id");

  /* IsExternal is an IfcBoolean in the catalog's Pset_WallCommon. */
  const char *real_wrapper = "IFCREAL";
  OpenbimIfcValueNode real[2];
  real[0] = node(OPENBIM_IFC_KIND_TYPED, 0, 0, 0, strlen(real_wrapper));
  real[1] = node(OPENBIM_IFC_KIND_REAL, 0, 0, 0, 0);
  real[1].real_value = 1.0;
  uint64_t id = 0;
  CHECK(openbim_ifc_v0_1_model_set_property(
            model, 3, (const uint8_t *)"Pset_WallCommon", 15, (const uint8_t *)"IsExternal", 10,
            real, 2, (const uint8_t *)real_wrapper, strlen(real_wrapper), NULL, 0, &id) ==
            OPENBIM_IFC_STATUS_TEMPLATE_VIOLATION,
        "the catalog's data type");
  OK(openbim_ifc_v0_1_model_remove_property(model, 3, (const uint8_t *)"Pset_WallCommon", 15,
                                            (const uint8_t *)"IsExternal", 10));
  OK(openbim_ifc_v0_1_model_destroy(model));
  return 0;
}

/* ---- Schema-checked creation (#330) ------------------------------------ */

/* A growing value tape: pre-order nodes plus one string buffer. */
typedef struct {
  OpenbimIfcValueNode nodes[512];
  size_t count;
  char strings[8192];
  size_t len;
} Tape;

static void t_push(Tape *t, int32_t kind, uint32_t children, int64_t integer, double real,
                   const char *text) {
  OpenbimIfcValueNode *n = &t->nodes[t->count++];
  memset(n, 0, sizeof *n);
  n->kind = kind;
  n->child_count = children;
  n->int_value = integer;
  n->real_value = real;
  if (text != NULL) {
    size_t len = strlen(text);
    n->str_offset = t->len;
    n->str_len = len;
    memcpy(t->strings + t->len, text, len);
    t->len += len;
  }
}
static void t_list(Tape *t, uint32_t n) { t_push(t, OPENBIM_IFC_KIND_LIST, n, 0, 0.0, NULL); }
static void t_text(Tape *t, const char *s) { t_push(t, OPENBIM_IFC_KIND_TEXT, 0, 0, 0.0, s); }
static void t_enum(Tape *t, const char *s) { t_push(t, OPENBIM_IFC_KIND_ENUM, 0, 0, 0.0, s); }
static void t_ref(Tape *t, uint64_t id) {
  t_push(t, OPENBIM_IFC_KIND_REF, 0, (int64_t)id, 0.0, NULL);
}
static void t_int(Tape *t, int64_t v) { t_push(t, OPENBIM_IFC_KIND_INTEGER, 0, v, 0.0, NULL); }
static void t_real(Tape *t, double v) { t_push(t, OPENBIM_IFC_KIND_REAL, 0, 0, v, NULL); }
static void t_reals(Tape *t, double x, double y, double z) {
  t_list(t, 3);
  t_real(t, x);
  t_real(t, y);
  t_real(t, z);
}
/* The handle of operation `i`'s entity. */
static uint64_t handle(uint64_t i) { return OPENBIM_IFC_HANDLE_BASE + i; }
/* An operation with `fields` name/value pairs; the caller writes them. */
static void t_op(Tape *t, const char *name, uint32_t fields) {
  t_list(t, 1 + 2 * fields);
  t_enum(t, name);
}
/* A `create` of `type` with `attributes` (name, value) pairs to follow. */
static void t_create(Tape *t, const char *type, uint32_t attributes) {
  t_op(t, "CREATE", 2);
  t_text(t, "type");
  t_text(t, type);
  t_text(t, "attributes");
  t_list(t, attributes);
}
static void t_pair(Tape *t, const char *name) {
  t_list(t, 2);
  t_text(t, name);
}

static uint8_t *write_step(OpenbimIfcModel model, size_t *len) {
  openbim_ifc_v0_1_model_write(model, NULL, 0, len);
  uint8_t *bytes = (uint8_t *)malloc(*len);
  if (openbim_ifc_v0_1_model_write(model, bytes, *len, len) != OPENBIM_IFC_STATUS_OK) {
    free(bytes);
    return NULL;
  }
  return bytes;
}

/* The single-call example published on the docs site's C page. */
static int documented_authoring(OpenbimIfcModel model) {
  // docs:snippet c-authoring
  /* Create an IfcBuildingElementProxy named "Proxy": the attributes are a
   * LIST of (TEXT name, value) pairs, checked against the release the
   * header declares; the GlobalId is generated. */
  static const char names[] = "NameProxy";
  OpenbimIfcValueNode attributes[4];
  memset(attributes, 0, sizeof attributes);
  attributes[0].kind = OPENBIM_IFC_KIND_LIST;
  attributes[0].child_count = 1;
  attributes[1].kind = OPENBIM_IFC_KIND_LIST;
  attributes[1].child_count = 2;
  attributes[2].kind = OPENBIM_IFC_KIND_TEXT; /* "Name" */
  attributes[2].str_len = 4;
  attributes[3].kind = OPENBIM_IFC_KIND_TEXT; /* "Proxy" */
  attributes[3].str_offset = 4;
  attributes[3].str_len = 5;
  const char type[] = "IfcBuildingElementProxy";
  uint64_t proxy = 0;
  OpenbimIfcStatus status = openbim_ifc_v0_1_model_create_entity(
      model, (const uint8_t *)type, strlen(type), attributes, 4, (const uint8_t *)names,
      strlen(names), &proxy);
  // docs:end
  return status == OPENBIM_IFC_STATUS_OK && proxy != 0 ? 0 : 1;
}

/* Build a model from nothing in one batch, add a property set, validate,
 * and round-trip it through STEP and ifcXML. */
static int authoring(void) {
  OpenbimIfcModel model = 0;
  OK(openbim_ifc_v0_1_model_create(&model));
  static Tape t;
  memset(&t, 0, sizeof t);

  /* The header: ten fields, all empty but the schema, which the XSD
   * layout's header carries as well as STEP's. */
  t_list(&t, 10);
  t_list(&t, 0);
  t_text(&t, "");
  t_text(&t, "");
  t_text(&t, "");
  t_list(&t, 0);
  t_list(&t, 0);
  t_text(&t, "");
  t_text(&t, "");
  t_text(&t, "");
  t_list(&t, 1);
  t_text(&t, "IFC4");
  OK(openbim_ifc_v0_1_model_set_header(model, t.nodes, t.count, (const uint8_t *)t.strings,
                                       t.len));
  memset(&t, 0, sizeof t);

  t_list(&t, 15);
  /* 0..4: a length unit and the 3D model context. */
  t_create(&t, "IfcSIUnit", 2);
  t_pair(&t, "UnitType");
  t_enum(&t, "LENGTHUNIT");
  t_pair(&t, "Name");
  t_enum(&t, "METRE");
  t_create(&t, "IfcUnitAssignment", 1);
  t_pair(&t, "Units");
  t_list(&t, 1);
  t_ref(&t, handle(0));
  t_create(&t, "IfcCartesianPoint", 1);
  t_pair(&t, "Coordinates");
  t_reals(&t, 0.0, 0.0, 0.0);
  t_create(&t, "IfcAxis2Placement3D", 1);
  t_pair(&t, "Location");
  t_ref(&t, handle(2));
  t_create(&t, "IfcGeometricRepresentationContext", 4);
  t_pair(&t, "ContextType");
  t_text(&t, "Model");
  t_pair(&t, "CoordinateSpaceDimension");
  t_int(&t, 3);
  t_pair(&t, "Precision");
  t_real(&t, 1e-5);
  t_pair(&t, "WorldCoordinateSystem");
  t_ref(&t, handle(3));
  /* 5: the project. */
  t_op(&t, "PROJECT", 1);
  t_text(&t, "attributes");
  t_list(&t, 3);
  t_pair(&t, "Name");
  t_text(&t, "Demo");
  t_pair(&t, "UnitsInContext");
  t_ref(&t, handle(1));
  t_pair(&t, "RepresentationContexts");
  t_list(&t, 1);
  t_ref(&t, handle(4));
  /* 6..11: site, building and storey, each placed in its parent. */
  t_op(&t, "PLACEMENT", 0);
  t_op(&t, "SPATIAL", 3);
  t_text(&t, "type");
  t_text(&t, "IfcSite");
  t_text(&t, "parent");
  t_ref(&t, handle(5));
  t_text(&t, "placement");
  t_ref(&t, handle(6));
  t_op(&t, "PLACEMENT", 1);
  t_text(&t, "relative_to");
  t_ref(&t, handle(6));
  t_op(&t, "SPATIAL", 3);
  t_text(&t, "type");
  t_text(&t, "IfcBuilding");
  t_text(&t, "parent");
  t_ref(&t, handle(7));
  t_text(&t, "placement");
  t_ref(&t, handle(8));
  t_op(&t, "PLACEMENT", 1);
  t_text(&t, "relative_to");
  t_ref(&t, handle(8));
  t_op(&t, "SPATIAL", 4);
  t_text(&t, "type");
  t_text(&t, "IfcBuildingStorey");
  t_text(&t, "parent");
  t_ref(&t, handle(9));
  t_text(&t, "placement");
  t_ref(&t, handle(10));
  t_text(&t, "attributes");
  t_list(&t, 1);
  t_pair(&t, "Name");
  t_text(&t, "Level 0");
  /* 12: the wall type. */
  t_op(&t, "TYPE_OBJECT", 2);
  t_text(&t, "type");
  t_text(&t, "IfcWallType");
  t_text(&t, "attributes");
  t_list(&t, 1);
  t_pair(&t, "PredefinedType");
  t_enum(&t, "STANDARD");
  /* 13, 14: the wall, placed in the storey, contained and typed. */
  t_op(&t, "PLACEMENT", 2);
  t_text(&t, "relative_to");
  t_ref(&t, handle(10));
  t_text(&t, "location");
  t_reals(&t, 1.0, 2.0, 0.0);
  t_op(&t, "PRODUCT", 5);
  t_text(&t, "type");
  t_text(&t, "IfcWall");
  t_text(&t, "container");
  t_ref(&t, handle(11));
  t_text(&t, "placement");
  t_ref(&t, handle(13));
  t_text(&t, "type_object");
  t_ref(&t, handle(12));
  t_text(&t, "attributes");
  t_list(&t, 1);
  t_pair(&t, "Name");
  t_text(&t, "Wall");

  uint64_t ids[15];
  size_t count = 0;
  CHECK(openbim_ifc_v0_1_model_author(model, t.nodes, t.count, (const uint8_t *)t.strings,
                                      t.len, ids, 14, &count) ==
            OPENBIM_IFC_STATUS_BUFFER_TOO_SMALL,
        "a short id buffer is refused before anything is written");
  CHECK(count == 15, "fifteen operations");
  OK(openbim_ifc_v0_1_model_author(model, t.nodes, t.count, (const uint8_t *)t.strings, t.len,
                                   ids, 15, &count));
  uint64_t storey = ids[11], wall = ids[14];
  CHECK(wall != 0 && storey != 0, "the wall and the storey");
  CHECK(documented_authoring(model) == 0, "the documented authoring example runs");

  /* A property set through the #316 call: IFCLABEL('W-01'). */
  static const char label[] = "IFCLABELW-01";
  OpenbimIfcValueNode value[2];
  value[0] = node(OPENBIM_IFC_KIND_TYPED, 0, 0, 0, 8);
  value[1] = node(OPENBIM_IFC_KIND_TEXT, 0, 0, 8, 4);
  uint64_t property = 0;
  OK(openbim_ifc_v0_1_model_set_property(model, wall, (const uint8_t *)"ACME_WallData", 13,
                                         (const uint8_t *)"Mark", 4, value, 2,
                                         (const uint8_t *)label, strlen(label), NULL, 0,
                                         &property));

  OpenbimIfcValidationSummary summary;
  memset(&summary, 0, sizeof summary);
  size_t nodes_needed = 0, strings_needed = 0;
  openbim_ifc_v0_1_model_validate(model, 0, &summary, NULL, 0, &nodes_needed, NULL, 0,
                                  &strings_needed);
  CHECK(summary.errors == 0 && summary.evaluation_errors == 0, "the model validates clean");

  /* STEP and ifcXML (native and XSD) round trips give the same file. */
  size_t step_len = 0, again_len = 0;
  uint8_t *step = write_step(model, &step_len);
  CHECK(step != NULL, "written");
  OpenbimIfcModel again = 0;
  OK(openbim_ifc_v0_1_model_parse(step, step_len, &again, NULL, 0));
  uint8_t *rewritten = write_step(again, &again_len);
  CHECK(again_len == step_len && memcmp(rewritten, step, step_len) == 0, "STEP round trip");
  free(rewritten);
  OK(openbim_ifc_v0_1_model_destroy(again));
  const char *profiles[] = {"", "IFC4"};
  for (int i = 0; i < 2; i++) {
    size_t profile_len = strlen(profiles[i]), xml_len = 0;
    const uint8_t *profile = profile_len ? (const uint8_t *)profiles[i] : NULL;
    openbim_ifc_v0_1_model_write_ifcxml(model, profile, profile_len, NULL, 0, &xml_len);
    uint8_t *xml = (uint8_t *)malloc(xml_len);
    OK(openbim_ifc_v0_1_model_write_ifcxml(model, profile, profile_len, xml, xml_len,
                                           &xml_len));
    OK(openbim_ifc_v0_1_model_parse_ifcxml(xml, xml_len, profile, profile_len, &again, NULL,
                                           0));
    free(xml);
    rewritten = write_step(again, &again_len);
    CHECK(again_len == step_len && memcmp(rewritten, step, step_len) == 0,
          "ifcXML round trip");
    free(rewritten);
    OK(openbim_ifc_v0_1_model_destroy(again));
  }

  /* A refused batch leaves the model byte-identical: a second wall is
   * fine, but the first one is already contained. */
  memset(&t, 0, sizeof t);
  t_list(&t, 2);
  t_op(&t, "PRODUCT", 2);
  t_text(&t, "type");
  t_text(&t, "IfcWall");
  t_text(&t, "container");
  t_ref(&t, storey);
  t_op(&t, "CONTAIN", 2);
  t_text(&t, "structure");
  t_ref(&t, storey);
  t_text(&t, "elements");
  t_list(&t, 1);
  t_ref(&t, wall);
  CHECK(openbim_ifc_v0_1_model_author(model, t.nodes, t.count, (const uint8_t *)t.strings,
                                      t.len, ids, 15, &count) ==
            OPENBIM_IFC_STATUS_INVALID_MODEL,
        "a second containment is refused");
  char code[32];
  size_t need = 0;
  OK(openbim_ifc_v0_1_last_error_code(model, (uint8_t *)code, sizeof code, &need));
  CHECK(strcmp(code, "invalid-model") == 0, "the shared code");
  uint8_t *after = write_step(model, &again_len);
  CHECK(again_len == step_len && memcmp(after, step, step_len) == 0,
        "a refused batch leaves the model unchanged");
  free(after);
  free(step);

  /* Removal with relationships; a removal something still needs. */
  CHECK(openbim_ifc_v0_1_entity_remove_with_relationships(model, ids[10]) ==
            OPENBIM_IFC_STATUS_STILL_REFERENCED,
        "the storey's placement is still needed");
  OK(openbim_ifc_v0_1_entity_remove_with_relationships(model, wall));
  CHECK(openbim_ifc_v0_1_entity_remove_with_relationships(model, wall) ==
            OPENBIM_IFC_STATUS_MISSING_ENTITY,
        "removed");
  OK(openbim_ifc_v0_1_model_destroy(model));
  return 0;
}

/* The by-name example published on the docs site's C page. */
static int documented_by_name(OpenbimIfcModel model) {
  // docs:snippet c-attribute-by-name
  /* Read and write #1's Name by name: the slot comes from the release the
   * header declares, so the same call works for IFC2X3, IFC4 and IFC4X3. */
  OpenbimIfcValueNode value[4];
  uint8_t text[64];
  size_t value_nodes = 0, text_len = 0;
  const char name[] = "Name";
  if (openbim_ifc_v0_1_entity_attribute_by_name(model, 1, (const uint8_t *)name,
                                                strlen(name), value, 4, &value_nodes,
                                                text, sizeof text,
                                                &text_len) != OPENBIM_IFC_STATUS_OK) {
    return 1;
  }
  printf("Name: %.*s\n", (int)value[0].str_len, (const char *)text + value[0].str_offset);

  /* The new value is a one-value tape: TEXT at offset 0 of `renamed`. */
  const char renamed[] = "Renamed";
  OpenbimIfcValueNode edit;
  memset(&edit, 0, sizeof edit);
  edit.kind = OPENBIM_IFC_KIND_TEXT;
  edit.str_len = strlen(renamed);
  OpenbimIfcStatus status = openbim_ifc_v0_1_entity_set_attribute_by_name(
      model, 1, (const uint8_t *)name, strlen(name), &edit, 1,
      (const uint8_t *)renamed, strlen(renamed));
  // docs:end
  return status == OPENBIM_IFC_STATUS_OK ? 0 : 1;
}

/* The plain-value example published on the docs site's C page. */
static int documented_plain(OpenbimIfcModel model) {
  // docs:snippet c-attribute-plain
  /* A plain TEXT for an enumeration attribute: coerced against the declared
   * type (IfcWallTypeEnum) and written .STANDARD.; a label would be written
   * as text, a SELECT member as its typed parameter. */
  const char attribute[] = "PredefinedType";
  const char item[] = "standard";
  OpenbimIfcValueNode value;
  memset(&value, 0, sizeof value);
  value.kind = OPENBIM_IFC_KIND_TEXT;
  value.str_len = strlen(item);
  OpenbimIfcStatus status = openbim_ifc_v0_1_entity_set_attribute_by_name_plain(
      model, 1, (const uint8_t *)attribute, strlen(attribute), &value, 1,
      (const uint8_t *)item, strlen(item));
  // docs:end
  return status == OPENBIM_IFC_STATUS_OK ? 0 : 1;
}

/* Attributes by name (#326). Returns 0 on success. */
static int named_attributes(void) {
  OpenbimIfcModel model = 0;
  OK(openbim_ifc_v0_1_model_parse((const uint8_t *)FILE_TEXT, strlen(FILE_TEXT),
                                  &model, NULL, 0));
  /* IfcWall in IFC4: nine slots, each a LIST of seven fields. */
  OpenbimIfcValueNode nodes[128];
  uint8_t strings[1024];
  size_t count = 0, node_count = 0, string_len = 0;
  OK(openbim_ifc_v0_1_entity_attribute_names(model, 1, &count, nodes, 128, &node_count,
                                             strings, sizeof strings, &string_len));
  CHECK(count == 9 && nodes[0].kind == OPENBIM_IFC_KIND_LIST && nodes[0].child_count == 9,
        "nine attribute records");
  CHECK(nodes[1].kind == OPENBIM_IFC_KIND_LIST && nodes[1].child_count == 7, "seven fields");
  CHECK(nodes[2].kind == OPENBIM_IFC_KIND_TEXT && nodes[2].str_len == 8 &&
            memcmp(strings + nodes[2].str_offset, "GlobalId", 8) == 0,
        "first name in schema spelling");
  CHECK(nodes[3].kind == OPENBIM_IFC_KIND_INTEGER && nodes[3].int_value == 0, "its slot");
  CHECK(nodes[7].kind == OPENBIM_IFC_KIND_BOOL && nodes[7].int_value == 0, "not derived");

  CHECK(documented_by_name(model) == 0, "the documented by-name example runs");
  OK(openbim_ifc_v0_1_entity_attribute(model, 1, 2, nodes, 128, &node_count, strings,
                                       sizeof strings, &string_len));
  CHECK(nodes[0].kind == OPENBIM_IFC_KIND_TEXT && nodes[0].str_len == 7 &&
            memcmp(strings + nodes[0].str_offset, "Renamed", 7) == 0,
        "the write landed in slot 2");

  /* Case-insensitive; the description slot holds `*`. */
  const char description[] = "DESCRIPTION";
  OK(openbim_ifc_v0_1_entity_attribute_by_name(model, 1, (const uint8_t *)description,
                                               strlen(description), nodes, 128, &node_count,
                                               strings, sizeof strings, &string_len));
  CHECK(nodes[0].kind == OPENBIM_IFC_KIND_DERIVED, "a `*` reads as stored");

  const char inverse[] = "IsDefinedBy";
  CHECK(openbim_ifc_v0_1_entity_attribute_by_name(model, 1, (const uint8_t *)inverse,
                                                  strlen(inverse), nodes, 128, &node_count,
                                                  strings, sizeof strings, &string_len) ==
            OPENBIM_IFC_STATUS_UNKNOWN_ATTRIBUTE,
        "an inverse attribute has no slot");
  char code[64];
  size_t need = 0;
  OK(openbim_ifc_v0_1_last_error_code(model, (uint8_t *)code, sizeof code, &need));
  CHECK(strcmp(code, "unknown-attribute") == 0, "shared code");

  /* IfcSIUnit derives Dimensions: refused, nothing written. */
  static const char UNIT[] = "LENGTHUNITMETRE";
  OpenbimIfcValueNode unit[4] = {
      node(OPENBIM_IFC_KIND_DERIVED, 0, 0, 0, 0),
      node(OPENBIM_IFC_KIND_ENUM, 0, 0, 0, 10),
      node(OPENBIM_IFC_KIND_NULL, 0, 0, 0, 0),
      node(OPENBIM_IFC_KIND_ENUM, 0, 0, 10, 5),
  };
  uint64_t si = 0;
  const char si_type[] = "IFCSIUNIT";
  OK(openbim_ifc_v0_1_entity_add(model, (const uint8_t *)si_type, strlen(si_type), 4, unit, 4,
                                 (const uint8_t *)UNIT, strlen(UNIT), &si));
  const char dimensions[] = "Dimensions";
  OpenbimIfcValueNode null_value = node(OPENBIM_IFC_KIND_NULL, 0, 0, 0, 0);
  CHECK(openbim_ifc_v0_1_entity_set_attribute_by_name(model, si, (const uint8_t *)dimensions,
                                                      strlen(dimensions), &null_value, 1,
                                                      NULL, 0) ==
            OPENBIM_IFC_STATUS_DERIVED_ATTRIBUTE,
        "a derived slot is refused");
  OK(openbim_ifc_v0_1_entity_attribute(model, si, 0, nodes, 128, &node_count, strings,
                                       sizeof strings, &string_len));
  CHECK(nodes[0].kind == OPENBIM_IFC_KIND_DERIVED, "the refused write changed nothing");

  /* Plain values (#342): a string names an enumeration item, in any case. */
  CHECK(documented_plain(model) == 0, "the documented plain-value example runs");
  OK(openbim_ifc_v0_1_entity_attribute(model, 1, 8, nodes, 128, &node_count, strings,
                                       sizeof strings, &string_len));
  CHECK(nodes[0].kind == OPENBIM_IFC_KIND_ENUM && nodes[0].str_len == 8 &&
            memcmp(strings + nodes[0].str_offset, "STANDARD", 8) == 0,
        "the item, as the schema spells it");
  static const char CURVED[] = "curved";
  OpenbimIfcValueNode plain = node(OPENBIM_IFC_KIND_TEXT, 0, 0, 0, strlen(CURVED));
  const char predefined[] = "PredefinedType";
  CHECK(openbim_ifc_v0_1_entity_set_attribute_by_name_plain(
            model, 1, (const uint8_t *)predefined, strlen(predefined), &plain, 1,
            (const uint8_t *)CURVED, strlen(CURVED)) == OPENBIM_IFC_STATUS_TYPE_MISMATCH,
        "no IfcWallTypeEnum item is CURVED");
  OK(openbim_ifc_v0_1_last_error_code(model, (uint8_t *)code, sizeof code, &need));
  CHECK(strcmp(code, "type-mismatch") == 0, "shared code");
  /* IfcPropertySingleValue.NominalValue is IfcValue: a string fits many. */
  static const char VALUE[] = "x";
  OpenbimIfcValueNode ambiguous = node(OPENBIM_IFC_KIND_TEXT, 0, 0, 0, 1);
  const char nominal[] = "NominalValue";
  CHECK(openbim_ifc_v0_1_entity_set_attribute_by_name_plain(
            model, 2, (const uint8_t *)nominal, strlen(nominal), &ambiguous, 1,
            (const uint8_t *)VALUE, 1) == OPENBIM_IFC_STATUS_AMBIGUOUS_VALUE,
        "an ambiguous SELECT member is refused");
  OK(openbim_ifc_v0_1_model_destroy(model));
  return 0;
}

/* A millimetre file with one wall 1 m east and 2 m north of the origin,
 * its Body a 4 m x 0.2 m x 3 m extrusion (#328). */
static const char GEOMETRY_TEXT[] =
    "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n"
    "FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n"
    "#1=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);\n"
    "#2=IFCUNITASSIGNMENT((#1));\n"
    "#3=IFCCARTESIANPOINT((0.,0.,0.));\n"
    "#4=IFCAXIS2PLACEMENT3D(#3,$,$);\n"
    "#5=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#4,$);\n"
    "#6=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,(#5),#2);\n"
    "#7=IFCCARTESIANPOINT((1000.,2000.,0.));\n"
    "#8=IFCAXIS2PLACEMENT3D(#7,$,$);\n"
    "#9=IFCLOCALPLACEMENT($,#8);\n"
    "#10=IFCAXIS2PLACEMENT2D(#11,$);\n"
    "#11=IFCCARTESIANPOINT((0.,0.));\n"
    "#12=IFCRECTANGLEPROFILEDEF(.AREA.,$,#10,4000.,200.);\n"
    "#13=IFCDIRECTION((0.,0.,1.));\n"
    "#14=IFCEXTRUDEDAREASOLID(#12,#4,#13,3000.);\n"
    "#15=IFCSHAPEREPRESENTATION(#5,'Body','SweptSolid',(#14));\n"
    "#16=IFCPRODUCTDEFINITIONSHAPE($,$,(#15));\n"
    "#17=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,#9,#16,$,.STANDARD.);\n"
    "ENDSEC;\nEND-ISO-10303-21;\n";

/* The geometry example published on the docs site's C page. */
static int documented_geometry(OpenbimIfcModel model) {
  // docs:snippet c-geometry
  /* Level 1, in every build: each product's placement and Body as a tape of
   * ProductPlacement records; null ids with count 0 select every product. */
  OpenbimIfcValueNode nodes[128];
  uint8_t strings[1024];
  size_t products = 0, node_count = 0, string_len = 0;
  openbim_ifc_v0_1_model_product_placements(model, NULL, 0, &products, nodes, 128,
                                            &node_count, strings, sizeof strings, &string_len);
  /* nodes[5] is the first record's transform: a LIST of 16 REALs, a
   * column-major 4x4 in metres; its translation is nodes[18..20]. */
  printf("%zu product(s); first at %g %g %g\n", products, nodes[18].real_value,
         nodes[19].real_value, nodes[20].real_value);

  /* Level 3, in a library built with the `mesh` feature: compile once into
   * a set, then copy each product's arrays out. */
  OpenbimIfcMeshes meshes = 0;
  if (openbim_ifc_v0_1_model_product_meshes(model, NULL, 0, &meshes) == OPENBIM_IFC_STATUS_OK) {
    size_t need = 0;
    openbim_ifc_v0_1_meshes_positions(meshes, 0, NULL, 0, &need);
    float *positions = (float *)malloc(need * sizeof(float)); /* x y z, metres */
    openbim_ifc_v0_1_meshes_positions(meshes, 0, positions, need, &need);
    openbim_ifc_v0_1_meshes_indices(meshes, 0, NULL, 0, &need);
    uint32_t *indices = (uint32_t *)malloc(need * sizeof(uint32_t)); /* 3 per triangle */
    openbim_ifc_v0_1_meshes_indices(meshes, 0, indices, need, &need);
    printf("%zu triangle(s)\n", need / 3);
    free(positions);
    free(indices);
    openbim_ifc_v0_1_meshes_destroy(meshes);
  }
  // docs:end
  return products == 1 ? 0 : 1;
}

/* The neutral-graph example published on the docs site's C page (#367). */
static int documented_graph(OpenbimIfcModel model) {
  // docs:snippet c-geometry-graph
  /* Level 2, in a library built with the `graph` feature: lower and encode
   * once into a set, then copy each product's wire payload out. */
  OpenbimIfcGraphs graphs = 0;
  size_t products = 0;
  if (openbim_ifc_v0_1_model_product_geometry(model, NULL, 0, OPENBIM_IFC_GEOMETRY_JSON,
                                              &graphs) == OPENBIM_IFC_STATUS_OK) {
    OpenbimIfcValueNode nodes[128];
    uint8_t strings[1024];
    size_t node_count = 0, string_len = 0, need = 0;
    /* ProductGeometry records: id, global id, type, transform, encoding,
     * payload size, refusal. */
    openbim_ifc_v0_1_graphs_records(graphs, &products, nodes, 128, &node_count, strings,
                                    sizeof strings, &string_len);
    openbim_ifc_v0_1_graphs_payload(graphs, 0, NULL, 0, &need);
    char *json = (char *)malloc(need + 1);
    openbim_ifc_v0_1_graphs_payload(graphs, 0, (uint8_t *)json, need, &need);
    json[need] = '\0'; /* {"format":"axiolid-geometry-graph","version":"1.1",...} */
    printf("%zu graph(s); %.52s...\n", products, json);
    free(json);
    openbim_ifc_v0_1_graphs_destroy(graphs);
  }
  // docs:end
  return products == 1 ? 0 : 1;
}

/* Geometry (#328, #367): placements in every build, graphs and meshes when
 * compiled in. */
static int geometry(void) {
  OpenbimIfcModel model = 0;
  OK(openbim_ifc_v0_1_model_parse((const uint8_t *)GEOMETRY_TEXT, strlen(GEOMETRY_TEXT),
                                  &model, NULL, 0));
  OpenbimIfcValueNode nodes[128];
  uint8_t strings[1024];
  size_t count = 0, node_count = 0, string_len = 0;
  OK(openbim_ifc_v0_1_model_product_placements(model, NULL, 0, &count, nodes, 128,
                                               &node_count, strings, sizeof strings,
                                               &string_len));
  CHECK(count == 1 && nodes[1].kind == OPENBIM_IFC_KIND_LIST && nodes[1].child_count == 6,
        "one ProductPlacement record of six fields");
  CHECK(nodes[2].kind == OPENBIM_IFC_KIND_REF && nodes[2].int_value == 17, "the wall");
  CHECK(nodes[5].kind == OPENBIM_IFC_KIND_LIST && nodes[5].child_count == 16,
        "a 4x4 matrix");
  CHECK(nodes[18].real_value > 0.999 && nodes[18].real_value < 1.001 &&
            nodes[19].real_value > 1.999 && nodes[19].real_value < 2.001,
        "millimetres placed in metres");
  CHECK(nodes[22].kind == OPENBIM_IFC_KIND_LIST && nodes[22].child_count == 7,
        "a SelectedRepresentation record");
  CHECK(nodes[23].int_value == 15, "the Body representation");
  CHECK(nodes[30].kind == OPENBIM_IFC_KIND_NULL, "no refusal");

  uint64_t missing = 99;
  OK(openbim_ifc_v0_1_model_product_placements(model, &missing, 1, &count, nodes, 128,
                                               &node_count, strings, sizeof strings,
                                               &string_len));
  /* Record: id, global id, type, transform NULL, representation NULL, refusal. */
  CHECK(count == 1 && nodes[5].kind == OPENBIM_IFC_KIND_NULL, "no placement");
  CHECK(nodes[7].kind == OPENBIM_IFC_KIND_LIST && nodes[8].str_len == 17 &&
            memcmp(strings + nodes[8].str_offset, "missing-reference", 17) == 0,
        "a typed refusal, not a failed call");

  OpenbimIfcMeshes meshes = 0;
  OpenbimIfcStatus status = openbim_ifc_v0_1_model_product_meshes(model, NULL, 0, &meshes);
  if (status == OPENBIM_IFC_STATUS_FEATURE_DISABLED) {
    char code[32];
    size_t need = 0;
    OK(openbim_ifc_v0_1_last_error_code(model, (uint8_t *)code, sizeof code, &need));
    CHECK(strcmp(code, "feature-disabled") == 0, "meshes are opt-in");
  } else {
    CHECK(status == OPENBIM_IFC_STATUS_OK, "meshes compile");
    OK(openbim_ifc_v0_1_meshes_records(meshes, &count, nodes, 128, &node_count, strings,
                                       sizeof strings, &string_len));
    CHECK(count == 1 && nodes[1].child_count == 7, "one ProductMesh record");
    size_t need = 0;
    CHECK(openbim_ifc_v0_1_meshes_indices(meshes, 0, NULL, 0, &need) ==
              OPENBIM_IFC_STATUS_BUFFER_TOO_SMALL && need >= 36,
          "a box has twelve triangles");
    CHECK(openbim_ifc_v0_1_meshes_indices(meshes, 1, NULL, 0, &need) ==
              OPENBIM_IFC_STATUS_OUT_OF_RANGE,
          "one mesh only");
    OK(openbim_ifc_v0_1_meshes_destroy(meshes));
    CHECK(openbim_ifc_v0_1_meshes_destroy(meshes) == OPENBIM_IFC_STATUS_INVALID_HANDLE,
          "destroyed once");
  }
  OpenbimIfcGraphs graphs = 0;
  status = openbim_ifc_v0_1_model_product_geometry(model, NULL, 0, 7, &graphs);
  CHECK(status == OPENBIM_IFC_STATUS_INVALID_ARGUMENT && graphs == 0, "an unknown encoding");
  status = openbim_ifc_v0_1_model_product_geometry(model, NULL, 0, OPENBIM_IFC_GEOMETRY_JSON,
                                                   &graphs);
  int graphs_enabled = status != OPENBIM_IFC_STATUS_FEATURE_DISABLED;
  if (!graphs_enabled) {
    char code[32];
    size_t need = 0;
    OK(openbim_ifc_v0_1_last_error_code(model, (uint8_t *)code, sizeof code, &need));
    CHECK(strcmp(code, "feature-disabled") == 0, "graphs are opt-in");
  } else {
    /* The version is the lowest the content needs, "1.0" or "1.1". */
    static const char ENVELOPE[] = "{\"format\":\"axiolid-geometry-graph\",\"version\":\"1.";
    static const char GRAPH[] = "\",\"graph\":{\"nodes\":[";
    CHECK(status == OPENBIM_IFC_STATUS_OK, "graphs lower");
    OK(openbim_ifc_v0_1_graphs_records(graphs, &count, nodes, 128, &node_count, strings,
                                       sizeof strings, &string_len));
    /* Record: id, global id, type, transform (16), encoding, size, refusal. */
    CHECK(count == 1 && nodes[1].child_count == 7, "one ProductGeometry record");
    CHECK(nodes[22].kind == OPENBIM_IFC_KIND_TEXT && nodes[22].str_len == 4 &&
              memcmp(strings + nodes[22].str_offset, "json", 4) == 0,
          "the encoding");
    CHECK(nodes[24].kind == OPENBIM_IFC_KIND_NULL, "no refusal");
    size_t need = 0;
    CHECK(openbim_ifc_v0_1_graphs_payload(graphs, 0, NULL, 0, &need) ==
              OPENBIM_IFC_STATUS_BUFFER_TOO_SMALL &&
              need == (size_t)nodes[23].int_value,
          "the payload is the size its record states");
    uint8_t *payload = (uint8_t *)malloc(need);
    OK(openbim_ifc_v0_1_graphs_payload(graphs, 0, payload, need, &need));
    const size_t at = sizeof ENVELOPE - 1;
    CHECK(need > at + sizeof GRAPH && memcmp(payload, ENVELOPE, at) == 0 &&
              (payload[at] == '0' || payload[at] == '1') &&
              memcmp(payload + at + 1, GRAPH, sizeof GRAPH - 1) == 0,
          "Axiolid's wire envelope, format 1.0 or 1.1");
    free(payload);
    CHECK(openbim_ifc_v0_1_graphs_payload(graphs, 1, NULL, 0, &need) ==
              OPENBIM_IFC_STATUS_OUT_OF_RANGE,
          "one graph only");
    OK(openbim_ifc_v0_1_graphs_destroy(graphs));
    CHECK(openbim_ifc_v0_1_graphs_destroy(graphs) == OPENBIM_IFC_STATUS_INVALID_HANDLE,
          "destroyed once");

    /* CBOR: a map of three entries, the first the text "format". */
    OK(openbim_ifc_v0_1_model_product_geometry(model, NULL, 0, OPENBIM_IFC_GEOMETRY_CBOR,
                                               &graphs));
    uint8_t head[8];
    CHECK(openbim_ifc_v0_1_graphs_payload(graphs, 0, head, sizeof head, &need) ==
              OPENBIM_IFC_STATUS_BUFFER_TOO_SMALL,
          "a CBOR payload longer than its head");
    payload = (uint8_t *)malloc(need);
    OK(openbim_ifc_v0_1_graphs_payload(graphs, 0, payload, need, &need));
    CHECK(payload[0] == 0xa3 && payload[1] == 0x66 && memcmp(payload + 2, "format", 6) == 0,
          "a CBOR envelope");
    free(payload);
    OK(openbim_ifc_v0_1_graphs_destroy(graphs));
  }
  int failed = documented_geometry(model);
  if (graphs_enabled) failed |= documented_graph(model);
  OK(openbim_ifc_v0_1_model_destroy(model));
  return failed;
}

int main(void) {
  OpenbimIfcVersion version;
  OK(openbim_ifc_v0_1_version(&version));
  CHECK(version.abi_major == 0 && version.abi_minor == 1, "ABI version 0.1");

  OpenbimIfcModel model = 0;
  OK(openbim_ifc_v0_1_model_parse((const uint8_t *)FILE_TEXT,
                                  strlen(FILE_TEXT), &model, NULL, 0));
  size_t count = 0;
  OK(openbim_ifc_v0_1_model_len(model, &count));
  CHECK(count == 2, "two entities");

  /* Size query, then fetch: the protocol every buffer export uses. */
  size_t need = 0;
  CHECK(openbim_ifc_v0_1_entity_type(model, 1, NULL, 0, &need) ==
            OPENBIM_IFC_STATUS_BUFFER_TOO_SMALL,
        "size query");
  char *type = (char *)malloc(need);
  OK(openbim_ifc_v0_1_entity_type(model, 1, (uint8_t *)type, need, &need));
  CHECK(strcmp(type, "IFCWALL") == 0, "type name");
  free(type);

  /* Kinds a C host could confuse must stay distinct. */
  OpenbimIfcValueNode nodes[8];
  uint8_t strings[64];
  size_t node_count = 0, string_len = 0;
  OK(read_attribute(model, 1, 3, nodes, 8, &node_count, strings, 64,
                    &string_len));
  CHECK(node_count == 1 && nodes[0].kind == OPENBIM_IFC_KIND_DERIVED, "* is derived");
  OK(read_attribute(model, 1, 4, nodes, 8, &node_count, strings, 64,
                    &string_len));
  CHECK(nodes[0].kind == OPENBIM_IFC_KIND_NULL, "$ is null");
  OK(read_attribute(model, 2, 2, nodes, 8, &node_count, strings, 64,
                    &string_len));
  CHECK(node_count == 2 && nodes[0].kind == OPENBIM_IFC_KIND_TYPED &&
            nodes[1].kind == OPENBIM_IFC_KIND_UNKNOWN,
        "IFCLOGICAL(.U.) is typed(unknown)");
  CHECK(string_len == 10 && memcmp(strings, "IFCLOGICAL", 10) == 0,
        "wrapper name in the string buffer");

  /* Edit: rename the wall. */
  const char *name = "Renamed";
  /* memset rather than {0}: valid, warning-free C and C++ alike. */
  OpenbimIfcValueNode text;
  memset(&text, 0, sizeof text);
  text.kind = OPENBIM_IFC_KIND_TEXT;
  text.str_len = (uint32_t)strlen(name);
  OK(openbim_ifc_v0_1_entity_set_attribute(model, 1, 2, &text, 1,
                                           (const uint8_t *)name, strlen(name)));

  /* Write a `*` into slot 4 (currently `$`): exercises the decoder, which
   * reading alone never reaches. */
  OpenbimIfcValueNode derived;
  memset(&derived, 0, sizeof derived);
  derived.kind = OPENBIM_IFC_KIND_DERIVED;
  OK(openbim_ifc_v0_1_entity_set_attribute(model, 1, 4, &derived, 1, NULL, 0));

  /* Add: a point with a list of two reals. */
  OpenbimIfcValueNode point[3];
  memset(point, 0, sizeof point);
  point[0].kind = OPENBIM_IFC_KIND_LIST;
  point[0].child_count = 2;
  point[1].kind = OPENBIM_IFC_KIND_REAL;
  point[1].real_value = 1.5;
  point[2].kind = OPENBIM_IFC_KIND_REAL;
  point[2].real_value = -2.0;
  uint64_t added = 0;
  const char *point_type = "IfcCartesianPoint";
  OK(openbim_ifc_v0_1_entity_add(model, (const uint8_t *)point_type,
                                 strlen(point_type), 1, point, 3, NULL, 0,
                                 &added));
  CHECK(added == 3, "next id after #2");

  /* Misuse is a status, never a crash. */
  CHECK(openbim_ifc_v0_1_entity_remove(model, 99) ==
            OPENBIM_IFC_STATUS_MISSING_ENTITY,
        "missing entity");
  char code[32];
  OK(openbim_ifc_v0_1_last_error_code(model, (uint8_t *)code, sizeof code, &need));
  CHECK(strcmp(code, "missing-entity") == 0, "stable error code");
  CHECK(openbim_ifc_v0_1_model_len(0, &count) == OPENBIM_IFC_STATUS_INVALID_HANDLE,
        "zero handle");

  /* Write, destroy, re-parse: the edit and the addition survive. */
  CHECK(openbim_ifc_v0_1_model_write(model, NULL, 0, &need) ==
            OPENBIM_IFC_STATUS_BUFFER_TOO_SMALL,
        "write size query");
  uint8_t *bytes = (uint8_t *)malloc(need);
  size_t written = 0;
  OK(openbim_ifc_v0_1_model_write(model, bytes, need, &written));
  OK(openbim_ifc_v0_1_model_destroy(model));
  CHECK(openbim_ifc_v0_1_model_destroy(model) == OPENBIM_IFC_STATUS_INVALID_HANDLE,
        "double destroy");

  OpenbimIfcModel again = 0;
  OK(openbim_ifc_v0_1_model_parse(bytes, written, &again, NULL, 0));
  free(bytes);
  OK(read_attribute(again, 1, 2, nodes, 8, &node_count, strings, 64, &string_len));
  CHECK(nodes[0].kind == OPENBIM_IFC_KIND_TEXT && string_len == 7 &&
            memcmp(strings, "Renamed", 7) == 0,
        "edit survived");
  OK(read_attribute(again, 1, 4, nodes, 8, &node_count, strings, 64, &string_len));
  CHECK(node_count == 1 && nodes[0].kind == OPENBIM_IFC_KIND_DERIVED,
        "written * survived as *, not $");
  OK(read_attribute(again, 3, 0, nodes, 8, &node_count, strings, 64, &string_len));
  CHECK(node_count == 3 && nodes[2].real_value == -2.0, "added point survived");
  OK(openbim_ifc_v0_1_model_destroy(again));

  /* Opening from a path: owned and mapped reads see the same file. */
  char path[512];
  {
    /* The platform's temporary directory: TMPDIR on Unix, TEMP on Windows. */
    const char *dir = getenv("TMPDIR");
    FILE *file = NULL;
    if (dir == NULL || dir[0] == '\0') dir = getenv("TEMP");
    if (dir == NULL || dir[0] == '\0') dir = "/tmp";
    CHECK(strlen(dir) < sizeof path - 40, "temp directory path fits");
    snprintf(path, sizeof path, "%s/openbim_ifc_smoke_%d.ifc", dir, (int)rand());
    file = fopen(path, "wb");
    CHECK(file != NULL, "temp file");
    CHECK(fwrite(FILE_TEXT, 1, sizeof FILE_TEXT - 1, file) == sizeof FILE_TEXT - 1,
          "temp file written");
    fclose(file);
  }
  OpenbimIfcModel opened = 0;
  OK(openbim_ifc_v0_1_model_open((const uint8_t *)path, strlen(path), &opened, NULL, 0));
  OK(openbim_ifc_v0_1_model_len(opened, &count));
  CHECK(count == 2, "opened file has two entities");
  OK(openbim_ifc_v0_1_model_destroy(opened));
  OK(openbim_ifc_v0_1_model_open_mapped((const uint8_t *)path, strlen(path), &opened,
                                        NULL, 0));
  OK(openbim_ifc_v0_1_model_len(opened, &count));
  CHECK(count == 2, "mapped file has two entities");
  OK(openbim_ifc_v0_1_model_destroy(opened));
  remove(path);
  const char missing[] = "/nonexistent/openbim_ifc_smoke.ifc";
  char message[128];
  CHECK(openbim_ifc_v0_1_model_open((const uint8_t *)missing, strlen(missing), &opened,
                                    (uint8_t *)message, sizeof message) ==
            OPENBIM_IFC_STATUS_IO,
        "a missing file is an io error");
  CHECK(strlen(message) > 0, "io error message");

  CHECK(documented_example() == 0, "the documented example runs");
  CHECK(capabilities() == 0, "the #244 surface works from C");
  CHECK(domains() == 0, "the domain views work from C");
  CHECK(geometry() == 0, "placements and meshes cross to C");
  CHECK(property_edits() == 0, "property sets are written from C");
  CHECK(named_attributes() == 0, "attributes are read and written by name from C");
  CHECK(authoring() == 0, "a model is built from nothing from C");

  size_t live = 1;
  OK(openbim_ifc_v0_1_live_models(&live));
  CHECK(live == 0, "no leaked models");
  puts("openbim-ifc C ABI smoke: ok");
  return 0;
}
