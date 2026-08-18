//! Python bindings — **not yet wired**, deliberately.
//!
//! # Why this crate exists but is empty
//!
//! Python is the reason IfcOpenShell is dominant: `ifcopenshell-python` is how
//! most people actually touch IFC. A Rust library without Python bindings does
//! not compete with it, so the slot is reserved here rather than discovered
//! late — binding ergonomics constrain API design, and finding that out after
//! the API is fixed is expensive.
//!
//! # Why `pyo3` is not in the manifest yet
//!
//! `pyo3` requires Python development headers at build time. Adding it now
//! would make `cargo build --workspace` fail on any machine lacking them, which
//! trades a working build for an empty capability. It gets added in the same
//! commit that wires the first real binding.
//!
//! # Intended shape
//!
//! - Built with `maturin`, abi3 wheels so one artifact covers many CPython
//!   versions.
//! - Module name `nehirde`, exposing model open/query and property access
//!   first — the geometry surface can follow, since most Python IFC work is
//!   property and relationship work.
//! - The GIL is released around parse and geometry calls. This is the
//!   structural win over `ifcopenshell-python`: the Rust side is already
//!   parallel, so Python callers get multicore parsing without threading in
//!   Python.
//!
//! # Status
//!
//! Reserved. See `docs/ROADMAP.md` Stage 7.
