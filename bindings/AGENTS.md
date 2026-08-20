# AGENTS.md — bindings/

Language bindings. Neither is wired yet: no `pyo3` or `wasm-bindgen` dependency
is in any manifest, because both need build-time toolchain support and would
break `cargo build --workspace` on a machine lacking it. The crates exist to
hold the decision and the doc.

## `python/` — planned, in the workspace

Target: pyo3 + maturin, abi3 wheels, exposing the read path first
(parse → model → properties/quantities). This is the binding that matters
commercially — the existing IFC ecosystem is Python, and ifcopenshell-python is
what a user would be switching from.

## `_deferred-wasm/` — DEFERRED, not a workspace member

**WASM is explicitly later, not now.** The directory is prefixed with `_` and
listed in the root `Cargo.toml` `exclude`, so it is not built, not tested, and
not linted. Re-enabling it is a deliberate act: remove the `exclude` entry and
rename the directory.

Why deferred rather than deleted: the constraint it imposes is worth recording
while the kernel is still being designed. A wasm target means no threads by
default, no `is_x86_feature_detected!`, and a hard size budget — which is
precisely the argument for `axiolid-kernel`'s backend selection being runtime and
feature-gated rather than assumed. When wasm returns it should map to a
`backend::wasm` (or simply scalar-only) build, not a fork of the kernel.

Do not add dependencies or code here until it is un-deferred.
