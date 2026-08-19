//! EXPRESS `SELECT` and defined types of the three geometry schemas.
//!
//! # Why selects need real code
//!
//! A STEP attribute declared as `IfcBooleanOperand` is just an entity
//! reference in the file. Nothing in the record says which of the five
//! permitted kinds it is; you must resolve the reference and inspect the
//! target's type. Every consumer that skips this grows an ad-hoc `match` on
//! type-name strings, and those drift from the schema.
//!
//! # The trap: select members are usually abstract
//!
//! `IfcBooleanOperand` permits `IfcSolidModel`, which is ABSTRACT -- no file
//! ever contains one. Files contain `IfcExtrudedAreaSolid`, four levels below.
//! Resolution must be **subtype-aware**, so comparing the target's type name
//! against the member list directly rejects every valid file. [`is_a`] answers
//! that question from a compiled-in table.
//!
//! Each select resolves to the *branch* taken, so callers write an exhaustive
//! `match` the compiler checks instead of string comparisons they must keep in
//! sync with the schema.

pub mod aggregate_selects;
pub mod entity_selects;
pub mod subtype;

pub use aggregate_selects::{
    ArcIndex, CurveOnSurface, CurveOrEdgeCurve, DimensionCount, GeometricSetSelect,
    GridPlacementDirectionSelect, LineIndex, PointOrVertexPoint, SegmentIndexSelect,
    SurfaceOrFaceSurface,
};
pub use entity_selects::{
    Axis2Placement, BooleanOperand, CsgSelect, SolidOrShell, TrimmingSelect, VectorOrDirection,
};
pub use subtype::{is_a, known_entities, supertypes_of};
