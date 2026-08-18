//! `geom-measure` — metric properties of shapes.
//!
//! # Why this is its own crate
//!
//! Quantity takeoff — the commercial reason much BIM software exists — needs
//! areas and volumes and nothing else. Isolating measurement means a takeoff
//! tool never compiles a boolean kernel, and it gives the boolean kernel an
//! independent way to be *checked*: volume is a triangulation-invariant, so
//! `volume(a \ b) + volume(a ∩ b) == volume(a)` validates a boolean without
//! comparing index buffers.
//!
//! # Scope
//!
//! - Mesh volume (signed tetrahedron sum) and surface area
//! - Profile area, perimeter, centroid, second moments (section properties)
//! - Centroid and inertia tensor of a solid
//! - Oriented and axis-aligned bounds
//!
//! # Precision note
//!
//! Signed-volume accumulation loses precision when a solid sits far from the
//! origin — routine in IFC, where site coordinates can be national-grid values
//! in the millions. Measurement is therefore performed **relative to the
//! shape's own centroid**, not the world origin.
