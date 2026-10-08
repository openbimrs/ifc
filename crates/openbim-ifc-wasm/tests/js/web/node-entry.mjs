// The packed package from Node, resolved by name through its `exports` map
// (#40): `require` and `import` both reach the CommonJS build, and the `web`
// build loads in Node when handed its module bytes. The same for the mesh
// entry, `@openbim/ifc/mesh` (#369). Run from a directory whose node_modules
// holds the unpacked tarball; see tools/check-package.mjs.
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { meshes, smoke } from "./smoke-core.mjs";

import { IfcModel as Imported } from "@openbim/ifc";
import init, { IfcModel as Web } from "@openbim/ifc/web";
import { IfcModel as MeshImported } from "@openbim/ifc/mesh";
import initMesh, { IfcModel as MeshWeb } from "@openbim/ifc/mesh/web";

const require = createRequire(import.meta.url);
const { IfcModel: Required } = require("@openbim/ifc");
if (Required !== Imported) {
  throw new Error("require and import of @openbim/ifc reach different modules");
}
const { IfcModel: MeshRequired } = require("@openbim/ifc/mesh");
if (MeshRequired !== MeshImported) {
  throw new Error("require and import of @openbim/ifc/mesh reach different modules");
}
if (MeshImported === Imported) {
  throw new Error("@openbim/ifc/mesh reaches the default module");
}

async function bytesOf(specifier) {
  return readFile(new URL("openbim_ifc_wasm_bg.wasm", import.meta.resolve(specifier)));
}
await init({ module_or_path: await bytesOf("@openbim/ifc/web") });
await initMesh({ module_or_path: await bytesOf("@openbim/ifc/mesh/web") });

async function entry(IfcModel, mesh) {
  return { ...(await smoke(IfcModel)), meshes: meshes(IfcModel, mesh) };
}

export const result = {
  node: await entry(Imported, false),
  web: await entry(Web, false),
  "mesh/node": await entry(MeshImported, true),
  "mesh/web": await entry(MeshWeb, true),
};
