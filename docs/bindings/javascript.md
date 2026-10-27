# JavaScript and TypeScript

`@openbim/ifc` is the WebAssembly build of the IFC core
([`openbim-ifc-wasm`](/reference/crates/openbim-ifc-wasm)), published to npm
with TypeScript declarations. The package carries three builds of one
module, for Node, for bundlers and for browsers without a bundler, and each
is tested from the packed tarball.

```bash
npm install @openbim/ifc
```

Task-sized recipes are in the [JavaScript cookbook](/cookbook/javascript),
and the [playground](/playground) runs the package in this site.

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
| `@openbim/ifc/mesh` in Node | the same, with [meshes](#geometry) and [exact graphs](#exact-geometry-graphs) | on `require`, from disk |
| `@openbim/ifc/mesh` in a bundler | the same, with meshes and graphs | through the bundler |
| `@openbim/ifc/mesh/web` | the same, with meshes and graphs | when `init()` is awaited |

The `mesh` entries are the same API built with the cargo features `mesh`
and `graph`, which add `productMeshes`, `productGeometry` and 2.8 MB to
the module; an application that does not import them downloads nothing of
them ([Geometry](#geometry), [Package size](#package-size)).

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
  // Names resolve against the release the header declares.
  const name = model.attributeByName(wall, "Name"); // { kind: "text", value: "Wall" }
  // A plain value is coerced against the declared type (IfcLabel: text).
  model.setAttributeByName(wall, "Name", `${name.value} (checked)`);
  // An IfcWallTypeEnum item, in any case; an IfcValue is written exactly.
  model.setAttributeByName(wall, "PredefinedType", "partitioning");
}

const out = model.write(); // a Uint8Array, ready to save
```

<!-- /SNIPPET -->

Attributes are addressed by name or by position. `attributeByName(id,
"Name")` and `setAttributeByName(id, "Name", value)` resolve the name
against the release the file's header declares, so the same code reads
`IfcTask.Status` from slot 6 of an IFC2X3 file and slot 7 of an IFC4 one.
`attributeNames(id)` lists every explicit attribute in slot order,
inherited first, as `{ name, index, typeName, optional, aggregate,
derived, declaredBy }`. Names match case-insensitively and come back in
the schema's spelling. An `INVERSE` name (#462) reads through
`attributeByName` too, following the declared release's `FOR` clause
(IFC2X3's `IsDefinedBy` holds type and property relationships, IFC4's
only property ones beside `IsTypedBy`): a `{ kind: "list" }` of `ref`s
in ascending id order for a `SET` or `BAG`, `SET [0:1]` inverses such as
`ContainedInStructure` included (zero or one item, never unwrapped); a
`ref` or `{ kind: "null" }` for a single-valued inverse (`VoidsElements`),
where two referrers throw `invalid-model`. Writing one throws
`inverse-attribute`. `inverseAttributeNames(id)` lists the declarations
as `{ name, entity, forAttribute, aggregate, lower, upper, declaredBy }`,
`inverseReferences(id)` every referrer as `{ id, index }`, and
`totalInverses(id)` counts the distinct referrers. A slot the entity's type derives is listed
with `derived: true`, reads as stored (`{ kind: "derived" }`) and refuses
a write (`derived-attribute`). An entity type the declared release does
not have, or a release the build leaves out, is `unsupported-schema`.
`attribute(id, index)` and `setAttribute(id, index, value)` stay the raw,
release-independent slot access.

`setAttributeByName` takes an `IfcValue`, written exactly, or a plain
value (#342), coerced against the attribute's declared type: a string
becomes a label (written bare, `'x'`) or the enumeration item it names in
any case (`.STANDARD.`); a `bigint` or a safe-integer `number` an
`INTEGER`, or a `REAL` where one is declared; any other `number` a `REAL`;
a boolean a `BOOLEAN` or `LOGICAL`; an array an aggregate, element by
element, an `IfcValue` in it exact; `null` `$`. In a SELECT, the one member
that takes the value is written as its typed parameter
(`IFCDESCRIPTIVEMEASURE('by layer')`). A value that does not fit throws
`type-mismatch`; one several SELECT members take (`"x"` for `IfcValue`)
throws `ambiguous-value`, naming them: write the exact `IfcValue` then. A
reference is written exactly as `{ kind: "ref", id }`; JavaScript has no
entity handle to check.

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
  rule, entity and slot. `validateExpress(express, maxFindings?)` also
  evaluates every WHERE, type and global rule of the declared release from
  `express`, the text of that release's `.exp` file, which you supply (the
  schemas are CC BY-ND, so none is bundled); `indeterminate` counts the
  rules that evaluated to UNKNOWN. The text is parsed on every call. Text
  that declares another schema than the model's, or disagrees with the
  release's tables, throws `express-mismatch`; text that does not parse
  throws `invalid-express`.
- **ifcXML.** `writeIfcXml()` and `IfcModel.parseIfcXml(bytes)` use this
  library's lossless layout; pass `"IFC4"` or `"IFC4X3_ADD2"` for the
  buildingSMART XSD layout of that release, which refuses with `write`
  what it cannot carry exactly.
- **ifcZIP.** `IfcModel.parse` reads an ifcZIP archive, recognized by its
  ZIP signature: its one STEP or ifcXML model (either layout), within a
  decompression budget. `writeIfcZip()` writes STEP inside as `model.ifc`;
  `writeIfcZip("ifcxml", profile)` writes ifcXML as `model.ifcXML`. An
  archive is refused with `parse` when it holds no `.ifc` or `.ifcXML`
  entry at its root or several, `unsupported` for an encrypted entry or a
  compression other than Stored and Deflate, and `budget-exceeded` for an
  entry that would inflate past 4 GiB or 200 times its compressed size.
  The `zip` cargo feature, on by default, carries it.
- **Reachability.** `unreachableProducts()` lists products no viewer will
  draw, each with a stable `reason`.

Validation, ifcXML and the reachability lint are cargo features of
`openbim-ifc-wasm` (`validate`, `validate-express`, `ifcxml`,
`unreachable`), on by default.
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

<!-- SNIPPET:js-property-sets-many -->

```js
const every = model.propertySetsMany(); // every object definition, one pass
const walls = model.propertySetsMany([30n, 31n]); // or the ids given, in order
for (const { object, sets, refusal } of walls) {
  // refusal: the code and message propertySets(object) would throw
  console.log(object, refusal?.code ?? sets.map((set) => set.name));
}
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
  `propertySetsMany(ids?)` (#358) answers for many objects, or with no
  ids every object definition, in one call: the file's property
  relationships are validated once for the call rather than once per
  object, so a pass over every object is linear in the model where a loop
  of `propertySets` is quadratic. Each `ObjectPropertySets` holds exactly
  what `propertySets` returns for its object, or the `refusal` (`code`,
  `message`) it throws; no index outlives the call.
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
(about 205 KB, 65 KB under `gzip -9`), on by default (see
[module size](#module-size)). A `Pset_`/`Qto_` set is checked against the
PSD/QTO catalog, which the module does not embed: load it first.

#### Loading the PSD/QTO catalog

<!-- SNIPPET:js-catalog-load -->

```js
// The module embeds no PSD/QTO catalog: load the release's edition once,
// before the first write to a Pset_ or Qto_ set.
await IfcModel.loadCatalog(model.schema); // reads catalog/ifc4-add2-tc1.bin
```

<!-- /SNIPPET -->

The package ships the catalog as one file per edition, `catalog/<file>`
beside the module, and the module embeds none of it
([#318](https://github.com/openbimrs/ifc/issues/318)).
`IfcModel.loadCatalog(release?, options?)` loads the edition a release
reads: `"IFC2X3"` reads IFC2X3 TC1, `"IFC4"` IFC4 ADD2 TC1, and `"IFC4X3"`
(or `"IFC4X3_ADD2"`) IFC4X3 ADD2, so `model.schema` names the right one;
without a release it loads all three. Each file is checked against the
SHA-256 the module pins, then kept for the module instance, so a second
call, or a concurrent one, reads nothing.

| Edition | File | Bytes | gzip -9 | brotli 11 |
| --- | --- | ---: | ---: | ---: |
| IFC2X3 TC1 | `catalog/ifc2x3-tc1.bin` | 325,279 | 105,382 | 87,630 |
| IFC4 ADD2 TC1 | `catalog/ifc4-add2-tc1.bin` | 1,015,315 | 368,172 | 280,574 |
| IFC4X3 ADD2 | `catalog/ifc4x3-add2.bin` | 1,086,514 | 401,727 | 308,335 |

Where the bytes come from:

- **Node** (either build) reads the file from the package directory.
- **A browser** without a bundler fetches it relative to the module, as
  `init()` fetches the wasm module.
- **A bundler** finds each file as a `new URL("./catalog/...",
  import.meta.url)` asset, emits it and fetches it from there. webpack 5
  does, and the package check bundles with it; a bundler that does not
  handle the pattern for a dependency needs `baseUrl`.
- `{ baseUrl }` fetches `<baseUrl>/<file>` instead, for a CDN or a copy you
  serve yourself; `{ bytes }` (a `Uint8Array` or `ArrayBuffer`, for one
  release) reads nothing. `IfcModel.catalogFile(release)` names the file,
  and `IfcModel.loadCatalogBytes(release, bytes)` is the synchronous load.

The files are not compressed: serve them with HTTP compression (`gzip` or
`brotli`; the sizes are above), as a static host does for `.wasm`.

Until its release is loaded, a write to a `Pset_`/`Qto_` set throws
`catalog-not-loaded` and nothing is written; it is never written
unchecked. Removals and sets without that prefix need no catalog.
`IfcModel.catalogLoaded(release)` tells whether a release is loaded. Bytes
that do not match the pin (another edition's file, another version, a
damaged download) throw `invalid-value` and load nothing; a file that
cannot be read rejects with `io`, and a release without a catalog with
`unsupported-schema`.

A build with `property-catalog` instead of the default
`property-catalog-runtime` embeds every edition (1.4 MB); `loadCatalog`
is then a no-op. A build with neither throws `feature-disabled` for such a
write. The Python and C bindings embed the catalog, so they need no
loading.

Not bound yet: checked multi-edit transactions over arbitrary entities
(`Transaction`, `Applied`, `Conflict`), deferred until a host asks for
them. Use the Rust crates for those.

## GlobalIds, types and openings

`model.byGuid(globalId)` finds an entity by `GlobalId` through an index
built on the first lookup and kept until the model changes; a `GlobalId`
several entities carry throws `invalid-model` naming them all, and
`duplicateGlobalIds()` lists every one. `typeObject(id)` and
`occurrencesOf(type)` read `IfcRelDefinesByType` in every release
(IFC2X3's `IsDefinedBy` included); `openingsOf`, `fillingsOf` and
`voidedElementOf` read `IfcRelVoidsElement` and `IfcRelFillsElement`.
`IfcModel.newGuid()` draws a random version-4 `GlobalId` from the host's
`crypto.getRandomValues` (browsers, Node 19 and later), and
`IfcModel.nameBasedGuid(parts)` derives a reproducible version-8 one. The
[cookbook](/cookbook/javascript#find-entities-by-globalid-types-and-openings)
has a recipe.

## Selectors and values by path

`model.select(query, base?)` returns the ids a
[selector](/guide/selectors) selects, in ascending order.
`model.getValue(id, path)` reads a value by path as an `IfcSelectedValue`
(a property set as `{ kind: "properties", entries }`) and
`model.setValue(id, path, value)` writes one as a single checked
transaction. `IfcModel.canonicalSelector(query)` prints a selector in
canonical form and `IfcModel.selectorGrammarVersion()` names the grammar.
A selector that does not parse throws `invalid-selector` with `span`. The
[cookbook](/cookbook/javascript#select-entities-and-read-values-by-path)
has a recipe.

## Tables

`model.exportTable(query, columns, { format, key, null })` returns the
entities a selector selects as a [table](/guide/tables), a `Uint8Array` of
CSV (the default) or TSV: the key column first (`GlobalId`, or `id` with
`key: "id"`), then one column per value path. `model.importTable(bytes, {
format, key, null, dryRun })` applies a table back by its key as one
checked transaction and returns a `TableImport`: `applied`, every
`TableChange` and every `TableRefusal` by row and column, with `code` and,
for a header that does not parse, `span`. Refusals are data, not
exceptions: any refusal means nothing was written, as does `dryRun`. The
npm package leaves XLSX out (`feature-disabled`); a build of the crate with
`sheet-xlsx` carries it. The
[cookbook](/cookbook/javascript#export-a-table-and-import-it-back) has a
recipe.

## Geometry

Geometry crosses at three levels ([ADR 0021](/adr/0021-bindings-carry-placements-and-opt-in-meshes)):
placements, the exact neutral representation and meshes.
`productPlacements(ids?)` is in every build: for each product with a
shape (or for the `bigint` ids given), its world placement and the Body
representation a viewer draws.

<!-- SNIPPET:js-geometry-placements -->

```js
const model = IfcModel.parse(bytes);
for (const product of model.productPlacements()) {
  if (product.refusal) {
    console.warn(product.id, product.refusal.code, product.refusal.message);
    continue;
  }
  // A column-major 4x4 in metres: three.js reads it with Matrix4.fromArray.
  const [x, y, z] = product.transform.slice(12, 15);
  const body = product.representation; // undefined for an axis-only product
  console.log(product.typeName, [x, y, z], body?.identifier, body?.representationType);
}
```

<!-- /SNIPPET -->

`transform` is a 4x4 column-major matrix in metres, the layout WebGL and
three.js (`Matrix4.fromArray`) read; `representation` names the
`IfcShapeRepresentation` selected as the Body (`identifier`,
`representationType`) and its context (`contextType`,
`contextIdentifier`, `targetView`), and is `undefined` for a product with
an axis or footprint only. A product that cannot be placed or selected is
not a thrown call: its record carries a `refusal` with a `code` --
`unsupported`, `invalid-model`, `missing-reference` or `budget-exceeded`
-- the `entity` at fault and a `message`, and every other product is
still placed. The call itself throws only `unsupported-schema` and,
without the `placements` feature, `feature-disabled`.

`productMeshes(ids?)` adds triangles, compiled by the reference backend of
`ifc-geometry` with a one-millimetre tolerance. It links a geometry
kernel, so it is the cargo feature `mesh`, which the package's default
entry leaves out: there it throws `feature-disabled`. The package's mesh
entry carries it ([#369](https://github.com/openbimrs/ifc/issues/369)):
import `@openbim/ifc/mesh` instead of `@openbim/ifc`, in Node or a
bundler,

<!-- SNIPPET:js-mesh-import -->

```js
import { IfcModel } from "@openbim/ifc/mesh"; // productMeshes included
```

<!-- /SNIPPET -->

or `@openbim/ifc/mesh/web` without a bundler:

<!-- SNIPPET:js-mesh-web-init -->

```js
import init, { IfcModel } from "@openbim/ifc/mesh/web";

await init(); // fetches mesh/web/openbim_ifc_wasm_bg.wasm
```

<!-- /SNIPPET -->

The mesh entry is a separate module instance, with its own `IfcModel`: a
model parsed by one entry is not passed to the other, and each loads its
own PSD/QTO catalog. Then:

<!-- SNIPPET:js-geometry-meshes -->

```js
const model = IfcModel.parse(bytes);
for (const mesh of model.productMeshes()) {
  if (mesh.refusal) {
    console.log(mesh.id, mesh.refusal.code); // e.g. 65n "unsupported"
    continue;
  }
  // positions: Float32Array, x y z per vertex in metres relative to
  // mesh.transform; indices: Uint32Array, three per triangle.
  console.log(mesh.typeName, mesh.positions.length / 3, mesh.indices.length / 3);
}
```

<!-- /SNIPPET -->

Each `ProductMesh` has `positions` (`Float32Array`, `x, y, z` per vertex)
and `indices` (`Uint32Array`, three per triangle), copies owned by
JavaScript, and its product's `transform`. Positions are relative to
that transform, not world coordinates: a site surveyed kilometres from
the origin keeps millimetres in the `f64` matrix that an `f32` vertex
would lose. A product with no Body has empty arrays and no refusal; one
whose lowering or compilation is refused carries the `refusal`, typed as
above.

Each mesh also carries its surface `colour`
([#449](https://github.com/openbimrs/ifc/issues/449)): `red`, `green`,
`blue` and `transparency` (each in `[0, 1]`; `transparency` is
`undefined` when the file states none, which reads as opaque), the
`source` that supplied it, `item`, `layer` or `material` in that
precedence, the `sourceEntity` and the `surfaceStyle`. The colour comes
from the Body item's `IfcStyledItem`, else its styled presentation layer,
else the `IfcMaterialDefinitionRepresentation` of the material the
product (or its type) is made of: an `IfcMaterial`, the one material of a
layer or profile set, or, for a constituent set, the constituent whose
`IfcShapeAspect` names the item. `colour` is `undefined` when nothing
colours the product, and `colourRefusal` says why it is not resolved,
with code `ambiguous` for a set of several materials or Body items of
different colours. A colour never refuses the mesh.

`productCurves(ids?, representation?)` gives the curves of a curve
representation as polylines
([#458](https://github.com/openbimrs/ifc/issues/458)), from the same mesh
entry: an `Axis`, a `FootPrint`, the curves of an `IfcAnnotation`, which
have no area and so no triangles. `representation` picks the
representation as the context selector does: `"Plan"` (the default)
takes the best drawing curves, `Plan`, then `Annotation`, `FootPrint`
and `Axis`, preferring one in a `PLAN_VIEW` context; `"Axis"`,
`"FootPrint"`, `"Annotation"`, `"Body"` or `"Surface"` name one
`RepresentationIdentifier`; any other throws `invalid-value`:

<!-- SNIPPET:js-geometry-curves -->

```js
const model = IfcModel.parse(bytes);
for (const product of model.productCurves(undefined, "Axis")) {
  for (const line of product.polylines) {
    // positions: Float32Array, x y z per vertex in metres relative to
    // product.transform; this curve is a slice of them.
    const xyz = product.positions.subarray(
      3 * line.firstVertex,
      3 * (line.firstVertex + line.vertexCount),
    );
    console.log(product.id, xyz.length / 3, line.closed, line.chordBound);
  }
}
```

<!-- /SNIPPET -->

Each `ProductCurves` has one `positions` `Float32Array`, relative to its
`transform` as a mesh's are, holding every polyline's vertices in turn,
and a `ProductPolyline` per curve: its slice (`firstVertex`,
`vertexCount`), `closed` (the last vertex repeats the first), the IFC
`curve` it came from, and `chordBound`, the reference backend's certified
bound in metres on how far the exact curve lies from the polyline (`0`
for a polyline the file authored), against `chordRequested`, one
millimetre. Draw each slice as a `LINE_STRIP`. Trimmed, composite,
offset and intrinsic curves (an `IfcTrimmedCurve` axis, an
`IfcCompositeCurve` footprint, an IFC4X3 clothoid) compile
([#506](https://github.com/openbimrs/ifc/issues/506)); a curve the
reference compiler has no certified polyline for (an implicit curve, an
offset of an offset) is refused with code `unsupported`, naming the
product. A representation that also holds text or a fill area (a dimension
and its value) gives its curves; each `IfcTextLiteral`,
`IfcTextLiteralWithExtent` and `IfcAnnotationFillArea` of it is listed
by entity id in `skipped`, not drawn and not a refusal
([#512](https://github.com/openbimrs/ifc/issues/512)).

`productMeshes(ids, representation)` meshes a representation other than
the Body the same way, such as `"Surface"`: its surfaces and solids as
one mesh, its curves left to `productCurves`. Every mesh says what it
bounds in `closure`: `"solid"`, `"surface"` (a surface model, a face, a
rectangular trimmed surface: an area and no volume) or `"unknown"` for an
empty or refused mesh. Only a Body mesh carries a colour.

`crates/openbim-ifc-wasm/examples/viewer/` draws a file's meshes with
plain WebGL2 and no build step beyond the module. Its `viewer.mjs` imports
`@openbim/ifc/mesh/web`, and the page's import map says where that is:

- from this repository, `build.sh --serve` builds the mesh module into the
  example's `pkg/` (where the import map points), serves the repository
  and prints the page's URL;
- from the npm package, point the import map at
  `node_modules/@openbim/ifc/mesh/web/openbim_ifc_wasm.js` (or a CDN's copy
  of it) and serve the page, or let a bundler resolve the bare specifier.

Its `scene.mjs` subtracts one scene origin from every transform in `f64`
before handing `f32` matrices to the GPU.

### Exact geometry graphs

`productGeometry(ids?, encoding?)` hands over each product's Body as
Axiolid's neutral geometry graph instead of triangles
([#367](https://github.com/openbimrs/ifc/issues/367)): exact extrusions,
sweeps, B-splines and unevaluated booleans, for a host that evaluates
them with its own kernel. It is the graph the mesh level compiles, so it
links the lowering; it is the cargo feature `graph`, which the default
entry leaves out (`feature-disabled`) and the mesh entry carries.

<!-- SNIPPET:js-geometry-graphs -->

```js
const model = IfcModel.parse(bytes);
for (const product of model.productGeometry(undefined, "object")) {
  if (product.refusal) {
    console.log(product.id, product.refusal.code); // e.g. 65n "unsupported"
    continue;
  }
  if (!product.payload) continue; // no Body: an axis-only product
  // { format: "axiolid-geometry-graph", version: "1.0", graph }, exact,
  // in world coordinates (metres): hand it to your own kernel.
  const { format, version, graph } = product.payload;
  // The lowest wire version the content needs: "1.0"; "1.1" when a
  // station carries a seam-snapping window, "1.2" when a run's sections do,
  // "1.3" when an offset run continues to a basis end the kernel measures,
  // "1.4" when a polygonal bounded half-space clips.
  console.log(product.typeName, format, version, Object.keys(graph.nodes.at(-1))[0]);
}
```

<!-- /SNIPPET -->

The payload is Axiolid's versioned wire format (Axiolid ADR 0085), the
envelope `{ format: "axiolid-geometry-graph", version: "1.0", graph: {
nodes, roots } }`, typed as `GeometryGraphEnvelope`. Its `version` is the lowest the content needs: `1.0` unless a
station carries a seam-snapping window
([#423](https://github.com/openbimrs/ifc/issues/423)), then `1.1`, or a
run's sections do ([#429](https://github.com/openbimrs/ifc/issues/429),
a sectioned solid or surface along such an offset), then `1.2`, or an
offset run continues to a basis end the kernel measures
([#433](https://github.com/openbimrs/ifc/issues/433), an
`IfcOffsetCurveByDistances` along such an offset), then `1.3`, or a
polygonal bounded half-space clips
([#515](https://github.com/openbimrs/ifc/issues/515), an
`IfcPolygonalBoundedHalfSpace` bounded along its `Position`'s Z), then
`1.4` (axiolid/kernel#297, #301, #303, #321). A reader on `axiolid-model` 0.3.8 or
older reads a `1.0` payload and refuses `1.1`; one on 0.3.10 or older
refuses `1.2`, one on 0.3.11 or older `1.3`, one on 0.3.13 or older
`1.4`. Nodes come in
insertion order, which is topological; each is tagged by its kind
(`Profile`, `SolidOperation`, `Instance`, ...) and a reference to another
node is that node's index. The graph is in world coordinates, metres,
with the product's placement already applied; the record's `transform`
is that placement, as `productPlacements` gives it, and is never applied
to the graph again. `encoding` chooses the `payload`:

| `encoding` | `payload` |
| --- | --- |
| `"json"` (default) | the JSON text, a `string` |
| `"object"` | the text parsed, a `GeometryGraphEnvelope` |
| `"cbor"` | the CBOR bytes (RFC 8949), a `Uint8Array`, about a quarter smaller |

`payloadSize` is the payload's length in bytes (UTF-8 for JSON). A
product with no Body has no payload and no refusal; one whose placement
or lowering is refused carries the `refusal`, typed as above. A reader of
the format refuses a newer version, another major, and any kind, variant
or field it does not know; the binding writes at most 1.1, and a major
version of the wire format would be a breaking release of this package
(ADR 0021).

## Creating entities

<!-- SNIPPET:js-authoring -->

```js
const h = IfcModel.handle; // h(i): the entity operation i of the batch produces
const result = model.author([
  { op: "project", attributes: { Name: text("Demo") } }, // 0
  { op: "placement" }, // 1: at the origin
  { op: "spatial", type: "IfcSite", parent: h(0), placement: h(1) }, // 2
  { op: "spatial", type: "IfcBuilding", parent: h(2) }, // 3
  { op: "placement", relativeTo: h(1) }, // 4
  { op: "spatial", type: "IfcBuildingStorey", parent: h(3), placement: h(4) }, // 5
  { op: "typeObject", type: "IfcWallType", attributes: { PredefinedType: token("STANDARD") } }, // 6
  { op: "placement", relativeTo: h(4), location: [1, 2, 0] }, // 7
  {
    op: "product",
    type: "IfcWall",
    container: h(5), // IfcRelContainedInSpatialStructure
    placement: h(7),
    typeObject: h(6), // IfcRelDefinesByType
    attributes: { Name: text("Wall") },
  },
]);
const wall = result.ids[8]; // a bigint; every IfcRoot got a GlobalId
```

<!-- /SNIPPET -->

`model.author(ops)` creates and edits entities as one checked transaction
against the release the header declares: every operation, in order, or, when
any is refused, none, and the model is unchanged. An operation names the
entity an earlier operation of the same batch produced by
`IfcModel.handle(index)` (a `bigint`), anywhere an id goes, attribute values
included, and the result holds per operation the id its entity received.
`createEntity(typeName, attributes)` and `removeWithRelationships(id)` are
the one-operation forms. A model built from nothing needs a header naming
its release first (`setHeader`).

| Operation | What it writes |
| --- | --- |
| `create` | one entity by `type` and named `attributes` |
| `edit` | named `attributes` of `entity`; the whole entity is checked again |
| `remove` | removes `entity` and takes it out of every relationship; a relationship left without an end goes too |
| `project` | the model's one `IfcProject` |
| `spatial` | a spatial element of `type` and its `IfcRelAggregates` under `parent` |
| `product` | a product, its `IfcRelContainedInSpatialStructure` in `container` and its `IfcRelDefinesByType` by `typeObject` |
| `typeObject` | a type object (`IfcWallType`, ...) |
| `assignType`, `contain`, `aggregate` | one relationship; an object already related is refused unless `replace` (see [Assigning types](#assigning-types)) |
| `placement` | an `IfcLocalPlacement` over an `IfcAxis2Placement3D` at `location`, relative to `relativeTo`; `axis` and `refDirection` both or neither |
| `ownerHistory` | an `IfcOwnerHistory` with its person, organization and application |
| `shapeRepresentation` | an `IfcShapeRepresentation` of `items` in `context`, its type checked or inferred ([below](#shape-representations)) |
| `assignRepresentation`, `unassignRepresentation` | a shape representation on or off a product (its `IfcProductDefinitionShape`) or a type (an `IfcRepresentationMap`), by `representation` and `to` or `from` |
| `copyRepresentation`, `removeRepresentation` | a copy of `representation`; its removal with what only it uses |

Every record is built by attribute name through `ifc-author` against the
declared release, and refused with the shared codes: a type the release does
not declare (`unsupported-schema`) or an abstract one (`wrong-entity-type`);
an unknown name (`unknown-attribute`); a value of the wrong type or form, or
an aggregate outside its declared bounds (`invalid-value`); a required
attribute left unset (`missing-attribute`); a derived one set
(`derived-attribute`); a reference to an entity that does not exist
(`missing-reference`) or of a type the attribute does not accept
(`wrong-entity-type`). An object is contained, aggregated and typed once and
a model holds one `IfcProject` (`invalid-model`); a removal an entity other
than a relationship still needs is `still-referenced`.

An `IfcRoot` created without a `GlobalId` gets a fresh one. `OwnerHistory`
is never invented: a builder writes the one it is given on every record it
creates, and IFC2X3, which requires it, refuses a record without
(`missing-attribute`). The module has no entropy of its own, so the seed
fresh `GlobalId`s derive from is drawn from `Math.random`; the TypeScript
declarations type every operation as `AuthorOp`.

## Assigning types

The type assignment propagates the type to each object when asked (#480);
every option is off by default, and the operation then writes only the
`IfcRelDefinesByType`.

| Option | What it does |
| --- | --- |
| `mapRepresentations: true` | maps each of the type's `RepresentationMaps` onto the object: an `IfcShapeRepresentation` in the map's context with its `RepresentationIdentifier`, `RepresentationType` `MappedRepresentation` and one `IfcMappedItem`, added to the object's `IfcProductDefinitionShape` (created when it has none) |
| `mappingTarget` | the `IfcCartesianTransformationOperator` every mapped item uses; by default a fresh identity operator (3D, or 2D for a 2D `MappingOrigin`) per object, so the geometry sits at the object's placement |
| `materialUsage: true` | derives the object's material usage from the type's set: an `IfcMaterialLayerSetUsage` (`DirectionSense` `POSITIVE`, `OffsetFromReferenceLine` 0) for a layer set, an `IfcMaterialProfileSetUsage` (no `CardinalPoint`) for a profile set, one per object in its own `IfcRelAssociatesMaterial`; any other material applies through the type |
| `layerSetDirection` | the layer direction, `"AXIS1"` to `"AXIS3"`; by default the one the IFC documentation fixes for the class: `AXIS2` for `IfcWall`, `AXIS3` for `IfcSlab` and `IfcPlate` |
| `predefinedType` | "ignore" (default) leaves `PredefinedType`; "validate" refuses an object whose own value is set, not `NOTDEFINED`, and differs from the one the type provides (for `USERDEFINED`, `ObjectType` against the type's `ElementType`); "sync" unsets the object's (and its `USERDEFINED` `ObjectType`) where the type provides one, as IFC asks |
| `replace: true` | unassigns the previous type instead of refusing: the object leaves its `IfcRelDefinesByType` (removed when left relating nothing), and the representations mapped from the previous type's maps and the usage derived from its set are removed with what only they use |

A type provides a `PredefinedType` when its own is set and not `NOTDEFINED`;
nothing is synchronised where the object or the type has no such attribute
in the declared release (IFC2X3 `IfcWall`, `IfcDoorStyle`). The containment
and aggregation take `replace` too: the object leaves its previous container
or whole, whose relationship goes when it is left relating nothing.

Refused, with the model unchanged: a representation that clashes with one
the object has (same identifier, or unnamed in the same context) or a
product definition shape another product shares, a contradicting
`PredefinedType`, and an object that already has a material
(`invalid-model`); a map whose representation is not an
`IfcShapeRepresentation`, a type with several material associations or a
usage instead of a set, and a layer set usage for a class with no documented
direction when none is given (`unsupported`).

<!-- SNIPPET:cookbook-js-assign-type -->

```js
// Door 1 (#100) typed again by D-90 (#111) with replace: what the old
// assignment mapped is taken back, the type's representation map is
// mapped afresh, and PredefinedType is left to the type.
model.author([
  { op: "assignType", typeObject: 111n, objects: [100n], mapRepresentations: true, predefinedType: "sync", replace: true },
]);
// A new door, typed the same way in the batch that creates it.
const { ids } = model.author([
  { op: "product", type: "IfcDoor", container: 14n, attributes: { Name: { kind: "text", value: "Door 2" } } },
  { op: "assignType", typeObject: 111n, objects: [IfcModel.handle(0)], mapRepresentations: true },
]);
const shape = model.attributeByName(ids[0], "Representation").id;
const body = model.attributeByName(shape, "Representations").items[0].id;
log(model.attributeByName(body, "RepresentationType").value); // "MappedRepresentation"
// Wall B (#40) moves into the building (#13); without replace the second
// containment is refused with invalid-model.
model.author([{ op: "contain", structure: 13n, elements: [40n], replace: true }]);
```

<!-- /SNIPPET -->

## Shape representations

`shapeRepresentation` assembles an `IfcShapeRepresentation` of existing items (built
in the same batch with `create`) in a geometric representation context or
sub-context. Its `representationType` is checked against the items by the release's
`IfcShapeRepresentationTypes` (`CorrectItemsForType`): a mismatch, items no
listed type admits, a topological item other than a vertex point, edge curve
or face surface, and a missing identifier are `invalid-value`; a label the
release does not list is written as given. Without a type the most specific
one admitting every item is inferred (`SweptSolid`, `AdvancedSweptSolid`,
`Brep`, `Clipping`, `CSG`, `Tessellation`, `Curve2D`, `Curve3D`,
`MappedRepresentation`, ...; IFC2X3 lists fewer). The identifier defaults
to the context's `ContextIdentifier`; IFC4 and IFC4X3 require one.

A shape representation belongs to exactly one product definition shape,
representation map or shape aspect (`IfcShapeModel.WR11`). Assigning adds it
to a product's `IfcProductDefinitionShape` (created when missing; one several
products share is refused) or wraps it in a new `IfcRepresentationMap` of a
type, at the origin; one already used is `invalid-model`. Unassigning takes
it off again (an emptied product shape, or the type's map with its origin,
goes) and leaves it for the batch to assign elsewhere or remove. A copy
copies every item, profile, point, direction and placement it reaches and
the styled items of the copied items, and shares the context, styles,
colours and the type map a mapped item maps; it joins the layers of the
original. Removal takes it off what uses it and removes it with its items
down to what nothing else uses, by the cascade rules below; a type map an
occurrence still maps is `still-referenced`.

`model.removeRepresentation(id, { dryRun })` is the one-operation removal,
returning its `RemovalReport`.

## Removing products and purging

`model.removeProduct(id, options?)` removes a product, or a type object,
with everything it solely owns, as one checked transaction, and returns a
`RemovalReport`: `removed`, `kept` (each with its `reason` and `users`),
`detached`, `rebased` and `applied`. With `{ dryRun: true }` it only
reports.

The cascade decides "solely owned" by counting references within the
same transaction: an entity reached from a removed one goes when nothing
that stays needs it, a relationship that only loses a list member or a
presentation layer assignment not counting. It removes the product's
`ObjectPlacement`, its representation down to the items no one else uses,
the property and quantity sets only it is defined by with their
properties, its material usage (not the material definitions), its
openings (each with the same cascade), its ports, and every relationship
left without an end. It keeps, and reports with a reason, what other
entities still use (`shared`, with the users: a property set another wall
uses, a door type's representation maps, a style or point used elsewhere),
objects of their own (`related-object`: the type, the container, the
fillings of a removed opening) and definitions it never deletes
(`definition`: material definitions, classification references, units,
contexts, the owner history).

It throws `still-referenced` while other placements that stay are relative
to the product's placement (a window placed relative to the opening of the
wall being removed); `{ rebasePlacements: true }` re-bases them onto the
removed placement's parent instead, composing the transforms so nothing
moves. A type object that still types objects throws `still-referenced`
unless `{ untypeOccurrences: true }`.

`model.purge()` removes every resource entity no `IfcRoot` reaches, nor
annotates something reached (a styled item, a layer assignment, a shape
aspect); `model.purge({ classes: ["IfcPropertySet"], includeRoots: true })`
removes the unreferenced entities of the named classes instead. `IfcRoot`s
and the owner history with its actors stay unless asked (`includeRoots`,
`includeOwnerHistory`); `dryRun` reports without removing. The
[cookbook](/cookbook/javascript#remove-a-product-and-purge) has a recipe.

`model.remove(id)` is deprecated (`@deprecated` in the declarations): it
leaves references dangling. Use `removeWithRelationships(id)` for one
entity or `removeProduct(id)` for a product.

## Copying products and appending

`model.copyProduct(id)` copies a product, or a type object, with what
it solely owns, as one checked transaction, and returns a `CopyReport`:
`copy`, `copied` (each `CopiedEntity` with `original`, `copy` and `typeName`), `reused`, `joined`, `skipped`, `regenerated` and `applied`. With `{ dryRun: true }` it only reports.

What the copy owns is what `removeProduct` would remove with the product, by the
same reference count, and the copy copies it: the product (with a new
`GlobalId`), its `ObjectPlacement` (relative to the same parent, moved by
`offset: [x, y, z]` in the parent's frame, or replaced by `placement`), its
representation (items and profiles copied; the context, styles and a
type's representation maps shared), the property and quantity sets only it
is defined by with their properties, its material usage (not the material
definitions), its ports, and its openings and the doors and windows
filling them, placed relative to the copied placements. A type object
copies its own representation maps and property sets. What the removal
would keep is shared: the copy joins the original's type assignment, the
shared property sets, its classification, document and library
references, groups and presentation layers, its spatial container and
the whole it is a part of. `openings: false`, `fillings: false`, `container: false` and `aggregate: false` leave openings, fillings, the container
or the whole out. Relationships that would bind another object to the
copy (a connection, a space boundary, the parts of a whole, a type's
occurrences) are not carried and are listed in `skipped`. Entities a
`UNIQUE` rule binds (an `IfcPropertyEnumeration`, an `IfcApplication`)
are always shared.

`model.append(source, ids)` copies entities of another model into this one with what belongs
to them: what they reference (a product's placement chain, representation
and owner history), their property sets, type, material and
classification with their relationships, their openings, fillings and
ports, and what attaches to their geometry (styled items, layers). A
relationship to an object that is not copied (the source's spatial
container) is skipped; relate the copies afterwards. Units, contexts, the
owner history, materials and styles equal to one this model holds (same
type, same values all the way down) are reused instead of copied
(`reuse` names the kinds: `units`, `contexts`, `ownerHistory`, `materials`, `styles`). `GlobalId`s are kept (`globalIds: "keep"`, the default, refused when this model holds
one), merged (`"merge"`: an `IfcRoot` this model holds by `GlobalId` and
type is that entity, and its relationships are joined), or regenerated
(`"regenerate"`, or `"regenerate-colliding"` for those this model holds). Between releases each attribute moves by name; a type,
attribute or enumeration constant the target release lacks is refused
with `unsupported-schema`, never dropped. Models whose length or plane
angle units differ are refused with `unsupported`: convert one with
`convertLengthUnit` first. The
[cookbook](/cookbook/javascript#copy-a-product-and-append-from-another-model) has a recipe.

In a batch, the `copyProduct` and `copyEntity` operations do the same (the copy is the operation's
produced entity), and a low-level deep copy follows an entity's references
by an explicit policy: classes always copied, classes always shared, and
whether other `IfcRoot`s and definitions are copied (by default shared).

## Units and a model's length unit

`IfcModel.convertUnit(value, from, to)` converts between units named as
text: an SI unit with its prefix written before it (`"METRE"`,
`"MILLIMETRE"`, `"MILLI METRE"`, `"SQUARE_MILLIMETRE"`, `"KILOGRAM"`) or a
named unit of IFC4 Table 673 with its exact legal factor (`"inch"`,
`"foot"`, `"square foot"`, `"gallon US"`, `"pound"` ...). Units of
different dimensions throw `type-mismatch`; a unit with an offset (degree
Fahrenheit) throws `unsupported` until #110 is settled.
`model.convertUnitValue(value, from, to)` converts between two unit
entities of the model. `IfcModel.formatLength(value, unit, { style,
places, displayUnit, suppressZeroInches })` writes a length as decimal text
or as feet and inches with fractions (`5' - 6 1/2"`).

`model.convertLengthUnit(target, { areaAndVolume, removeReplacedUnits,
dryRun })` assigns a new project length unit (an SI `METRE` with any
prefix, or `"inch"`, `"foot"`, `"yard"`, `"mile"`, `"US survey foot"`) and
rescales every length, area and volume value as one checked transaction;
values with a unit of their own keep it. The `LengthUnitReport` lists what
was rescaled (`converted`), what was kept and why (`kept`), the factors and
the units written and removed; `dryRun` only reports. A value the
conversion cannot carry refuses the whole conversion with `unsupported`,
naming each. The
[cookbook](/cookbook/javascript#convert-units-and-a-model-s-length-unit) has
a recipe.

## Undo and redo

History is off until `model.setHistoryLimit(n)` bounds the undo entries (0
turns it off and drops what is held); then every method that changes the model
is one entry, `author` for its whole batch. `model.history()` returns a
`HistoryState` (`limit`, `undo`, `redo`). `model.undo()` restores the model to
the state before the last entry, entities under their ids and at their file
positions, the header included, and returns a `HistoryChange` (`created`,
`removed`, `modified`, `header`); `model.redo()` reverses an undo, and
`model.clearHistory()` drops the entries. With nothing to step through, or
while history is off, they throw `history-empty`; after a change made outside
the history, `history-stale`, with the model unchanged. Undoing a creation
frees its id again.


## Module size

Size of the module after `wasm-bindgen` (0.2.128), without `wasm-opt`,
built with `cargo build -p openbim-ifc-wasm --target wasm32-unknown-unknown
--release`: the default, or `--no-default-features --features <features>`.

| Features | Raw | gzip -9 | brotli 11 |
| --- | ---: | ---: | ---: |
| default: five releases, every capability and domain, the writer, the catalog loaded at runtime | 2,648,193 | 894,392 | 586,376 |
| the same, the catalog embedded (`property-catalog`, format 3) | 4,062,769 | 1,439,501 | 977,880 |
| default before #318: the catalog embedded, format 2 | 6,282,871 | 1,855,125 | 956,057 |
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
| `ifc4,property-catalog` (brings `properties-write`) | 2,649,010 | 1,026,914 | 770,665 |
| `ifc4,property-catalog-runtime` (brings `properties-write`) | 1,235,600 | 482,197 | 379,155 |
| `ifc4` + the three capabilities | 1,303,035 | 510,415 | 398,226 |
| `ifc4` + the seven domains | 1,266,205 | 487,576 | 378,385 |
| `ifc4` + every capability and domain | 1,760,329 | 656,425 | 499,492 |
| the same + `properties-write` | 1,963,827 | 719,252 | 544,142 |
| the same + `property-catalog` | 3,470,078 | 1,296,903 | 959,002 |
| the same + `property-catalog-runtime` | 2,055,520 | 751,465 | 566,302 |

The `ifc4` and the last three rows, and the default, were measured with
the writer of [#123](https://github.com/openbimrs/ifc/issues/123); the
other single-feature rows before it. Its methods stay in every build, as
a left-out feature's do, which costs about 10 KB (`ifc4` was 759,820
bytes). The writer itself adds about 205 KB (65 KB under `gzip -9`).

The PSD/QTO catalog rows were measured with
[#318](https://github.com/openbimrs/ifc/issues/318). Loaded at runtime,
the default, the catalog adds 90 KB to the module (31 KB under `gzip -9`,
22 KB under brotli): the snapshot decoder, the SHA-256 pin check and the
writer's catalog checks; its data comes in the per-edition files of
[Loading the PSD/QTO catalog](#loading-the-psd-qto-catalog), only the one a
release needs. Embedded (`property-catalog`), it carries the IFC2X3, IFC4
and IFC4X3 catalogs whatever releases the build names: 1.4 MB in the
compact format of [#317](https://github.com/openbimrs/ifc/issues/317)
(0.55 MB under `gzip -9`), where the bincode snapshots before it took
3.7 MB (0.99 MB). Under brotli the two embedded formats are within 2% of
each other: the compact format removes repetition that brotli's window
already found.

Every capability and domain links only the schema tables of the releases
the build names ([#306](https://github.com/openbimrs/ifc/issues/306)).
Before, each linked all five: `ifc4,validate` was 1,521,685 bytes and
`ifc4` with everything was 2,342,546, the size of the five-release
default.

Geometry ([#328](https://github.com/openbimrs/ifc/issues/328)), measured
on 2026-10-08 on the same toolchain; the default now includes
`placements`:

| Features | Raw | gzip -9 | brotli 11 |
| --- | ---: | ---: | ---: |
| default without `placements` | 2,867,002 | 963,660 | 629,682 |
| default | 2,909,468 | 978,432 | 640,878 |
| default + `mesh` | 5,052,660 | 1,735,307 | 1,150,357 |
| `ifc4` | 815,769 | 342,047 | 274,746 |
| `ifc4,placements` | 875,610 | 364,579 | 290,761 |
| `ifc4,mesh` (brings `placements`) | 3,626,301 | 1,262,911 | 832,210 |
| `ifc4` + every capability and domain | 2,018,760 | 736,249 | 552,157 |
| the same + `placements` | 2,062,414 | 750,806 | 563,166 |
| the same + `mesh` | 4,802,334 | 1,651,262 | 1,092,238 |

Placements cost 42 KB in the default module (15 KB under `gzip -9`), less
than in an IFC4-only one (60 KB) because the default's reachability lint
already links representation selection. Meshes add the reference compiler
and its boolean engine: 2.1 MB to the default (757 KB under `gzip -9`,
509 KB under brotli), which is why the package's default entry leaves them
out and a separate entry carries them.

Exact geometry graphs ([#367](https://github.com/openbimrs/ifc/issues/367)),
measured on 2026-10-10 on the same toolchain (`gzip -9` from the `gzip`
command, brotli 11 from `node:zlib`):

| Features | Raw | gzip -9 | brotli 11 |
| --- | ---: | ---: | ---: |
| default | 2,962,323 | 995,024 | 655,098 |
| default + `graph` | 4,021,564 | 1,336,245 | 868,377 |
| default + `mesh` | 5,506,168 | 1,880,291 | 1,253,132 |
| default + `mesh,graph` (the mesh entry) | 5,792,007 | 1,968,040 | 1,299,307 |
| `ifc4` | 844,192 | 351,925 | 282,661 |
| `ifc4,placements` | 933,537 | 383,533 | 305,674 |
| `ifc4,graph` (brings `placements`) | 2,578,117 | 863,269 | 543,599 |
| `ifc4,mesh` | 4,077,559 | 1,406,808 | 934,350 |
| `ifc4,mesh,graph` | 4,360,873 | 1,493,165 | 979,988 |

On its own, `graph` adds 1.06 MB to the default module (341 KB under
`gzip -9`, 213 KB under brotli), against Level 1's 15 KB: it links the
lowering into Axiolid's graph, the representation crates, `ifc-alignment`
(which still links every release's schema table, so an IFC4-only build
grows by 1.64 MB) and the serde encoders of the wire format. That is why
it is opt-in. Beside `mesh`, which links the lowering already, it adds
286 KB (88 KB under `gzip -9`, 46 KB under brotli), so the mesh entry
carries both.

EXPRESS rules ([#538](https://github.com/openbimrs/ifc/issues/538)),
measured on 2026-10-26 the same way (`gzip -9` from the `gzip` command,
brotli 11 from `node:zlib`), at the commit that added `validate-express`:

| Features | Raw | gzip -9 | brotli 11 |
| --- | ---: | ---: | ---: |
| default without `validate-express` | 4,057,299 | 1,387,141 | 936,472 |
| default | 4,364,373 | 1,491,016 | 1,007,194 |
| `ifc4,validate` | 1,458,404 | 562,413 | 434,546 |
| `ifc4,validate,validate-express` | 1,765,081 | 665,314 | 507,062 |

The EXPRESS evaluator adds 307 KB to the default module (104 KB under
`gzip -9`, 71 KB under brotli), so the default carries it and a size-trimmed
build can leave `validate-express` out; `validateExpress` then throws
`feature-disabled`. The earlier tables were measured before later domains
and geometry grew the default; compare rows within a table.

### Package size

The npm package carries two entries ([#369](https://github.com/openbimrs/ifc/issues/369)),
each one module bound for three targets (Node, bundler, web), plus the
PSD/QTO catalog files they share. Measured on 2026-10-08 from `npm pack`
of `scripts/build-npm-pkg.sh`'s output (version 0.4.1 plus #369), which
`tools/check-package.mjs` prints on every run; module sizes after
`wasm-bindgen`, compressed with `node:zlib` at `gzip` level 9 and brotli
quality 11:

| Module | Raw | gzip -9 | brotli 11 |
| --- | ---: | ---: | ---: |
| default entry (`@openbim/ifc`, `/bundler`, `/web`) | 2,940,278 | 988,129 | 647,986 |
| mesh entry (`@openbim/ifc/mesh`, `/mesh/bundler`, `/mesh/web`) | 5,101,651 | 1,753,297 | 1,163,026 |

With [#367](https://github.com/openbimrs/ifc/issues/367) the mesh entry
carries `graph` too: on 2026-10-10 (version 0.4.3 plus #367) its module is
5,792,007 bytes and the default entry's 2,962,323, and the tarball is
9,920,911 bytes packed and 29,293,897 unpacked, the mesh entry's three
targets 17,673,507 of that.

| Package | Tarball | Unpacked | Files |
| --- | ---: | ---: | ---: |
| without the mesh entry | 3,883,556 | 11,525,818 | 21 |
| with the mesh entry (published) | 9,177,796 | 27,100,319 | 36 |

Of the unpacked size, the default entry's three targets take 9,086,124
bytes, the mesh entry's (`mesh/`) 15,573,958 and the catalog files
2,427,580. The mesh entry costs an `npm install` 5.3 MB more to download;
it costs an application nothing unless it imports the entry, since a
bundler or a page loads only the module it imports. Each target carries
its own copy of its entry's module (the three are byte-identical), as the
default entry always has.

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
`crates/openbim-ifc-wasm/src/model.rs`. The
[TypeDoc reference](/api/typedoc/index.html){target="_self"} documents every
class, interface and type the package's `.d.ts` declares.

<!-- API:JS:BEGIN -->

| Member | Throws `IfcError` | Description |
| --- | --- | --- |
| `new IfcModel()` |  | An empty model. |
| `IfcModel.parse(bytes: Uint8Array): IfcModel` | yes | Parse a STEP (`.ifc`) file from its bytes, or an ifcZIP archive holding STEP or ifcXML, recognized by its ZIP signature and read within a decompression budget (`budget-exceeded` past it). |
| `IfcModel.parseWithOptions(bytes: Uint8Array, options: ParseOptions): IfcModel` | yes | Parse a STEP file under explicit read options. With `onMalformed: "skip"` a damaged record is dropped and reported by `diagnostics()` instead of failing the read; omitted options are strict. |
| `IfcModel.parseIfcXml(bytes: Uint8Array, profile: string \| undefined): IfcModel` | yes | Parse an ifcXML document: the library's lossless layout without a profile, or the buildingSMART XSD layout of `"IFC4"` or `"IFC4X3_ADD2"`. |
| `model.write(): Uint8Array` | yes | Serialize as STEP bytes. |
| `model.writeIfcXml(profile: string \| undefined): Uint8Array` | yes | Serialize as ifcXML bytes, in the layout `parseIfcXml` reads. |
| `model.writeIfcZip(inner: string \| undefined, profile: string \| undefined): Uint8Array` | yes | Serialize as ifcZIP bytes: one Deflate entry holding STEP (`model.ifc`), or with `inner` `"ifcxml"` ifcXML (`model.ifcXML`) in the layout `profile` names, as for `writeIfcXml`. `parse` reads it back. |
| `model.header(): IfcHeader` |  | The STEP file header: description, name, time stamp, author, organization, preprocessor, originating system, authorization and schema tokens. |
| `model.setHeader(header: IfcHeader): void` | yes | Replace the STEP file header; every field is required. |
| `model.unreachableProducts(): UnreachableProduct[]` | yes | Products no viewer will draw (outside the spatial structure, or with geometry only in non-model contexts), with a stable `reason`. |
| `model.productPlacements(ids: bigint[] \| BigUint64Array \| undefined): ProductPlacement[]` | yes | Each product's world placement (a column-major 4x4 in metres) and the Body representation a viewer draws, for `ids` or, without, for every product with a shape. A product that cannot be placed is a record with a typed `refusal`; the call itself throws only `unsupported-schema` or `feature-disabled` (feature `placements`). |
| `model.productGeometry(ids: bigint[] \| BigUint64Array \| undefined, encoding: GeometryPayloadEncoding \| undefined): ProductGeometry[]` | yes | Each product's Body as Axiolid's neutral geometry graph (#367), exact (extrusions, sweeps, B-splines, unevaluated booleans), for `ids` or, without, every product with a shape, in Axiolid's versioned wire format: `{ format: "axiolid-geometry-graph", version: "1.0", graph: { nodes, roots } }` (the lowest version the content needs, `"1.0"` to `"1.4"`), in world coordinates, metres. `encoding` picks the `payload`: `"json"` (the default) the text, `"object"` the text parsed, `"cbor"` the CBOR bytes as a `Uint8Array`. A product that cannot be lowered has a typed `refusal`; the call itself throws only `unsupported-schema`, `feature-disabled` (feature `graph`) or `invalid-value` for an unknown encoding. |
| `model.productMeshes(ids: bigint[] \| BigUint64Array \| undefined, representation: RepresentationName \| undefined): ProductMesh[]` | yes | Each product's Body as triangles from the reference backend: `positions` (`Float32Array`, metres, relative to `transform`) and `indices` (`Uint32Array`), for `ids` or, without, every product with a shape. A product that cannot be meshed has a typed `refusal`. `representation` meshes another one instead (#458), such as `"Surface"`; its curves are left to `productCurves`, and `closure` says `"solid"` or `"surface"`. Only a Body mesh carries a colour. Opt-in: a build without the `mesh` feature (the npm package's default entry) throws `feature-disabled`; import `@openbim/ifc/mesh` for it. |
| `model.productCurves(ids: bigint[] \| BigUint64Array \| undefined, representation: RepresentationName \| undefined): ProductCurves[]` | yes | Each product's curves as polylines from the reference backend (#458), for `ids` or, without, every product with a shape: `positions` (`Float32Array`, metres, relative to `transform`), every polyline's vertices in turn, and per curve its slice, whether it is closed and its certified chord bound. `representation` is `"Plan"` by default (the best drawing curves), or `"Axis"`, `"FootPrint"`, `"Annotation"`, `"Body"`, `"Surface"`; another throws `invalid-value`. Trimmed, composite, offset and intrinsic curves compile (#506); a curve the reference compiler has no certified polyline for (an implicit curve, an offset of an offset) is an `unsupported` refusal naming the product. Text literals and annotation fill areas beside the curves (a dimension's value) are listed in `skipped`, not refused (#512). Opt-in with the meshes: import `@openbim/ifc/mesh`. |
| `model.propertySets(id: bigint): PropertySet[]` | yes | The property sets, quantity sets and predefined property sets that apply to object `id`: its own first, then those inherited from its type object, an occurrence property overriding an inherited one. Values keep their declared IFC type (`typed IFCLENGTHMEASURE(...)`). |
| `model.propertySetsMany(ids: bigint[] \| BigUint64Array \| undefined): ObjectPropertySets[]` | yes | The property sets of each of `ids`, in that order, or, with no ids, of every object definition (`IfcObjectDefinition` and its subtypes) in file order, in one pass (#358): the file's property relationships are validated once for the call, so resolving every object is linear in the model. Each `ObjectPropertySets` holds exactly what `propertySets` returns for its object, or, in `refusal`, the code and message it throws; only a refusal of the whole model throws. No index outlives the call. |
| `model.resolveUnit(measureType: string, unit: bigint \| undefined): ResolvedUnit` | yes | The effective unit of a `measureType` value (`"IFCAREAMEASURE"`): `unit` when given (a property's stated unit), otherwise the project default, resolved exactly to SI. |
| `model.spatialTree(): SpatialTree` | yes | The spatial containment tree: every container with its parent, sub-containers and contained elements. |
| `model.classifications(id: bigint): Classification[]` | yes | The classifications that apply to object `id`: its own, then its type object's. |
| `model.material(id: bigint): MaterialAssignment \| undefined` | yes | The material association that applies to object `id` (its own, or its type object's), or `undefined` when there is none. |
| `model.systems(): Systems` | yes | Every system with its members and served structures, and the memberships the reader could not honour. |
| `model.cost(): Cost` | yes | Every cost schedule and cost item, with values in the tagged encoding. |
| `model.georeferencing(): MapConversion[]` | yes | Every coordinate operation (map conversion) resolved with the project length unit; empty when the model has none. IFC2X3 is read from its `ePSet_MapConversion` and `ePSet_ProjectedCRS` property sets. |
| `model.setProperties(edits: PropertyEdit[]): PropertyEditResult` | yes | Write and remove property and quantity values as one checked transaction: every edit, in order, or none, and a refused batch leaves the model unchanged. A write's `value` is the read side's `value`; an inherited value is overridden on the occurrence, never changed on the shared type set. |
| `model.setProperty(object: bigint, set: string, name: string, value: IfcValue, setType: string \| undefined): bigint` | yes | Write one value (`setProperties` with one edit); returns the entity holding it (`bigint`). |
| `IfcModel.catalogFile(release: string): string` | yes | The file name of `release`'s PSD/QTO catalog snapshot, such as `"ifc4x3-add2.bin"`; the package ships it as `catalog/<name>`. `release` is a header schema token: `"IFC2X3"`, `"IFC4"` or `"IFC4X3"` (`"IFC4X3_ADD2"`). |
| `IfcModel.catalogLoaded(release: string): boolean` | yes | Whether `release`'s catalog is loaded in this module instance, so a write to its `Pset_`/`Qto_` sets can be checked. |
| `IfcModel.loadCatalogBytes(release: string, bytes: Uint8Array): void` | yes | Load `release`'s catalog from the bytes of its snapshot file, checked against the pinned SHA-256 (`invalid-value` otherwise). The synchronous half of `IfcModel.loadCatalog`, for a host that reads the file itself. |
| `model.removeProperty(object: bigint, set: string, name: string): void` | yes | Remove one property from the object's own set (`setProperties` with one edit). |
| `model.size: number` |  | Number of entities. |
| `model.schema: string \| undefined` |  | The first `FILE_SCHEMA` token, e.g. `"IFC4"`, or `undefined`. |
| `model.diagnostics(): string[]` |  | Non-fatal problems found while reading. |
| `model.ids(): bigint[]` |  | Every entity id (`bigint`), in file order. |
| `model.idsOfType(typeName: string): bigint[]` |  | Ids of every entity of exactly `typeName`, case-insensitive. |
| `model.idsOfTypeIncludingSubtypes(typeName: string): bigint[]` | yes | Ids of every entity of `typeName` or any subtype, per the file's declared schema: `IfcWall` also finds `IFCWALLSTANDARDCASE`. |
| `model.byGuid(globalId: string): bigint \| undefined` | yes | The id of the entity whose `GlobalId` is `globalId`, or `undefined`. |
| `model.duplicateGlobalIds(): DuplicateGlobalId[]` | yes | Every `GlobalId` several entities carry, with its holders in file order; empty for a conformant file. |
| `model.typeObject(id: bigint): bigint \| undefined` | yes | The type object of occurrence `id` (`IfcRelDefinesByType`), or `undefined`. Throws `wrong-entity-type` for a non-`IfcObject` and `invalid-model` when the file states two types. |
| `model.occurrencesOf(id: bigint): bigint[]` | yes | Every object type object `id` types, in file order. |
| `model.openingsOf(id: bigint): bigint[]` | yes | Every opening voiding element `id` (`IfcRelVoidsElement`). |
| `model.fillingsOf(id: bigint): bigint[]` | yes | Every element filling opening `id` (`IfcRelFillsElement`). |
| `model.voidedElementOf(id: bigint): bigint \| undefined` | yes | The element opening `id` voids, or `undefined`. |
| `IfcModel.newGuid(): string` | yes | A new random `GlobalId`: an RFC 9562 version-4 UUID from the host's `crypto.getRandomValues`, in IFC's 22-character form. Throws `unsupported` on a host without it (Node 18). |
| `IfcModel.nameBasedGuid(parts: string[]): string` |  | The `GlobalId` named by `parts` (RFC 9562 version 8): the same parts give the same `GlobalId` in every binding and release. |
| `model.typeOf(id: bigint): string` | yes | The upper-case type name of entity `id`. |
| `model.attributes(id: bigint): IfcValue[]` | yes | Every attribute of entity `id`, as tagged values. |
| `model.attribute(id: bigint, slot: number): IfcValue` | yes | Attribute `index` of entity `id`, as a tagged value. |
| `model.setAttribute(id: bigint, slot: number, value: IfcValue): IfcValue` | yes | Set attribute `index` of entity `id`; returns the previous value. |
| `model.attributeNames(id: bigint): AttributeInfo[]` | yes | Every explicit attribute of entity `id` in slot order, inherited first, as the release the header declares defines them. |
| `model.attributeByName(id: bigint, name: string): IfcValue` | yes | Attribute `name` of entity `id` (case-insensitive, e.g. `"Name"`), resolved against the declared release, as a tagged value. |
| `model.setAttributeByName(id: bigint, name: string, value: IfcValue \| IfcPlainValue): IfcValue` | yes | Set attribute `name` of entity `id`; returns the previous value. A derived attribute is refused (`derived-attribute`), an `INVERSE` one (`inverse-attribute`). |
| `model.add(typeName: string, attributes: IfcValue[]): bigint` | yes | Append an entity; returns its id (`bigint`). |
| `model.remove(id: bigint): void` | yes | Remove entity `id`, leaving references to it dangling. |
| `model.author(ops: AuthorOp[]): AuthoringResult` | yes | Apply authoring operations as one checked transaction against the release the header declares: every operation, in order, or none, and a refused batch leaves the model unchanged. An operation names the entity an earlier one produced by `IfcModel.handle(index)`. `result.ids` holds, per operation, the id of the entity it produced. |
| `model.createEntity(typeName: string, attributes: Record<string, IfcValue>): bigint` | yes | Create one entity of `typeName` from named attributes, checked against the declared release (`author` with one `create`); an `IfcRoot` without a `GlobalId` gets one. Returns its id (`bigint`). |
| `model.removeWithRelationships(id: bigint): void` | yes | Remove entity `id` with its relationships, leaving nothing dangling (`author` with one `remove`); refused with `still-referenced` while an entity other than a relationship needs it. |
| `IfcModel.handle(index: number): bigint` |  | The handle of the entity operation `index` of an `author` batch produces, usable wherever a later operation takes an id (`bigint`). |
| `model.danglingReferences(): [bigint, bigint][]` |  | Every `[from, to]` pair (`bigint`s) where `to` does not exist. |
| `model.copyProduct(id: bigint, options: CopyProductOptions \| undefined): CopyReport` | yes | Copy product (or type object) `id` with everything it solely owns, as one checked transaction: what `removeProduct` would remove with it is copied (its placement, representation, own property sets, material usage, ports and, unless `openings` or `fillings` is `false`, its openings and the doors and windows filling them), what it would keep is shared, the copy joining the relationship (its type, shared property sets, classification and, unless `container` or `aggregate` is `false`, its container and whole). `offset` moves the copy in its parent placement's frame; `placement` places it by that `IfcObjectPlacement` instead. The report maps every original to its copy; with `dryRun` nothing changes. Refused with `wrong-entity-type` for neither a product nor a type object, `invalid-value` for both `offset` and `placement`, `unsupported` for an offset onto a placement that is not a local placement with a Cartesian location, or a moved copy whose openings are not placed relative to it. |
| `model.append(source: IfcModel, ids: bigint[] \| BigUint64Array, options: AppendOptions \| undefined): CopyReport` | yes | Copy entities `ids` of `source` into this model with what belongs to them, as one checked transaction: what they reference, their property sets, type, material and classification, their openings, fillings and ports, and what attaches to their geometry. Units, contexts, the owner history, materials and styles equal to this model's are reused (`reuse` names the kinds; all by default). Relationships to objects not copied (a spatial container) are skipped and reported. `globalIds`: `keep` (the default; one this model holds is refused), `merge` (an `IfcRoot` this model holds by `GlobalId` and type is reused, its relationships joined), `regenerate` or `regenerate-colliding`. Refused with `unsupported` when the models' length or plane angle units differ, `unsupported-schema` for an entity this model's release cannot hold, `invalid-value` for a kept `GlobalId` this model holds. |
| `model.setHistoryLimit(limit: number): void` | yes | Keep at most `limit` undo entries; `0` (the default) turns history off and drops what is held. While it is on, every method that changes the model is one entry (`author` is one entry for its whole batch). Refused with `invalid-value` for a `limit` that is not a whole number of at least 0 that fits the platform. |
| `model.history(): HistoryState` |  | The history bound and how many entries each direction holds. |
| `model.undo(): HistoryChange` | yes | Restore the model to the state before the last change: entities under their ids and at their file positions, the header included. Undoing a creation frees its id again. Refused with `history-empty` when there is nothing to undo (also while history is off) and `history-stale` when the model changed outside the history; the model is unchanged then. |
| `model.redo(): HistoryChange` | yes | Reverse the last `undo`. Refused with `history-empty` when there is nothing to redo and `history-stale` as `undo` is. |
| `model.clearHistory(): void` |  | Drop every undo and redo entry, keeping the limit. |
| `model.inverseAttributeNames(id: bigint): InverseAttributeInfo[]` | yes | Every `INVERSE` attribute of entity `id`, inherited ones first, as the declared release defines them (#462). |
| `model.inverseReferences(id: bigint): InverseReference[]` | yes | Every reference to entity `id`: the referencing entity and the attribute index it references through, in ascending order (#462). |
| `model.totalInverses(id: bigint): number` | yes | How many distinct entities reference entity `id` (#462). |
| `model.removeProduct(id: bigint, options: RemoveProductOptions \| undefined): RemovalReport` | yes | Remove product (or type object) `id` with everything it solely owns -- its placement, representation, own property and quantity sets, material usage, openings and ports, and the relationships left without an end -- as one checked transaction, keeping what is shared (a property set another object uses, a type's representation maps). The report says what went, what stayed and why; with `dryRun` nothing changes. Refused with `still-referenced` while other placements are relative to its placement (unless `rebasePlacements`) or a type object types objects (unless `untypeOccurrences`), `wrong-entity-type` for neither a product nor a type object, `unsupported` for a placement that cannot be re-based. |
| `model.purge(options: PurgeOptions \| undefined): RemovalReport` | yes | Remove unreferenced entities as one checked transaction; with `dryRun`, only report. Without `classes`, every resource entity no `IfcRoot` reaches (nor annotates something reached); with them, the entities of those classes nothing references, an `IfcRoot` only with `includeRoots`. The owner history stays unless `includeOwnerHistory`. Refused with `unsupported-schema` for a class the release does not declare. |
| `model.removeRepresentation(id: bigint, options: RemoveRepresentationOptions \| undefined): RemovalReport` | yes | Remove shape representation `id` with what only it uses (#481), as one checked transaction: taken off its product definition shape (which goes when left empty), representation map or shape aspects, then removed with its items down to what nothing else uses. Shared items, profiles, points and styles stay, and the report says why; with `dryRun` nothing changes. Refused with `still-referenced` for a representation map an `IfcMappedItem` still maps, `wrong-entity-type` for an entity that is not an `IfcShapeModel`. |
| `model.select(query: string, base: bigint[] \| BigUint64Array \| undefined): bigint[]` | yes | The ids selector `query` selects, ascending (#464); with `base`, from those entities alone. |
| `model.getValue(id: bigint, path: string): IfcSelectedValue` | yes | The value `path` reads from entity `id`: `"type.Name"`, `"Pset_WallCommon.FireRating"`, `"materials.Name"`, `"occurrences.count"`. |
| `model.setValue(id: bigint, path: string, value: IfcValue \| IfcPlainValue): void` | yes | Write `value` at `path` of entity `id` as one checked transaction: an attribute coerced as `setAttributeByName` coerces a plain value, or a property from a typed value, or a plain one when the property already states its type. A refused write changes nothing. |
| `IfcModel.canonicalSelector(query: string): string` | yes | Selector `query` in its canonical form: spacing, quoting and aliases normalised. Throws `invalid-selector`. |
| `IfcModel.selectorGrammarVersion(): string` |  | The version of the selector grammar this build parses (`"1.0"`). |
| `model.exportTable(query: string, columns: string[], options: TableOptions \| undefined): Uint8Array` | yes | The entities selector `query` selects as a table (#474), as bytes: `options.format` `"csv"` (default), `"tsv"` or `"xlsx"` (only in a build with `sheet-xlsx`; the npm package throws `feature-disabled`), the key column first (`"GlobalId"`, or `"id"` with `options.key`), then one column per value path of `columns` (`"Name"`, `"type.Name"`, `"Pset_WallCommon.FireRating"`), each cell the value the path reads as text, `NULL` as `options.null` (empty by default). Throws `invalid-selector` for a query or path that does not parse, `invalid-value` for an unknown format or key, and `wrong-entity-type` for an entity without a `GlobalId` in a table keyed by it. |
| `model.importTable(data: Uint8Array, options: TableImportOptions \| undefined): TableImport` | yes | Apply a table (`Uint8Array`) back to the model by its key column (#474), as one checked transaction: every other cell is written by its header's value path, typed by the attribute's declared type or the property's stated one; a cell that reads what the model holds changes nothing, so an exported table imported unchanged is a no-op. The answer lists every change and every refusal by row and column; any refusal means nothing was written, as does `options.dryRun`. Throws only for a call that cannot start: an unknown format or key, bytes that are no table of the format. |
| `IfcModel.convertUnit(value: number, from: string, to: string): number` | yes | Convert `value` from the unit named `from` to the unit named `to`: an SI unit with its prefix (`METRE`, `MILLIMETRE`, `MILLI METRE`, `SQUARE_MILLIMETRE`, `KILOGRAM`) or a named unit of IFC4 Table 673 with exact factors (`inch`, `foot`, `square foot`, `gallon US`, `pound` ...). Throws `type-mismatch` for units of different dimensions, `invalid-value` for an unknown unit, `unsupported` for a unit with an offset (degree Fahrenheit, #110). |
| `IfcModel.formatLength(value: number, unit: string, options: LengthFormatOptions \| undefined): string` | yes | Write `value`, a length in the unit named `unit`, as text: decimal in `displayUnit`, or feet and inches with fractions (`5' - 6 1/2"`). |
| `model.convertUnitValue(value: number, from: bigint, to: bigint): number` | yes | Convert `value` from unit entity `from` to unit entity `to` of this model (`IfcSIUnit`, `IfcConversionBasedUnit`, `IfcDerivedUnit`). |
| `model.convertLengthUnit(target: string, options: LengthUnitOptions \| undefined): LengthUnitReport` | yes | Convert the model's length unit to the unit named `target` (an SI `METRE` with any prefix, or `inch`, `foot`, `yard`, `mile`, `US survey foot`), rescaling every length, area and volume value as one checked transaction; with `dryRun`, only report. Values with a unit of their own keep it; values the conversion cannot carry (a p-curve off a plane, a polynomial curve, a parameter on an undefined parameter space) refuse the whole conversion with `unsupported`, each named. |
| `model.validate(maxFindings: number \| undefined): ValidationReport` | yes | Validate against the schema the header declares; findings are sorted by severity, rule, entity and slot. `maxFindings` caps the report (default 10,000) and sets `truncated` when reached. |
| `model.validateExpress(express: string, maxFindings: number \| undefined): ValidationReport` | yes | Validate with the EXPRESS rules of the declared release, read from `express`, the text of that release's `.exp` file (#538). Throws `express-mismatch` for text that declares another schema or disagrees with the release's tables, and `invalid-express` for text that does not parse. `maxFindings` is as for `validate`. |

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

/**
 * A plain value for `IfcModel.setAttributeByName` (#342), coerced against
 * the attribute's declared type: a string (a label, or the enumeration
 * item it names), a number or bigint (an integer, or a real), a boolean,
 * `null` (`$`), or an array of these or of `IfcValue`s (an aggregate). An
 * `IfcValue` is written exactly.
 */
export type IfcPlainValue =
  | string
  | number
  | bigint
  | boolean
  | null
  | ReadonlyArray<IfcPlainValue | IfcValue>;

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
  | "wrong-entity-type"
  | "template-violation"
  | "missing-property"
  | "catalog-not-loaded"
  | "unknown-attribute"
  | "derived-attribute"
  | "missing-attribute"
  | "still-referenced"
  | "type-mismatch"
  | "ambiguous-value"
  | "invalid-selector"
  | "inverse-attribute"
  | "express-mismatch"
  | "invalid-express"
  | "history-empty"
  | "history-stale";

/**
 * A value `IfcModel.getValue` reads by path (#464): a tagged value, a list
 * of them, or a property set's members as `[name, value]` pairs.
 */
export type IfcSelectedValue =
  | IfcValue
  | { kind: "list"; items: IfcSelectedValue[] }
  | { kind: "properties"; entries: [string, IfcSelectedValue][] };

/**
 * Where `IfcModel.loadCatalog` reads a catalog snapshot from. By default
 * Node reads `catalog/<file>` from the package directory, and a browser or
 * bundle fetches it relative to the module (`new URL(..., import.meta.url)`).
 */
export interface CatalogLoadOptions {
  /** The snapshot bytes themselves, for one release; nothing is read. */
  bytes?: Uint8Array | ArrayBuffer;
  /** A directory URL holding the `*.bin` files, used instead of the package's. */
  baseUrl?: string | URL;
}

export declare namespace IfcModel {
  /**
   * Load the PSD/QTO catalog of `release` (`"IFC2X3"`, `"IFC4"`,
   * `"IFC4X3"`), or of all three when omitted, into this module instance.
   * Each edition is read once, checked against its pinned SHA-256 and
   * cached; loading it again is a no-op. Until its release is loaded, a
   * write to a `Pset_`/`Qto_` set throws `catalog-not-loaded`.
   */
  function loadCatalog(release?: string, options?: CatalogLoadOptions): Promise<void>;
}

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
  severity:
    | "error"
    | "evaluation-error"
    | "warning"
    | "indeterminate"
    | "unsupported";
  rule: string;
  entity: bigint | undefined;
  attributeIndex: number | undefined;
  attributeName: string | undefined;
  path: string;
  message: string;
}

/** The result of `IfcModel.validate` or `IfcModel.validateExpress`. */
export interface ValidationReport {
  /** No errors and no evaluation errors; unsupported rules do not count. */
  conformant: boolean;
  /** The finding budget was reached: counts are lower bounds. */
  truncated: boolean;
  errors: number;
  evaluationErrors: number;
  warnings: number;
  /** EXPRESS rules that evaluated to UNKNOWN; 0 without EXPRESS rules. */
  indeterminate: number;
  unsupported: number;
  findings: ValidationFinding[];
}

/** A `GlobalId` several entities carry, from `IfcModel.duplicateGlobalIds`. */
export interface DuplicateGlobalId {
  globalId: string;
  /** Its holders, in file order. */
  ids: bigint[];
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

/** One explicit attribute of an entity, from `IfcModel.attributeNames`. */
export interface AttributeInfo {
  /** The declared name in the schema's spelling, e.g. `GlobalId`. */
  name: string;
  /** Its slot: the `index` of `attribute` and `setAttribute`. */
  index: number;
  /** The declared type, or an aggregate's element type. */
  typeName: string;
  /** `{ kind: "null" }` is a valid value. */
  optional: boolean;
  /** A `LIST`, `SET`, `BAG` or `ARRAY`. */
  aggregate: boolean;
  /** Derived for this entity: written `*`, refused by `setAttributeByName`. */
  derived: boolean;
  /** The entity that declares the attribute, e.g. `IfcRoot`. */
  declaredBy: string;
}

/** One `INVERSE` attribute of an entity, from `IfcModel.inverseAttributeNames`. */
export interface InverseAttributeInfo {
  /** The declared name in the schema's spelling, e.g. `IsDefinedBy`. */
  name: string;
  /** The referencing entity type before `FOR`; its subtypes count too. */
  entity: string;
  /** The attribute of `entity` after `FOR`, e.g. `RelatedObjects`. */
  forAttribute: string;
  /** A `SET` or `BAG` (read as a `list`), not a single `ref`. */
  aggregate: boolean;
  /** The declared lower bound; `1` for a single-valued inverse. */
  lower: number;
  /** The declared upper bound, `undefined` for `?`. */
  upper?: number;
  /** The entity that declares the inverse, e.g. `IfcObject`. */
  declaredBy: string;
}

/** One reference to an entity, from `IfcModel.inverseReferences`. */
export interface InverseReference {
  /** The referencing entity. */
  id: bigint;
  /** The attribute index of `id` holding the reference. */
  index: number;
}
```

<!-- API:JS:END -->
