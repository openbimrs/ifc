//! `ifc-geometry` — lower IFC representation items into geometry.
//!
//! # This crate is the seam
//!
//! It is the primary place where IFC meets geometry, and it meets it through
//! [`geom_kernel`] **traits**, never through a backend implementation. That is
//! what makes the geometry kernel swappable:
//!
//! ```text
//!   ifc-step ─→ ifc-model ─→ ifc-geometry ──uses trait──→ geom-kernel
//!                                                              ▲
//!                              a better kernel implements it ──┘
//! ```
//!
//! # Module map — one representation family per module
//!
//! IFC4 has ~119 curve/surface entities, 11 swept-solid forms and ~37 topology
//! entities (counted from `references/ifc-spec/`). Lowering them in one file is
//! precisely how a 5,000-line module happens, so the split is by representation
//! family:
//!
//! | Module | IFC entities it handles |
//! |---|---|
//! | [`lowerer`] | the `ShapeLowerer` façade and kernel injection |
//! | [`placement`] | `IfcLocalPlacement`, `IfcAxis2Placement2D/3D` chains |
//! | [`profile`] | the 23 `IfcProfileDef` subtypes → 2D profiles |
//! | [`swept`] | `IfcExtrudedAreaSolid`, `IfcRevolvedAreaSolid`, swept disks |
//! | [`brep`] | `IfcFacetedBrep`, `IfcAdvancedBrep`, shells and faces |
//! | [`csg`] | `IfcBooleanResult`, `IfcHalfSpaceSolid`, clipping |
//! | [`tessellated`] | `IfcTriangulatedFaceSet`, `IfcPolygonalFaceSet` |
//! | [`mapped`] | `IfcMappedItem` instancing and nested transforms |
//! | [`opening`] | `IfcRelVoidsElement` void cutting |
//! | [`units`] | length/angle unit scaling into kernel units |
//! | [`context`] | `IfcGeometricRepresentationContext`, precision |
//! | [`error`] | why a lowering failed, per representation |
//!
//! # Status
//!
//! Scaffold: the generic seam is defined and tested with a stub backend. The
//! representation lowerings are Stage 3 in `docs/ROADMAP.md`.

pub mod brep;
pub mod context;
pub mod csg;
pub mod error;
pub mod lowerer;
pub mod mapped;
pub mod opening;
pub mod placement;
pub mod profile;
pub mod swept;
pub mod tessellated;
pub mod units;

pub use error::ShapeError;
pub use lowerer::ShapeLowerer;
