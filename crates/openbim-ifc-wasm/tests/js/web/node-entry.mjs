// The packed package from Node, resolved by name through its `exports` map
// (#40): `require` and `import` both reach the CommonJS build, and the `web`
// build loads in Node when handed its module bytes. Run from a directory
// whose node_modules holds the unpacked tarball; see tools/check-package.mjs.
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { smoke } from "./smoke-core.mjs";

import { IfcModel as Imported } from "@openbim/ifc";
import init, { IfcModel as Web } from "@openbim/ifc/web";

const { IfcModel: Required } = createRequire(import.meta.url)("@openbim/ifc");
if (Required !== Imported) {
  throw new Error("require and import of @openbim/ifc reach different modules");
}

const wasm = new URL("openbim_ifc_wasm_bg.wasm", import.meta.resolve("@openbim/ifc/web"));
await init({ module_or_path: await readFile(wasm) });

export const result = { node: await smoke(Imported), web: await smoke(Web) };
