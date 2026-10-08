// The mesh entry's `web` target in a browser, without a bundler (#369),
// through an import map for `@openbim/ifc/mesh/web`; see
// tools/check-package.mjs.
import { meshes, smoke } from "./smoke-core.mjs";

// docs:snippet js-mesh-web-init
import init, { IfcModel } from "@openbim/ifc/mesh/web";

await init(); // fetches mesh/web/openbim_ifc_wasm_bg.wasm
// docs:end

export const result = { ...(await smoke(IfcModel)), meshes: meshes(IfcModel, true) };
