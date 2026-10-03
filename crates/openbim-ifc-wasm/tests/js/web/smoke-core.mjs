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

  return { schema: model.schema, size: model.size, walls: walls.length, catalog: "IFC4" };
}
