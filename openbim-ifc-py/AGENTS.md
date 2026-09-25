# openbim-ifc-py instructions

Purpose: Python bindings for the IFC facade (#39, ADR 0013), shipped as one
abi3 wheel (`openbim-ifc` on the Python side, import name `openbim_ifc`).
`README.md` is the user documentation and the wheel's long description.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

- Production dependencies: `openbim-ifc-binding-core` and `pyo3` only, plus
  `mimalloc` behind the opt-in `mimalloc` feature (#49; never default --
  `package_architecture` enforces that).
- pyo3 directly, not a layer over the C ABI (ADR 0013): no second manual
  memory protocol.
- Glue only. IFC behaviour belongs in the facade; checks shared by all hosts
  belong in the binding core.
- `publish = false`: this ships as a wheel, never to crates.io.

## Layout

- `src/lib.rs`: the `openbim_ifc._native` extension module.
- `src/model.rs`: `NativeModel`; `src/convert.rs`: `Tagged` <-> dicts;
  `src/error.rs`: `IfcError` with `.code`.
- `python/openbim_ifc/`: the public package -- `IfcModel`, the frozen value
  dataclasses in `values.py`, and `_native.pyi` stubs. Users import only
  from here; `_native` is private.
- `tests/python/`: unittest smoke and corpus suites (no pytest dependency).
- `scripts/check-python.sh`: builds a release wheel with maturin in a
  throwaway uv venv, installs it, runs the suites against the installed
  wheel.

## Invariants

- Bare Python values are refused (`TypeError`), never guessed: `3` is not
  known to be `Integer` or `Real`.
- `bool` is never accepted as an integer, and no truthy object as a bool.
- `NativeModel` is not `unsendable`: that would make cross-thread use a Rust
  panic. The GIL serialises calls; the binding core pins `Send + Sync`.
- No pyo3 `extension-module` feature (deprecated in 0.29); maturin sets
  `PYO3_BUILD_EXTENSION_MODULE`, and leaving it off keeps
  `cargo test --workspace` linkable.

## Verification

```sh
openbim-ifc-py/scripts/check-python.sh   # needs uv; builds + tests the wheel
```
