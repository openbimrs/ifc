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

## Python and CLI

Neither exists for this repository today. Sibling repositories
([`openbim-idm`](https://openbimrs.github.io/idm/)) ship PyO3 bindings and a
CLI; the same approach applies here when there is demand.
