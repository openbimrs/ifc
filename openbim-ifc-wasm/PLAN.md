# openbim-ifc-wasm implementation plan

Status: record-model bindings implemented for Node; not yet published.
Last updated: 2026-09-23

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Bounded contract

Parse, inspect, edit and write IFC STEP models from JavaScript through the
record model: entities, attributes as lossless tagged values, exact and
subtype-aware type queries, dangling references. This does not imply schema
validation, domain views, geometry, ifcXML, or a tested browser bundle.

## Planned file map

- `src/lib.rs`: crate contract and re-exports
- `src/error.rs`: `BindingError` -> JS `IfcError`
- `src/value.rs`: `Tagged` <-> JS objects
- `src/model.rs`, `src/model/types.rs`: `IfcModel` and its TypeScript types
- native tests live in `../openbim-ifc-binding-core` (moved there with the
  shared code when the C and Python bindings were added, #38/#39)
- `tests/js/smoke.mjs`, `tests/js/corpus.mjs`: Node suites
- `scripts/build-node-pkg.sh`: pinned-CLI build plus Node suites
- `npm/package.json`: npm manifest

## Work queue

- [x] `WASM-BUILD` - the facade builds for wasm32-unknown-unknown (ahash seed per target)
- [x] `WASM-VALUE` - lossless tagged value encoding, both directions
- [x] `WASM-MODEL` - parse, write, read, edit, add, remove, type queries
- [x] `WASM-SMOKE` - Node smoke and corpus round-trip suites in the gate
- [ ] `WASM-NPM` - publish `@openbim/ifc` to npm (npm org `openbim`; first publish manual, then trusted publishing)
- [ ] `WASM-WEB` - a tested browser/bundler build (`--target web` or `bundler`)
- [ ] `WASM-SIZE` - per-schema features: the three bundled schemas are ~594 KB of a 1.06 MB module
- [ ] `WASM-DOMAIN` - expose domain views (properties, spatial) once the facade API settles

## Completion log

- `WASM-BUILD` - `ahash` 0.8 defaulted to `runtime-rng`, whose getrandom 0.3
  does not build on wasm32-unknown-unknown; this broke even the default
  facade. Native keeps `runtime-rng`; wasm32 uses `compile-time-rng`. The
  gate builds the facade for wasm32 with default and widest pure-Rust
  features.
- `WASM-VALUE/MODEL` - 12 native tests: every value kind round-trips, a
  2^53 + 1 integer keeps every bit, refused inputs change nothing.
- `WASM-SMOKE` - 6 smoke tests plus a round trip of all 46 committed
  fixtures, run through the real wasm module in Node. Mutation proof: 8/8
  mutants killed on a green baseline (`.U.` as false, `*` as `$`, integer
  as f64, NaN accepted, identifier check removed, error code dropped, edit
  ignored, unsafe integral number accepted).
- README example run against `issue_098_wall_W.ifc` (IFC2X3, Revit export):
  finds the `IFCWALLSTANDARDCASE` through the subtype query and renames it.
