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
            for id in model.ids_of_type(type_name) {
                let mut session = LoweringSession::new(&model, &scale, tol);
                let error = lower_representation_item(&mut session, *id, Transform::identity())
                    .expect_err("a planned family must not silently succeed");
                assert!(
                    error.to_string().contains(detail),
                    "{type_name} must report {detail:?}, got: {error}"
                );
                checked += 1;
            }
        }
    }

    assert!(
        checked > 0,
        "the corpus must contain at least one not-yet-lowered family"
    );
    println!("verified {checked} planned-family reports");
}

/// A nested failure names the innermost unlowerable entity.
///
/// Ground truth read directly out of the fixture:
///
/// ```text
/// #206= IFCEXTRUDEDAREASOLID(#202,#203,#205,700.0);
/// #207= IFCBOOLEANRESULT(.DIFFERENCE.,#200,#206);
/// #208= IFCCSGSOLID(#207);
/// ```
///
/// The boolean at `#207` and its operands lower fine; `#208` wraps them in a
/// family that does not. Reporting `#207` would send a caller to inspect a
/// record that is perfectly fine, so the report must name `#208`.
///
/// This previously used the half-space flyaway fixture. That case now lowers
/// end to end (see `tests/lower_halfspace.rs`), so the assertion moved to a
/// nesting that is still genuinely unsupported rather than being deleted.
#[test]
fn a_nested_failure_names_the_innermost_unlowerable_entity() {
    let path = fixture_root().join("ifclite-geometry/bath_csg_solid.ifc");
    let model = StepCodec.read_path(&path).expect("fixture parses");
    let scale = units::resolve(&model);
    let tol = Tolerance::building_scale();

    let csg = ifc_model::EntityId(208);

    let mut session = LoweringSession::new(&model, &scale, tol);
    let error = lower_representation_item(&mut session, csg, Transform::identity())
        .expect_err("CSG solids are not lowered yet");

    assert_eq!(
        error.entity(),
        Some(csg),
        "the report must name the unsupported CSG solid"
    );
    assert!(error.is_unsupported(), "this is a gap, not corruption");
    assert!(
        error.to_string().contains("CSG primitive solids"),
        "the report must state the documented reason, got: {error}"
    );
}
