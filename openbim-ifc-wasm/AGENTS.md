# openbim-ifc-wasm instructions

Purpose: WebAssembly bindings for the `openbim-ifc` facade, so JavaScript
and TypeScript can read, edit and write IFC STEP files (#34, ADR 0013).

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

- Production dependencies: `openbim-ifc` (the only IFC crate allowed),
  `wasm-bindgen`, `js-sys`. Never an `ifc-*` crate directly;
  `ifc-model/tests/package_architecture.rs` enforces this.
- Glue only. No IFC semantics, validation or domain logic here. If the
  bindings need behaviour the facade lacks, add it to the facade first.
- Exception, narrowly: checks that stop the STEP writer emitting a malformed
  file (identifier syntax, finite reals, nesting depth) live in
  `src/value/`, because they guard the JS input boundary, not IFC meaning.

## Layout

- `src/value/tagged.rs`: `Tagged`, the host-independent lossless encoding,
  and its total `Value -> Tagged` / checked `Tagged -> Value` mapping.
- `src/value/js.rs` (wasm32 only): `Tagged` <-> JS objects.
- `src/model.rs`: `IfcModel` operations as plain Rust, tested natively.
- `src/model/js.rs` (wasm32 only): the `#[wasm_bindgen]` surface.
- `src/model/types.rs` (wasm32 only): TypeScript `IfcValue` declarations.
- `npm/package.json`: npm manifest; its version must equal `Cargo.toml`'s.
- `tests/js/`: Node smoke and corpus round-trip suites.

## Invariants

- Every `Value` round-trips: `$`/`*`, `.U.`/`.F.`, integer/real, typed
  wrappers and 64-bit integers (`bigint`) stay distinct.
- A refused JS input is an `IfcError` with a stable `code` and leaves the
  model unchanged; never a panic, never a coerced value.
- Keep native logic out of `*/js.rs` so `cargo test` covers it without a
  JS host.

## Verification

```sh
cargo test -p openbim-ifc-wasm
openbim-ifc-wasm/scripts/build-node-pkg.sh   # wasm build + Node suites
```

`scripts/gate.sh` runs both. The wasm-bindgen CLI must equal the crate pin
exactly; the build script refuses a mismatch and prints the install command.
