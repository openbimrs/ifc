//! `ifc-model` — indexed semantic views over a parsed model.
//!
//! Type buckets, the spatial containment tree (project → site → building →
//! storey → space), property sets, and attribute access.
//!
//! # Boundary
//!
//! This crate holds **no geometry**. `IfcWall` here is an entity with
//! attributes and relationships; its shape is produced by `ifc-geometry`. Keeping
//! semantics free of geometry is what lets a consumer that only needs a
//! quantity takeoff or a property audit avoid the geometry stack entirely —
//! a capability the IfcOpenShell+OpenCascade stack does not offer.
//!
//! # Status
//!
//! Scaffold. Stage 1 in `docs/ROADMAP.md`.
