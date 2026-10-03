// The `web` target in a browser, without a bundler (#40). Served next to the
// unpacked package with an import map for `@openbim/ifc/web`; see
// tools/check-package.mjs.
import { smoke } from "./smoke-core.mjs";

// docs:snippet js-web-init
import init, { IfcModel } from "@openbim/ifc/web";

await init(); // fetches openbim_ifc_wasm_bg.wasm from next to the module
// docs:end

export const result = await smoke(IfcModel);
