# Rust API

**[Browse the generated API reference →](/ifc/api/rustdoc/ifc/index.html)**

Every public item across the 23 workspace crates is documented and the lint is
enforced: `missing_docs` is denied workspace-wide, so an undocumented public
item is a build failure rather than a review comment.

The reference is rebuilt from source on every push to `main` and published with
this site. The verification gate builds it with `RUSTDOCFLAGS="-D warnings"`, so
a broken intra-doc link fails CI.

The crates are not on docs.rs yet. To read the same reference locally:

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

```rust
entity.attribute(0)   // Option<&Value>
entity.text(2)        // Option<&str>
entity.number(3)      // Option<f64>
entity.reference(4)   // Option<EntityId>
entity.references()   // Vec<EntityId> — all outgoing refs
entity.is_type("IFCWALL")
```

Index constants belong in named `*_slot` modules, following the pattern in
`ifc-geometry` — bare numeric literals at call sites are how attribute bugs get
written.

### Querying the model

```rust
model.ids_of_type("IFCWALL")     // &[EntityId] — indexed, not a scan
model.of_type("IFCWALL")         // Iterator<(EntityId, &Entity)>
model.type_histogram()           // Vec<(&str, usize)> — great for triage
model.dangling_references()      // Vec<(EntityId, EntityId)>
```

Build the optional reverse index only when an operation needs incoming
references:

```rust
use ifc_model::{EntityId, Model, ReverseIndex};

fn print_referrers(model: &Model, target: EntityId) {
    let reverse = ReverseIndex::build(model);
    for hit in reverse.referrers(target) {
        println!("referenced by {:?} in attribute slot {}", hit.from, hit.slot);
    }
}
```

The index is a deterministic snapshot and records the top-level attribute slot
for every referrer. Rebuild it after mutating the model.

::: tip Reverse indexes are deliberately on demand
Codecs that only read and rewrite a model do not pay the memory or load-time
cost. Traversal-heavy applications build the index once and reuse it.
:::

### Codecs

```rust
use ifc::{Codec, StepCodec};

let model = StepCodec.read_bytes(bytes)?;
let out   = StepCodec.write_bytes(&model)?;
```

`XmlCodec` behaves identically behind the `ifcxml` feature. Conversion is a read
with one and a write with the other.

## Reading a file without decoding it

`Index::scan` walks the DATA section for record boundaries and keeps
four columns plus a table of distinct type names. It never decodes an
attribute, so it answers what is in a file for a fraction of the cost of
building a `Model`:

| 529 MB export, 9,000,008 records | time | resident |
|---|---|---|
| `Index::scan` | 0.66 s | 206 MB |
| `StepCodec::read_bytes` | 18.2 s | 2366 MB |


That is 27x faster holding 11x less, because it does not build what the
caller is going to throw away. Use it for a type census, for picking a
subset out of a large file, or to decide whether a full parse is worth it.

`Index::entity` decodes one record on request. `materialize_closure`
builds a real `Model` from a chosen subset plus everything it references,
so the result has no dangling `Ref`. `materialize` gives the raw subset
and will leave references pointing at records it did not include.

The index is not a second parser: `Index::entity` wraps the record bytes
and hands them to the same decoder, and `tests/index_agreement.rs` asserts
on every fixture that scan and parse agree on both ids and content.
## Scale and memory

The model is built eagerly: `read_path` reads the whole file and keeps every
entity in memory. There is no streaming or lazy-loading API. That is a
deliberate trade -- random access by id, reverse indices and checked mutation
all assume the full graph is present -- but it sets a hard ceiling on file
size, so the cost is stated here rather than left to be discovered.

Measured on one machine (x86-64, glibc malloc, release build) with a
synthetic export of 2,000,008 entities in 115 MB, the shape a real building
model has -- placement chains, shape representations, property sets:

| | openbim/ifc | ifcopenshell 0.8.5 |
|---|---|---|
| parse | 2.0 s | 8.2 s |
| resident after parse | 750 MB | 2087 MB |
| RSS / file size | 6.5x | 18.1x |

Both figures are for the same file on the same machine. Records are converted
as the parser emits them, so the generic STEP records never accumulate: an
earlier buffering implementation measured 10.7x and 3.2 s on this file.

**Plan for roughly 6-7x the file size in RAM.** A 500 MB export needs
~3.3 GB and a 2 GB export will not open on a 16 GB machine. If you are
bounded by this, the options today are to split the model upstream or to
run on a larger machine. Parsing already streams; what remains resident is
the model itself, which every consumer API assumes is fully present.

Where the memory goes, for anyone considering a change: at 2M entities the
model holds ~7.2M live heap allocations -- one `Vec` per entity, one per
nested aggregate, one `Arc<str>` per type name. Interning the type names was
implemented and measured: it removes 2M redundant allocations but does not
move resident memory, because the string data is a small share of the total.
The allocation count itself is the cost, and it is structural. Attribute vectors account
for ~231 MB, entity structs ~76 MB, type names ~124 MB (across only 15
distinct names, so they are not interned), and text payload ~19 MB. The
remainder is per-allocation overhead.

Interning the type names looks like the obvious win and is not: it was
implemented and measured at 1236 MB resident against a 1232 MB baseline
on the same 2M-entity file, with parse time unchanged at 3.1 s. The
`Arc<str>` per entity is genuinely deduplicated -- but `openbim-step`
has already allocated a `String` per record name upstream, and every
record is materialised into a `Vec` before conversion begins, so both
representations are live at peak. That double-materialisation, not the
type names, is what sets the ratio. A streaming conversion that consumes
records as they parse is the change that would matter.

`ifc-step/tests/scale.rs` pins the ratio so a regression fails the gate.
## Python and CLI

Neither exists for this repository today. Sibling repositories
([`openbim-idm`](https://openbimrs.github.io/idm/)) ship PyO3 bindings and a
CLI; the same approach applies here when there is demand.
