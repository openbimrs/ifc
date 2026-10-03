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

The binding exposes the record model over STEP -- parse, read and edit
attributes, and write -- plus lenient reads, the file header, validation,
ifcXML and the reachability lint (see
[Beyond the record model](#beyond-the-record-model)).

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
([#112](https://github.com/openbimrs/ifc/issues/112)); it can also leave out the
capabilities described [below](#beyond-the-record-model).

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

## Beyond the record model

<!-- SNIPPET:js-beyond-records -->

```js
// A damaged export: skip what cannot be read, and say what was skipped.
const model = IfcModel.parseWithOptions(bytes, { onMalformed: "skip" });
const skipped = model.diagnostics(); // one message per dropped record

const header = model.header(); // { name, timeStamp, author, schema, ... }
model.setHeader({ ...header, author: ["Reviewer"] });

const report = model.validate(); // { conformant, errors, findings, ... }
const errors = report.findings.filter((f) => f.severity === "error");
// each finding: { rule, entity, attributeName, path, message, ... }

const xml = model.writeIfcXml(); // lossless ifcXML; or writeIfcXml("IFC4")
const fromXml = IfcModel.parseIfcXml(xml);
```

<!-- /SNIPPET -->

- **Lenient reads.** `IfcModel.parseWithOptions(bytes, options)` takes
  `onMalformed: "skip"`, `checkReferences` and `acceptRealWithoutPoint`;
  every recovery is listed by `diagnostics()`. Omitted options are strict.
- **Header.** `header()` returns every `FILE_DESCRIPTION`, `FILE_NAME` and
  `FILE_SCHEMA` field; `setHeader()` replaces them all.
- **Validation.** `validate(maxFindings?)` checks the model against the
  schema its header declares and returns a `ValidationReport`: counts by
  severity, `conformant`, `truncated`, and findings sorted by severity,
  rule, entity and slot.
- **ifcXML.** `writeIfcXml()` and `IfcModel.parseIfcXml(bytes)` use this
  library's lossless layout; pass `"IFC4"` or `"IFC4X3_ADD2"` for the
  buildingSMART XSD layout of that release, which refuses with `write`
  what it cannot carry exactly.
- **Reachability.** `unreachableProducts()` lists products no viewer will
  draw, each with a stable `reason`.

Validation, ifcXML and the reachability lint are cargo features of
`openbim-ifc-wasm` (`validate`, `ifcxml`, `unreachable`), on by default.
A size-trimmed browser build can leave them out, as it can leave out IFC
releases; their methods then throw `feature-disabled`. Measured after
`wasm-bindgen`, without `wasm-opt`: ifcXML adds about 315 KB, validation
about 170 KB with every release bundled (about 760 KB on an IFC4-only
build, because the validator links every release's schema table), and the
reachability lint about 65 KB.

Not bound yet: checked multi-edit transactions (`Transaction`, `Applied`,
`Conflict`), deferred until a host asks for them, and the domain views
such as property sets or the spatial tree
([#123](https://github.com/openbimrs/ifc/issues/123)), which come next as
opt-in features. Use the Rust crates for those.

## API

Generated from the `#[wasm_bindgen]` exports in
`crates/openbim-ifc-wasm/src/model.rs`.

<!-- API:JS:BEGIN -->

| Member | Throws `IfcError` | Description |
| --- | --- | --- |
| `new IfcModel()` |  | An empty model. |
| `IfcModel.parse(bytes: Uint8Array): IfcModel` | yes | Parse a STEP (`.ifc`) file from its bytes. |
| `IfcModel.parseWithOptions(bytes: Uint8Array, options: ParseOptions): IfcModel` | yes | Parse a STEP file under explicit read options. With `onMalformed: "skip"` a damaged record is dropped and reported by `diagnostics()` instead of failing the read; omitted options are strict. |
| `IfcModel.parseIfcXml(bytes: Uint8Array, profile: string \| undefined): IfcModel` | yes | Parse an ifcXML document: the library's lossless layout without a profile, or the buildingSMART XSD layout of `"IFC4"` or `"IFC4X3_ADD2"`. |
| `model.write(): Uint8Array` | yes | Serialize as STEP bytes. |
| `model.writeIfcXml(profile: string \| undefined): Uint8Array` | yes | Serialize as ifcXML bytes, in the layout `parseIfcXml` reads. |
| `model.header(): IfcHeader` |  | The STEP file header: description, name, time stamp, author, organization, preprocessor, originating system, authorization and schema tokens. |
| `model.setHeader(header: IfcHeader): void` | yes | Replace the STEP file header; every field is required. |
| `model.validate(maxFindings: number \| undefined): ValidationReport` | yes | Validate against the schema the header declares; findings are sorted by severity, rule, entity and slot. `maxFindings` caps the report (default 10,000) and sets `truncated` when reached. |
| `model.unreachableProducts(): UnreachableProduct[]` | yes | Products no viewer will draw (outside the spatial structure, or with geometry only in non-model contexts), with a stable `reason`. |
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
  | "unsupported-schema"
  | "io"
  | "unsupported-profile"
  | "feature-disabled";

/** How `IfcModel.parseWithOptions` treats damaged input; omitted fields are strict. */
export interface ParseOptions {
  /** `"skip"` drops a malformed record and reports it in `diagnostics()`. */
  onMalformed?: "abort" | "skip";
  /** Report duplicate ids and references to undefined ids as diagnostics. */
  checkReferences?: boolean;
  /** Read `1E-05` (no decimal point) as a real, with a diagnostic. */
  acceptRealWithoutPoint?: boolean;
}

/** The STEP file header (`FILE_DESCRIPTION`, `FILE_NAME`, `FILE_SCHEMA`). */
export interface IfcHeader {
  description: string[];
  implementationLevel: string;
  name: string;
  timeStamp: string;
  author: string[];
  organization: string[];
  preprocessorVersion: string;
  originatingSystem: string;
  authorization: string;
  schema: string[];
}

/** One validation finding. `entity` is `undefined` for the file as a whole. */
export interface ValidationFinding {
  severity: "error" | "evaluation-error" | "warning" | "unsupported";
  rule: string;
  entity: bigint | undefined;
  attributeIndex: number | undefined;
  attributeName: string | undefined;
  path: string;
  message: string;
}

/** The result of `IfcModel.validate`. */
export interface ValidationReport {
  /** No errors and no evaluation errors; unsupported rules do not count. */
  conformant: boolean;
  /** The finding budget was reached: counts are lower bounds. */
  truncated: boolean;
  errors: number;
  evaluationErrors: number;
  warnings: number;
  unsupported: number;
  findings: ValidationFinding[];
}

/** A product no viewer will draw, from `IfcModel.unreachableProducts`. */
export interface UnreachableProduct {
  id: bigint;
  reason:
    | "not-contained-in-spatial-structure"
    | "no-representation-in-model-context"
    | "representation-without-context";
  /** Target views the geometry was found in instead, for the second reason. */
  foundViews: string[];
  message: string;
}
```

<!-- API:JS:END -->
