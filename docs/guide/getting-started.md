# Getting started

## Install

```bash
cargo add openbim-ifc
```

The package is `openbim-ifc`, but its **library target is named `ifc`**, so
imports read as a facade: `use ifc::{Codec, StepCodec};`. The short name
`ifc` is taken on crates.io by an unrelated crate, which is why the two
differ. The [Install](/guide/install) page has the current version of every
package, including the JavaScript, Python and C bindings.

## Choosing features

Features are the main design lever. The default is deliberately minimal:
reading and writing STEP, and nothing else. A domain in `default` would
make every downstream build fat. The facade's
[feature table](/reference/crates/openbim-ifc#features) lists every feature
and the crate it enables.

A thin viewer:

```bash
cargo add openbim-ifc --no-default-features --features step
```

compiles no domain code and no geometry stack, while still round-tripping
every entity in the file. `crates/openbim-ifc/tests/thin_build.rs` enforces that
property. It reads the facade's optional dependencies from the manifest, so
a new domain cannot slip into the thin build unnoticed.

::: warning Enabling a domain feature is not the same as capability
A feature compiles a crate; what that crate interprets is bounded. For
example, `style` gives typed presentation and annotation views plus selected
writers, not rendering. `structural` gives analysis-model views, not a
solver. Check the [capability matrix](/capabilities) for the exact surface.
:::

## Reading a file

<!-- SNIPPET:getting-started-read -->

```rust
use ifc::{Codec, StepCodec};

let bytes = std::fs::read(&path)?;
let model = StepCodec.read_bytes(&bytes)?;

println!("schema: {:?}", model.header().schema);
println!("entities: {}", model.len());

for (name, count) in model.type_histogram().iter().take(10) {
    println!("{count:>7}  {name}");
}
```

<!-- /SNIPPET -->

`type_histogram` is a fast way to understand an unfamiliar file before
writing any interpretation code.

## Finding entities

The type index is the supported query path:

<!-- SNIPPET:getting-started-find -->

```rust
// Type names are the upper-case STEP form.
for &id in model.ids_of_type("IFCANNOTATION") {
    let entity = model.get(id).expect("an indexed id resolves");
    // Attributes are positional. IfcAnnotation inherits IfcRoot:
    // 0 = GlobalId, 1 = OwnerHistory, 2 = Name, 3 = Description.
    if let Some(name) = entity.text(2) {
        println!("annotation {id}: {name}");
    }
}
```

<!-- /SNIPPET -->

Attribute-name lookup is available through schema-aware authoring. Incoming
references use the optional, on-demand `ReverseIndex`; see the
[Rust API guide](/api/rust#querying-the-model) for its snapshot semantics.

## Writing a file

<!-- SNIPPET:getting-started-write -->

```rust
let bytes = StepCodec.write_bytes(&model)?;
std::fs::write(&out_path, bytes)?;
```

<!-- /SNIPPET -->

Converting between encodings means reading with one codec and writing with
another, because both implement the same `Codec` trait over the same
`Model`:

<!-- SNIPPET:getting-started-convert -->

```rust
use ifc::{Codec, StepCodec, XmlCodec}; // XmlCodec needs the `ifcxml` feature

let model = StepCodec.read_bytes(step_bytes)?;
let xml = XmlCodec::default().write_bytes(&model)?;
```

<!-- /SNIPPET -->

## Verifying a build

The repository ships one gate that decides on exit codes:

```bash
scripts/gate.sh
```

It runs:
- formatting, a workspace build and the full test suite;
- Clippy and rustdoc, both with `-D warnings`;
- the architecture and context tests, and a feature-combination matrix over
  the facade;
- the three language bindings' test suites;
- every docs check: generated pages, snippets, the docs build and the
  leakage check.

Do not summarise a run by piping `cargo test` through `grep`: the pipe hides
the exit code.

## Next steps

- [Capabilities and status](/capabilities): what is actually implemented.
- [Crate reference](/reference/): every crate, generated from the crate
  itself.
- [Use cases](/use-cases/): end-to-end scenarios against the real code.
- [Architecture](/architecture/): why the model, codecs and domains are
  split.
