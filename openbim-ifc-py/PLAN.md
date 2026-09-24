# openbim-ifc-py implementation plan

Status: bindings implemented; release workflow builds wheels; not yet on PyPI.
Last updated: 2026-09-24

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Bounded contract

The record-model surface from Python, with values as frozen dataclasses and
failures as `IfcError(code=...)`. This does not imply PyPI publication,
macOS/Windows wheels, domain views, or geometry.

## Planned file map

- `src/lib.rs`: the `openbim_ifc._native` module
- `src/model.rs`: `NativeModel`
- `src/convert.rs`: `Tagged` <-> dicts
- `src/error.rs`: `IfcError`
- `python/openbim_ifc/__init__.py`, `model.py`, `values.py`: public package
- `python/openbim_ifc/_native.pyi`, `py.typed`: typing
- `tests/python/test_smoke.py`, `test_corpus.py`: unittest suites
- `scripts/check-python.sh`: build wheel, install, test

## Work queue

- [x] `PY-BIND` - NativeModel over the binding core; GIL released while parsing
- [x] `PY-VALUES` - frozen dataclasses; bare values refused
- [x] `PY-WHEEL` - maturin abi3-py39 wheel built and tested in the gate
- [ ] `PY-PLATFORMS` - Linux x86_64/aarch64 (manylinux_2_28), macOS universal2 and
  Windows x64 wheels, each tested, in the Release workflow; checked off at
  the first release that builds all four
- [ ] `PY-PUBLISH` - `openbim-ifc` on PyPI via trusted publishing (pending
  publisher: workflow `release.yml`, environment `release`); checked off
  when `openbim-ifc-py-v0.1.0` is live

## Completion log

- `PY-BIND/VALUES/WHEEL` - 14 unittest cases against the installed release
  wheel (`cp39-abi3-manylinux_2_34_x86_64`), including all 46 fixtures
  round-tripping; misuse raises `IfcError` with the shared codes.
- Decision: not `unsendable`. The first draft was, and cross-thread use
  surfaced as a Rust panic (`PanicException`), which `except Exception`
  does not catch. The model is `Send + Sync` and the GIL serialises calls.
