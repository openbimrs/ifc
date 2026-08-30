#![cfg(feature = "lowering")]
//! Requires the `lowering` feature: this suite exercises the neutral DAG.
//! The dispatcher, exercised against the committed fixture corpus.
//!
//! # Why a corpus test and not only synthetic models
//!
//! A dispatcher that is total against hand-written models can still panic or
//! mis-route on the first real exporter record. These tests walk every
//! representation item in the committed corpus and assert the crate contract
//! holds for all of them: either a node comes back, or a typed error naming
//! the source entity does. Never a panic, never a silent substitute.

use ifc_geometry::lower::dispatch::{IMPLEMENTED, PLANNED};
use ifc_geometry::lower::{lower_representation_item, LoweringSession, Tolerance};
use ifc_geometry::transform::Transform;
use ifc_geometry::units;
use ifc_model::Codec;
use ifc_step::StepCodec;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures")
}

fn collect_ifc(dir: &std::path::Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_ifc(&path, files);
        } else if path.extension().and_then(|e| e.to_str()) == Some("ifc") {
            files.push(path);
        }
    }
}

/// Every representation item either lowers or reports a typed reason.
///
/// This is the totality contract. A panic here means the dispatcher met a
/// family it did not classify, which is exactly the failure mode
/// `#[non_exhaustive]` plus a wildcard no-op would hide.
#[test]
fn every_corpus_representation_item_lowers_or_reports_a_typed_reason() {
    let mut files = Vec::new();
    collect_ifc(&fixture_root(), &mut files);
    assert!(files.len() >= 19, "expected the committed corpus");

    let tol = Tolerance::building_scale();
    let mut lowered = 0usize;
    let mut unsupported: BTreeMap<String, usize> = BTreeMap::new();

    for path in &files {
        let Ok(model) = StepCodec.read_path(path) else {
            continue;
        };
        let scale = units::resolve(&model);

        for type_name in IMPLEMENTED.iter().chain(PLANNED.iter().map(|(n, _)| n)) {
            for id in model.ids_of_type(type_name) {
                let mut session = LoweringSession::new(&model, &scale, tol);
                match lower_representation_item(&mut session, *id, Transform::identity()) {
                    Ok(_) => lowered += 1,
                    Err(error) => {
                        // A nested failure attributes to the innermost entity
                        // that could not be lowered, not the outer item that
                        // merely referenced it. For
                        // `IFCBOOLEANCLIPPINGRESULT(#a,#b)` with an
                        // unimplemented half-space operand, the actionable id
                        // is the half-space. Assert it names *some* real
                        // entity in this model so a caller can always resolve
                        // the report back to a record.
                        let named = error.entity().expect("every failure names an entity");
                        assert!(
                            model.get(named).is_some(),
                            "{path:?}: reported entity {named} is not in the model"
                        );
                        // `nested_mapped_item_cycle.ifc` exists to be
                        // malformed: its mapping graph closes on itself, so a
                        // structural report is the CORRECT outcome there and
                        // proves the cycle guard fires on real file input.
                        // Everywhere else a failure must mean "valid IFC we do
                        // not lower yet".
                        let cyclic_fixture = path
                            .file_name()
                            .is_some_and(|name| name.to_string_lossy().contains("cycle"));
                        // That file is deliberately malformed in two ways:
                        // the mapping graph closes on itself, and `#31=
                        // IFCMAPPEDITEM(#17,$)` leaves the schema-mandatory
                        // MappingTarget empty. Both must produce a typed,
                        // entity-naming report rather than a panic.
                        if cyclic_fixture && !error.is_unsupported() {
                            let text = error.to_string();
                            assert!(
                                text.contains("cyclic") || text.contains("has no MappingTarget"),
                                "{path:?} {id}: expected a structural report, got: {error}"
                            );
                            continue;
                        }
                        assert!(
                            error.is_unsupported(),
                            "{path:?} {id}: unexpected hard failure: {error}"
                        );
                        *unsupported.entry((*type_name).to_string()).or_default() += 1;
                    }
                }
            }
        }
    }

    println!("dispatched: {lowered} lowered, unsupported by family: {unsupported:?}");
    assert!(
        lowered > 0,
        "the corpus must exercise at least one implemented family"
    );
}

/// A planned-but-unimplemented family reports its documented reason.
///
/// Pins the stub contract: the ledger in `dispatch` is the single source of
/// truth, so a family cannot quietly start returning a generic message.
#[test]
fn planned_families_report_their_documented_reason() {
    let mut files = Vec::new();
    collect_ifc(&fixture_root(), &mut files);
    let tol = Tolerance::building_scale();
    let mut checked = 0usize;

    for path in &files {
        let Ok(model) = StepCodec.read_path(path) else {
            continue;
        };
        let scale = units::resolve(&model);

        for (type_name, detail) in PLANNED {
            // A reason starting with "conditional:" means the family lowers for
            // some inputs and is refused for others -- IfcSweptDiskSolidPolygonal
            // lowers with sharp corners but is refused when FilletRadius is
            // present. Such a family MUST still report the documented reason
            // when it does refuse, but must not be required to always fail.
            let conditional = detail.starts_with("conditional:");
            // The prefix classifies the entry; the text after it is the reason
            // the lowerer actually reports.
            let expected = detail.strip_prefix("conditional: ").unwrap_or(detail);
            for id in model.ids_of_type(type_name) {
                let mut session = LoweringSession::new(&model, &scale, tol);
                let outcome = lower_representation_item(&mut session, *id, Transform::identity());
                let error = match outcome {
                    Ok(_) if conditional => continue,
                    Ok(_) => panic!("{type_name}: a planned family must not silently succeed"),
                    Err(error) => error,
                };
                assert!(
                    error.to_string().contains(expected),
                    "{type_name} must report {expected:?}, got: {error}"
                );
                checked += 1;
            }
        }
    }

    // The corpus now lowers every family it contains, so `checked` is
    // legitimately 0. Requiring a corpus instance here would make the gate
    // fail purely because coverage improved. What still must hold is the
    // contract itself: PLANNED and IMPLEMENTED are disjoint, and every
    // PLANNED entry carries a non-empty reason a caller can act on.
    println!("verified {checked} planned-family reports");
    for (type_name, detail) in PLANNED {
        assert!(
            !IMPLEMENTED.contains(type_name),
            "{type_name} is listed as both planned and implemented"
        );
        assert!(
            !detail.trim().is_empty(),
            "{type_name} must state why it is not lowered yet"
        );
    }
}

/// A nested failure names the innermost unlowerable entity.
///
/// The corpus no longer contains an unlowerable family, so the nesting is
/// built here: an `IfcBooleanResult` whose second operand is an
/// `IfcSectionedSpine` -- still declared in `PLANNED`. The boolean itself is
/// implemented, so a naive implementation would report the boolean and send a
/// caller to inspect a record that is perfectly fine.
///
/// This assertion has now survived two coverage jumps: it originally used the
/// half-space flyaway fixture, then `bath_csg_solid.ifc`, and both became
/// lowerable. Building the model inline keeps the contract under test
/// independent of how much of the corpus we can lower.
#[test]
fn a_nested_failure_names_the_innermost_unlowerable_entity() {
    let mut model = ifc_model::Model::new();
    let inner = ifc_model::EntityId(1);
    let outer = ifc_model::EntityId(2);
    // Any family the dispatcher does not classify works as the inner gap. This
    // deliberately uses a name from a LATER schema rather than a real planned
    // family, so implementing another family cannot silently defuse this test
    // the way IFCSECTIONEDSPINE did once it started lowering.
    model.insert(
        inner,
        ifc_model::Entity::new("IFCSEGMENTEDREFERENCECURVE", vec![]),
    );
    model.insert(
        outer,
        ifc_model::Entity::new(
            "IFCBOOLEANRESULT",
            vec![
                ifc_model::Value::Enum("DIFFERENCE".into()),
                ifc_model::Value::Ref(inner),
                ifc_model::Value::Ref(inner),
            ],
        ),
    );

    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let error = lower_representation_item(&mut session, outer, Transform::identity())
        .expect_err("the spine operand is not lowered yet");

    assert_eq!(
        error.entity(),
        Some(inner),
        "the report must name the innermost gap, not the boolean that wraps it"
    );
    assert!(error.is_unsupported(), "this is a gap, not corruption");
    assert!(
        error
            .to_string()
            .contains("representation item family is not lowered yet"),
        "the report must state the documented reason, got: {error}"
    );
}

/// Every family named IMPLEMENTED must actually lower when the corpus has one.
///
/// `IMPLEMENTED` is a hand-maintained claim, and nothing else checks it
/// against behaviour: the census above only visits families already named in
/// these lists, so silently deleting a name makes its instances invisible
/// rather than failing. This closes that hole from the other side -- it walks
/// the corpus by ENTITY TYPE and asserts that anything claimed implemented
/// really lowers, and that nothing lowering is left unclaimed.
#[test]
fn implemented_families_lower_and_lowering_families_are_claimed() {
    let mut files = Vec::new();
    collect_ifc(&fixture_root(), &mut files);
    let tol = Tolerance::building_scale();

    for path in &files {
        // Malformed-input fixtures exist to prove the lowerer REJECTS them;
        // a family being implemented does not mean every file using it is
        // valid. Only well-formed fixtures can carry this assertion.
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if name.contains("cycle") || name.contains("invalid") || name.contains("malformed") {
            continue;
        }
        let Ok(model) = StepCodec.read_path(path) else {
            continue;
        };
        let scale = units::resolve(&model);
        for family in IMPLEMENTED {
            for id in model.ids_of_type(family) {
                let mut session = LoweringSession::new(&model, &scale, tol);
                let result = lower_representation_item(&mut session, *id, Transform::identity());
                assert!(
                    result.is_ok(),
                    "{family} is listed IMPLEMENTED but {id:?} in {} failed: {}",
                    path.display(),
                    result.unwrap_err()
                );
            }
        }
    }
}

/// Anything in the corpus that lowers must be CLAIMED in IMPLEMENTED.
///
/// The companion test above iterates `IMPLEMENTED`, so removing a name simply
/// skips it -- the claim list cannot police its own omissions. This walks the
/// corpus by entity type instead: if a representation item lowers
/// successfully but no list names it, the inventory is understating what the
/// crate supports and the census figure is wrong.
#[test]
fn every_family_that_lowers_is_named_in_the_inventory() {
    let mut files = Vec::new();
    collect_ifc(&fixture_root(), &mut files);
    let tol = Tolerance::building_scale();
    let mut unclaimed: BTreeMap<String, usize> = BTreeMap::new();

    for path in &files {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if name.contains("cycle") || name.contains("invalid") || name.contains("malformed") {
            continue;
        }
        let Ok(model) = StepCodec.read_path(path) else {
            continue;
        };
        let scale = units::resolve(&model);
        for (id, entity) in model.iter() {
            let type_name = entity.type_name.to_ascii_uppercase();
            let claimed = IMPLEMENTED
                .iter()
                .any(|n| n.eq_ignore_ascii_case(&type_name))
                || PLANNED
                    .iter()
                    .any(|(n, _)| n.eq_ignore_ascii_case(&type_name));
            if claimed {
                continue;
            }
            let mut session = LoweringSession::new(&model, &scale, tol);
            if lower_representation_item(&mut session, id, Transform::identity()).is_ok() {
                *unclaimed.entry(type_name).or_default() += 1;
            }
        }
    }

    assert!(
        unclaimed.is_empty(),
        "these families lower but are named in neither IMPLEMENTED nor PLANNED: {unclaimed:?}"
    );
}
