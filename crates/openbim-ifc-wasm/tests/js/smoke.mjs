// Round-trip smoke test for the openbim-ifc-wasm Node package (#34).
//
// Proves the binding is not just a build artifact: parse a small IFC file,
// read entities, edit one, add one, write the file, parse it back, and check
// every error path throws an `IfcError` with a stable `code`.
//
// Usage: IFC_WASM_PKG=<package-dir> node --test smoke.mjs
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import test from "node:test";

const pkg = path.resolve(process.env.IFC_WASM_PKG ?? "pkg");
const { IfcModel } = createRequire(import.meta.url)(
  path.join(pkg, "openbim_ifc_wasm.js"),
);

const FILE = `ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('smoke.ifc','2026-09-23T00:00:00',('a'),('o'),'p','s','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'Project',*,$,$,$,$,$);
#2=IFCPROPERTYSINGLEVALUE('Flags',$,IFCLOGICAL(.U.),$);
#3=IFCPROPERTYSINGLEVALUE('Count',$,IFCINTEGER(9007199254740993),$);
#5=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,$,$,$,.STANDARD.);
#6=IFCRELDEFINESBYPROPERTIES('2YvctVUKr0kugbFTf53O9L',$,$,$,(#5),#1);
ENDSEC;
END-ISO-10303-21;
`;

const bytes = new TextEncoder().encode(FILE);
const parse = () => IfcModel.parse(bytes);

/** `fn` must throw an IfcError with this code. */
function throwsCode(fn, code) {
  assert.throws(fn, (error) => {
    assert.equal(error.name, "IfcError");
    assert.equal(error.code, code, error.message);
    return true;
  });
}

test("parse reads the header and every entity in file order", () => {
  const model = parse();
  assert.equal(model.schema, "IFC4");
  assert.equal(model.size, 5);
  assert.deepEqual(model.ids(), [1n, 2n, 3n, 5n, 6n]);
  assert.deepEqual(model.diagnostics(), []);
  assert.equal(model.typeOf(5n), "IFCWALL");
  assert.deepEqual(model.idsOfType("ifcWall"), [5n]);
  assert.deepEqual(model.idsOfTypeIncludingSubtypes("IfcBuildingElement"), [5n]);
  assert.deepEqual(model.idsOfType("IfcBuildingElement"), [], "exact query");
});

test("values keep the distinctions a naive binding would lose", () => {
  const model = parse();
  assert.deepEqual(model.attribute(1n, 1), { kind: "null" });
  assert.deepEqual(model.attribute(1n, 3), { kind: "derived" });
  assert.deepEqual(model.attribute(2n, 2), {
    kind: "typed",
    type: "IFCLOGICAL",
    value: { kind: "unknown" },
  });
  // 2^53 + 1: a JS number would round this to 2^53.
  assert.deepEqual(model.attribute(3n, 2), {
    kind: "typed",
    type: "IFCINTEGER",
    value: { kind: "integer", value: 9007199254740993n },
  });
  assert.deepEqual(model.attribute(5n, 8), { kind: "enum", value: "STANDARD" });
  assert.deepEqual(model.attribute(6n, 4), {
    kind: "list",
    items: [{ kind: "ref", id: 5n }],
  });
});

test("an edit and an added entity survive write and re-parse", () => {
  const model = parse();
  const previous = model.setAttribute(5n, 2, { kind: "text", value: "Renamed" });
  assert.deepEqual(previous, { kind: "text", value: "Wall" });
  const point = model.add("IfcCartesianPoint", [
    { kind: "list", items: [{ kind: "real", value: 1.5 }, { kind: "real", value: 0 }] },
  ]);
  assert.equal(point, 7n);

  const reparsed = IfcModel.parse(model.write());
  assert.deepEqual(reparsed.attribute(5n, 2), { kind: "text", value: "Renamed" });
  assert.equal(reparsed.typeOf(point), "IFCCARTESIANPOINT");
  assert.deepEqual(reparsed.attribute(point, 0), {
    kind: "list",
    items: [{ kind: "real", value: 1.5 }, { kind: "real", value: 0 }],
  });
  // Untouched values round-trip exactly, including the 64-bit integer.
  for (const id of model.ids()) {
    if (id === 5n) continue;
    assert.deepEqual(reparsed.attributes(id), model.attributes(id), `#${id}`);
  }
});

test("removing an entity leaves reported dangling references", () => {
  const model = parse();
  model.remove(5n);
  assert.deepEqual(model.danglingReferences(), [[6n, 5n]]);
});

test("every failure is an IfcError with a stable code", () => {
  const model = parse();
  throwsCode(() => IfcModel.parse(new TextEncoder().encode("nope")), "parse");
  throwsCode(() => model.typeOf(99n), "missing-entity");
  throwsCode(
    () => IfcModel.parse(new TextEncoder().encode(FILE.replace("'IFC4'", "'IFC9'")))
      .idsOfTypeIncludingSubtypes("IfcWall"),
    "unsupported-schema",
  );
  throwsCode(() => model.setAttribute(5n, 0, { kind: "bogus" }), "invalid-value");
  throwsCode(() => model.setAttribute(5n, 0, { kind: "real", value: NaN }), "invalid-value");
  throwsCode(() => model.setAttribute(5n, 0, { kind: "enum", value: "A B" }), "invalid-value");
  throwsCode(() => model.setAttribute(5n, 0, "plain string"), "invalid-value");
  throwsCode(() => model.add("Ifc Wall", []), "invalid-value");
  throwsCode(
    () => model.setAttribute(5n, 0, { kind: "integer", value: 2n ** 64n }),
    "out-of-range",
  );
  // An integral number beyond 2^53 may already be rounded, so it is refused.
  throwsCode(
    () => model.setAttribute(5n, 0, { kind: "integer", value: 2 ** 60 }),
    "invalid-value",
  );
  // A refused edit leaves the model unchanged.
  assert.deepEqual(model.attribute(5n, 0), {
    kind: "text",
    value: "1YvctVUKr0kugbFTf53O9L",
  });
});

test("an empty model can be built from scratch and written", () => {
  const model = new IfcModel();
  const id = model.add("IFCCARTESIANPOINT", [
    { kind: "list", items: [{ kind: "real", value: 0 }] },
  ]);
  assert.equal(id, 1n);
  const text = new TextDecoder().decode(model.write());
  assert.match(text, /#1=IFCCARTESIANPOINT\(\(0\.\)\);/);
});

test("documented example: read, edit and write a file", () => {
  // docs:snippet js-read-edit-write
  const model = IfcModel.parse(bytes); // bytes: the .ifc file as a Uint8Array
  const schema = model.schema; // "IFC4"

  for (const wall of model.idsOfType("IfcWall")) {
    const name = model.attribute(wall, 2); // { kind: "text", value: "Wall" }
    model.setAttribute(wall, 2, { kind: "text", value: `${name.value} (checked)` });
  }

  const out = model.write(); // a Uint8Array, ready to save
  // docs:end
  assert.equal(schema, "IFC4");
  assert.deepEqual(IfcModel.parse(out).attribute(5n, 2), {
    kind: "text",
    value: "Wall (checked)",
  });
});

// --- #244: lenient reads, header, validation, ifcXML, reachability --------

const DAMAGED = FILE.replace(
  "#6=IFCRELDEFINESBYPROPERTIES",
  "#4=IFCWALL('x',,;\n#6=IFCRELDEFINESBYPROPERTIES",
);
const damagedBytes = new TextEncoder().encode(DAMAGED);

test("a lenient read skips a damaged record and reports it", () => {
  throwsCode(() => IfcModel.parse(damagedBytes), "parse");
  throwsCode(() => IfcModel.parseWithOptions(damagedBytes, {}), "parse");
  const model = IfcModel.parseWithOptions(damagedBytes, { onMalformed: "skip" });
  assert.deepEqual(model.ids(), [1n, 2n, 3n, 5n, 6n]);
  assert.equal(model.diagnostics().length, 1);
  throwsCode(
    () => IfcModel.parseWithOptions(damagedBytes, { onMalformed: "drop" }),
    "invalid-value",
  );
  throwsCode(
    () => IfcModel.parseWithOptions(damagedBytes, { checkReferences: 1 }),
    "invalid-value",
  );
});

test("the header reads every field and a replacement is written", () => {
  const model = parse();
  const header = model.header();
  assert.deepEqual(header, {
    description: ["ViewDefinition [CoordinationView]"],
    implementationLevel: "2;1",
    name: "smoke.ifc",
    timeStamp: "2026-09-23T00:00:00",
    author: ["a"],
    organization: ["o"],
    preprocessorVersion: "p",
    originatingSystem: "s",
    authorization: "",
    schema: ["IFC4"],
  });
  model.setHeader({ ...header, name: "edited.ifc", author: ["Zoë"] });
  const reparsed = IfcModel.parse(model.write());
  assert.equal(reparsed.header().name, "edited.ifc");
  assert.deepEqual(reparsed.header().author, ["Zoë"]);
  throwsCode(() => model.setHeader({ ...header, name: 3 }), "invalid-value");
  throwsCode(() => model.setHeader({ name: "x" }), "invalid-value");
  assert.equal(model.header().name, "edited.ifc", "a refused header changes nothing");
});

test("validation returns structured findings", () => {
  const model = parse();
  model.remove(1n); // #6 now references a missing #1
  const report = model.validate();
  assert.equal(report.conformant, false);
  assert.equal(report.truncated, false);
  const finding = report.findings.find(
    (f) => f.severity === "error" && f.entity === 6n,
  );
  assert.ok(finding, JSON.stringify(report.findings.map((f) => f.path)));
  assert.equal(typeof finding.rule, "string");
  assert.equal(typeof finding.message, "string");
  assert.equal(report.errors, report.findings.filter((f) => f.severity === "error").length);
  const capped = model.validate(1);
  assert.equal(capped.findings.length, 1);
  assert.equal(capped.truncated, true);
  throwsCode(
    () => IfcModel.parse(new TextEncoder().encode(FILE.replace("'IFC4'", "'IFC9'"))).validate(),
    "unsupported-schema",
  );
});

test("ifcXML round-trips in both layouts", () => {
  const model = parse();
  const native = IfcModel.parseIfcXml(model.writeIfcXml());
  assert.deepEqual(native.ids(), model.ids());
  for (const id of model.ids()) {
    assert.deepEqual(native.attributes(id), model.attributes(id), `#${id}`);
  }
  // The XSD layout refuses what it cannot carry: `*` in IfcProject.Description.
  throwsCode(() => model.writeIfcXml("IFC4"), "write");
  const person = IfcModel.parse(
    new TextEncoder().encode(
      FILE.replace(/DATA;[\s\S]*ENDSEC;\nEND/, "DATA;\n#1=IFCPERSON($,'Doe',$,$,$,$,$,$);\nENDSEC;\nEND"),
    ),
  );
  const xsd = person.writeIfcXml("IFC4");
  assert.match(new TextDecoder().decode(xsd), /IFC4\/ADD2_TC1\/XML/);
  const back = IfcModel.parseIfcXml(xsd, "IFC4");
  assert.deepEqual(back.attributes(1n), person.attributes(1n));
  throwsCode(() => model.writeIfcXml("IFC2X3"), "unsupported-profile");
  throwsCode(() => IfcModel.parseIfcXml(new TextEncoder().encode("<a><b></a>")), "parse");
});

test("unreachable products carry a stable reason", () => {
  const file = FILE.replace(
    "#5=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,$,$,$,.STANDARD.);",
    "#10=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,$,$);\n" +
      "#11=IFCSHAPEREPRESENTATION(#10,'Body','SweptSolid',());\n" +
      "#12=IFCPRODUCTDEFINITIONSHAPE($,$,(#11));\n" +
      "#5=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,$,#12,$,.STANDARD.);",
  );
  const model = IfcModel.parse(new TextEncoder().encode(file));
  const [product, ...rest] = model.unreachableProducts();
  assert.equal(rest.length, 0);
  assert.equal(product.id, 5n);
  assert.equal(product.reason, "not-contained-in-spatial-structure");
  assert.deepEqual(product.foundViews, []);
  assert.equal(typeof product.message, "string");
});

test("documented example: lenient read, header, validation and ifcXML", () => {
  const bytes = damagedBytes;
  // docs:snippet js-beyond-records
  // A damaged export: skip what cannot be read, and say what was skipped.
  const model = IfcModel.parseWithOptions(bytes, { onMalformed: "skip" });
  const skipped = model.diagnostics(); // one message per dropped record

  const header = model.header(); // { name, timeStamp, author, schema, ... }
  model.setHeader({ ...header, author: ["Reviewer"] });

  const report = model.validate(); // { conformant, errors, findings, ... }
  const errors = report.findings.filter((f) => f.severity === "error");
  // each finding: { rule, entity, attributeName, path, message, ... }

  const xml = model.writeIfcXml(); // lossless ifcXML; or writeIfcXml("IFC4")
  const fromXml = IfcModel.parseIfcXml(xml);
  // docs:end
  assert.equal(skipped.length, 1);
  assert.deepEqual(fromXml.header().author, ["Reviewer"]);
  assert.equal(errors.length, report.errors);
  assert.deepEqual(fromXml.ids(), model.ids());
});

// --- Domain views (#123) ----------------------------------------------------

const fixtures = path.resolve(
  path.dirname(new URL(import.meta.url).pathname),
  "../../../../test/fixtures",
);
const openFixture = (name) =>
  IfcModel.parse(readFileSync(path.join(fixtures, name)));

/** A wall classified directly and through its type, with a layered type. */
const CLASSIFIED = `ISO-10303-21;
HEADER;FILE_DESCRIPTION((''),'2;1');FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;
DATA;
#2=IFCWALLTYPE('1YvctVUKr0kugbFTf53O9L',$,'WT',$,$,(#30),$,$,$,.SOLIDWALL.);
#3=IFCWALL('2YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,$,$,$,.STANDARD.);
#30=IFCPROPERTYSET('3ZvctVUKr0kugbFTf53O9L',$,'Pset_WallCommon',$,(#31));
#31=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$);
#4=IFCRELDEFINESBYTYPE('3YvctVUKr0kugbFTf53O9L',$,$,$,(#3),#2);
#10=IFCCLASSIFICATION('CSI',$,$,'Uniclass 2015',$,$,$);
#12=IFCCLASSIFICATIONREFERENCE($,'Ss_25_10','Wall systems',#10,$,$);
#13=IFCRELASSOCIATESCLASSIFICATION('0ZvctVUKr0kugbFTf53O9L',$,$,$,(#3),#12);
#20=IFCMATERIAL('Concrete',$,$);
#22=IFCMATERIALLAYER(#20,0.2,.U.,'Core',$,$,$);
#24=IFCMATERIALLAYERSET((#22),'WT-200',$);
#26=IFCRELASSOCIATESMATERIAL('2ZvctVUKr0kugbFTf53O9L',$,$,$,(#2),#24);
ENDSEC;
END-ISO-10303-21;
`;

test("property sets resolve type inheritance with typed values", () => {
  const model = openFixture("synthetic-properties/synthetic_properties.ifc");
  const sets = model.propertySets(30n);
  assert.deepEqual(
    sets.map((set) => [set.name, set.source, set.sourceId]),
    [
      ["Pset_WallCommon", "occurrence", undefined],
      ["Qto_WallBaseQuantities", "occurrence", undefined],
      ["Pset_WallCommon", "type", 29n],
    ],
  );
  assert.equal(sets[0].globalId, "0Mz9fPfwfBYBYTYsJjiw_L");
  assert.deepEqual(sets[0].properties[0].value, {
    kind: "typed",
    type: "IFCBOOLEAN",
    value: { kind: "bool", value: false },
  });
  const width = sets[1].properties[0];
  assert.equal(width.typeName, "IFCQUANTITYLENGTH");
  assert.equal(width.unit, 7n);
  assert.deepEqual(width.value, {
    kind: "typed",
    type: "IFCLENGTHMEASURE",
    value: { kind: "real", value: 200 },
  });
  const unit = model.resolveUnit("IFCLENGTHMEASURE", width.unit);
  assert.equal(unit.fromProject, false);
  assert.ok(Math.abs(unit.scale - 0.001) < 1e-15);
  assert.equal(model.resolveUnit("IFCLENGTHMEASURE").fromProject, true);
});

test("the spatial tree, systems, cost and georeferencing cross as records", () => {
  const tree = openFixture("synthetic-properties/synthetic_properties.ifc").spatialTree();
  assert.deepEqual(
    tree.nodes.map((node) => [node.id, node.kind]),
    [[22n, "project"], [23n, "site"], [24n, "building"], [25n, "storey"]],
  );
  assert.deepEqual(tree.nodes[3].elements, [30n, 31n]);

  const { systems } = openFixture("synthetic-systems/synthetic_systems.ifc").systems();
  const heating = systems.find((system) => system.id === 14n);
  assert.equal(heating.predefinedType, "HEATING");
  assert.deepEqual(heating.servicedBuildings, [12n]);

  const cost = openFixture("synthetic-cost-schedule/synthetic_cost_schedule.ifc").cost();
  assert.deepEqual(cost.schedules[0].items, [38n, 42n, 44n]);
  const setup = cost.items.find((item) => item.id === 44n).values[0];
  assert.equal(setup.operator, "ADD");
  assert.deepEqual(setup.components[0].appliedValue, {
    kind: "typed",
    type: "IFCMONETARYMEASURE",
    value: { kind: "real", value: 320 },
  });

  const [map] = openFixture("synthetic-surfaces/synthetic_conic_offset_bounded.ifc").georeferencing();
  assert.equal(map.kind, "map-conversion");
  assert.equal(map.targetCrs.name, "EPSG:25832");
  assert.deepEqual(map.translation, [1, 2, 0.01]);
});

test("classification and material read through the type object", () => {
  const bytes = new TextEncoder().encode(CLASSIFIED);
  const log = [];
  const console = { log: (...args) => log.push(args) };
  // docs:snippet js-domain-views
  const model = IfcModel.parse(bytes);
  const [wall] = model.idsOfType("IfcWall");

  // Property sets: the wall's own first, then its type's; values typed.
  for (const set of model.propertySets(wall)) {
    for (const p of set.properties) console.log(set.name, p.name, p.value);
  }
  const classes = model.classifications(wall); // [{ identification, system, ... }]
  const material = model.material(wall); // { kind: "layer-set", layers, ... }
  const tree = model.spatialTree(); // { nodes: [{ kind: "storey", elements }] }
  // docs:end
  assert.equal(classes[0].identification, "Ss_25_10");
  assert.equal(classes[0].system.name, "Uniclass 2015");
  assert.equal(material.kind, "layer-set");
  assert.equal(material.source, "type");
  assert.deepEqual(material.layers[0].isVentilated, { kind: "unknown" });
  assert.equal(material.layers[0].material.name, "Concrete");
  assert.equal(tree.nodes.length, 0);
  assert.deepEqual(log, [
    ["Pset_WallCommon", "IsExternal", { kind: "typed", type: "IFCBOOLEAN", value: { kind: "bool", value: true } }],
  ]);
});

test("domain refusals carry the shared codes", () => {
  const model = IfcModel.parse(new TextEncoder().encode(CLASSIFIED));
  throwsCode(() => model.propertySets(999n), "missing-entity");
  const ifc2x3 = IfcModel.parse(
    new TextEncoder().encode(CLASSIFIED.replace("'IFC4'", "'IFC2X3'")),
  );
  throwsCode(() => ifc2x3.georeferencing(), "unsupported-schema");
  const ifc4x1 = IfcModel.parse(
    new TextEncoder().encode(CLASSIFIED.replace("'IFC4'", "'IFC4X1'")),
  );
  throwsCode(() => ifc4x1.propertySets(3n), "unsupported-schema");
});
