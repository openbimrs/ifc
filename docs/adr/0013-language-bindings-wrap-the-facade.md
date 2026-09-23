# 0013 — Language bindings wrap the facade, one crate per target

- **Status:** Accepted
- **Date:** 2026-09-23
- **Deciders:** Friedrich Schrödter
- **Supersedes:** —

## Context

The IFC crates reach only Rust consumers (#34). JavaScript, Python and native
hosts need the same model — parse a file, read and edit entities, write it
back — without adopting Rust types or Rust's unstable ABI. The three targets
differ in calling convention and ownership, but not in what they expose.

Axiolid already ships a versioned C ABI (its ADR 0040). IFC bindings should
feel the same to a host that loads both.

## Decision

We will expose the IFC layer to other languages through **one binding crate
per target**, each depending **only on `openbim-ifc`**, never on an `ifc-*`
crate directly.

- `openbim-ifc-wasm` (browser and Node, via `wasm-bindgen`) is first.
  `openbim-ifc-capi` (C ABI) and a Python package follow as separate crates.
- A binding adds calling-convention glue only: no IFC semantics, no
  validation, no domain logic. If a binding needs behaviour the facade lacks,
  the behaviour goes into the facade first.
- The surface is the record model — `Model`, `Entity`, `Value`, the STEP
  codec — not the domain crates. Domain projections are a later, opt-in layer.
- `Value` crosses the boundary as a **lossless tagged encoding** (a JS object
  per variant), not as native numbers and strings. IFC distinguishes `$`
  from `*`, `.U.` from `.F.`, integer from real, and typed wrappers such as
  `IFCLENGTHMEASURE(2.5)` from their payload. Folding any of these into a
  host-native value would make a round trip silently lossy.
- Integers cross as JS `BigInt`, because IFC integers are 64-bit and JS
  numbers lose precision above 2^53.
- Every binding has an executable round-trip smoke test in its host language
  that parses a file, reads an entity, edits one, and writes the file back.

## Alternatives considered

| Option | Why not |
| --- | --- |
| One crate exporting all three targets behind features | Each target pulls its own toolchain (`wasm-bindgen`, `pyo3`, `cbindgen`) and its own `crate-type`; mixing them makes every build pay for all three and every change risk all three. |
| Bind the `ifc-*` crates directly | Duplicates the facade's feature selection per target and lets a binding bypass the layering rules the facade enforces. |
| Python and WASM as thin layers over the C ABI | Adds a second unsafe boundary and a manual memory protocol to hosts that already have safe, idiomatic Rust binding generators. |
| Map `Value` to plain host values (number, string, null) | Lossy: `$`/`*`, `.U.`, int/real and typed wrappers do not survive. |

## Consequences

**Positive**

- A consumer of any binding sees the same model the Rust facade does.
- Binding crates can version and release independently of the IFC crates.
- The layering rules keep holding: bindings are a new outermost layer.

**Negative / costs**

- The tagged `Value` encoding is more verbose than host-native values.
- WASM builds need `wasm32-unknown-unknown`-compatible dependencies; the
  gate checks this so a native-only dependency cannot slip in.

**Follow-ups / risks to watch**

- C ABI and Python are tracked as separate issues under #34.
- npm publication needs a package name and registry credentials; until then
  the WASM package is built and tested but not published.

## Relation to existing code

- `openbim-ifc-wasm/`
- `openbim-ifc/` (the only allowed dependency)
- `ifc-model/tests/package_architecture.rs` (enforces the dependency rule)
