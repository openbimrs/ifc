// A minimal WebGL2 viewer for `IfcModel.productMeshes()` (#328): no
// framework, no build step. ./build.sh builds the mesh-enabled module this
// imports from ./pkg; scene.mjs decides what is drawn and where.
import init, { IfcModel } from "./pkg/openbim_ifc_wasm.js";
import { buildScene, colourOf } from "./scene.mjs";

const FIXTURE = "../../../../test/fixtures/synthetic-bindings/binding_geometry.ifc";

const canvas = document.querySelector("canvas");
const status = document.querySelector("#status");
const gl = canvas.getContext("webgl2");
if (!gl) throw new Error("WebGL2 is not available");

const VERTEX = `#version 300 es
uniform mat4 viewProjection;
uniform mat4 model;
in vec3 position;
out vec3 world;
void main() {
  vec4 p = model * vec4(position, 1.0);
  world = p.xyz;
  gl_Position = viewProjection * p;
}`;
// Flat shading from screen-space derivatives: the meshes carry no normals.
const FRAGMENT = `#version 300 es
precision highp float;
uniform vec3 colour;
in vec3 world;
out vec4 fragment;
void main() {
  vec3 normal = normalize(cross(dFdx(world), dFdy(world)));
  float light = 0.35 + 0.65 * abs(dot(normal, normalize(vec3(0.4, 0.6, 0.8))));
  fragment = vec4(colour * light, 1.0);
}`;

function compile(type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader));
  return shader;
}
const program = gl.createProgram();
gl.attachShader(program, compile(gl.VERTEX_SHADER, VERTEX));
gl.attachShader(program, compile(gl.FRAGMENT_SHADER, FRAGMENT));
gl.linkProgram(program);
if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program));
const uniform = (name) => gl.getUniformLocation(program, name);
const at = { viewProjection: uniform("viewProjection"), model: uniform("model"), colour: uniform("colour") };

let items = [];
let scene;
const camera = { yaw: -0.8, pitch: 0.5, distance: 1 };

function upload(drawn) {
  for (const item of items) gl.deleteVertexArray(item.vao);
  items = drawn.map((mesh) => {
    const vao = gl.createVertexArray();
    gl.bindVertexArray(vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
    gl.bufferData(gl.ARRAY_BUFFER, mesh.positions, gl.STATIC_DRAW);
    const position = gl.getAttribLocation(program, "position");
    gl.enableVertexAttribArray(position);
    gl.vertexAttribPointer(position, 3, gl.FLOAT, false, 0, 0);
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, gl.createBuffer());
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, mesh.indices, gl.STATIC_DRAW);
    gl.bindVertexArray(null);
    return { vao, count: mesh.indices.length, matrix: mesh.matrix, colour: colourOf(mesh.typeName) };
  });
}

function perspective(fovy, aspect, near, far) {
  const f = 1 / Math.tan(fovy / 2);
  const d = 1 / (near - far);
  return [f / aspect, 0, 0, 0, 0, f, 0, 0, 0, 0, (far + near) * d, -1, 0, 0, 2 * far * near * d, 0];
}

// IFC is Z-up.
function lookAt(eye, target) {
  const sub = (a, b) => a.map((v, i) => v - b[i]);
  const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
  const unit = (v) => v.map((x) => x / Math.hypot(...v));
  const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
  const z = unit(sub(eye, target));
  const x = unit(cross([0, 0, 1], z));
  const y = cross(z, x);
  return [x[0], y[0], z[0], 0, x[1], y[1], z[1], 0, x[2], y[2], z[2], 0, -dot(x, eye), -dot(y, eye), -dot(z, eye), 1];
}

function multiply(a, b) {
  const out = new Float32Array(16);
  for (let c = 0; c < 4; c++)
    for (let r = 0; r < 4; r++)
      for (let k = 0; k < 4; k++) out[c * 4 + r] += a[k * 4 + r] * b[c * 4 + k];
  return out;
}

function draw() {
  const width = canvas.clientWidth * devicePixelRatio;
  const height = canvas.clientHeight * devicePixelRatio;
  if (canvas.width !== width || canvas.height !== height) Object.assign(canvas, { width, height });
  gl.viewport(0, 0, width, height);
  gl.clearColor(0.96, 0.96, 0.95, 1);
  gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
  if (!scene || items.length === 0) return;
  gl.enable(gl.DEPTH_TEST);
  gl.useProgram(program);
  const r = scene.radius * camera.distance * 2.5;
  const [cx, cy, cz] = scene.center;
  const eye = [
    cx + r * Math.cos(camera.pitch) * Math.cos(camera.yaw),
    cy + r * Math.cos(camera.pitch) * Math.sin(camera.yaw),
    cz + r * Math.sin(camera.pitch),
  ];
  const projection = perspective(Math.PI / 4, width / height, r / 100, r * 4);
  gl.uniformMatrix4fv(at.viewProjection, false, multiply(projection, lookAt(eye, scene.center)));
  for (const item of items) {
    gl.uniformMatrix4fv(at.model, false, item.matrix);
    gl.uniform3fv(at.colour, item.colour);
    gl.bindVertexArray(item.vao);
    gl.drawElements(gl.TRIANGLES, item.count, gl.UNSIGNED_INT, 0);
  }
}

function load(bytes, name) {
  const model = IfcModel.parse(bytes);
  try {
    scene = buildScene(model.productMeshes());
  } finally {
    model.free();
  }
  upload(scene.drawn);
  const refused = scene.refused.map((r) => `#${r.id} ${r.code}`).join(", ");
  status.textContent = `${name}: ${scene.drawn.length} meshes` + (refused ? `; refused: ${refused}` : "");
  draw();
  document.body.dataset.drawn = String(scene.drawn.length);
}

canvas.addEventListener("pointermove", (event) => {
  if (event.buttons !== 1) return;
  camera.yaw -= event.movementX * 0.01;
  camera.pitch = Math.max(-1.5, Math.min(1.5, camera.pitch + event.movementY * 0.01));
  draw();
});
canvas.addEventListener("wheel", (event) => {
  event.preventDefault();
  camera.distance *= Math.exp(event.deltaY * 0.001);
  draw();
});
addEventListener("resize", draw);
document.querySelector("input").addEventListener("change", async (event) => {
  const [file] = event.target.files;
  if (file) load(new Uint8Array(await file.arrayBuffer()), file.name);
});

await init();
const file = new URL(location.href).searchParams.get("file") ?? FIXTURE;
load(new Uint8Array(await (await fetch(file)).arrayBuffer()), file.split("/").pop());
