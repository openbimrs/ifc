//! `ifc-alignment` -- Linear referencing and alignment -- the IFC4x3 civil layer.
//!
//!
//! IFC4x3 adds 14 alignment entities plus spiral curve types (`IfcClothoid`,
//! `IfcCosineSpiral`). Isolated in its own crate because building-only
//! consumers should never compile clothoid integration.
//!
//! # Module map
//!
//! | Module | Role |
//! |---|---|
//! | `alignment` | `IfcAlignment` and its horizontal/vertical/cant parts |
//! | `horizontal` | Horizontal segments: line, arc, spiral transitions |
//! | `vertical` | Vertical segments: grades and parabolic curves |
//! | `cant` | Superelevation (`IfcAlignmentCant`) for rail |
//! | `referent` | `IfcReferent` stationing and chainage |
//! | `placement` | `IfcLinearPlacement` and distance expressions |
//! | `error` | Why an alignment operation failed |
//!
//! # Status
//!
//! Scaffold -- modules are reserved with intent, not implemented. See
//! `../PLAN.md` for the stage that fills them.

mod alignment;
mod cant;
mod curve;
mod error;
mod horizontal;
mod placement;
mod referent;
mod vertical;
