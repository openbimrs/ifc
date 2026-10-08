// The mesh entry's ES module export through a bundler (#369), bundled and
// run as bundler-entry.mjs is; see tools/check-package.mjs.
import { meshes, smoke } from "./smoke-core.mjs";

// docs:snippet js-mesh-import
import { IfcModel } from "@openbim/ifc/mesh"; // productMeshes included
// docs:end

export const result = { ...(await smoke(IfcModel)), meshes: meshes(IfcModel, true) };
