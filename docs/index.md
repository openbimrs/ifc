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
  - title: Measured, not claimed
    details: Every capability claim names the file that proves it. The capability matrix, the coverage counts and the crate reference are generated from the code, and the build fails when a page drifts from it.
    link: /coverage
    linkText: What is covered
  - title: Geometry without a CAD kernel
    details: ifc-geometry answers what an IFC entity means geometrically and lowers it into the format-neutral Axiolid DAG. Meshing is an opt-in feature with a swappable backend; the default build compiles no kernel at all.
    link: /architecture/axiolid-boundary
    linkText: The Axiolid boundary
  - title: Rust, JavaScript, Python and C
    details: One core, published to crates.io, npm and PyPI, with a versioned C ABI. A file read in one language reads the same in the others, down to the difference between $ and *.
    link: /guide/install
    linkText: Install
---

## Install

```bash
cargo add openbim-ifc
```

The [Install](/guide/install) page lists the current release of every
package: the Rust crates on crates.io, `@openbim/ifc` on npm and
`openbim-ifc` on PyPI.

The library target is named `ifc`, so call sites read as a facade:

<!-- SNIPPET:index-read -->

```rust
use ifc::{Codec, StepCodec};

let model = StepCodec.read_bytes(source)?;
println!("{} entities", model.len());
```

<!-- /SNIPPET -->

## Before you build on this

Read the [capability matrix](/capabilities) before scoping work. It
separates what each crate implements from what it deliberately leaves to
an application, and each claim cites the file that proves it; its tables
are generated from the code. [Coverage](/coverage) counts the same thing
across the schema.

No capability should be inferred from a crate name, a module path, or an
IFC entity appearing in the schema.

If you are evaluating the stack for a specific application, the
[use cases](/use-cases/) work an end-to-end scenario against the current
state of the code, including what the application author must still build.
