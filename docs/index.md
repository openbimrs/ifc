---
layout: home

hero:
  name: openbim-ifc
  text: Pure-Rust IFC infrastructure
  tagline: An entity graph that round-trips data it does not understand, codecs that never import domain semantics, and geometry that lowers into a neutral kernel. No C++ in the dependency graph.
  image:
    src: /logo.svg
    alt: openbim-ifc
  actions:
    - theme: brand
      text: Get started
      link: /guide/getting-started
    - theme: alt
      text: Capabilities and status
      link: /capabilities
    - theme: alt
      text: View on GitHub
      link: https://github.com/openbimrs/ifc

features:
  - title: Lossless by construction
    details: The model stores entities structurally rather than as domain structs. A file full of cost entities parses and re-exports intact in a build compiled with no cost crate at all — verified by tests, not asserted by prose.
    link: /architecture/
    linkText: How the separations work
  - title: Pay only for what you parse
    details: A thin viewer takes default-features = false, features = ["step"] and compiles no domain code and no geometry stack. Domains and codecs are cargo features over one shared vocabulary.
    link: /guide/getting-started
    linkText: Choosing features
  - title: Honest about what is not built
    details: Several crates are deliberate architecture scaffolds that own module names without implementing behaviour. The capability matrix distinguishes implemented, partial, scaffold, and absent — per entity.
    link: /capabilities
    linkText: Read the matrix
  - title: Geometry without a CAD kernel
    details: ifc-geometry answers what an IFC entity means geometrically and lowers implemented families into the format-neutral Axiolid DAG. It never triangulates, evaluates NURBS, or picks an execution provider.
    link: /architecture/axiolid-boundary
    linkText: The Axiolid boundary
---

## Install

```toml
[dependencies]
openbim-ifc = { git = "https://github.com/openbimrs/ifc.git", rev = "a7c4949bb941504ce874bdec13bd81d33491b5cb" }
```

The workspace crates are not published on crates.io yet. Cargo locks this
immutable Git source in `Cargo.lock`.

The library target is named `ifc`, so call sites read as a facade:

```rust
use ifc::{Codec, Model, StepCodec};

let model = StepCodec.read_bytes(source)?;
println!("{} entities", model.len());
# Ok::<(), ifc::ModelError>(())
```

## Before you build on this

This project publishes a [capability matrix](/capabilities) that separates
**implemented behaviour** from **reserved module ownership**. Some domain crates
are currently scaffolds: files that own a name and a
doc comment so the architecture is reviewable, without implementing the entity.

Read the matrix before scoping work. No capability should be inferred from a
crate name, a module path, or an IFC entity appearing in the schema.

If you are evaluating this crate for a specific application, the
[use cases](/use-cases/) section works an end-to-end scenario against the real
current state of the code, including what the application author must still
build themselves.
