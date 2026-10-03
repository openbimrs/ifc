# JavaScript and TypeScript

`@openbim/ifc` is the WebAssembly build of the IFC core
([`openbim-ifc-wasm`](/reference/crates/openbim-ifc-wasm)), published to npm
with TypeScript declarations. The package carries three builds of one
module, for Node, for bundlers and for browsers without a bundler, and each
is tested from the packed tarball.

```bash
npm install @openbim/ifc
```

In Node the package is CommonJS: `const { IfcModel } = require("@openbim/ifc");`,
or `import { IfcModel } from "@openbim/ifc"` from ES modules.

The binding exposes the record model over STEP: parse, read and edit
attributes, and write. Domain views such as property sets or the spatial tree
([#123](https://github.com/openbimrs/ifc/issues/123)), ifcXML, validation and
checked transactions ([#244](https://github.com/openbimrs/ifc/issues/244)) are
not bound yet; use the Rust crates for those.

## Node, bundlers and browsers

The package's `exports` map picks the build:

| Import | Build | The wasm module loads |
| --- | --- | --- |
| `@openbim/ifc` in Node | CommonJS (`wasm-bindgen --target nodejs`) | on `require`, from disk |
| `@openbim/ifc` in a bundler | ES module (`--target bundler`) | through the bundler |
| `@openbim/ifc/web` | ES module (`--target web`) | when `init()` is awaited |

The bundler build imports its `.wasm` file as an ES module, so the API is
the same as in Node:

<!-- SNIPPET:js-bundler-import -->

```js
import { IfcModel } from "@openbim/ifc"; // the bundler loads the wasm module
```

<!-- /SNIPPET -->

webpack 5 bundles it with `experiments: { asyncWebAssembly: true }`, which
is what the package test uses. Vite and Rollup need a wasm plugin such as
`vite-plugin-wasm`.

Without a bundler, or with one that cannot import wasm such as esbuild,
import the `web` build and initialise it once before the first call:

<!-- SNIPPET:js-web-init -->

```js
import init, { IfcModel } from "@openbim/ifc/web";

await init(); // fetches openbim_ifc_wasm_bg.wasm from next to the module
```

<!-- /SNIPPET -->

`init` also takes `{ module_or_path }`: a URL, a `Response`, the module's
bytes or a compiled `WebAssembly.Module`. Serve `.wasm` files as
`application/wasm` so the browser compiles while it downloads.

The published module bundles the schema of every IFC release, most of its
size; `wasm-opt -Oz` was measured, not applied, because it increases the
gzip and brotli size. A build from source with
`--no-default-features --features ifc4` carries one
([#112](https://github.com/openbimrs/ifc/issues/112)).

## Read, edit and write

<!-- SNIPPET:js-read-edit-write -->

```js
const model = IfcModel.parse(bytes); // bytes: the .ifc file as a Uint8Array
const schema = model.schema; // "IFC4"

for (const wall of model.idsOfType("IfcWall")) {
  const name = model.attribute(wall, 2); // { kind: "text", value: "Wall" }
  model.setAttribute(wall, 2, { kind: "text", value: `${name.value} (checked)` });
}

const out = model.write(); // a Uint8Array, ready to save
```

<!-- /SNIPPET -->

Entity ids are `bigint`, because IFC ids exceed JavaScript's safe integer
range in real files. Attribute values use a tagged encoding that keeps
every distinction STEP makes: `$` from `*`, `.U.` from `.F.`, an integer
from a real, and a typed wrapper such as `IFCLENGTHMEASURE(2.5)` from its
payload. A file read and written back is unchanged.

Every failure throws an `IfcError` whose `code` is one of the
`IfcErrorCode` values below, the same codes the Python and C bindings use.

## API

Generated from the `#[wasm_bindgen]` exports in
`crates/openbim-ifc-wasm/src/model.rs`.

<!-- API:JS:BEGIN -->

| Member | Throws `IfcError` | Description |
| --- | --- | --- |
| `new IfcModel()` |  | An empty model. |
| `IfcModel.parse(bytes: Uint8Array): IfcModel` | yes | Parse a STEP (`.ifc`) file from its bytes. |
| `model.write(): Uint8Array` | yes | Serialize as STEP bytes. |
| `model.size: number` |  | Number of entities. |
| `model.schema: string \| undefined` |  | The first `FILE_SCHEMA` token, e.g. `"IFC4"`, or `undefined`. |
| `model.diagnostics(): string[]` |  | Non-fatal problems found while reading. |
| `model.ids(): bigint[]` |  | Every entity id (`bigint`), in file order. |
| `model.idsOfType(typeName: string): bigint[]` |  | Ids of every entity of exactly `typeName`, case-insensitive. |
| `model.idsOfTypeIncludingSubtypes(typeName: string): bigint[]` | yes | Ids of every entity of `typeName` or any subtype, per the file's declared schema: `IfcWall` also finds `IFCWALLSTANDARDCASE`. |
| `model.typeOf(id: bigint): string` | yes | The upper-case type name of entity `id`. |
| `model.attributes(id: bigint): IfcValue[]` | yes | Every attribute of entity `id`, as tagged values. |
| `model.attribute(id: bigint, slot: number): IfcValue` | yes | Attribute `index` of entity `id`, as a tagged value. |
| `model.setAttribute(id: bigint, slot: number, value: IfcValue): IfcValue` | yes | Set attribute `index` of entity `id`; returns the previous value. |
| `model.add(typeName: string, attributes: IfcValue[]): bigint` | yes | Append an entity; returns its id (`bigint`). |
| `model.remove(id: bigint): void` | yes | Remove entity `id`, leaving references to it dangling. |
| `model.danglingReferences(): [bigint, bigint][]` |  | Every `[from, to]` pair (`bigint`s) where `to` does not exist. |

Attribute values and error codes are typed by the package's `.d.ts`:

```ts
/** One IFC attribute value, in the lossless tagged encoding (ADR 0013). */
export type IfcValue =
  | { kind: "null" }
  | { kind: "derived" }
  | { kind: "bool"; value: boolean }
  | { kind: "unknown" }
  | { kind: "integer"; value: bigint }
  | { kind: "real"; value: number }
  | { kind: "text"; value: string }
  | { kind: "binary"; value: string }
  | { kind: "enum"; value: string }
  | { kind: "ref"; id: bigint }
  | { kind: "list"; items: IfcValue[] }
  | { kind: "typed"; type: string; value: IfcValue };

/** The `code` of an `IfcError`. */
export type IfcErrorCode =
  | "parse"
  | "write"
  | "missing-entity"
  | "invalid-value"
  | "out-of-range"
  | "unsupported-schema";
```

<!-- API:JS:END -->
