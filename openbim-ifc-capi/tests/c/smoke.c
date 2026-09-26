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
  char path[64];
  {
    FILE *file = NULL;
    snprintf(path, sizeof path, "/tmp/openbim_ifc_smoke_%d.ifc", (int)rand());
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

  size_t live = 1;
  OK(openbim_ifc_v0_1_live_models(&live));
  CHECK(live == 0, "no leaked models");
  puts("openbim-ifc C ABI smoke: ok");
  return 0;
}
