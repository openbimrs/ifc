//! WebAssembly bindings — **not yet wired**, deliberately.
//!
//! # Why this is a strategic target
//!
//! IfcOpenShell cannot follow us here cheaply: OpenCascade is a very large C++
//! dependency, and browser-side IFC tooling has largely had to reimplement
//! parsing from scratch as a result. A pure-Rust library with no C++ in its
//! graph compiles to wasm essentially for free. **This is the clearest payoff
//! of the no-OpenCascade decision** (`docs/adr/0003`) — it is not merely a
//! lighter build, it is a platform the incumbent struggles to reach.
//!
//! # Constraints this target imposes on the rest of the workspace
//!
//! Recorded here because they must hold *before* the binding is written:
//!
//! - **No native backend may be mandatory.** This crate takes `geom-kernel`
//!   with `default-features = false`; `std::arch` SIMD and any GPU path must
//!   stay behind features. (wasm has its own `simd128`, a separate future
//!   backend.)
//! - **No mmap in the parse path's only implementation.** `memmap2` does not
//!   exist in the browser, so `ifc-step` must also accept a byte slice. The
//!   parser is designed around a `&[u8]` core with mmap as an input strategy,
//!   which keeps this open.
//! - **No `std::fs` or thread spawning below the API surface.**
//!
//! # Status
//!
//! Reserved. See `docs/ROADMAP.md` Stage 7.
