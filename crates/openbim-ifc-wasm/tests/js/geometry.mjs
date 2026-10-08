// Geometry through the WASM binding (#328): placements in every build,
// meshes in a build with the `mesh` feature, each refusal typed per
// product.
//
// Usage: IFC_WASM_PKG=<package-dir> node --test geometry.mjs
//
// With IFC_WASM_MESH=1 the package must be a `--features mesh` build (the
// npm package's mesh entry, `<pkg>/mesh`, #369) and the mesh tests run,
// together with the browser example's scene code; without it,
// `productMeshes` must throw `feature-disabled`, as the default entry does.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const pkg = path.resolve(process.env.IFC_WASM_PKG ?? "pkg");
const { IfcModel } = createRequire(import.meta.url)(
  path.join(pkg, "openbim_ifc_wasm.js"),
);
const mesh = process.env.IFC_WASM_MESH === "1";

const here = path.dirname(fileURLToPath(import.meta.url));
const FIXTURE = path.resolve(
  here,
  "../../../../test/fixtures/synthetic-bindings/binding_geometry.ifc",
);
const bytes = readFileSync(FIXTURE);

function near(actual, expected, what) {
  assert.equal(actual.length, expected.length, what);
  actual.forEach((value, i) =>
    assert.ok(Math.abs(value - expected[i]) < 1e-6, `${what}[${i}]: ${actual} != ${expected}`),
  );
}

test("every product with a shape is placed and its Body selected", () => {
  const console = { log() {}, warn() {} };
  // docs:snippet js-geometry-placements
  const model = IfcModel.parse(bytes);
  for (const product of model.productPlacements()) {
    if (product.refusal) {
      console.warn(product.id, product.refusal.code, product.refusal.message);
      continue;
    }
    // A column-major 4x4 in metres: three.js reads it with Matrix4.fromArray.
    const [x, y, z] = product.transform.slice(12, 15);
    const body = product.representation; // undefined for an axis-only product
    console.log(product.typeName, [x, y, z], body?.identifier, body?.representationType);
  }
  // docs:end
  const placements = model.productPlacements();
  assert.deepEqual(
    placements.map((p) => p.id),
    [36n, 46n, 53n, 65n],
  );
  const [wall, , axisOnly] = placements;
  assert.equal(wall.typeName, "IFCWALL");
  assert.equal(wall.globalId, "2nR5uK8Lw3eT6yH1aJ9sD0");
  near(
    wall.transform,
    [0, 1, 0, 0, -1, 0, 0, 0, 0, 0, 1, 0, 512002, 5403001, 3, 1],
    "wall transform",
  );
  assert.deepEqual(wall.representation, {
    id: 30n,
    identifier: "Body",
    representationType: "SweptSolid",
    context: 7n,
    contextType: "Model",
    contextIdentifier: "Body",
    targetView: "MODEL_VIEW",
  });
  assert.equal(wall.refusal, undefined);
  assert.equal(axisOnly.representation, undefined, "an Axis is not a body");
  assert.equal(axisOnly.refusal, undefined);
});

test("a selection takes bigint ids, and a missing one is a typed refusal", () => {
  const model = IfcModel.parse(bytes);
  const [text, missing] = model.productPlacements([65n, 9999n]);
  assert.equal(text.representation.id, 63n);
  assert.equal(missing.refusal.code, "missing-reference");
  assert.deepEqual(
    model.productPlacements(new BigUint64Array([36n])).map((p) => p.id),
    [36n],
  );
  assert.throws(
    () => model.productPlacements([36]),
    (error) => error.name === "IfcError" && error.code === "invalid-value",
  );
});

test("a broken placement chain is one record, not a thrown call", () => {
  const model = IfcModel.parse(
    new TextEncoder().encode(`ISO-10303-21;
HEADER;FILE_DESCRIPTION((''),'2;1');FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;
DATA;
#1=IFCCARTESIANPOINT((0.,0.,0.));
#2=IFCAXIS2PLACEMENT3D(#1,$,$);
#3=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',());
#4=IFCPRODUCTDEFINITIONSHAPE($,$,(#3));
#20=IFCLOCALPLACEMENT(#21,#2);
#21=IFCLOCALPLACEMENT(#20,#2);
#22=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'cyclic',$,$,#20,#4,$,$);
ENDSEC;
END-ISO-10303-21;
`),
  );
  const [cyclic] = model.productPlacements();
  assert.equal(cyclic.transform, undefined);
  assert.equal(cyclic.refusal.code, "budget-exceeded");
  assert.equal(cyclic.refusal.entity, 20n);
});

if (!mesh) {
  test("meshes are opt-in: this build refuses them", () => {
    const model = IfcModel.parse(bytes);
    assert.throws(
      () => model.productMeshes(),
      (error) => error.name === "IfcError" && error.code === "feature-disabled",
    );
  });
} else {
  test("meshes come as typed arrays relative to each product", () => {
    const log = [];
    const console = { log: (...args) => log.push(args) };
    // docs:snippet js-geometry-meshes
    const model = IfcModel.parse(bytes);
    for (const mesh of model.productMeshes()) {
      if (mesh.refusal) {
        console.log(mesh.id, mesh.refusal.code); // e.g. 65n "unsupported"
        continue;
      }
      // positions: Float32Array, x y z per vertex in metres relative to
      // mesh.transform; indices: Uint32Array, three per triangle.
      console.log(mesh.typeName, mesh.positions.length / 3, mesh.indices.length / 3);
    }
    // docs:end
    assert.deepEqual(log, [
      ["IFCWALL", log[0][1], log[0][2]],
      ["IFCSLAB", log[1][1], log[1][2]],
      ["IFCBUILDINGELEMENTPROXY", 0, 0],
      [65n, "unsupported"],
    ]);
    const [wall] = model.productMeshes([36n]);
    assert.ok(wall.positions instanceof Float32Array);
    assert.ok(wall.indices instanceof Uint32Array);
    assert.equal(wall.vertexCount * 3, wall.positions.length);
    assert.equal(wall.triangleCount * 3, wall.indices.length);
    assert.ok(wall.triangleCount >= 12);
    const max = [0, 1, 2].map((axis) =>
      Math.max(...wall.positions.filter((_, i) => i % 3 === axis)),
    );
    near(max, [2, 0.1, 2.8], "wall extent, local metres");
    near(wall.transform, model.productPlacements([36n])[0].transform, "one placement");
  });

  test("the browser example's scene fits every mesh in view", async () => {
    const { buildScene } = await import("../../examples/viewer/scene.mjs");
    const scene = buildScene(IfcModel.parse(bytes).productMeshes());
    assert.equal(scene.drawn.length, 2, "wall and slab");
    assert.deepEqual(
      scene.refused.map((r) => [r.id, r.code]),
      [[65n, "unsupported"]],
    );
    // The scene origin absorbs the 5,403 km offset in f64; what reaches
    // the GPU is within metres of zero.
    for (const item of scene.drawn) {
      assert.ok(item.matrix instanceof Float32Array);
      assert.ok(Math.abs(item.matrix[12]) < 100 && Math.abs(item.matrix[13]) < 100);
    }
    assert.ok(scene.radius > 2 && scene.radius < 20, `radius ${scene.radius}`);
  });
}
