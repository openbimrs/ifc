# Rust API

The workspace crates are not published on docs.rs yet. Build the generated API
reference from the current source and its doc comments:

```bash
cargo doc --workspace --all-features --no-deps --open
```

The verification gate runs the same documentation build with
`RUSTDOCFLAGS="-D warnings"`, so a broken intra-doc link fails CI.

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
| parse | 3.6 s | 8.2 s |
| peak RSS | 1518 MB | 2087 MB |
| RSS / file size | 13.2x | 18.1x |

Both figures are for the same file on the same machine. The comparison is
included because "13x" alone reads as bad; against the reference C++
implementation it is 2.3x faster using 27% less memory. It is still 13x.

**Plan for roughly 13-15x the file size in RAM.** A 500 MB export needs
~6.5 GB and a 1 GB export will not open on a 16 GB machine. If you are
bounded by this, the options today are to split the model upstream or to
run on a larger machine; a streaming reader is not implemented.

Where the memory goes, for anyone considering a change: at 2M entities the
model holds ~7.2M live heap allocations -- one `Vec` per entity, one per
nested aggregate, one `Arc<str>` per type name. Attribute vectors account
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
