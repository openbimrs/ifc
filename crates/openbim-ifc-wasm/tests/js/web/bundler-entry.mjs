// The package's default ES module export through a bundler (#40). Bundled by
// webpack with `experiments.asyncWebAssembly` for the browser, then run in
// the same headless browser page as the `web` target; see
// tools/check-package.mjs.
import { meshes, smoke } from "./smoke-core.mjs";

// docs:snippet js-bundler-import
import { IfcModel } from "@openbim/ifc"; // the bundler loads the wasm module
// docs:end

export const result = { ...(await smoke(IfcModel)), meshes: meshes(IfcModel, false) };
