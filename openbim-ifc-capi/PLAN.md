# openbim-ifc-capi implementation plan

Status: ABI 0.1 implemented and tested from C and C++; not yet packaged.
Last updated: 2026-09-23

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Bounded contract

A versioned, allocation-neutral C ABI over the binding core: the same
record-model surface as the WASM and Python bindings. This does not imply
a CMake package, prebuilt binaries, domain views, or ABI stability before a
0.1 release is tagged.

## Planned file map

- `src/lib.rs`: crate contract and re-exports
- `src/status.rs`: statuses, version, panic boundary
- `src/buffer.rs`: audited pointer helpers
- `src/registry.rs`: handle table
- `src/tape.rs`, `src/tape/tests.rs`: value tape and its tests
- `src/model.rs`, `src/model/tests.rs`: exports and boundary tests
- `src/errors.rs`: last-error exports
- `include/openbim_ifc.h`: generated header
- `tests/header.rs`: header drift and versioned-symbol checks
- `tests/c/smoke.c`, `scripts/check-c.sh`: compiled C/C++ smoke test

## Work queue

- [x] `CAPI-ABI` - handles, buffers, statuses, panic containment (ADR 0040 conventions)
- [x] `CAPI-TAPE` - lossless nested values as a pre-order node tape
- [x] `CAPI-HEADER` - cbindgen header with drift check
- [x] `CAPI-SMOKE` - C11 and C++17 smoke test in the gate
- [ ] `CAPI-CMAKE` - CMake package and install layout, following Axiolid's `native/`
- [ ] `CAPI-DIST` - prebuilt libraries per platform on release

## Completion log

- `CAPI-ABI/TAPE` - 17 Rust tests: round trip through the exports, misuse
  (null, zero/stale/double-destroyed handles, short buffers untouched,
  bad UTF-8), last-error set and cleared, malformed tapes refused without
  changing the model, panic containment.
- `CAPI-HEADER` - generated header, 2 tests: committed copy equals fresh
  output; every export carries the `v0_1_` prefix.
- `CAPI-SMOKE` - compiles with `-std=c11 -pedantic -Werror` and
  `-std=c++17 -Werror`; parse, read, edit, add, write, re-parse, no leaks.
  `-Werror` caught a wrong argument order in the first draft of the test.
- Design: nested values use a tape rather than recursive structs or a
  cursor API -- one call returns a whole value, the caller owns both
  buffers, and no allocation crosses the ABI (issue #38 left this open).
