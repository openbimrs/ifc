//! `IfcLightSource` projections and photometric light distribution data.
//!
//! Light sources are `IfcGeometricRepresentationItem` subtypes, so they can
//! appear inside a shape representation's `Items` alongside solids and curves.
//! They carry no shape: nothing about a light contributes a face, an edge, or
//! a volume. That is why they live here and not in `ifc-geometry` -- the
//! geometry crate classifies all five as `non-shape` with this crate as owner
//! (`ifc-geometry/data/ifc4-representation-item-dispositions.tsv`), and this
//! module is what makes that ownership claim true rather than aspirational.
//!
//! The layout is identical across IFC2x3, IFC4 ADD2 TC1, and IFC4X3 ADD2 --
//! every attribute below was read from all three bundled EXPRESS schemas --
//! but slots are still resolved by name through `ifc_schema`, never by index,
//! so a future layout change surfaces as a typed error instead of a silently
//! shifted read.
//!
//! Radiometry is deliberately absent. These views report authored photometric
//! values (`IfcLuminousFluxMeasure`, colour temperature, an intensity
//! distribution curve); converting them into a renderer's units, or evaluating
//! the distribution between sampled angles, is an application concern.

mod distribution;
mod goniometric;
mod positional;
mod source;

pub use distribution::{LightDistributionData, LightIntensityDistribution};
pub use goniometric::LightSourceGoniometric;
pub use positional::{LightSourcePositional, LightSourceSpot};
pub use source::{LightSource, LightSourceAmbient, LightSourceDirectional, LightSourceKind};
