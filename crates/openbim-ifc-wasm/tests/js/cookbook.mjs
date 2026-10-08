// The JavaScript cookbook (docs/cookbook/javascript.md, #331): every recipe
// on that page is a `docs:snippet` region below, run against the built
// package and checked against the fixture it reads.
//
// Usage: IFC_WASM_PKG=<package-dir> node --test cookbook.mjs
// The mesh recipe runs against <package-dir>/mesh, the `@openbim/ifc/mesh`
// entry, which scripts/build-npm-pkg.sh builds beside the default one.
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const pkg = path.resolve(process.env.IFC_WASM_PKG ?? "pkg");
const require = createRequire(import.meta.url);
const { IfcModel } = require(path.join(pkg, "openbim_ifc_wasm.js"));

const here = path.dirname(fileURLToPath(import.meta.url));
const fixtures = path.resolve(here, "../../../../test/fixtures");
const PROPERTIES = path.join(fixtures, "synthetic-properties/synthetic_properties.ifc");
const GEOMETRY = path.join(fixtures, "synthetic-bindings/binding_geometry.ifc");

// docs:snippet cookbook-js-plain
// A tagged IfcValue as a plain JavaScript value: typed wrappers unwrapped,
// a list as an array, a reference as its id, `$`, `*` and `.U.` as null.
function plain(value) {
  switch (value.kind) {
    case "typed":
      return plain(value.value);
    case "list":
      return value.items.map(plain);
    case "ref":
      return value.id;
    case "null":
    case "derived":
    case "unknown":
      return null;
    default:
      return value.value;
  }
}
// docs:end

test("open a file from disk and from a browser File", async () => {
  const path = PROPERTIES;
  // docs:snippet cookbook-js-open
  // Node: a Buffer is a Uint8Array.
  const model = IfcModel.parse(readFileSync(path));
  console.log(model.schema, model.size); // IFC4 68
  console.log(model.header().name, model.header().originatingSystem);
  // docs:end
  assert.equal(model.schema, "IFC4");
  assert.equal(model.size, 68);

  const file = new Blob([readFileSync(path)]); // what an <input type="file"> yields
  // docs:snippet cookbook-js-open-browser
  // A browser: a File from <input type="file"> or a drop; nothing is uploaded.
  const fromFile = IfcModel.parse(new Uint8Array(await file.arrayBuffer()));
  // docs:end
  assert.equal(fromFile.size, model.size);
});

test("read the property sets of one object", () => {
  const model = IfcModel.parse(readFileSync(PROPERTIES));
  const printed = [];
  const console = { log: (...args) => printed.push(args) };
  // docs:snippet cookbook-js-property-sets
  const wall = model.idsOfType("IfcWall").find((id) => plain(model.attributeByName(id, "Name")) === "Wall A");
  for (const set of model.propertySets(wall)) {
    // source: "occurrence" (the wall's own) or "type" (inherited from its type)
    for (const property of set.properties) {
      console.log(`${set.name}.${property.name}`, plain(property.value), set.source);
    }
  }
  // Pset_WallCommon.IsExternal false occurrence
  // Qto_WallBaseQuantities.Width 200 occurrence ...
  // Pset_WallCommon.FireRating F30 type
  // docs:end
  assert.deepEqual(printed[0], ["Pset_WallCommon.IsExternal", false, "occurrence"]);
  assert.deepEqual(printed[1], ["Qto_WallBaseQuantities.Width", 200, "occurrence"]);
  assert.deepEqual(printed.at(-1), ["Pset_WallCommon.FireRating", "F30", "type"]);
});

test("read the property sets of many objects in one pass", () => {
  const model = IfcModel.parse(readFileSync(PROPERTIES));
  // docs:snippet cookbook-js-property-sets-many
  // One call for every wall: the property index is built once, so this stays
  // linear in the model where a loop of propertySets is quadratic.
  const rows = [];
  for (const { object, sets, refusal } of model.propertySetsMany(model.idsOfType("IfcWall"))) {
    if (refusal) {
      console.warn(object, refusal.code, refusal.message);
      continue;
    }
    const row = { id: object, Name: plain(model.attributeByName(object, "Name")) };
    for (const set of sets) {
      for (const property of set.properties) row[`${set.name}.${property.name}`] ??= plain(property.value);
    }
    rows.push(row);
  }
  console.table(rows); // one row per wall, one column per "Set.Property"
  // docs:end
  assert.equal(rows.length, 2);
  assert.equal(rows[0].Name, "Wall A");
  assert.equal(rows[0]["Pset_WallCommon.IsExternal"], false, "the occurrence value wins");
  assert.equal(rows[1]["Pset_WallCommon.IsExternal"], true, "inherited from the type");
  assert.equal(rows[1]["Pset_WallCommon.FireRating"], "F30");
});

test("list storeys and their elements", () => {
  const model = IfcModel.parse(readFileSync(PROPERTIES));
  const printed = [];
  const console = { log: (line) => printed.push(line) };
  // docs:snippet cookbook-js-storeys
  const tree = model.spatialTree();
  for (const storey of tree.nodes.filter((node) => node.kind === "storey")) {
    console.log(`${storey.name} (#${storey.id})`);
    for (const id of storey.elements) {
      console.log(`  ${model.typeOf(id)} #${id} ${plain(model.attributeByName(id, "Name"))}`);
    }
  }
  // Level 0 (#25)
  //   IFCWALL #30 Wall A
  //   IFCWALL #31 Wall B
  // docs:end
  assert.deepEqual(printed, ["Level 0 (#25)", "  IFCWALL #30 Wall A", "  IFCWALL #31 Wall B"]);
  // The tree also names the containers above the storey.
  const kinds = tree.nodes.map((node) => node.kind);
  assert.deepEqual(kinds, ["project", "site", "building", "storey"]);
});

test("validate and list the findings", () => {
  // Wall A's PredefinedType names no IfcWallTypeEnum item.
  const broken = readFileSync(PROPERTIES, "utf8").replace("'Wall A',$,$,$,$,$,$)", "'Wall A',$,$,$,$,$,.NOTANENUM.)");
  const model = IfcModel.parse(new TextEncoder().encode(broken));
  const printed = [];
  const console = { log: (line) => printed.push(line) };
  // docs:snippet cookbook-js-validate
  const report = model.validate(); // against the schema the header declares
  console.log(report.conformant ? "conformant" : `${report.errors} error(s), ${report.warnings} warning(s)`);
  for (const finding of report.findings) {
    // severity: "error", "evaluation-error", "warning" or "unsupported"
    const at = finding.entity === undefined ? "file" : `#${finding.entity}`;
    console.log(`${finding.severity} ${finding.rule} ${at} ${finding.attributeName ?? ""}: ${finding.message}`);
  }
  // docs:end
  assert.equal(report.conformant, false);
  assert.ok(report.errors > 0);
  assert.ok(printed.some((line) => line.startsWith("error ") && line.includes("#30")), printed.join("\n"));

  const clean = IfcModel.parse(readFileSync(PROPERTIES)).validate();
  assert.equal(clean.errors, 0, JSON.stringify(clean.findings, (_, v) => (typeof v === "bigint" ? `${v}` : v)));
});

test("convert to and from ifcXML", () => {
  const model = IfcModel.parse(readFileSync(GEOMETRY));
  // docs:snippet cookbook-js-ifcxml
  // STEP to ifcXML, in this library's lossless layout, and back.
  const xml = model.writeIfcXml(); // a Uint8Array of UTF-8 XML
  const back = IfcModel.parseIfcXml(xml);
  const step = back.write(); // the same entities, as STEP again

  // The buildingSMART XSD layout of a release, for tools that read it.
  const xsd = model.writeIfcXml("IFC4");
  const fromXsd = IfcModel.parseIfcXml(xsd, "IFC4");
  // docs:end
  assert.equal(back.size, model.size);
  assert.deepEqual(back.ids(), model.ids());
  assert.equal(IfcModel.parse(step).size, model.size);
  assert.match(new TextDecoder().decode(xml.subarray(0, 100)), /^<\?xml/);
  assert.equal(fromXsd.idsOfType("IfcWall").length, 1);
});

test("edit a property set and attributes", async () => {
  const model = IfcModel.parse(readFileSync(PROPERTIES));
  const [, wallB] = model.idsOfType("IfcWall");
  // docs:snippet cookbook-js-edit
  // Pset_ and Qto_ writes are checked against the release's PSD/QTO
  // catalog, which the module loads once (from catalog/ beside it).
  await IfcModel.loadCatalog(model.schema);

  const label = (value) => ({ kind: "typed", type: "IFCLABEL", value: { kind: "text", value } });
  // One checked transaction: every edit, or none and the model unchanged.
  model.setProperties([
    // Wall B inherits FireRating from its type: this overrides it on the wall.
    { object: wallB, set: "Pset_WallCommon", name: "FireRating", value: label("F90") },
    // A set the wall lacks is created with its relationship.
    { object: wallB, set: "Checks", name: "Reviewer", value: label("QA") },
  ]);

  // Attributes take plain values, coerced against their declared type.
  model.setAttributeByName(wallB, "Name", "Wall B (checked)"); // IfcLabel
  model.setAttributeByName(wallB, "PredefinedType", "solidwall"); // IfcWallTypeEnum: .SOLIDWALL.

  const bytes = model.write(); // save with fs.writeFileSync or a download link
  // docs:end
  const again = IfcModel.parse(bytes);
  const own = again.propertySets(wallB).filter((set) => set.source === "occurrence");
  const value = (setName, name) =>
    plain(own.find((set) => set.name === setName).properties.find((p) => p.name === name).value);
  assert.equal(value("Pset_WallCommon", "FireRating"), "F90");
  assert.equal(value("Checks", "Reviewer"), "QA");
  assert.equal(plain(again.attributeByName(wallB, "Name")), "Wall B (checked)");
  assert.deepEqual(again.attributeByName(wallB, "PredefinedType"), { kind: "enum", value: "SOLIDWALL" });
  // The type's shared set is unchanged.
  const typeSet = again.propertySets(29n).find((set) => set.name === "Pset_WallCommon");
  assert.equal(plain(typeSet.properties.find((p) => p.name === "FireRating").value), "F30");
});

test("read placements", () => {
  const model = IfcModel.parse(readFileSync(GEOMETRY));
  const printed = [];
  const console = { log: (...args) => printed.push(args) };
  // docs:snippet cookbook-js-placements
  for (const product of model.productPlacements()) {
    if (product.refusal) {
      console.log(product.id, product.refusal.code); // per product, never thrown
      continue;
    }
    // A column-major 4x4 in metres; the translation is its last column.
    const [x, y, z] = product.transform.slice(12, 15);
    console.log(product.typeName, x, y, z, product.representation?.representationType);
  }
  // IFCWALL 512002 5403001 3 SweptSolid ...
  // docs:end
  assert.deepEqual(printed[0], ["IFCWALL", 512002, 5403001, 3, "SweptSolid"]);
  assert.equal(printed.length, 4);
});

const meshEntry = path.join(pkg, "mesh/openbim_ifc_wasm.js");
test("read meshes through the mesh entry", { skip: !existsSync(meshEntry) && "no mesh entry built" }, () => {
  const { IfcModel } = require(meshEntry);
  const model = IfcModel.parse(readFileSync(GEOMETRY));
  // docs:snippet cookbook-js-meshes
  // From `@openbim/ifc/mesh`: triangles relative to each product's transform.
  const triangles = new Map();
  for (const mesh of model.productMeshes()) {
    if (mesh.refusal || mesh.indices.length === 0) continue; // refused, or no Body
    const world = new Float64Array(mesh.positions.length);
    const m = mesh.transform; // column-major, metres, f64
    for (let i = 0; i < mesh.positions.length; i += 3) {
      const [x, y, z] = mesh.positions.subarray(i, i + 3);
      for (let axis = 0; axis < 3; axis++) {
        world[i + axis] = m[axis] * x + m[4 + axis] * y + m[8 + axis] * z + m[12 + axis];
      }
    }
    triangles.set(mesh.id, { type: mesh.typeName, world, indices: mesh.indices });
  }
  // docs:end
  assert.deepEqual([...triangles.keys()], [36n, 46n]);
  const wall = triangles.get(36n);
  assert.equal(wall.type, "IFCWALL");
  assert.equal(wall.indices.length, 36, "a box: twelve triangles");
  // Every vertex of the wall lies at the site, kilometres from the origin.
  for (let i = 0; i < wall.world.length; i += 3) {
    assert.ok(Math.abs(wall.world[i] - 512002) < 3);
    assert.ok(Math.abs(wall.world[i + 1] - 5403001) < 3);
  }
});

test("create a model from nothing", () => {
  // docs:snippet cookbook-js-create
  const model = new IfcModel();
  model.setHeader({
    description: ["ViewDefinition [DesignTransferView]"],
    implementationLevel: "2;1",
    name: "new.ifc",
    timeStamp: new Date().toISOString().slice(0, 19),
    author: [""],
    organization: [""],
    preprocessorVersion: "@openbim/ifc",
    originatingSystem: "cookbook",
    authorization: "",
    schema: ["IFC4"], // the release every operation is checked against
  });
  const text = (value) => ({ kind: "text", value });
  const h = IfcModel.handle; // h(i): the entity operation i of this batch creates
  const { ids } = model.author([
    { op: "project", attributes: { Name: text("Demo") } }, // 0
    { op: "placement" }, // 1
    { op: "spatial", type: "IfcSite", parent: h(0), placement: h(1) }, // 2
    { op: "spatial", type: "IfcBuilding", parent: h(2) }, // 3
    { op: "spatial", type: "IfcBuildingStorey", parent: h(3), attributes: { Name: text("Level 0") } }, // 4
    { op: "placement", relativeTo: h(1), location: [4, 0, 0] }, // 5
    { op: "product", type: "IfcWall", container: h(4), placement: h(5), attributes: { Name: text("Wall") } }, // 6
  ]);
  const wall = ids[6]; // a bigint; every IfcRoot got a GlobalId
  const bytes = model.write();
  // docs:end
  const again = IfcModel.parse(bytes);
  assert.equal(again.typeOf(wall), "IFCWALL");
  const storey = again.spatialTree().nodes.find((node) => node.kind === "storey");
  assert.equal(storey.name, "Level 0");
  assert.deepEqual(storey.elements, [wall]);
  assert.equal(again.validate().errors, 0);
});
