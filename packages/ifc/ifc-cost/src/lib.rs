//! `ifc-cost` — `IfcCostItem` / `IfcCostSchedule` (5D).
//!
//! Cost lives in its own crate because it is a distinct domain that most
//! consumers do not need, and because it depends on quantities
//! (`ifc-properties`) rather than on geometry: a cost item references a
//! quantity, which may or may not have been derived from a shape.
//!
//! # Status
//!
//! Reserved. See `docs/ROADMAP.md` Stage 6.
