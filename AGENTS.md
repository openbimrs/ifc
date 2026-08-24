# packages/ifc/ instructions

Applies to `packages/ifc/**`. Read the nearest deeper `AGENTS.md` before
editing a crate or complex module; deeper files add local rules and do not
repeat this file.

## Context protocol

`AGENTS.md` is stable ambient context: purpose, boundaries, invariants, and
gates. `PLAN.md` is implementation state. Read a plan only when assigned
roadmap work, architecture review, or a blocked dependency. When finishing a
plan item, check it off, add the proof command/result, and record newly found
follow-up work there. Do not put progress logs or speculative TODOs in
`AGENTS.md`.

## Package role

This package interprets and serializes IFC. IFC resource names are evidence,
not crate boundaries: the schema mixes storage, geometry input, presentation,
and domain semantics. Partition code by role in the pipeline (ADR 0008):

1. Geometry input is lowered by `ifc-geometry`, `ifc-alignment`, or
   `ifc-georef` into format-neutral geometry values.
2. Domain semantics are borrowed projections over `ifc-model` in crates such
   as `ifc-material`, `ifc-properties`, and `ifc-style`.
3. Geometry-derived outputs such as area and volume are computed outside the
   semantic crate, then written through that crate by an application service.

One IFC entity may therefore have projections in two crates. `ifc-model` owns
the record; neither projection owns or duplicates it.

## Dependency tiers

```text
L0 record core       ifc-model
L1 schema metadata   ifc-schema                 -> ifc-model only when needed
L1 codecs            ifc-step, ifc-xml          -> ifc-model
L2 domain views       ifc-{material,...,cost}    -> ifc-model
L2 validation         ifc-validate               -> ifc-model + ifc-schema
L2 geometry bridges   ifc-geometry/alignment/georef
                                               -> ifc-model + neutral axiolid crates
L3 facade             ifc                       -> selected L1/L2 crates
L4 orchestration      apps / openbim / bindings (outside this package)
```

Dependencies point down. Sibling domain crates do not depend on one another.
Cross-domain workflows belong in L4. Codecs never import domain semantics;
domain crates never import codecs. `ifc-model` remains schema-, codec-, and
domain-agnostic.


## Geometry boundary

- IFC adapters resolve source units, source references, representation choice,
  and IFC placement semantics.
- They preserve exact intent in neutral geometry values/DAG nodes. They do not
  tessellate, heal, execute booleans, or select CPU/GPU implementations.
- Source IDs and diagnostics stay in an IFC-side provenance table; they do not
  leak into generic geometry values.
- Only `ifc-geometry`, `ifc-alignment`, and `ifc-georef` may depend on neutral
  `axiolid-*` representation/contract crates. No IFC crate may depend on a
  concrete geometry backend.
- `ifc-style` may reference the ID of a representation item but never changes
  its shape. `ifc-properties` stores quantities but never computes geometry.

## Model and view rules

- Typed projections borrow `&Model`; do not copy the entity graph into owned
  domain objects.
- Absolute STEP slot indices include inherited attributes. Cite the EXPRESS
  declaration or generated manifest beside non-obvious slot constants.
- Unknown entities and unknown attributes must survive codec round trips.
- Traversal is iterative or explicitly budgeted. Detect reference cycles where
  the schema permits malformed cyclic files.
- Mutation must be transactional enough that failed validation cannot leave a
  half-written model. Do not add ad hoc setters to borrowed read views.
- Unsupported, invalid, missing-reference, and budget-exceeded are distinct
  structured errors. Never silently substitute geometry or semantics.

## Module and API rules

- `lib.rs` delegates and re-exports; implementation belongs in modules.
- Split data/view definitions, resolution, mutation, validation, and traversal
  before they grow together. Prefer roughly 500 lines; 800 is the hard gate.
- Scaffold modules may document ownership without inventing a public API.
  Keep child modules crate-private until a real public type is implemented and
  deliberately re-exported by its parent.
- Every `.rs` file must be in the compiled module tree. Future-only file names
  belong in `PLAN.md`, not as orphan source files.
- Public values implement `Debug` and `Clone`; derive stronger traits only when
  semantically honest. Mark extensible public errors/enums non-exhaustive.

## Authoritative evidence

Use the checked-out official EXPRESS and HTML docs under
`references/ifc-spec/`; never make schema claims from memory. Generated
manifests may be committed, but references are read-only and never a build
dependency. IFC4 ADD2 TC1 is the current geometry baseline; version-specific
behavior must be explicit rather than folded into guessed common behavior.

## Gates

Run targeted tests while iterating. Before an IFC-wide merge:

```bash
cargo test -p ifc-model --test package_architecture
cargo test -p ifc-model --test progressive_context
cargo test -p ifc-model --test module_reachability
cargo test -p ifc-model --test no_monolithic_files
cargo test -p ifc-geometry --test declaration_manifest
cargo test -p ifc-geometry --test no_backend_dependency
scripts/gate.sh
```

Architecture and context gates must be mutation-verified before being trusted.
On shared master, stage only owned paths and re-check HEAD before committing.
