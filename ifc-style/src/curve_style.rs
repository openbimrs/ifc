//! Curve colours, widths, fonts, and model/draughting mode.
//!
//! Readers use schema-resolved inherited attributes, avoiding the IFC2x3/IFC4
//! slot drift around `IfcPresentationStyle` and `ModelOrDraughting`.
//!
//! Font pattern lengths and aggregate references are exposed without renderer
//! lowering. Dash tessellation and physical line-width interpretation belong in
//! drawing/rendering adapters.
//! All returned values borrow the source model; projections allocate no graph copy.

mod style;

pub use style::{CurveStyle, CurveStyleFont, CurveStyleFontPattern};
