# openbim-ifc-capi instructions

Purpose: the versioned C ABI over the IFC facade (#38, ADR 0013), for C,
C++ and any host with a C FFI. `README.md` documents the protocol for users.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

- Production dependencies: `openbim-ifc-binding-core` only, plus `mimalloc`
  behind the opt-in `mimalloc` feature (#49; never default --
  `package_architecture` enforces that). All IFC behaviour comes from the
  core; this crate is calling-convention glue.
- The only crate in the workspace allowed `unsafe`, under
  `#![deny(unsafe_op_in_unsafe_fn)]`. Every raw-pointer read or write lives
  in `src/buffer.rs`, each with a `SAFETY:` comment. Keep it that way: a new
  export composes those helpers; it does not dereference pointers itself.
- Mirrors Axiolid's ADR 0040 conventions on purpose; diverge only with a
  reason recorded in `PLAN.md`.

## Layout

- `src/status.rs`: `OpenbimIfcStatus`, `OpenbimIfcVersion`, `boundary`
  (the `catch_unwind` wrapper every export runs inside).
- `src/buffer.rs`: audited pointer helpers and the size-query protocol.
- `src/registry.rs`: handle table; per-model locks.
- `src/tape.rs`: the value tape, `Tagged` <-> nodes + string buffer.
- `src/model.rs`, `src/errors.rs`: the exports.
- `include/openbim_ifc.h`: generated; `tests/header.rs` fails on drift.
- `tests/c/smoke.c` + `scripts/check-c.sh`: C11 and C++17 smoke test.

## Invariants

- Every export: `#[no_mangle]`, prefixed `openbim_ifc_v0_1_`, wrapped in
  `boundary`, C integers validated before use (never taken as a Rust enum).
- `out_required` is always written; a short buffer is never touched.
- A failed call changes nothing, and records the model's last error; a
  successful call on that model clears it.
- ABI 0.1 symbols are frozen once released: change them by adding `v0_2_`.

## Verification

```sh
cargo test -p openbim-ifc-capi                 # boundary tests + header drift
openbim-ifc-capi/scripts/check-c.sh            # compiled C and C++ smoke
UPDATE_HEADER=1 cargo test -p openbim-ifc-capi --test header   # after an ABI change
```
