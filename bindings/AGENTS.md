# AGENTS.md — bindings/

Language bindings. Both are **reserved, not wired** — the crates compile and
document intent, and neither `pyo3` nor `wasm-bindgen` is in a manifest yet.

That is deliberate: both need build-time toolchain support (Python dev headers;
a wasm target), and adding them before there is anything to bind would trade a
working `cargo build --workspace` for an empty capability. Each gets added in the
same commit that wires its first real binding.

| Crate | Target | Kernel features |
| --- | --- | --- |
| `nehirde-python` | pyo3 + maturin, abi3 wheels | `["scalar","simd"]` |
| `nehirde-wasm` | wasm-bindgen | **contract only** |

## Why these targets matter

- **Python** is why IfcOpenShell is dominant. Competing means shipping this.
  Bindings also constrain API design, so the slot is reserved early rather than
  discovered late.
- **wasm** is where the no-OpenCascade decision (`docs/adr/0003`) pays off most:
  a pure-Rust graph compiles to wasm nearly for free, and the incumbent's C++
  geometry dependency makes following expensive.

## Constraints wasm imposes on the whole workspace

These must hold *before* the binding is written, so they are listed where they
will be read:

- **No native backend may be mandatory** — `nehirde-wasm` takes `geom-kernel`
  with `default-features = false`. Keep `std::arch` SIMD and any GPU path behind
  features.
- **No mmap-only parse path** — `memmap2` does not exist in the browser.
  `ifc-step` must accept a `&[u8]`, with mmap as one input strategy.
- **No `std::fs` or thread spawning below the API surface.**

## Status

Reserved. See `docs/ROADMAP.md` Stage 7.
