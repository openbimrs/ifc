# Rust API

Full generated API documentation lives on **docs.rs**, built automatically from
the doc comments in the source on every publish:

- [`openbim-ifc`](https://docs.rs/openbim-ifc) — the facade (imports as `ifc`)
- [`ifc-model`](https://docs.rs/ifc-model) — the entity graph
- [`ifc-step`](https://docs.rs/ifc-step) · [`ifc-xml`](https://docs.rs/ifc-xml) — codecs
- [`ifc-schema`](https://docs.rs/ifc-schema) — EXPRESS metadata
- [`ifc-geometry`](https://docs.rs/ifc-geometry) — geometry bridge
- [`ifc-cost`](https://docs.rs/ifc-cost) · [`ifc-material`](https://docs.rs/ifc-material) — domain views

::: tip Building it locally
Generated docs for unpublished work:

```bash
cargo doc --workspace --all-features --no-deps --open
```

The verification gate builds documentation with `RUSTDOCFLAGS="-D warnings"`, so
a broken intra-doc link fails CI.
:::

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

::: warning No reverse index yet
`ids_of_type` is indexed and fast. The inverse — "which entities reference
*this* one" — is [not implemented](/project/roadmap#r2-relationship-and-spatial-traversal);
`index/reverse.rs` is a scaffold. Applications needing it currently build their
own by iterating once and inverting `entity.references()`.
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
