//! `ifc-georef` — georeferencing and coordinate reference systems.
//!
//! # Why this is its own crate
//!
//! IFC4 has **8 georeferencing entities** and IFC4x3 adds `IfcGeographicCRS`
//! and `IfcMapConversionScaled`. This is small in entity count and large in
//! consequence: it is how a model's local millimetre coordinates relate to a
//! real position on Earth, which is what makes federation with survey data, GIS
//! and civil alignments possible at all.
//!
//! # Scope
//!
//! - `IfcMapConversion` (eastings/northings, orthogonal height, X-axis
//!   abscissa/ordinate, scale) and the projected CRS
//! - Site placement and the local ↔ map transform in both directions
//! - True north versus project north
//!
//! # Pitfalls
//!
//! - **Precision.** National-grid coordinates are large numbers; applying the
//!   map conversion in `f32`, or before centring geometry, destroys precision.
//!   Keep models in local coordinates and convert at the boundary.
//! - **Two competing conventions.** Older models express georeferencing through
//!   site latitude/longitude plus a rotated placement rather than
//!   `IfcMapConversion`. Both occur in the wild.
