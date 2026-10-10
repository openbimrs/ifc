// Geometry through the WASM binding (#328, #367): placements in every
// build, graphs and meshes in a build with the `graph` and `mesh` features,
// each refusal typed per product.
//
// Usage: IFC_WASM_PKG=<package-dir> node --test geometry.mjs
//
// With IFC_WASM_MESH=1 the package must be a `--features mesh,graph` build
// (the npm package's mesh entry, `<pkg>/mesh`, #369, #367) and the graph and
// mesh tests run, together with the browser example's scene code; without
// it, `productGeometry` and `productMeshes` must throw `feature-disabled`,
// as the default entry does.
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

/**
 * Decode one CBOR item (RFC 8949) of the kinds the geometry wire format
 * writes: integers, text, arrays, maps, floats of every width, null and
 * booleans. Test-only, so the CBOR payload can be compared with the JSON.
 */
function decodeCbor(bytes) {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let at = 0;
  const length = (info) => {
    if (info < 24) return info;
    const [width, read] = {
      24: [1, () => view.getUint8(at)],
      25: [2, () => view.getUint16(at)],
      26: [4, () => view.getUint32(at)],
      27: [8, () => Number(view.getBigUint64(at))],
    }[info];
    const value = read();
    at += width;
    return value;
  };
  const half = (bits) => {
    const exponent = (bits >> 10) & 0x1f;
    const fraction = bits & 0x3ff;
    const sign = bits & 0x8000 ? -1 : 1;
    if (exponent === 0) return sign * 2 ** -14 * (fraction / 1024);
    if (exponent === 31) return fraction ? NaN : sign * Infinity;
    return sign * 2 ** (exponent - 15) * (1 + fraction / 1024);
  };
  const item = () => {
    const head = view.getUint8(at++);
    const major = head >> 5;
    const info = head & 0x1f;
    if (major === 7) {
      if (info === 20) return false;
      if (info === 21) return true;
      if (info === 22) return null;
      const [width, read] = {
        25: [2, () => half(view.getUint16(at))],
        26: [4, () => view.getFloat32(at)],
        27: [8, () => view.getFloat64(at)],
      }[info];
      const value = read();
      at += width;
      return value;
    }
    const n = length(info);
    if (major === 0) return n;
    if (major === 1) return -1 - n;
    if (major === 3) {
      const text = new TextDecoder().decode(bytes.subarray(at, at + n));
      at += n;
      return text;
    }
    if (major === 4) return Array.from({ length: n }, item);
    if (major === 5) {
      const map = {};
      for (let i = 0; i < n; i++) map[item()] = item();
      return map;
    }
    throw new Error(`CBOR major type ${major} is not in the wire format`);
  };
  const value = item();
  assert.equal(at, bytes.length, "one item, no trailing bytes");
  return value;
}

if (!mesh) {
  test("graphs are opt-in: this build refuses them", () => {
    const model = IfcModel.parse(bytes);
    assert.throws(
      () => model.productGeometry(),
      (error) => error.name === "IfcError" && error.code === "feature-disabled",
    );
  });

  test("meshes are opt-in: this build refuses them", () => {
    const model = IfcModel.parse(bytes);
    assert.throws(
      () => model.productMeshes(),
      (error) => error.name === "IfcError" && error.code === "feature-disabled",
    );
  });
} else {
  test("graphs come as Axiolid's wire format, per product", () => {
    const log = [];
    const console = { log: (...args) => log.push(args) };
    // docs:snippet js-geometry-graphs
    const model = IfcModel.parse(bytes);
    for (const product of model.productGeometry(undefined, "object")) {
      if (product.refusal) {
        console.log(product.id, product.refusal.code); // e.g. 65n "unsupported"
        continue;
      }
      if (!product.payload) continue; // no Body: an axis-only product
      // { format: "axiolid-geometry-graph", version: "1.0", graph }, exact,
      // in world coordinates (metres): hand it to your own kernel.
      const { format, version, graph } = product.payload;
      console.log(product.typeName, format, version, Object.keys(graph.nodes.at(-1))[0]);
    }
    // docs:end
    assert.deepEqual(log, [
      ["IFCWALL", "axiolid-geometry-graph", "1.0", "Instance"],
      ["IFCSLAB", "axiolid-geometry-graph", "1.0", "Instance"],
      [65n, "unsupported"],
    ]);

    const graphs = model.productGeometry();
    assert.deepEqual(
      graphs.map((g) => [g.id, g.encoding, typeof g.payload]),
      [
        [36n, "json", "string"],
        [46n, "json", "string"],
        [53n, "json", "undefined"],
        [65n, "json", "undefined"],
      ],
    );
    const [wall, , axisOnly, text] = graphs;
    assert.equal(wall.payloadSize, new TextEncoder().encode(wall.payload).length);
    const envelope = JSON.parse(wall.payload);
    assert.equal(envelope.format, "axiolid-geometry-graph");
    assert.equal(envelope.version, "1.0");
    assert.deepEqual(envelope.graph.roots, [envelope.graph.nodes.length - 1]);
    // The extrusion: 4 m by 0.2 m, 2.8 m deep, placed 5,403 km out in f64.
    const [, extrusion, instance] = envelope.graph.nodes;
    assert.equal(extrusion.SolidOperation.Extrusion.profile, 0);
    assert.ok(Math.abs(extrusion.SolidOperation.Extrusion.depth - 2.8) < 1e-12);
    assert.deepEqual(instance.Instance.transform.slice(9), [512002, 5403001, 3]);
    near(wall.transform, model.productPlacements([36n])[0].transform, "one placement");
    assert.equal(axisOnly.refusal, undefined, "no Body is not a failure");
    assert.equal(axisOnly.payloadSize, 0);
    assert.equal(text.refusal.code, "unsupported");
    assert.equal(text.refusal.entity, 62n);
    assert.equal(text.transform, undefined);
  });

  test("the CBOR payload decodes to the JSON payload's envelope", () => {
    const model = IfcModel.parse(bytes);
    const json = model.productGeometry([36n, 46n]);
    const cbor = model.productGeometry(new BigUint64Array([36n, 46n]), "cbor");
    for (const [i, record] of cbor.entries()) {
      assert.equal(record.encoding, "cbor");
      assert.ok(record.payload instanceof Uint8Array);
      assert.equal(record.payloadSize, record.payload.length);
      assert.ok(record.payload.length < json[i].payloadSize, "CBOR is smaller");
      assert.deepEqual(decodeCbor(record.payload), JSON.parse(json[i].payload));
    }
    assert.throws(
      () => model.productGeometry(undefined, "xml"),
      (error) => error.name === "IfcError" && error.code === "invalid-value",
    );
  });

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
