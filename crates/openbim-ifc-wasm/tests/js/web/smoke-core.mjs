// The round trip every packaged target must pass (#40).
//
// Plain ECMAScript, no Node or DOM API, so the same checks run in Node, in a
// webpack bundle and in a browser. `await smoke(IfcModel)` throws on the
// first failed check and otherwise returns what it saw, for the caller to
// report. It loads the PSD/QTO catalog the way each target does by default
// (#318): Node from the package directory, a browser by fetching the file
// next to the module, a bundle from the asset webpack emitted.
// The full Node suites (../smoke.mjs, ../corpus.mjs) cover the API itself;
// this proves each build loads its wasm module and calls into it.
//
// `meshes(IfcModel, enabled)` checks the geometry of one entry (#369,
// #367): the mesh entry (`@openbim/ifc/mesh`) returns a wall's triangles as
// typed arrays and its graph in Axiolid's wire format; the default entry
// refuses both with `feature-disabled`.

const FILE = `ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('smoke.ifc','2026-10-03T00:00:00',('a'),('o'),'p','s','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'Project',*,$,$,$,$,$);
#3=IFCPROPERTYSINGLEVALUE('Count',$,IFCINTEGER(9007199254740993),$);
#5=IFCWALLSTANDARDCASE('1YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,$,$,$,.STANDARD.);
ENDSEC;
END-ISO-10303-21;
`;

function check(condition, what) {
  if (!condition) throw new Error(`smoke check failed: ${what}`);
}

export async function smoke(IfcModel) {
  const model = IfcModel.parse(new TextEncoder().encode(FILE));
  check(model.schema === "IFC4", `schema ${model.schema}`);
  check(model.size === 3, `size ${model.size}`);

  // The bundled IFC4 schema answers the subtype query.
  const walls = model.idsOfTypeIncludingSubtypes("IfcWall");
  check(walls.length === 1 && walls[0] === 5n, `walls ${walls}`);

  // 64-bit integers cross as bigint, losslessly.
  const count = model.attribute(3n, 2);
  check(
    count.kind === "typed" && count.value.value === 9007199254740993n,
    `count ${JSON.stringify(count, (_, v) => (typeof v === "bigint" ? `${v}n` : v))}`,
  );

  // Attributes by name (#326), resolved against the declared IFC4.
  model.setAttributeByName(5n, "Name", { kind: "text", value: "Renamed" });
  const again = IfcModel.parse(model.write());
  const name = again.attributeByName(5n, "name");
  check(again.attributeNames(5n)[2].name === "Name", "attributeNames");
  check(name.kind === "text" && name.value === "Renamed", "edit survives write and re-parse");

  let code;
  try {
    model.attribute(99n, 0);
  } catch (error) {
    check(error.name === "IfcError", `error name ${error.name}`);
    code = error.code;
  }
  check(code === "missing-entity", `error code ${code}`);

  // The default capabilities (#244) are compiled into every packaged target.
  check(model.header().name === "smoke.ifc", "header");
  const report = model.validate();
  check(typeof report.conformant === "boolean" && Array.isArray(report.findings), "validate");
  const fromXml = IfcModel.parseIfcXml(model.writeIfcXml());
  check(fromXml.size === model.size, "ifcXML round trip");
  check(Array.isArray(model.unreachableProducts()), "unreachableProducts");
  const damaged = new TextEncoder().encode(FILE.replace("ENDSEC;\nEND", "#9=IFCWALL('x',,;\nENDSEC;\nEND"));
  const lenient = IfcModel.parseWithOptions(damaged, { onMalformed: "skip" });
  check(lenient.size === 3 && lenient.diagnostics().length === 1, "lenient read");

  // The domain views (#123) are compiled into every packaged target.
  check(model.propertySets(5n).length === 0, "propertySets");
  check(model.spatialTree().nodes.length === 1, "spatialTree");
  const note = { kind: "typed", type: "IFCLABEL", value: { kind: "text", value: "x" } };
  model.setProperty(5n, "Custom", "Note", note);
  const written = model.propertySets(5n)[0].properties[0].value;
  check(written.type === "IFCLABEL" && written.value.value === "x", "setProperty");

  // The catalog is loaded lazily (#318): a write to a Pset_ set before
  // loading its release's edition is refused, never written unchecked.
  const fireRating = { kind: "typed", type: "IFCLABEL", value: { kind: "text", value: "F90" } };
  check(IfcModel.catalogLoaded("IFC4") === false, "no catalog before loading");
  let refused;
  try {
    model.setProperty(5n, "Pset_WallCommon", "FireRating", fireRating);
  } catch (error) {
    refused = error;
  }
  check(refused?.code === "catalog-not-loaded", `refusal before loading: ${refused?.code}`);
  check(String(refused.message).includes("loadCatalog"), "the refusal names loadCatalog");
  await IfcModel.loadCatalog(model.schema);
  check(IfcModel.catalogLoaded("IFC4") && !IfcModel.catalogLoaded("IFC4X3"), "one edition loaded");
  model.setProperty(5n, "Pset_WallCommon", "FireRating", fireRating);
  const pset = model.propertySets(5n).find((set) => set.name === "Pset_WallCommon");
  check(pset?.properties[0].value.value.value === "F90", "Pset_ property written after loading");

  // Schema-checked creation (#330): a storey and a contained wall, built
  // from nothing, validate clean and survive ifcXML.
  const built = new IfcModel();
  built.setHeader({ ...built.header(), schema: ["IFC4"] });
  const h = IfcModel.handle;
  const { ids } = built.author([
    { op: "project", attributes: { Name: { kind: "text", value: "P" } } },
    { op: "placement" },
    { op: "spatial", type: "IfcSite", parent: h(0), placement: h(1) },
    { op: "spatial", type: "IfcBuildingStorey", parent: h(2) },
    { op: "typeObject", type: "IfcWallType", attributes: { PredefinedType: { kind: "enum", value: "STANDARD" } } },
    { op: "placement", relativeTo: h(1), location: [1, 2, 0] },
    { op: "product", type: "IfcWall", container: h(3), placement: h(5), typeObject: h(4) },
  ]);
  check(built.typeOf(ids[6]) === "IFCWALL", "author");
  const builtReport = built.validate();
  check(builtReport.errors + builtReport.evaluationErrors === 0, "authored model validates");
  check(IfcModel.parseIfcXml(built.writeIfcXml()).size === built.size, "authored ifcXML");
  let authorRefusal;
  try {
    built.author([{ op: "project" }]);
  } catch (error) {
    authorRefusal = error.code;
  }
  check(authorRefusal === "invalid-model", `a second project: ${authorRefusal}`);

  return { schema: model.schema, size: model.size, walls: walls.length, catalog: "IFC4" };
}

// One extruded wall, 4 m x 0.2 m x 2.8 m, in millimetres (#369).
const WALL = `ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [DesignTransferView]'),'2;1');
FILE_NAME('mesh.ifc','2026-10-08T00:00:00',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#2=IFCUNITASSIGNMENT((#1));
#3=IFCCARTESIANPOINT((0.,0.,0.));
#4=IFCAXIS2PLACEMENT3D(#3,$,$);
#5=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#4,$);
#6=IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#5,$,.MODEL_VIEW.,$);
#7=IFCPROJECT('2Bq7b3H0n0kQvNfOcJw1Sx',$,'Mesh',$,$,$,$,(#5),#2);
#8=IFCLOCALPLACEMENT($,#4);
#9=IFCCARTESIANPOINT((0.,0.));
#10=IFCAXIS2PLACEMENT2D(#9,$);
#11=IFCRECTANGLEPROFILEDEF(.AREA.,$,#10,4000.,200.);
#12=IFCDIRECTION((0.,0.,1.));
#13=IFCEXTRUDEDAREASOLID(#11,#4,#12,2800.);
#14=IFCSHAPEREPRESENTATION(#6,'Body','SweptSolid',(#13));
#15=IFCPRODUCTDEFINITIONSHAPE($,$,(#14));
#16=IFCWALL('2nR5uK8Lw3eT6yH1aJ9sD0',$,'Wall',$,$,#8,#15,$,.STANDARD.);
ENDSEC;
END-ISO-10303-21;
`;

export function meshes(IfcModel, enabled) {
  const model = IfcModel.parse(new TextEncoder().encode(WALL));
  if (!enabled) {
    let code;
    try {
      model.productMeshes();
    } catch (error) {
      code = error.code;
    }
    check(code === "feature-disabled", `productMeshes outside the mesh entry: ${code}`);
    let graphCode;
    try {
      model.productGeometry();
    } catch (error) {
      graphCode = error.code;
    }
    check(graphCode === "feature-disabled", `productGeometry outside the mesh entry: ${graphCode}`);
    return { productMeshes: code, productGeometry: graphCode };
  }
  // The neutral graph (#367): the mesh entry carries `graph` too.
  const [graph] = model.productGeometry(undefined, "object");
  check(graph.refusal === undefined, `graph refused: ${graph.refusal?.code}`);
  check(graph.payload.format === "axiolid-geometry-graph", "the wire format's name");
  check(graph.payload.version === "1.0", "wire format 1.0");
  const [cbor] = model.productGeometry([16n], "cbor");
  check(cbor.payload instanceof Uint8Array && cbor.payload.length > 0, "a CBOR payload");
  const [wall, ...rest] = model.productMeshes();
  check(rest.length === 0 && wall.id === 16n, "one mesh, the wall's");
  check(wall.refusal === undefined, `wall refused: ${wall.refusal?.code}`);
  check(wall.positions instanceof Float32Array, "positions are a Float32Array");
  check(wall.indices instanceof Uint32Array, "indices are a Uint32Array");
  check(wall.positions.length === wall.vertexCount * 3 && wall.vertexCount >= 8, "positions");
  check(wall.indices.length === wall.triangleCount * 3 && wall.triangleCount >= 12, "indices");
  check(wall.indices.every((index) => index < wall.vertexCount), "indices within the vertices");
  // A box from (-2, -0.1, 0) to (2, 0.1, 2.8), in metres.
  for (const [axis, low, high] of [[0, -2, 2], [1, -0.1, 0.1], [2, 0, 2.8]]) {
    const values = wall.positions.filter((_, i) => i % 3 === axis);
    check(
      Math.abs(Math.min(...values) - low) < 1e-6 && Math.abs(Math.max(...values) - high) < 1e-6,
      `extent along axis ${axis}`,
    );
  }
  return {
    vertices: wall.vertexCount,
    triangles: wall.triangleCount,
    graphNodes: graph.payload.graph.nodes.length,
  };
}
