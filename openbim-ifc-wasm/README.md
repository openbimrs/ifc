# openbim-ifc-wasm

WebAssembly bindings for [`openbim-ifc`](https://crates.io/crates/openbim-ifc):
read, edit and write IFC STEP (`.ifc`) files from JavaScript and TypeScript,
in Node or a browser.

Status: **0.1, Node build tested; not yet published to npm.** Browser
bundling works in principle (the crate builds for `wasm32-unknown-unknown`)
but has no tested recipe yet.

## What it does

- Parse a STEP file into a model; write a model back to STEP.
- List entities, filter by exact type or by type including subtypes (using
  the bundled IFC2X3, IFC4 or IFC4X3 schema the file declares), read and
  edit attributes, add and remove entities, find dangling references.
- Keep every value **lossless** across the boundary: `$` vs `*`, `.U.` vs
  `.F.`, integer vs real, typed wrappers such as `IFCLENGTHMEASURE(2.5)`,
  and 64-bit integers (as `bigint`).

## What it does not do (yet)

- No schema validation, no domain views (properties,
  quantities, spatial tree), no geometry. These exist in the Rust crates and
  will be exposed as the bindings grow; see ADR 0013.
- No ifcXML.

## Example (Node)

```js
const { readFileSync, writeFileSync } = require("node:fs");
const { IfcModel } = require("./pkg/openbim_ifc_wasm.js");

const model = IfcModel.parse(readFileSync("house.ifc"));
console.log(model.schema, model.size);

// Subtypes included: also finds IFCWALLSTANDARDCASE in IFC2X3/IFC4 files.
for (const id of model.idsOfTypeIncludingSubtypes("IfcWall")) {
  const name = model.attribute(id, 2); // { kind: "text", value: "..." } or { kind: "null" }
  if (name.kind === "text") console.log(id, name.value);
}

const [wall] = model.idsOfTypeIncludingSubtypes("IfcWall");
model.setAttribute(wall, 2, { kind: "text", value: "Renamed" });
writeFileSync("house-edited.ifc", model.write());
```

Ids are `bigint`s. Attribute slots are 0-based positions in the entity's
EXPRESS declaration.

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
`code`: `parse`, `write`, `missing-entity`, `invalid-value`,
`out-of-range` or `unsupported-schema`. A refused edit leaves the model unchanged.

## Building

```sh
cargo install wasm-bindgen-cli --version 0.2.128 --locked
openbim-ifc-wasm/scripts/build-node-pkg.sh   # builds pkg/ and runs the JS smoke test
```

## License

AGPL-3.0-or-later, like the rest of `openbimrs/ifc`.
