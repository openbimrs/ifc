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
ifcXML, the reachability lint and read-only domain views (see
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
`wasm-bindgen`, without `wasm-opt`, on an IFC4-only build: ifcXML adds
about 315 KB, validation about 170 KB and the reachability lint about
70 KB ([module size](#module-size) has every combination).

### Domain views

<!-- SNIPPET:js-domain-views -->

```js
const model = IfcModel.parse(bytes);
const [wall] = model.idsOfType("IfcWall");

// Property sets: the wall's own first, then its type's; values typed.
for (const set of model.propertySets(wall)) {
  for (const p of set.properties) console.log(set.name, p.name, p.value);
}
const classes = model.classifications(wall); // [{ identification, system, ... }]
const material = model.material(wall); // { kind: "layer-set", layers, ... }
const tree = model.spatialTree(); // { nodes: [{ kind: "storey", elements }] }
```

<!-- /SNIPPET -->

The domain views of the Rust facade cross as plain snapshot objects, keyed
by entity id (`bigint`) with the `globalId` where the entity has one. A
view reads the model as it is at the call; edit the model and call again.

- **Property sets.** `propertySets(id)` lists the object's own property
  sets, quantity sets and predefined sets, then those its type object
  holds; an occurrence property overrides an inherited one of the same set
  and name. Each `Property` keeps its declared type in the tagged encoding
  (`{ kind: "typed", type: "IFCLENGTHMEASURE", value: { kind: "real", ... } }`)
  with its `kind` (`value`, `enumerated`, `list`, `bounded`, `table`,
  `reference` or `complex`) and the `unit` it states.
  `resolveUnit(measureType, unit?)` resolves that unit, or the project
  default, exactly to SI. Read against IFC2X3, IFC4 or IFC4X3.
- **Spatial tree.** `spatialTree()` returns every container (`kind`
  `project` ... `space`) with its parent, children, contained and
  referenced elements, plus orphans, dangling references and anomalies.
- **Classification.** `classifications(id)` lists the object's own and its
  type's classification references, each with its code, name, location,
  the references above it and the system at the top.
- **Material.** `material(id)` returns the one association that applies
  (its own, or its type's): a material, list, layer, profile or constituent
  set, or a set usage with its placement; `undefined` when there is none.
- **Systems.** `systems()` lists every `IfcSystem` subtype the release has,
  with members and served structures, and reports memberships it could
  not honour.
- **Cost.** `cost()` lists cost schedules and items with their nesting,
  values (`appliedValue` tagged, `IFCMONETARYMEASURE` included), unit bases
  and component trees; operators are reported, never evaluated.
- **Georeferencing.** `georeferencing()` resolves every coordinate
  operation (IFC4, IFC4X3) with the project length unit: the projected
  CRS, the authored offsets, and the operation from project metres to map
  metres.

A release a view does not read is refused with `unsupported-schema`, the
same code in every host; malformed or ambiguous data with
`invalid-model`, a dangling reference with `missing-reference`, a cycle
with `budget-exceeded`, a construct a view does not interpret with
`unsupported`, and a query on an entity of the wrong kind with
`wrong-entity-type`. Each domain is a cargo feature of
`openbim-ifc-wasm` (`properties`, `spatial`, `classification`,
`material`, `systems`, `cost`, `georef`), on by default; a left-out
domain's methods throw `feature-disabled`. Measured after `wasm-bindgen`,
without `wasm-opt`, over a build with every release and capability: the
seven add 446 KB together (1.89 MB to 2.34 MB, 143 KB under `gzip -9`);
property sets about 156 KB, spatial 16 KB, classification 57 KB,
materials 83 KB, systems 56 KB, cost 37 KB, and georeferencing 55 KB on
top of property sets, which it needs for the project length unit.

#### Writing property sets

<!-- SNIPPET:js-domain-write -->

```js
const label = (value) => ({ kind: "typed", type: "IFCLABEL", value: { kind: "text", value } });
// Wall #31 inherits FireRating from its type: the write overrides it on
// the wall and never changes the type's shared set.
const result = model.setProperties([
  { object: 31n, set: "Pset_WallCommon", name: "FireRating", value: label("F60") },
  {
    object: 30n,
    set: "Qto_WallBaseQuantities",
    name: "Width",
    value: { kind: "typed", type: "IFCLENGTHMEASURE", value: { kind: "real", value: 250 } },
  },
  { object: 30n, set: "Pset_WallCommon", name: "IsExternal", remove: true },
]);
// result.properties: per edit, the entity now holding the value
const fireRating = model
  .propertySets(31n)
  .find((set) => set.source === "occurrence" && set.name === "Pset_WallCommon")
  .properties.find((p) => p.name === "FireRating");
```

<!-- /SNIPPET -->

`setProperties(edits)` writes and removes property and quantity values as
one checked transaction: every edit, in order, or, when any is refused,
none, and the model is unchanged. An edit addresses a property the way
`propertySets` reports it, by object, set name and property name, and its
`value` is that property's `value`: a typed `IfcValue`, a `list` of them
for an enumerated or list value, a typed measure for a quantity, whose
wrapper is dropped on write because the quantity's slot declares it.
`setProperty(object, set, name, value, setType?)` and
`removeProperty(object, set, name)` are the one-edit forms.

- A missing property is added to the object's own set, and a missing set
  is created with its `IfcRelDefinesByProperties` (or, for a type object,
  added to its `HasPropertySets`), with the object's owner history and a
  name-based `GlobalId`. A removal that empties a set removes the set.
- A value the occurrence inherits from its type is overridden on the
  occurrence, in the inherited property's form; the type's shared set is
  never changed through an occurrence. Pass the type object to change it.
- A set or property entity the object shares with others is copied for it
  first; the others keep their value.

Like the views, the edit refuses rather than writes when a value is not
admissible: `invalid-value` when the declared release (IFC2X3, IFC4 or
IFC4X3) has no such `IfcValue` member, the payload does not fit it, a
quantity's measure is another, or a stated unit fixes another measure;
`template-violation` when the release's PSD/QTO catalog entry for a
`Pset_`/`Qto_` set (data type, enumeration, form, quantity kind, a
property it does not declare) or the property's own enumeration refuses
it; `missing-property` for a removal of a property the object does not
state (one it only inherits included); `unsupported` for a bounded, table,
reference or complex value; and `wrong-entity-type` for a set type that
disagrees with the set. `properties-write` is a feature of its own
(about 205 KB, 65 KB under `gzip -9`), and the catalog `property-catalog`
another (3.7 MB, 0.99 MB), both on by default (see
[module size](#module-size)); without the catalog a write to a
`Pset_`/`Qto_` set throws `feature-disabled` rather than going unchecked.

Not bound yet: checked multi-edit transactions over arbitrary entities
(`Transaction`, `Applied`, `Conflict`), deferred until a host asks for
them. Use the Rust crates for those.

## Module size

Size of the module after `wasm-bindgen` (0.2.128), without `wasm-opt`,
built with `cargo build -p openbim-ifc-wasm --target wasm32-unknown-unknown
--release`: the default, or `--no-default-features --features <features>`.

| Features | Raw | gzip -9 | brotli 11 |
| --- | ---: | ---: | ---: |
| default: five releases, every capability and domain, the writer and the catalog | 6,282,871 | 1,855,125 | 956,057 |
| default before #123's writer | 2,343,181 | 796,234 | 516,930 |
| `ifc4` | 770,860 | 325,602 | 263,064 |
| `ifc4,validate` | 929,080 | 383,347 | 304,607 |
| `ifc4,unreachable` | 830,095 | 346,589 | 278,763 |
| `ifc4,ifcxml` | 1,074,540 | 430,635 | 342,298 |
| `ifc4,properties` | 926,986 | 382,926 | 306,185 |
| `ifc4,spatial` | 820,890 | 342,874 | 276,173 |
| `ifc4,classification` | 827,350 | 343,047 | 275,290 |
| `ifc4,material` | 852,448 | 347,343 | 277,759 |
| `ifc4,systems` | 831,006 | 346,749 | 278,506 |
| `ifc4,cost` | 802,733 | 336,375 | 270,411 |
| `ifc4,georef` (brings `properties`) | 990,688 | 406,569 | 323,150 |
| `ifc4,properties-write` (brings `properties`) | 1,144,013 | 451,456 | 355,458 |
| `ifc4,property-catalog` (brings `properties-write`) | 4,869,230 | 1,443,029 | 749,978 |
| `ifc4` + the three capabilities | 1,303,035 | 510,415 | 398,226 |
| `ifc4` + the seven domains | 1,266,205 | 487,576 | 378,385 |
| `ifc4` + every capability and domain | 1,760,329 | 656,425 | 499,492 |
| the same + `properties-write` | 1,963,827 | 719,252 | 544,142 |
| the same + `property-catalog` | 5,690,182 | 1,711,975 | 936,709 |

The `ifc4` and the last three rows, and the default, were measured with
the writer of [#123](https://github.com/openbimrs/ifc/issues/123); the
other single-feature rows before it. Its methods stay in every build, as
a left-out feature's do, which costs about 10 KB (`ifc4` was 759,820
bytes). The writer itself adds about 205 KB (65 KB under `gzip -9`), and
the PSD/QTO catalog, which embeds the IFC2X3, IFC4 and IFC4X3 catalogs
whatever releases the build names, 3.7 MB (0.99 MB).

Every capability and domain links only the schema tables of the releases
the build names ([#306](https://github.com/openbimrs/ifc/issues/306)).
Before, each linked all five: `ifc4,validate` was 1,521,685 bytes and
`ifc4` with everything was 2,342,546, the size of the five-release
default.

`wasm-opt -Oz` is not applied: with binaryen 132 it cut the module
measured in [#40](https://github.com/openbimrs/ifc/issues/40) from
1,337,025 to 1,295,241 bytes raw but grew it from 459,437 to 461,953
bytes under `gzip -9` and from 276,835 to 278,749 under brotli, which is
what a browser downloads.

The module is built at `opt-level = 3`, the workspace release profile.
`opt-level = "z"` and `"s"` were measured
([#303](https://github.com/openbimrs/ifc/issues/303)) and rejected: they
shrink the download by 15% and 10% but slow parsing by 85-112% and
50-67%. `crates/openbim-ifc-wasm/scripts/bench-opt-level.sh` reruns the
comparison: it builds the default feature set at each level and times
`IfcModel.parse` in Node, 30 interleaved runs per build and file after 5
warm-up runs. Sizes below are `node:zlib`'s; parse times are the median
with the interquartile range.

| `opt-level` | Raw | gzip -9 | brotli 11 |
| --- | ---: | ---: | ---: |
| 3 | 2,343,181 | 799,403 | 516,930 |
| `"s"` | 2,125,222 (-9.3%) | 714,425 (-10.6%) | 463,375 (-10.4%) |
| `"z"` | 2,029,260 (-13.4%) | 673,661 (-15.7%) | 441,768 (-14.5%) |

| File | Entities | 3 (ms) | `"s"` (ms) | `"z"` (ms) |
| --- | ---: | ---: | ---: | ---: |
| `meshing_coverage.ifc` | 230 | 0.285 (0.279-0.300) | 0.373 (+31%) | 0.430 (+51%) |
| `issue_098_wall_W.ifc` | 1,031 | 0.781 (0.740-0.829) | 1.169 (+50%) | 1.444 (+85%) |
| `shared_point_faceted_brep.ifc` | 6,393 | 3.41 (3.22-3.50) | 5.63 (+65%) | 7.21 (+112%) |
| synthetic, 1,800 walls (1 MB) | 18,008 | 11.7 (10.9-12.0) | 19.5 (+67%) | 23.7 (+103%) |
| synthetic, 18,000 walls (10 MB) | 180,008 | 118.5 (113.6-124.2) | 198.1 (+67%) | 241.8 (+104%) |

Measured 2026-10-03 on an Intel Xeon w7-3565X (20 threads, 63 GiB),
Linux 6.12, Node 22.22.3, load average 1.93 at the start and 2.65 at the
end. The fixtures are from `test/fixtures`; the synthetic files come from
`benchmarks/generate-fixture.awk`.

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
| `model.propertySets(id: bigint): PropertySet[]` | yes | The property sets, quantity sets and predefined property sets that apply to object `id`: its own first, then those inherited from its type object, an occurrence property overriding an inherited one. Values keep their declared IFC type (`typed IFCLENGTHMEASURE(...)`). |
| `model.resolveUnit(measureType: string, unit: bigint \| undefined): ResolvedUnit` | yes | The effective unit of a `measureType` value (`"IFCAREAMEASURE"`): `unit` when given (a property's stated unit), otherwise the project default, resolved exactly to SI. |
| `model.spatialTree(): SpatialTree` | yes | The spatial containment tree: every container with its parent, sub-containers and contained elements. |
| `model.classifications(id: bigint): Classification[]` | yes | The classifications that apply to object `id`: its own, then its type object's. |
| `model.material(id: bigint): MaterialAssignment \| undefined` | yes | The material association that applies to object `id` (its own, or its type object's), or `undefined` when there is none. |
| `model.systems(): Systems` | yes | Every system with its members and served structures, and the memberships the reader could not honour. |
| `model.cost(): Cost` | yes | Every cost schedule and cost item, with values in the tagged encoding. |
| `model.georeferencing(): MapConversion[]` | yes | Every coordinate operation (map conversion) resolved with the project length unit; empty when the model has none. |
| `model.setProperties(edits: PropertyEdit[]): PropertyEditResult` | yes | Write and remove property and quantity values as one checked transaction: every edit, in order, or none, and a refused batch leaves the model unchanged. A write's `value` is the read side's `value`; an inherited value is overridden on the occurrence, never changed on the shared type set. |
| `model.setProperty(object: bigint, set: string, name: string, value: IfcValue, setType: string \| undefined): bigint` | yes | Write one value (`setProperties` with one edit); returns the entity holding it (`bigint`). |
| `model.removeProperty(object: bigint, set: string, name: string): void` | yes | Remove one property from the object's own set (`setProperties` with one edit). |
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
  | "feature-disabled"
  | "invalid-model"
  | "missing-reference"
  | "budget-exceeded"
  | "unsupported"
  | "wrong-entity-type";

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
