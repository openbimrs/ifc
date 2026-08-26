# Getting started

## Install

```toml
[dependencies]
openbim-ifc = "0.1"
```

The published crate is `openbim-ifc`; its **library target is named `ifc`**, so
imports read as a facade:

```rust
use ifc::{Codec, Model, StepCodec};
```

The short name `ifc` is taken on crates.io by an unrelated crate, which is why
the package and the library name differ.

## Choosing features

Features are the main design lever in this crate. The default is deliberately
minimal — reading STEP and nothing else — because a domain in `default` would
make every downstream build fat.

| Feature | Pulls in | For |
| --- | --- | --- |
| `step` *(default)* | `ifc-step` | Reading and writing `.ifc` |
| `ifcxml` | `ifc-xml` | Reading and writing `.ifcxml` |
| `schema` | `ifc-schema` | Subtype queries, conformant XML names |
| `author` | `ifc-author` (+ `schema`) | Schema-checked construction of new entities |
| `spatial` | `ifc-spatial` | Containment tree and relationship traversal |
| `geometry` | `ifc-geometry` | Lowering representations to neutral geometry |
| `material`, `cost`, `properties`, … | one domain crate each | Interpreting that domain |
| `material-templates` | `ifc-material` + template catalog | Material PSD applicability |
| `codecs` | both codecs | |
| `domains` | every domain view | |
| `full` | everything | |

A thin viewer:

```toml
openbim-ifc = { version = "0.1", default-features = false, features = ["step"] }
```

compiles no domain code and no geometry stack, while still round-tripping every
entity in the file. That property is enforced by
`openbim-ifc/tests/thin_build.rs`, not left to convention.

::: warning Enabling a domain feature is not the same as capability
Several domain crates are architecture scaffolds with no behaviour. Turning on
`features = ["style"]` compiles `ifc-style`, which currently reserves module
names without implementing them. Check the
[capability matrix](/capabilities) first.
:::

## Reading a file

```rust
use ifc::{Codec, StepCodec};

let bytes = std::fs::read("model.ifc")?;
let model = StepCodec.read_bytes(&bytes)?;

println!("schema: {:?}", model.header().schema_token());
println!("entities: {}", model.len());

for (name, count) in model.type_histogram().iter().take(10) {
    println!("{count:>7}  {name}");
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

`type_histogram` is a fast way to understand an unfamiliar file before writing
any interpretation code.

## Finding entities

The type index is the supported query path:

```rust
# use ifc::{Codec, Model, StepCodec};
# let model = Model::new();
// Type names are the upper-case STEP form.
for id in model.ids_of_type("IFCWALL") {
    let entity = model.get(id).expect("indexed id resolves");
    // Attributes are positional. IfcWall inherits IfcRoot: 0 = GlobalId,
    // 1 = OwnerHistory, 2 = Name, 3 = Description.
    if let Some(name) = entity.text(2) {
        println!("wall {id:?}: {name}");
    }
}
```

There is no attribute-name lookup and no reverse-reference index; see
[capabilities](/capabilities) for what that costs you.

## Writing a file

```rust
use ifc::{Codec, StepCodec};
# use ifc::Model;
# let model = Model::new();

let bytes = StepCodec.write_bytes(&model)?;
std::fs::write("out.ifc", bytes)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Converting between encodings is reading with one codec and writing with another,
because both implement the same `Codec` trait over the same `Model`:

```rust,ignore
let model = StepCodec.read_bytes(&step_bytes)?;
let xml = XmlCodec.write_bytes(&model)?;   // requires the `ifcxml` feature
```

## Verifying a build

The repository ships one gate that decides on exit codes:

```bash
scripts/gate.sh
```

It runs formatting, a workspace build, the full test suite, Clippy with
`-D warnings`, rustdoc with `-D warnings`, the architecture and progressive-context
tests, and a feature-combination matrix over the facade crate.

Do not summarise a run by piping `cargo test` through `grep` — the pipe hides the
exit code.

## Next steps

- [Capabilities and status](/capabilities) — what is actually implemented.
- [Use cases](/use-cases/) — end-to-end scenarios against the real code.
- [Architecture](/architecture/) — why the model, codecs, and domains are split.
