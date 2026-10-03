# Rust API

<script setup>
import facts from '../.vitepress/data/facts.json'
</script>

**[Browse the generated API reference →](/api/rustdoc/ifc/index.html){target="_self"}**

Every public item across the {{ facts.crates.total }} workspace crates is
documented, and `missing_docs` is denied workspace-wide, so an undocumented
public item is a build failure rather than a review comment.

The reference is rebuilt from source on every push to `main` and published with
this site. The verification gate builds it with `RUSTDOCFLAGS="-D warnings"`, so
a broken intra-doc link fails CI. Released crates are also on docs.rs; each
crate's [reference page](/reference/) links both. To read the reference
locally:

```bash
cargo doc --workspace --all-features --no-deps --open
```

## Orientation

The generated reference is exhaustive; this page is the map.

### `ifc-model` — start here

| Type | Role |
| --- | --- |
| `Model` | The entity graph. Insert, get, iterate, index by type |
| `Entity` | Type name plus positional attributes |
| `Value` | The encoding-independent value model |
| `EntityId` | Stable in-file identity (`#42`) |
| `Header` | File metadata and the declared schema token |
| `Codec` | The read/write contract every encoding implements |
| `GlobalId` | IFC's base-64 GUID encoding |

`Value` is the type to understand first, because everything in the graph is one:

| Variant | STEP form | Note |
| --- | --- | --- |
| `Null` | `$` | Attribute not set |
| `Derived` | `*` | Derived in a supertype — **distinct from `Null`** |
| `Bool` | `.T.` / `.F.` | |
| `LogicalUnknown` | `.U.` | The third boolean state |
| `Integer`, `Real` | `42`, `2.5` | |
| `Text` | `'wall'` | Already unescaped to UTF-8 |
| `Binary` | `"0123ABC"` | |
| `Enum` | `.ELEMENT.` | Unquoted constant |
| `Ref` | `#42` | Reference to another entity |
| `List` | `(...)` | List, set, array, or bag |
| `Typed` | `IFCLENGTHMEASURE(2.5)` | Typed wrapper |

::: warning `Null` and `Derived` are not the same
Collapsing `*` to `$` corrupts files on write. IFC uses `*` to mean "this
attribute is redeclared as derived in a subtype", which is semantically distinct
from "not set".
:::

### Reading attributes

`Entity` offers positional accessors with typed convenience:

<!-- SNIPPET:api-entity-accessors -->

```rust
let first: Option<&Value> = entity.attribute(0);
let name: Option<&str> = entity.text(2);
let height: Option<f64> = entity.number(3);
let placement: Option<EntityId> = entity.reference(4);
let outgoing: Vec<EntityId> = entity.references(); // every outgoing reference
let is_wall: bool = entity.is_type("IFCWALL");
```

<!-- /SNIPPET -->

Index constants belong in named `*_slot` modules, following the pattern in
`ifc-geometry` — bare numeric literals at call sites are how attribute bugs get
written.

### Querying the model

<!-- SNIPPET:api-model-queries -->

```rust
let walls: &[EntityId] = model.ids_of_type("IFCWALL"); // indexed, not a scan
let pairs: Vec<(EntityId, &Entity)> = model.of_type("IFCWALL").collect();
let histogram: Vec<(&str, usize)> = model.type_histogram(); // good for triage
let dangling: Vec<(EntityId, EntityId)> = model.dangling_references(); // (from, missing)
```

<!-- /SNIPPET -->

Build the optional reverse index only when an operation needs incoming
references:

<!-- SNIPPET:api-reverse-index -->

```rust
use ifc_model::{EntityId, Model, ReverseIndex};

fn print_referrers(model: &Model, target: EntityId) {
    let reverse = ReverseIndex::build(model);
    for hit in reverse.referrers(target) {
        println!(
            "referenced by {:?} in attribute slot {}",
            hit.from, hit.slot
        );
    }
}
```

<!-- /SNIPPET -->

The index is a deterministic snapshot and records the top-level attribute slot
for every referrer. Rebuild it after mutating the model.

::: tip Reverse indexes are deliberately on demand
Codecs that only read and rewrite a model do not pay the memory or load-time
cost. Traversal-heavy applications build the index once and reuse it.
:::

### Codecs

<!-- SNIPPET:api-round-trip -->

```rust
use ifc::{Codec, StepCodec};

let model = StepCodec.read_bytes(bytes)?;
let out = StepCodec.write_bytes(&model)?;
```

<!-- /SNIPPET -->

`XmlCodec` behaves identically behind the `ifcxml` feature. Conversion is a read
with one and a write with the other.

## Reading a file without decoding it

A strict read is already lazy (ADR 0015): `StepCodec.read_bytes` validates
every record but decodes an entity only when it is first accessed, and the
model keeps the file's bytes. Type queries, ids and counts need no decoding;
`Model::decode_all(threads)` decodes the rest in parallel when a consumer is
about to touch everything, and `StepReader::eager()` restores the old
decode-everything read.

| 109 MB Revit IFC, 1.3 M records | time | peak resident |
|---|---|---|
| eager read | 1.38 s | 616 MB |
| lazy read | 0.39 s | 341 MB |
| lazy read + `decode_all(8)` | 0.71 s | 635 MB |

`Index::scan` goes one step further for a census or a subset: it borrows
the bytes, frames every record with `openbim_step::scan` and keeps its id,
span and type, without validating the inside of a record. Framing is
strict -- junk between records or a second appended file is an error.
`Index::entity` decodes one record, `materialize_closure` builds a real
`Model` (with the file's header) from a subset plus everything it
references, and `materialize` gives the raw subset. Decode failures are
returned as errors.

## Scale and memory

A strict read validates every record up front and decodes an entity only
when it is first touched (see [above](#reading-a-file-without-decoding-it)
and [ADR 0015](/adr/0015-strict-step-reads-load-lazily)). Memory is
therefore the file's bytes plus whatever has been decoded. A consumer that
touches every entity pays for a fully decoded model, which is several times
the file size.

`crates/ifc-step/tests/scale.rs` holds a full parse of a large synthetic model
under 10x the input size in resident memory, so a regression fails the
gate. `benchmarks/README.md` records comparative measurements against
IfcOpenShell, with the machine and method they were taken on.

## Other languages

The same core is published for JavaScript and TypeScript (npm), Python
(PyPI) and C. See [Install](/guide/install) and the
[bindings](/bindings/javascript).
