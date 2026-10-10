//! IFC4X3 stations lower exactly onto Axiolid's station relations (#307).
//!
//! `IfcPointByDistanceExpression`, `IfcAxis2PlacementLinear`,
//! `IfcOffsetCurveByDistances`, `IfcSectionedSolidHorizontal` and
//! `IfcSectionedSurface` against IFC4.3 ADD2's definitions. The lowering
//! stores exact data only. These tests resolve it through the reference
//! evaluator (`axiolid-reference`, a dev-dependency), and, with the
//! `compile-reference-backend` feature, through the reference mesh compiler.
//! Each checks a hand-computed point, frame or volume. Every refusal is
//! checked by name. `stations/seams.rs` covers tangent discontinuities
//! (#346), `stations/relations.rs` stations along curve relations (#346)
//! and segments placed at stations (#311), `stations/offsets.rs` stations
//! along offsets (#414) and `stations/offset_bases.rs` along offsets whose
//! length is a quadrature, snapped by the kernel's seam window (#423).

#![cfg(feature = "lowering")]

mod stations {
    pub mod common;
    mod compile;
    mod offset;
    mod offset_bases;
    mod offsets;
    mod points;
    mod relations;
    pub mod seams;
    mod sectioned;
}
