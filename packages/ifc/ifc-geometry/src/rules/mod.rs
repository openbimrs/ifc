//! EXPRESS `WHERE` rules: the schema's own correctness conditions.
//!
//! # Why these are worth implementing
//!
//! A file can parse and still describe impossible geometry: a 2D direction
//! on a 3D placement, a boolean between operands of different dimensionality,
//! an extrusion parallel to the plane it extrudes. The schema states these
//! conditions as `WHERE` rules, and they are the difference between "the
//! parser accepted it" and "a kernel can build it".
//!
//! Checking them **here** rather than in the kernel matters: the kernel would
//! discover the problem as a numerical failure deep in an algorithm, where the
//! diagnostic is a degenerate matrix rather than "RefDirection is parallel to
//! Axis in #4711".
//!
//! # Scope
//!
//! IFC4 declares 95 where-rules across 56 geometry entities. Implemented here
//! are the ones a consumer can actually act on: rules about dimensionality,
//! degeneracy and operand agreement. Rules that merely restate a type
//! constraint the parser already enforces are noted as such and skipped.
//!
//! # Design
//!
//! A rule is a pure function from a resolved view to `Result<(), RuleViolation>`.
//! Rules never mutate, never allocate on the success path, and are grouped by
//! the entity they constrain. [`validate`] runs every rule that applies to an
//! entity, so a caller checks a whole model without knowing the rule list.

pub mod placement;
pub mod solid;
pub mod violation;

pub use violation::{RuleViolation, ViolationKind};

use ifc_model::{EntityId, Model};

/// Run every implemented where-rule that applies to this entity.
///
/// Returns all violations rather than the first, because a consumer fixing a
/// file wants the whole list, and because one bad placement often implies
/// several related failures.
pub fn validate(model: &Model, id: EntityId) -> Vec<RuleViolation> {
    let Some(entity) = model.get(id) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    placement::check(model, id, entity, &mut found);
    solid::check(model, id, entity, &mut found);
    found
}

/// Validate every entity in a model.
///
/// Linear in model size and allocation-free unless a rule actually fails,
/// so it is cheap enough to run as an import-time check.
pub fn validate_model(model: &Model) -> Vec<RuleViolation> {
    model
        .iter()
        .flat_map(|(id, _)| validate(model, id))
        .collect()
}
