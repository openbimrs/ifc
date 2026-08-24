//! `ifc-structural` -- Structural analysis model: members, connections, actions and reactions.
//!
//!
//! 39 entities in IFC4. This is an analysis view that parallels the physical
//! model rather than describing it -- a structural curve member is the idealised
//! line of a beam, not the beam's shape.
//!
//! # Module map
//!
//! | Module | Role |
//! |---|---|
//! | `model` | `IfcStructuralAnalysisModel` and its contents |
//! | `member` | Curve and surface members, and their varying forms |
//! | `connection` | Point, curve and surface connections; support conditions |
//! | `action` | Applied actions and load cases |
//! | `reaction` | Computed reactions |
//! | `load` | Load definitions, groups and combinations |
//! | `error` | Why a structural query failed |
//!
//! # Status
//!
//! Scaffold -- modules are reserved with intent, not implemented. See
//! `../PLAN.md` for the stage that fills them.

mod action;
mod connection;
mod error;
mod load;
mod member;
mod model;
mod reaction;

mod condition;
mod result;
