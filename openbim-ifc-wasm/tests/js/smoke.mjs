// Round-trip smoke test for the openbim-ifc-wasm Node package (#34).
//
// Proves the binding is not just a build artifact: parse a small IFC file,
// read entities, edit one, add one, write the file, parse it back, and check
// every error path throws an `IfcError` with a stable `code`.
//
// Usage: IFC_WASM_PKG=<package-dir> node --test smoke.mjs
import assert from "node:assert/strict";
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
