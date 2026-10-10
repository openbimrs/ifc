# JavaScript and TypeScript cookbook

Task-sized recipes for [`@openbim/ifc`](/bindings/javascript), the
WebAssembly build of the IFC core. Each recipe is a region of
`crates/openbim-ifc-wasm/tests/js/cookbook.mjs`, which runs on every pull
request against the built package and checks what the recipe prints, so
the code below is the code that ran. The comments show its output for the
fixtures it reads: `test/fixtures/synthetic-properties/synthetic_properties.ifc`
(two walls on one storey, with their property sets) and
`test/fixtures/synthetic-bindings/binding_geometry.ifc` (placed, extruded
products).

For the full surface, see the [binding page](/bindings/javascript) and the
[TypeScript API reference](/api/typedoc/index.html){target="_self"}; to try
the library without installing anything, open the
[playground](/playground).

## Set up

```bash
npm install @openbim/ifc
```

In Node and in a bundler, import the package by name:

<!-- SNIPPET:js-bundler-import -->

```js
import { IfcModel } from "@openbim/ifc"; // the bundler loads the wasm module
```

<!-- /SNIPPET -->

In a browser without a bundler, import the `web` build and initialise it
once:

<!-- SNIPPET:js-web-init -->

```js
import init, { IfcModel } from "@openbim/ifc/web";

await init(); // fetches openbim_ifc_wasm_bg.wasm from next to the module
```

<!-- /SNIPPET -->

Values cross in a tagged encoding that keeps every distinction STEP makes
(`{ kind: "typed", type: "IFCLABEL", value: { kind: "text", value: "F30" } }`).
The recipes print them through this helper, which drops what a report
does not need:

<!-- SNIPPET:cookbook-js-plain -->

```js
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
```

<!-- /SNIPPET -->

## Open a file

<!-- SNIPPET:cookbook-js-open -->

```js
// Node: a Buffer is a Uint8Array.
const model = IfcModel.parse(readFileSync(path));
console.log(model.schema, model.size); // IFC4 68
console.log(model.header().name, model.header().originatingSystem);
```

<!-- /SNIPPET -->

In a browser the bytes come from a `File`; the model is parsed in the page
and nothing leaves it:

<!-- SNIPPET:cookbook-js-open-browser -->

```js
// A browser: a File from <input type="file"> or a drop; nothing is uploaded.
const fromFile = IfcModel.parse(new Uint8Array(await file.arrayBuffer()));
```

<!-- /SNIPPET -->

A file that does not parse throws an `IfcError` with `code` `parse`;
`IfcModel.parseWithOptions(bytes, { onMalformed: "skip" })` reads a damaged
export and lists what it skipped in `model.diagnostics()`.

## Read property sets

One object's property sets, its own first, then those it inherits from
its type:

<!-- SNIPPET:cookbook-js-property-sets -->

```js
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
```

<!-- /SNIPPET -->

Many objects in one call, here every wall as a table row:

<!-- SNIPPET:cookbook-js-property-sets-many -->

```js
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
```

<!-- /SNIPPET -->

`propertySetsMany()` with no ids answers for every object definition in
the file.

## List storeys and their elements

<!-- SNIPPET:cookbook-js-storeys -->

```js
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
```

<!-- /SNIPPET -->

Each node also has its `parent` and `children`, so the same tree gives the
project, site and building above the storeys and the spaces below them;
`tree.orphans` lists elements no container holds.

## Validate

<!-- SNIPPET:cookbook-js-validate -->

```js
const report = model.validate(); // against the schema the header declares
console.log(report.conformant ? "conformant" : `${report.errors} error(s), ${report.warnings} warning(s)`);
for (const finding of report.findings) {
  // severity: "error", "evaluation-error", "warning" or "unsupported"
  const at = finding.entity === undefined ? "file" : `#${finding.entity}`;
  console.log(`${finding.severity} ${finding.rule} ${at} ${finding.attributeName ?? ""}: ${finding.message}`);
}
```

<!-- /SNIPPET -->

The recipe ran on the fixture with Wall A's `PredefinedType` set to an
item `IfcWallTypeEnum` does not have. `validate(maxFindings)` caps the
report; `report.truncated` says when the cap was reached.

## Convert to and from ifcXML

<!-- SNIPPET:cookbook-js-ifcxml -->

```js
// STEP to ifcXML, in this library's lossless layout, and back.
const xml = model.writeIfcXml(); // a Uint8Array of UTF-8 XML
const back = IfcModel.parseIfcXml(xml);
const step = back.write(); // the same entities, as STEP again

// The buildingSMART XSD layout of a release, for tools that read it.
const xsd = model.writeIfcXml("IFC4");
const fromXsd = IfcModel.parseIfcXml(xsd, "IFC4");
```

<!-- /SNIPPET -->

The lossless layout carries everything the STEP file does. An XSD layout
refuses, with `code` `write`, what it cannot carry exactly.

## Edit a property set and attributes

<!-- SNIPPET:cookbook-js-edit -->

```js
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
```

<!-- /SNIPPET -->

Property values are exact `IfcValue`s, typed as `propertySets` reports
them; attributes take plain JavaScript values (strings, numbers, booleans,
arrays, `null`) coerced against the attribute's declared type, or an exact
`IfcValue`. A refused edit throws and changes nothing: `invalid-value`,
`template-violation` (the PSD/QTO catalog disagrees), `type-mismatch` (a
plain value that does not fit).

## Read placements, meshes and exact geometry

Placements are in every build:

<!-- SNIPPET:cookbook-js-placements -->

```js
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
```

<!-- /SNIPPET -->

Meshes need the `mesh` entry, which carries the geometry kernel:

<!-- SNIPPET:js-mesh-import -->

```js
import { IfcModel } from "@openbim/ifc/mesh"; // productMeshes included
```

<!-- /SNIPPET -->

<!-- SNIPPET:cookbook-js-meshes -->

```js
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
```

<!-- /SNIPPET -->

Positions are `f32` relative to each product's `f64` transform, so a site
kilometres from the origin keeps its millimetres. The WebGL viewer in
`crates/openbim-ifc-wasm/examples/viewer/`, which the
[playground](/playground) uses, shows the same data on screen.

The exact representation instead of triangles
([#367](https://github.com/openbimrs/ifc/issues/367)): each Body as
Axiolid's neutral geometry graph in its wire format 1.0, for your own
kernel. The mesh entry carries it:

<!-- SNIPPET:cookbook-js-graphs -->

```js
// From `@openbim/ifc/mesh`: each Body as Axiolid's neutral graph, exact, in
// world metres. "object" parses it; "json" keeps the text, "cbor" the bytes.
const kinds = new Map();
for (const product of model.productGeometry(undefined, "object")) {
  if (product.refusal || !product.payload) continue; // refused, or no Body
  const { graph } = product.payload; // with format "axiolid-geometry-graph", version "1.0"
  // Each node is tagged by its kind; a reference is an earlier node's index.
  kinds.set(product.id, graph.nodes.map((node) => Object.keys(node)[0]));
}
// 36n => ["Profile", "SolidOperation", "Instance"], 46n => [...]
```

<!-- /SNIPPET -->

Each `payload` is the envelope `{ format, version, graph }`, typed as
`GeometryGraphEnvelope`, exact, in world coordinates, metres; the
[binding page](/bindings/javascript#exact-geometry-graphs) describes the
encodings.

## Create a model

<!-- SNIPPET:cookbook-js-create -->

```js
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
```

<!-- /SNIPPET -->

Every operation is checked against the release the header declares, and a
batch is one transaction. The [binding page](/bindings/javascript#creating-entities)
lists every operation.
