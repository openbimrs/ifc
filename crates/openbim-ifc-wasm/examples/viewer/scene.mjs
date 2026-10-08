// What the viewer draws, from `IfcModel.productMeshes()` (#328). Plain
// ECMAScript with no DOM or WebGL, so tests/js/geometry.mjs runs it in Node.
//
// Precision: each mesh's positions are f32 relative to its product, and
// its transform is f64. A georeferenced site sits kilometres from the
// origin, beyond what an f32 matrix keeps to the millimetre, so the scene
// subtracts one origin -- the first drawn product's translation -- from
// every translation in f64 before narrowing the matrices to f32.

/**
 * @param {import("./pkg/openbim_ifc_wasm.js").ProductMesh[]} meshes
 * @returns {{
 *   drawn: { id: bigint, typeName: string, positions: Float32Array,
 *            indices: Uint32Array, matrix: Float32Array }[],
 *   refused: { id: bigint, code: string, message: string }[],
 *   origin: number[], center: number[], radius: number,
 * }}
 */
export function buildScene(meshes) {
  const drawn = [];
  const refused = [];
  let origin;
  const min = [Infinity, Infinity, Infinity];
  const max = [-Infinity, -Infinity, -Infinity];
  for (const mesh of meshes) {
    if (mesh.refusal) {
      refused.push({ id: mesh.id, code: mesh.refusal.code, message: mesh.refusal.message });
      continue;
    }
    if (mesh.indices.length === 0) continue; // no Body: nothing to draw
    origin ??= mesh.transform.slice(12, 15);
    const matrix = Float64Array.from(mesh.transform);
    for (let axis = 0; axis < 3; axis++) matrix[12 + axis] -= origin[axis];
    const p = mesh.positions;
    for (let i = 0; i < p.length; i += 3) {
      for (let axis = 0; axis < 3; axis++) {
        const world =
          matrix[axis] * p[i] + matrix[4 + axis] * p[i + 1] + matrix[8 + axis] * p[i + 2] + matrix[12 + axis];
        min[axis] = Math.min(min[axis], world);
        max[axis] = Math.max(max[axis], world);
      }
    }
    drawn.push({
      id: mesh.id,
      typeName: mesh.typeName,
      positions: mesh.positions,
      indices: mesh.indices,
      matrix: Float32Array.from(matrix),
    });
  }
  if (drawn.length === 0) {
    return { drawn, refused, origin: [0, 0, 0], center: [0, 0, 0], radius: 1 };
  }
  const center = min.map((low, axis) => (low + max[axis]) / 2);
  const radius = Math.hypot(...max.map((high, axis) => high - min[axis])) / 2 || 1;
  return { drawn, refused, origin, center, radius };
}

/** A stable colour per entity type, for telling walls from slabs. */
export function colourOf(typeName) {
  let hash = 0;
  for (const c of typeName) hash = (hash * 31 + c.charCodeAt(0)) >>> 0;
  const hue = (hash % 360) / 360;
  const f = (n) => {
    const k = (n + hue * 12) % 12;
    return 0.6 - 0.35 * Math.max(-1, Math.min(k - 3, 9 - k, 1));
  };
  return [f(0), f(8), f(4)];
}
