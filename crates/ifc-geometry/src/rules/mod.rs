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
//! IFC4 declares 95 where-rules across 56 geometry entities, and all 95 are
//! implemented, each in every bundled release that declares it, under that
//! release's own name and text (#400; see `rules/release.rs`) (`data/ifc4-where-rules.tsv`, asserted by
//! `tests/where_rule_inventory.rs`). A rule whose check cannot fail on a
//! parsed model would be kept as `inventoried`, with the reason beside the
//! code, rather than implemented as a check that always passes.
//!
//! Rules do not re-check what the parser and typed views already enforce
//! (declared types, attribute arity, SELECT membership); a rule states only
//! the schema's own `WHERE` condition. Numerical validation of a built shape
//! belongs to the kernel, not here.
//!
//! # Design
//!
//! A rule is a pure function from a resolved view to `Result<(), RuleViolation>`.
//! Rules never mutate, never allocate on the success path, and are grouped by
//! the entity they constrain. [`validate`] runs every rule that applies to an
//! entity, so a caller checks a whole model without knowing the rule list.

mod bspline;
mod cardinality;
mod curve;
mod dimension;
mod express;
mod grid;
pub mod placement;
mod release;
mod scalar;
pub mod solid;
mod surface;
mod typing;
pub mod violation;

pub use violation::{RuleViolation, ViolationKind};

use std::collections::HashMap;

use ifc_model::{Entity, EntityId, Model};
use release::{Declared, Release, Subject};

/// Run every implemented where-rule that applies to this entity.
///
/// The rules are the ones the model's declared release states, named as
/// that release names them (see `rules/release.rs`).
///
/// Returns all violations rather than the first, because a consumer fixing a
/// file wants the whole list, and because one bad placement often implies
/// several related failures.
pub fn validate(model: &Model, id: EntityId) -> Vec<RuleViolation> {
    let Some(entity) = model.get(id) else {
        return Vec::new();
    };
    let release = Release::of(model);
    let name = entity.type_name.to_ascii_uppercase();
    let governing = release.governing(&name);
    let mut found = Vec::new();
    run(
        &Subject::new(model, &release, id, entity, &name, &governing),
        &mut found,
    );
    found
}

/// Validate every entity in a model.
///
/// Linear in model size and allocation-free unless a rule actually fails,
/// so it is cheap enough to run as an import-time check. The rules that
/// govern a type are resolved once per type, not once per instance.
pub fn validate_model(model: &Model) -> Vec<RuleViolation> {
    let release = Release::of(model);
    let mut governing: HashMap<String, Vec<&'static Declared>> = HashMap::new();
    let mut found = Vec::new();
    for (id, entity) in model.iter() {
        let name = entity.type_name.to_ascii_uppercase();
        if !governing.contains_key(&name) {
            governing.insert(name.clone(), release.governing(&name));
        }
        let rules = &governing[&name];
        if rules.is_empty() {
            continue;
        }
        run(
            &Subject::new(model, &release, id, entity, &name, rules),
            &mut found,
        );
    }
    found
}

/// Every rule module, for one entity in its release.
fn run(subject: &Subject<'_>, found: &mut Vec<RuleViolation>) {
    if !subject.is_governed() {
        return;
    }
    placement::run(subject, found);
    solid::run(subject, found);
    grid::check(subject, found);
    curve::check(subject, found);
    scalar::check(subject, found);
    cardinality::check(subject, found);
    typing::check(subject, found);
    surface::check(subject, found);
    bspline::check(subject, found);
}

/// Run one module's rules for an entity of `model`, resolving the release
/// as [`validate`] does. Backs the public per-module `check` functions.
fn run_one(
    model: &Model,
    id: EntityId,
    entity: &Entity,
    out: &mut Vec<RuleViolation>,
    module: fn(&Subject<'_>, &mut Vec<RuleViolation>),
) {
    let release = Release::of(model);
    let name = entity.type_name.to_ascii_uppercase();
    let governing = release.governing(&name);
    let subject = Subject::new(model, &release, id, entity, &name, &governing);
    if subject.is_governed() {
        module(&subject, out);
    }
}
