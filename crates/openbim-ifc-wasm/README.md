# openbim-ifc-wasm

WebAssembly bindings for [`openbim-ifc`](https://crates.io/crates/openbim-ifc):
read, edit and write IFC STEP (`.ifc`) files from JavaScript and TypeScript,
in Node or a browser.

Published to npm as `@openbim/ifc`, with three builds of the same module:
CommonJS for Node, an ES module for bundlers, and an ES module that loads
in a browser without one. Each is tested from the packed tarball: Node,
a webpack bundle, and headless Chrome.

Documentation: [JavaScript guide](https://openbimrs.github.io/ifc/bindings/javascript) · [source and issues](https://github.com/openbimrs/ifc)

## What it does

- Parse a STEP file into a model; write a model back to STEP.
- List entities, filter by exact type or by type including subtypes (using
  the bundled schema the file declares), read and edit attributes, add and
  remove entities, find dangling references.
- Keep every value **lossless** across the boundary: `$` vs `*`, `.U.` vs
  `.F.`, integer vs real, typed wrappers such as `IFCLENGTHMEASURE(2.5)`,
  and 64-bit integers (as `bigint`).
- Read damaged files leniently (`IfcModel.parseWithOptions`), read and
  replace the STEP header, validate against the declared schema, read and
  write ifcXML (lossless or the buildingSMART XSD layout), list products
  no viewer will draw, and place each product (`productPlacements`).
- Mesh each product (`productMeshes`) or read its exact Axiolid graph
  (`productGeometry`, #367) from the mesh entry, `@openbim/ifc/mesh` (#369).
- Read the domain views as snapshot objects: property sets and quantities
  with type inheritance (`propertySets`, `resolveUnit`), the spatial tree,
  classifications, materials, systems, cost and georeferencing (#123).
- Write property sets and quantities (`setProperties`) as one checked
  transaction; `await IfcModel.loadCatalog(release)` loads its PSD/QTO catalog.

Not bound yet: checked multi-edit transactions over arbitrary entities
(ADR 0013).

## Smaller builds

Validation, ifcXML, the reachability lint, the seven domain views, the
property writer and its PSD/QTO catalog are default cargo features
(`validate`, `ifcxml`, `unreachable`, `properties`, `spatial`,
`classification`, `material`, `systems`, `cost`, `georef`,
`properties-write`, `property-catalog-runtime`), like the IFC releases. A
browser build can leave any of them out, e.g. `--no-default-features
--features ifc4,ifcxml,spatial`; the left-out methods then throw
`feature-disabled`. The default module is 2,909,468 bytes; IFC4 alone is
815,769 ([sizes](https://openbimrs.github.io/ifc/bindings/javascript#module-size)).

## Example (Node)

```sh
npm install @openbim/ifc
```

```js
const { readFileSync, writeFileSync } = require("node:fs");
const { IfcModel } = require("@openbim/ifc");

const model = IfcModel.parse(readFileSync("house.ifc"));
console.log(model.schema, model.size);

// Subtypes included: also finds IFCWALLSTANDARDCASE in IFC2X3/IFC4 files.
for (const id of model.idsOfTypeIncludingSubtypes("IfcWall")) {
  const name = model.attributeByName(id, "Name"); // { kind: "text", value: "..." } or { kind: "null" }
  if (name.kind === "text") console.log(id, name.value);
}

const [wall] = model.idsOfTypeIncludingSubtypes("IfcWall");
model.setAttributeByName(wall, "Name", { kind: "text", value: "Renamed" });
writeFileSync("house-edited.ifc", model.write());
```

Ids are `bigint`s. Names resolve against the release the header declares;
`attribute(id, index)` addresses a slot by its 0-based position instead.
`author(ops)` creates entities, the spatial structure and placed, typed
products as one checked batch, refused whole or applied whole.

## Browsers and bundlers

| Import | Build | Loads the wasm module |
| --- | --- | --- |
| `@openbim/ifc` in Node (`require` or `import`) | CommonJS | synchronously, from disk |
| `@openbim/ifc` in a bundler | ES module (`--target bundler`) | through the bundler |
| `@openbim/ifc/web` | ES module (`--target web`) | when you call `init()` |

A bundler resolves `@openbim/ifc` to the bundler build, which imports the
`.wasm` file as an ES module. webpack 5 supports that with
`experiments: { asyncWebAssembly: true }`; Vite and Rollup need a wasm
plugin (for example `vite-plugin-wasm`). The API is the same as in Node.

Without a bundler, or with one that cannot import wasm (esbuild), use the
`web` build and initialise it once before the first call:

```js
import init, { IfcModel } from "@openbim/ifc/web";

await init(); // fetches openbim_ifc_wasm_bg.wasm from next to the module
const model = IfcModel.parse(new Uint8Array(await file.arrayBuffer()));
```

`init` also accepts `{ module_or_path }`: a URL, a `Response`, the bytes or
a compiled `WebAssembly.Module`. Serve `.wasm` as `application/wasm` so the
browser can compile it while it downloads.

## Values

Every attribute value is an object with a `kind`:

| `kind`    | fields                         | STEP              |
| --------- | ------------------------------ | ----------------- |
| `null`    | —                              | `$`               |
| `derived` | —                              | `*`               |
| `bool`    | `value: boolean`               | `.T.` / `.F.`     |
| `unknown` | —                              | `.U.`             |
| `integer` | `value: bigint`                | `42`              |
| `real`    | `value: number`                | `2.5`             |
| `text`    | `value: string`                | `'Wall'`          |
| `binary`  | `value: string`                | `"0123ABC"`       |
| `enum`    | `value: string`                | `.ELEMENT.`       |
| `ref`     | `id: bigint`                   | `#42`             |
| `list`    | `items: IfcValue[]`            | `(1,2)`           |
| `typed`   | `type: string, value: IfcValue`| `IFCLABEL('x')`   |

The TypeScript declarations export this union as `IfcValue`.

## Errors

Every failure throws an `Error` with `name === "IfcError"` and a stable
`code` (the `IfcErrorCode` type lists them); a code is never renamed or
reused. A refused edit leaves the model unchanged.

## Building

```sh
cargo install wasm-bindgen-cli --version 0.2.128 --locked
crates/openbim-ifc-wasm/scripts/build-npm-pkg.sh   # builds pkg/ and tests every target
```

The script builds both modules (default; `mesh,graph` into `pkg/mesh/`), binds each
per target, runs the Node suites, and checks the packed tarball in Node,
webpack and headless Chrome (`CHROME_BIN` names it if not on `PATH`). webpack comes pinned
from `tools/package-lock.json`.

Neither `wasm-opt -Oz` nor `opt-level = "z"` is applied (the guide has
why); `scripts/bench-opt-level.sh` reruns the opt-level comparison (#303).

## License

AGPL-3.0-or-later, like the rest of `openbimrs/ifc`.
