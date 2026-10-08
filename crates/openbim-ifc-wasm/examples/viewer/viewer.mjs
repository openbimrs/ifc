// A minimal WebGL2 viewer for `IfcModel.productMeshes()` (#328): no
// framework, no build step. It imports the npm package's mesh entry,
// `@openbim/ifc/mesh/web` (#369); index.html's import map resolves that to
// the module ./build.sh builds into ./pkg, or to an installed package's
// file. scene.mjs decides what is drawn and where, render.mjs draws it.
import init, { IfcModel } from "@openbim/ifc/mesh/web";
import { createViewer } from "./render.mjs";
import { buildScene, colourOf } from "./scene.mjs";

const FIXTURE = "../../../../test/fixtures/synthetic-bindings/binding_geometry.ifc";

const status = document.querySelector("#status");
const viewer = createViewer(document.querySelector("canvas"));

function load(bytes, name) {
  const model = IfcModel.parse(bytes);
  let scene;
  try {
    scene = buildScene(model.productMeshes());
  } finally {
    model.free();
  }
  viewer.show(scene, colourOf);
  const refused = scene.refused.map((r) => `#${r.id} ${r.code}`).join(", ");
  status.textContent = `${name}: ${scene.drawn.length} meshes` + (refused ? `; refused: ${refused}` : "");
  document.body.dataset.drawn = String(scene.drawn.length);
}

addEventListener("resize", viewer.draw);
document.querySelector("input").addEventListener("change", async (event) => {
  const [file] = event.target.files;
  if (file) load(new Uint8Array(await file.arrayBuffer()), file.name);
});

await init();
const file = new URL(location.href).searchParams.get("file") ?? FIXTURE;
load(new Uint8Array(await (await fetch(file)).arrayBuffer()), file.split("/").pop());
