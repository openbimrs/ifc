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
    // Names resolve against the release the header declares.
    const name = model.attributeByName(wall, "Name"); // { kind: "text", value: "Wall" }
    model.setAttributeByName(wall, "Name", { kind: "text", value: `${name.value} (checked)` });
  }

  const out = model.write(); // a Uint8Array, ready to save
  // docs:end
  assert.equal(schema, "IFC4");
  assert.deepEqual(IfcModel.parse(out).attribute(5n, 2), {
    kind: "text",
    value: "Wall (checked)",
  });
});

// --- #326: attributes by name ----------------------------------------------

test("attributes resolve by name against the declared release", () => {
  const model = parse();
  const names = model.attributeNames(5n);
  assert.deepEqual(names[0], {
    name: "GlobalId",
    index: 0,
    typeName: "IfcGloballyUniqueId",
    optional: false,
    aggregate: false,
    derived: false,
    declaredBy: "IfcRoot",
  });
  assert.deepEqual(names.map((attribute) => attribute.name).slice(-2), ["Tag", "PredefinedType"]);
  assert.deepEqual(model.attributeByName(5n, "predefinedtype"), { kind: "enum", value: "STANDARD" });
  const previous = model.setAttributeByName(5n, "Name", { kind: "text", value: "Renamed" });
  assert.deepEqual(previous, { kind: "text", value: "Wall" });
  assert.deepEqual(model.attribute(5n, 2), { kind: "text", value: "Renamed" });

  // The IFC2X3 task keeps Status one slot earlier than IFC4's.
  const task = (schema, data) =>
    IfcModel.parse(new TextEncoder().encode(FILE.replace("'IFC4'", `'${schema}'`).replace(
      "#1=IFCPROJECT", `${data}\n#1=IFCPROJECT`,
    )));
  const ifc2x3 = task("IFC2X3", "#9=IFCTASK('3YvctVUKr0kugbFTf53O9L',$,'T',$,$,'T1','Planned','W',.F.,1);");
  const ifc4 = task("IFC4", "#9=IFCTASK('3YvctVUKr0kugbFTf53O9L',$,'T',$,$,'T1',$,'Planned','W',.F.,1,$,$);");
  for (const [model, slot] of [[ifc2x3, 6], [ifc4, 7]]) {
    assert.deepEqual(model.attributeByName(9n, "Status"), { kind: "text", value: "Planned" });
    assert.equal(model.attributeNames(9n).find((a) => a.name === "Status").index, slot);
  }

  const before = model.write();
  throwsCode(() => model.attributeByName(5n, "IsDefinedBy"), "unknown-attribute");
  throwsCode(() => model.setAttributeByName(5n, "Nmae", { kind: "null" }), "unknown-attribute");
  throwsCode(() => model.attributeNames(99n), "missing-entity");
  throwsCode(() => ifc4.attributeByName(9n, "TaskId"), "unknown-attribute");
  const unit = model.add("IfcSIUnit", [
    { kind: "derived" },
    { kind: "enum", value: "LENGTHUNIT" },
    { kind: "null" },
    { kind: "enum", value: "METRE" },
  ]);
  assert.equal(model.attributeNames(unit)[0].derived, true);
  const withUnit = model.write();
  throwsCode(() => model.setAttributeByName(unit, "Dimensions", { kind: "null" }), "derived-attribute");
  assert.deepEqual(model.write(), withUnit, "a refused write changes nothing");
  assert.notDeepEqual(before, withUnit);
  throwsCode(
    () => IfcModel.parse(new TextEncoder().encode(FILE.replace("'IFC4'", "'IFC9'")))
      .attributeByName(5n, "Name"),
    "unsupported-schema",
  );
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

test("property sets are written as one checked transaction", async () => {
  const model = openFixture("synthetic-properties/synthetic_properties.ifc");
  const unchecked = { object: 31n, set: "Pset_WallCommon", name: "FireRating", value: { kind: "typed", type: "IFCLABEL", value: { kind: "text", value: "F60" } } };
  // Before its release's catalog is loaded, a Pset_/Qto_ write is refused
  // (#318) and the model is unchanged; other sets need no catalog.
  const unloaded = model.write();
  assert.equal(IfcModel.catalogLoaded("IFC4"), false);
  assert.throws(
    () => model.setProperties([unchecked]),
    (error) => error.code === "catalog-not-loaded" && /loadCatalog/.test(error.message),
  );
  assert.deepEqual(model.write(), unloaded);
  // docs:snippet js-catalog-load
  // The module embeds no PSD/QTO catalog: load the release's edition once,
  // before the first write to a Pset_ or Qto_ set.
  await IfcModel.loadCatalog(model.schema); // reads catalog/ifc4-add2-tc1.bin
  // docs:end
  assert.equal(IfcModel.catalogLoaded("IFC4"), true);
  assert.equal(IfcModel.catalogLoaded("IFC4X3"), false, "only the edition asked for");
  // docs:snippet js-domain-write
  const label = (value) => ({ kind: "typed", type: "IFCLABEL", value: { kind: "text", value } });
  // Wall #31 inherits FireRating from its type: the write overrides it on
  // the wall and never changes the type's shared set.
  const result = model.setProperties([
    { object: 31n, set: "Pset_WallCommon", name: "FireRating", value: label("F60") },
    {
      object: 30n,
      set: "Qto_WallBaseQuantities",
      name: "Width",
      value: { kind: "typed", type: "IFCLENGTHMEASURE", value: { kind: "real", value: 250 } },
    },
    { object: 30n, set: "Pset_WallCommon", name: "IsExternal", remove: true },
  ]);
  // result.properties: per edit, the entity now holding the value
  const fireRating = model
    .propertySets(31n)
    .find((set) => set.source === "occurrence" && set.name === "Pset_WallCommon")
    .properties.find((p) => p.name === "FireRating");
  // docs:end
  assert.equal(result.properties.length, 3);
  assert.equal(result.properties[2], undefined);
  assert.deepEqual(fireRating.value, label("F60"));
  assert.deepEqual(model.attribute(35n, 2), label("F30"), "the type's value is unchanged");

  // The values read back identically through STEP and ifcXML.
  const values = (m) =>
    m.propertySets(31n).flatMap((set) => set.properties.map((p) => [set.source, set.name, p.name, p.value]));
  const expected = values(model);
  assert.deepEqual(values(IfcModel.parse(model.write())), expected);
  assert.deepEqual(values(IfcModel.parseIfcXml(model.writeIfcXml())), expected);

  // Single-edit wrappers.
  const id = model.setProperty(31n, "Custom", "Note", label("x"));
  assert.equal(model.typeOf(id), "IFCPROPERTYSINGLEVALUE");
  model.removeProperty(31n, "Custom", "Note");

  // Every refusal has its code and leaves the model unchanged.
  const before = model.write();
  const refuses = (edits, code) => {
    throwsCode(() => model.setProperties(edits), code);
    assert.deepEqual(model.write(), before, code);
  };
  refuses([{ object: 31n, set: "Pset_WallCommon", name: "FireRating", value: { kind: "typed", type: "IFCREAL", value: { kind: "real", value: 1 } } }], "template-violation");
  refuses([{ object: 31n, set: "Pset_WallCommon", name: "IsExternal", remove: true }], "missing-property");
  refuses([{ object: 31n, set: "Custom", name: "A", value: { kind: "real", value: 1 } }], "invalid-value");
  refuses(
    [
      { object: 31n, set: "Custom", name: "A", value: label("valid") },
      { object: 999n, set: "Custom", name: "A", value: label("x") },
    ],
    "missing-entity",
  );
  refuses([{ object: 31n, set: "Custom", name: "A" }], "invalid-value");
  const ifc4x1 = IfcModel.parse(new TextEncoder().encode(CLASSIFIED.replace("'IFC4'", "'IFC4X1'")));
  throwsCode(() => ifc4x1.setProperty(3n, "Custom", "A", label("x")), "unsupported-schema");
});

test("the catalog loads per release, from bytes or a base URL, checked against its pin", async () => {
  const catalog = path.join(pkg, "catalog");
  const files = ["IFC2X3", "IFC4", "IFC4X3"].map((release) => IfcModel.catalogFile(release));
  assert.deepEqual(files, ["ifc2x3-tc1.bin", "ifc4-add2-tc1.bin", "ifc4x3-add2.bin"]);
  assert.equal(IfcModel.catalogFile("IFC4X3_ADD2"), "ifc4x3-add2.bin");
  throwsCode(() => IfcModel.catalogFile("IFC4X1"), "unsupported-schema");
  await assert.rejects(IfcModel.loadCatalog("IFC4X1"), (error) => error.code === "unsupported-schema");

  // Another edition's bytes, or damaged ones, load nothing.
  const ifc2x3 = readFileSync(path.join(catalog, "ifc2x3-tc1.bin"));
  const ifc4x3 = readFileSync(path.join(catalog, "ifc4x3-add2.bin"));
  throwsCode(() => IfcModel.loadCatalogBytes("IFC2X3", ifc4x3), "invalid-value");
  const damaged = Uint8Array.from(ifc2x3);
  damaged[damaged.length - 1] ^= 1;
  await assert.rejects(
    IfcModel.loadCatalog("IFC2X3", { bytes: damaged }),
    (error) => error.code === "invalid-value",
  );
  assert.equal(IfcModel.catalogLoaded("IFC2X3"), false);

  // The caller's bytes, then a base URL: each edition once.
  await IfcModel.loadCatalog("IFC2X3", { bytes: ifc2x3 });
  assert.equal(IfcModel.catalogLoaded("IFC2X3"), true);
  await assert.rejects(
    IfcModel.loadCatalog("IFC4X3", { baseUrl: new URL("file:///nonexistent/") }),
    (error) => error.code === "io",
  );
  const base = new URL(`file://${catalog}`); // no trailing slash: still a directory
  await Promise.all([IfcModel.loadCatalog("IFC4X3", { baseUrl: base }), IfcModel.loadCatalog("IFC4X3_ADD2")]);
  assert.equal(IfcModel.catalogLoaded("IFC4X3"), true);
  // Every edition is loaded now; loading all again is a no-op.
  await IfcModel.loadCatalog();
});

// --- #330: schema-checked entity creation ---------------------------------

/** An empty model whose header names the release creation checks against. */
function emptyModel(schema = "IFC4") {
  const model = new IfcModel();
  model.setHeader({ ...model.header(), schema: [schema] });
  return model;
}

const text = (value) => ({ kind: "text", value });
const token = (value) => ({ kind: "enum", value });
const ref = (id) => ({ kind: "ref", id });
const reals = (...values) => ({ kind: "list", items: values.map((value) => ({ kind: "real", value })) });

test("documented example: a model built from nothing", () => {
  const model = emptyModel("IFC4");
  // docs:snippet js-authoring
  const h = IfcModel.handle; // h(i): the entity operation i of the batch produces
  const result = model.author([
    { op: "project", attributes: { Name: text("Demo") } }, // 0
    { op: "placement" }, // 1: at the origin
    { op: "spatial", type: "IfcSite", parent: h(0), placement: h(1) }, // 2
    { op: "spatial", type: "IfcBuilding", parent: h(2) }, // 3
    { op: "placement", relativeTo: h(1) }, // 4
    { op: "spatial", type: "IfcBuildingStorey", parent: h(3), placement: h(4) }, // 5
    { op: "typeObject", type: "IfcWallType", attributes: { PredefinedType: token("STANDARD") } }, // 6
    { op: "placement", relativeTo: h(4), location: [1, 2, 0] }, // 7
    {
      op: "product",
      type: "IfcWall",
      container: h(5), // IfcRelContainedInSpatialStructure
      placement: h(7),
      typeObject: h(6), // IfcRelDefinesByType
      attributes: { Name: text("Wall") },
    },
  ]);
  const wall = result.ids[8]; // a bigint; every IfcRoot got a GlobalId
  // docs:end
  assert.equal(model.typeOf(wall), "IFCWALL");
  assert.equal(model.attributeByName(wall, "GlobalId").value.length, 22);
  const report = model.validate();
  assert.equal(report.errors + report.evaluationErrors, 0, JSON.stringify(report.findings));
});

/** The host test's model: units, a context, project, site, building and
 * storey, a wall type, and a placed wall contained in the storey and typed. */
function building(model) {
  const h = IfcModel.handle;
  return model.author([
    { op: "create", type: "IfcSIUnit", attributes: { UnitType: token("LENGTHUNIT"), Name: token("METRE") } },
    { op: "create", type: "IfcUnitAssignment", attributes: { Units: { kind: "list", items: [ref(h(0))] } } },
    { op: "create", type: "IfcCartesianPoint", attributes: { Coordinates: reals(0, 0, 0) } },
    { op: "create", type: "IfcAxis2Placement3D", attributes: { Location: ref(h(2)) } },
    {
      op: "create",
      type: "IfcGeometricRepresentationContext",
      attributes: {
        ContextType: text("Model"),
        CoordinateSpaceDimension: { kind: "integer", value: 3n },
        Precision: { kind: "real", value: 1e-5 },
        WorldCoordinateSystem: ref(h(3)),
      },
    },
    {
      op: "project",
      attributes: {
        Name: text("Demo"),
        UnitsInContext: ref(h(1)),
        RepresentationContexts: { kind: "list", items: [ref(h(4))] },
      },
    },
    { op: "placement" },
    { op: "spatial", type: "IfcSite", parent: h(5), placement: h(6) },
    { op: "placement", relativeTo: h(6) },
    { op: "spatial", type: "IfcBuilding", parent: h(7), placement: h(8) },
    { op: "placement", relativeTo: h(8) },
    { op: "spatial", type: "IfcBuildingStorey", parent: h(9), placement: h(10), attributes: { Name: text("Level 0") } },
    { op: "typeObject", type: "IfcWallType", attributes: { PredefinedType: token("STANDARD") } },
    { op: "placement", relativeTo: h(10), location: [1, 2, 0], axis: [0, 0, 1], refDirection: [1, 0, 0] },
    { op: "product", type: "IfcWall", container: h(11), placement: h(13), typeObject: h(12), attributes: { Name: text("Wall") } },
  ]).ids;
}

test("a model built from nothing validates and round-trips through STEP and ifcXML", () => {
  const model = emptyModel();
  const ids = building(model);
  const wall = ids[14];
  // A property set through the #316 call; no catalog needed for a custom set.
  model.setProperty(wall, "ACME_WallData", "Mark", { kind: "typed", type: "IFCLABEL", value: text("W-01") });
  const report = model.validate();
  assert.equal(report.errors + report.evaluationErrors, 0, JSON.stringify(report.findings));

  const step = model.write();
  assert.deepEqual(IfcModel.parse(step).write(), step);
  for (const profile of [undefined, "IFC4"]) {
    const back = IfcModel.parseIfcXml(model.writeIfcXml(profile), profile);
    assert.deepEqual(back.write(), step, `ifcXML ${profile ?? "native"}`);
  }
});

test("a refused batch leaves the model byte-identical", () => {
  const model = emptyModel();
  const ids = building(model);
  const before = model.write();
  throwsCode(
    () =>
      model.author([
        { op: "product", type: "IfcWall", container: ids[11] },
        { op: "contain", structure: ids[11], elements: [ids[14]] },
      ]),
    "invalid-model",
  );
  assert.deepEqual(model.write(), before);
});

test("createEntity and removeWithRelationships are one-operation batches", () => {
  const model = emptyModel();
  const ids = building(model);
  const proxy = model.createEntity("IfcBuildingElementProxy", { Name: text("Proxy") });
  assert.equal(model.typeOf(proxy), "IFCBUILDINGELEMENTPROXY");
  throwsCode(() => model.removeWithRelationships(ids[10]), "still-referenced");
  model.removeWithRelationships(ids[14]);
  assert.deepEqual(model.idsOfType("IfcRelContainedInSpatialStructure"), []);
  assert.deepEqual(model.danglingReferences(), []);
});

test("authoring refusals carry the shared codes", () => {
  const model = emptyModel();
  const ids = building(model);
  const refuse = (ops, code) => {
    const before = model.write();
    throwsCode(() => model.author(ops), code);
    assert.deepEqual(model.write(), before, code);
  };
  refuse([{ op: "create", type: "IfcWal" }], "unsupported-schema");
  refuse([{ op: "create", type: "IfcElement" }], "wrong-entity-type");
  refuse([{ op: "create", type: "IfcWall", attributes: { Nmae: text("x") } }], "unknown-attribute");
  refuse([{ op: "create", type: "IfcWallType" }], "missing-attribute");
  refuse([{ op: "create", type: "IfcWall", attributes: { Name: { kind: "integer", value: 3n } } }], "invalid-value");
  refuse([{ op: "create", type: "IfcWall", attributes: { ObjectPlacement: ref(99999n) } }], "missing-reference");
  refuse([{ op: "edit", entity: 99999n, attributes: {} }], "missing-entity");
  refuse([{ op: "edit", entity: ids[0], attributes: { Dimensions: { kind: "null" } } }], "derived-attribute");
  refuse([{ op: "project" }], "invalid-model");
  refuse([{ op: "remove", entity: ids[10] }], "still-referenced");
  // JavaScript-side conversion: the operation and its fields.
  refuse([{ op: "build" }], "invalid-value");
  refuse([{ op: "create", type: "IfcWall", colour: "red" }], "invalid-value");
  refuse([{ op: "placement", axis: [0, 0, 1] }], "invalid-value");
  refuse([{ op: "placement", location: [1, 2] }], "invalid-value");
  refuse([{ op: "contain", structure: ids[11], elements: "all" }], "invalid-value");
  refuse("not an array", "invalid-value");
  // A handle names an earlier operation.
  refuse([{ op: "create", type: "IfcWall", attributes: { ObjectPlacement: ref(IfcModel.handle(0)) } }], "invalid-value");
  // An IFC2X3 project needs an owner history.
  throwsCode(() => emptyModel("IFC2X3").author([{ op: "project" }]), "missing-attribute");
  throwsCode(() => new IfcModel().author([{ op: "project" }]), "unsupported-schema");
});
