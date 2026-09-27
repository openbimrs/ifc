# JavaScript and TypeScript

`@openbim/ifc` is the WebAssembly build of the IFC core
([`openbim-ifc-wasm`](/reference/crates/openbim-ifc-wasm)), published to npm
with TypeScript declarations. It is tested under Node; a tested browser and
bundler build is not published yet.

```bash
npm install @openbim/ifc
```

The package is CommonJS: `const { IfcModel } = require("@openbim/ifc");`,
or a default import from ES modules.

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
