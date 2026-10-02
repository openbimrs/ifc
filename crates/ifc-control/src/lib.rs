//! Bounded IFC control semantics: permits, project orders, action
//! requests, and performance history.
//!
//! This crate owns the `IfcControl` subtypes that govern work rather
//! than describe physical form. It stages them into a transaction and
//! validates what the schema states; it does not implement approval
//! workflow, authorization, or policy decisions, which are the
//! concern of whatever system issues the permit.
//!
//! Controls relate to the work they govern through
//! `IfcRelAssignsToControl`. The crate owning the relating control
//! writes that relationship, so this crate stages it for its own four
//! controls and refuses any other.
//!
//! Records are laid out by attribute name from one release's table
//! (#198). `IfcRoot.OwnerHistory` is required in IFC2X3 and optional from
//! IFC4 on, so the writers that leave it unset refuse IFC2X3 with
//! [`ControlError::AuthoringRequired`]; the `*_with_owner_history`
//! variants bind the model's declared release and take a caller-supplied
//! `IfcOwnerHistory` (#202).
//!
//! [`read_control`] and [`read_controls`] read the four controls back as
//! borrowed [`Control`] views, by attribute name in the same declared
//! release, together with the [`ControlAssignment`]s that govern work
//! (#100). A record that does not fit its release is refused, never read
//! as absent.
//!
//! `IfcCostItem`, `IfcCostSchedule`, `IfcWorkCalendar` and
//! `IfcWorkControl` are `IfcControl` subtypes too, but they belong to
//! `ifc-cost` and `ifc-schedule`: the crates split by domain, not by
//! supertype, so those entities do not move here.

mod assignment;
mod authoring;
mod error;
mod read;
mod release;

pub use assignment::{
    assign_to_control, assign_to_control_with_owner_history, ControlAssignmentDraft,
};
pub use authoring::{create_control, create_control_with_owner_history, ControlDraft, ControlKind};
pub use error::{ControlError, ControlResult};
pub use read::{read_control, read_controls, Control, ControlAssignment};
