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

use axiolid_model::{CurveRelation, GeometryNode};
use ifc_geometry::lower::dispatch::{IMPLEMENTED, PLANNED};
use ifc_geometry::lower::{lower_representation_item, LoweringSession};
use ifc_geometry::transform::Transform;
use ifc_geometry::units;
use ifc_geometry::GeometryError;
use ifc_model::Codec;
use ifc_step::StepCodec;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const DISPOSITIONS: &str = include_str!("../data/ifc4-representation-item-dispositions.tsv");

fn disposition_rows() -> Vec<[&'static str; 4]> {
    DISPOSITIONS
        .lines()
        .skip(1)
        .map(|line| {
            let fields: Vec<_> = line.split('\t').collect();
            assert_eq!(fields.len(), 4, "malformed disposition row: {line}");
            [fields[0], fields[1], fields[2], fields[3]]
        })
        .collect()
}

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

/// The schema, not the hand-maintained dispatch arrays, defines the complete
/// set of concrete representation-item families that need a disposition.
#[test]
fn every_concrete_ifc4_representation_item_is_classified() {
    let schema = ifc_schema::ifc4();
    let expected: BTreeSet<_> = schema
        .entity_names()
        .filter(|name| {
            schema.is_a(name, "IfcRepresentationItem")
                && !schema.entity(name).expect("known entity").abstract_
        })
        .map(str::to_ascii_uppercase)
        .collect();
    let rows = disposition_rows();
    let classified: BTreeSet<_> = rows.iter().map(|row| row[0].to_owned()).collect();
    assert_eq!(
        classified.len(),
        rows.len(),
        "duplicate representation-item disposition"
    );
    for row in &rows {
        assert!(
            matches!(
                row[1],
                "nested-exact" | "planned-exact" | "typed-refusal" | "non-shape"
            ),
            "unknown disposition for {}: {}",
            row[0],
            row[1]
        );
        assert!(!row[2].is_empty(), "{} has no owner", row[0]);
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let owner_path = if row[2] == "ifc-style" {
            manifest.join("../ifc-style/src/lib.rs")
        } else if let Some(module) = row[2].strip_prefix("lower::") {
            manifest.join("src/lower").join(format!("{module}.rs"))
        } else if let Some(module) = row[2].strip_prefix("resource::") {
            manifest.join("src/resource").join(format!("{module}.rs"))
        } else {
            panic!("{} has unknown owner syntax: {}", row[0], row[2]);
        };
        assert!(
            owner_path.is_file(),
            "{} names missing owner module {}",
            row[0],
            owner_path.display()
        );
        assert!(!row[3].is_empty(), "{} has no rationale", row[0]);
    }

    let implemented: BTreeSet<_> = IMPLEMENTED.iter().map(|name| (*name).to_owned()).collect();
    assert!(
        implemented.is_disjoint(&classified),
        "root-exact and non-root classifications overlap"
    );
    let actual: BTreeSet<_> = implemented.union(&classified).cloned().collect();
    let missing: Vec<_> = expected.difference(&actual).cloned().collect();
    let extra: Vec<_> = actual.difference(&expected).cloned().collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "representation-item disposition drift: missing={missing:?}; extra={extra:?}"
    );

    let planned: BTreeSet<_> = PLANNED.iter().map(|(name, _)| *name).collect();
    let classified_planned: BTreeSet<_> = rows
        .iter()
        .filter(|row| row[1] == "planned-exact")
        .map(|row| row[0])
        .collect();
    assert_eq!(
        planned, classified_planned,
        "runtime typed-refusal inventory drift"
    );
}

/// `IfcCompositeCurveOnSurface` carries the same ordered segment geometry as
/// `IfcCompositeCurve`; the subtype constraint says those segments share a
/// surface and must not make a valid standalone curve silently unsupported.
#[test]
fn a_committed_standalone_composite_curve_on_surface_lowers_exactly() {
    let path = fixture_root().join("synthetic-surfaces/synthetic_conic_offset_bounded.ifc");
    let model = StepCodec.read_path(&path).expect("fixture parses");
    let ids = model.ids_of_type("IFCCOMPOSITECURVEONSURFACE");
    assert_eq!(ids.len(), 1, "fixture has one standalone curve-on-surface");
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale);
    let root = lower_representation_item(&mut session, ids[0], Transform::identity())
        .expect("curve-on-surface lowers through the total dispatcher");
    let lowered = session.finish(root).expect("session finishes");
    let GeometryNode::CurveRelation(CurveRelation::Composite { segments }) =
        lowered.graph.get(root).expect("root exists")
    else {
        panic!("curve-on-surface must remain an exact composite relation");
    };
    assert_eq!(segments.len(), 1, "authored segment order is preserved");
    assert!(
        matches!(
            lowered.graph.get(segments[0].curve),
            Some(GeometryNode::CurveRelation(
                CurveRelation::ParameterCurve { .. }
            ))
        ),
        "the segment stays a surface-parameter p-curve"
    );
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

    let mut lowered = 0usize;
    let mut unsupported: BTreeMap<String, usize> = BTreeMap::new();

    for path in &files {
        let Ok(model) = StepCodec.read_path(path) else {
            continue;
        };
        let scale = units::resolve(&model);

        for type_name in IMPLEMENTED.iter().chain(PLANNED.iter().map(|(n, _)| n)) {
            for id in model.ids_of_type(type_name) {
                let mut session = LoweringSession::new(&model, &scale);
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
    let mut observed = BTreeSet::new();

    for path in &files {
        let Ok(model) = StepCodec.read_path(path) else {
            continue;
        };
        let scale = units::resolve(&model);

        for (type_name, detail) in PLANNED {
            for id in model.ids_of_type(type_name) {
                let mut session = LoweringSession::new(&model, &scale);
                let error = lower_representation_item(&mut session, *id, Transform::identity())
                    .expect_err("a planned family must not silently succeed");
                assert!(
                    error.to_string().contains(detail),
                    "{type_name} must report {detail:?}, got: {error}"
                );
                observed.insert(*type_name);
            }
        }
    }

    let planned: BTreeSet<_> = PLANNED.iter().map(|(type_name, _)| *type_name).collect();
    assert_eq!(
        observed, planned,
        "every planned typed refusal needs committed corpus discrimination"
    );
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
    let mut session = LoweringSession::new(&model, &scale);
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
fn every_implemented_family_has_committed_corpus_evidence() {
    let mut files = Vec::new();
    collect_ifc(&fixture_root(), &mut files);
    let mut counts = BTreeMap::<&str, usize>::new();

    for path in &files {
        let Ok(model) = StepCodec.read_path(path) else {
            continue;
        };
        for family in IMPLEMENTED {
            *counts.entry(family).or_default() += model.ids_of_type(family).len();
        }
    }

    let missing: Vec<_> = IMPLEMENTED
        .iter()
        .copied()
        .filter(|family| counts.get(family).copied().unwrap_or_default() == 0)
        .collect();
    assert!(
        missing.is_empty(),
        "IMPLEMENTED families without committed corpus instances: {missing:?}"
    );
}

#[test]
fn implemented_families_lower_and_lowering_families_are_claimed() {
    let mut files = Vec::new();
    collect_ifc(&fixture_root(), &mut files);
    let mut successes = BTreeMap::<&str, usize>::new();

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
                let mut session = LoweringSession::new(&model, &scale);
                match lower_representation_item(&mut session, *id, Transform::identity()) {
                    Ok(_) => *successes.entry(family).or_default() += 1,
                    Err(GeometryError::Unsupported { type_name, .. })
                        if PLANNED
                            .iter()
                            .any(|(planned, _)| planned.eq_ignore_ascii_case(&type_name)) =>
                    {
                        // The outer family remains implemented; this instance
                        // depends on an explicitly planned nested semantic.
                    }
                    Err(error) => panic!(
                        "{family} is listed IMPLEMENTED but {id:?} in {} failed: {error}",
                        path.display()
                    ),
                }
            }
        }
    }

    let without_success: Vec<_> = IMPLEMENTED
        .iter()
        .copied()
        .filter(|family| successes.get(family).copied().unwrap_or_default() == 0)
        .collect();
    assert!(
        without_success.is_empty(),
        "IMPLEMENTED families without a successful committed lowering: {without_success:?}"
    );
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
            let mut session = LoweringSession::new(&model, &scale);
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

/// The variant catalog is well-formed and cannot drift from the family tables.
///
/// `IMPLEMENTED` and `PLANNED` classify at family granularity. That is too
/// coarse for families whose support depends on how the instance is authored:
/// `IFCPCURVE` is implemented, but only for some reference-curve forms. This
/// gate holds the finer `PARTIAL` catalog to the same standard the family
/// tables get, so a partially supported family cannot quietly present itself
/// as fully implemented.
#[test]
fn every_partial_family_declares_both_admitted_and_refused_variants() {
    use ifc_geometry::lower::dispatch::{Support, PARTIAL};

    let implemented: BTreeSet<_> = IMPLEMENTED.iter().copied().collect();
    let planned: BTreeSet<_> = PLANNED.iter().map(|(name, _)| *name).collect();

    let mut families: BTreeSet<&str> = BTreeSet::new();
    for variant in PARTIAL {
        families.insert(variant.family);

        // A partial family is a refinement of an implemented one. Naming a
        // family that is wholly unimplemented here would assert that some of
        // it works when none of it does.
        assert!(
            implemented.contains(variant.family),
            "{} is in PARTIAL but not IMPLEMENTED; a partial family must be \
             one whose base support exists",
            variant.family
        );
        assert!(
            !planned.contains(variant.family),
            "{} is in both PARTIAL and PLANNED; a family cannot be partially \
             supported and entirely unimplemented at once",
            variant.family
        );
        assert!(
            !variant.variant.is_empty(),
            "{}: variant condition must be stated",
            variant.family
        );
        assert!(
            !variant.rationale.is_empty(),
            "{} / {}: rationale must be stated",
            variant.family,
            variant.variant
        );
    }

    // The point of the catalog is to record refusals hidden inside an
    // "implemented" family. A family with no refusal is not partial, and a
    // family with no admission is not implemented -- either way the row is
    // miscategorised and belongs in IMPLEMENTED or PLANNED instead.
    for family in &families {
        let admitted = PARTIAL
            .iter()
            .filter(|v| v.family == *family)
            .any(|v| v.support == Support::Admitted);
        let refused = PARTIAL
            .iter()
            .filter(|v| v.family == *family)
            .any(|v| v.support == Support::Refused);
        assert!(
            admitted,
            "{family} declares no admitted variant; it is not partially \
             supported and should be in PLANNED"
        );
        assert!(
            refused,
            "{family} declares no refused variant; it is fully supported and \
             should be in IMPLEMENTED alone"
        );
    }

    // Duplicate (family, variant) pairs would let two rows disagree about the
    // same case without anything noticing.
    let mut seen = BTreeSet::new();
    for variant in PARTIAL {
        assert!(
            seen.insert((variant.family, variant.variant)),
            "duplicate PARTIAL row for {} / {}",
            variant.family,
            variant.variant
        );
    }
}

/// Every refusal the catalog claims is actually reachable at runtime, and
/// every admission actually lowers.
///
/// A catalog that merely *asserts* a disposition is documentation. This drives
/// the real lowering path for one instance of each listed variant and requires
/// the runtime outcome to match the declared `Support`. Deleting a refusal
/// branch in the lowerer, or relabelling a refusal as admitted, fails here
/// rather than silently widening the crate's support claim.
///
/// Each case is keyed by its exact `PARTIAL` row, so a row that is renamed or
/// removed without updating this test fails too -- the catalog cannot drift
/// away from the probes that check it.
#[test]
fn declared_variant_support_matches_runtime_behaviour() {
    use ifc_geometry::lower::dispatch::{Support, PARTIAL};
    use ifc_model::{Entity, EntityId, Model, Value};

    fn ent(type_name: &str, values: Vec<Value>) -> Entity {
        Entity::new(type_name, values)
    }
    fn rf(id: u64) -> Value {
        Value::Ref(EntityId(id))
    }
    fn num(v: f64) -> Value {
        Value::Real(v)
    }
    fn pt(coords: Vec<f64>) -> Entity {
        ent(
            "IFCCARTESIANPOINT",
            vec![Value::List(coords.into_iter().map(num).collect())],
        )
    }

    /// A plane at the origin, plus whatever the probe adds on top.
    fn plane_model() -> Model {
        let mut model = Model::new();
        model.insert(EntityId(1), pt(vec![0.0, 0.0, 0.0]));
        model.insert(
            EntityId(2),
            ent("IFCAXIS2PLACEMENT3D", vec![rf(1), Value::Null, Value::Null]),
        );
        model.insert(EntityId(3), ent("IFCPLANE", vec![rf(2)]));
        model
    }

    // (family, variant) -> a model whose EntityId(9) is the item to lower.
    let probes: Vec<(&str, &str, Model)> = vec![
        {
            // Admitted: conic positioned by a 2D placement.
            let mut m = plane_model();
            m.insert(EntityId(4), pt(vec![2.0, 3.0]));
            m.insert(
                EntityId(5),
                ent("IFCAXIS2PLACEMENT2D", vec![rf(4), Value::Null]),
            );
            m.insert(EntityId(6), ent("IFCCIRCLE", vec![rf(5), num(1.5)]));
            m.insert(EntityId(9), ent("IFCPCURVE", vec![rf(3), rf(6)]));
            (
                "IFCPCURVE",
                "reference curve is an IfcLine, IfcCircle or IfcEllipse \
                 positioned by an IfcAxis2Placement2D",
                m,
            )
        },
        {
            // Refused: the same conic positioned by a 3D placement.
            let mut m = plane_model();
            m.insert(EntityId(6), ent("IFCCIRCLE", vec![rf(2), num(1.5)]));
            m.insert(EntityId(9), ent("IFCPCURVE", vec![rf(3), rf(6)]));
            (
                "IFCPCURVE",
                "reference conic positioned by an IfcAxis2Placement3D",
                m,
            )
        },
        {
            // Refused: a convention-only base spline has no authored knots.
            let mut m = plane_model();
            m.insert(EntityId(6), ent("IFCBSPLINECURVE", vec![Value::Integer(3)]));
            m.insert(EntityId(9), ent("IFCPCURVE", vec![rf(3), rf(6)]));
            (
                "IFCPCURVE",
                "reference curve is a convention-only IfcBSplineCurve, or a \
                 trimmed or composite curve",
                m,
            )
        },
        {
            // Admitted: an explicit-knot spline in parameter space.
            let mut m = plane_model();
            m.insert(EntityId(6), pt(vec![1.5, 2.5]));
            m.insert(EntityId(7), pt(vec![3.5, 4.5]));
            m.insert(
                EntityId(8),
                ent(
                    "IFCBSPLINECURVEWITHKNOTS",
                    vec![
                        Value::Integer(1),
                        Value::List(vec![rf(6), rf(7)]),
                        Value::Enum("UNSPECIFIED".into()),
                        Value::Bool(false),
                        Value::Bool(false),
                        Value::List(vec![Value::Integer(2), Value::Integer(2)]),
                        Value::List(vec![num(0.0), num(1.0)]),
                        Value::Enum("UNSPECIFIED".into()),
                    ],
                ),
            );
            m.insert(EntityId(9), ent("IFCPCURVE", vec![rf(3), rf(8)]));
            (
                "IFCPCURVE",
                "reference curve is an explicit-knot IfcBSplineCurveWithKnots \
                 or IfcRationalBSplineCurveWithKnots",
                m,
            )
        },
        {
            // Admitted: a plain polyline reference curve.
            let mut m = plane_model();
            m.insert(EntityId(4), pt(vec![0.0, 0.0]));
            m.insert(EntityId(5), pt(vec![1.0, 2.0]));
            m.insert(
                EntityId(6),
                ent("IFCPOLYLINE", vec![Value::List(vec![rf(4), rf(5)])]),
            );
            m.insert(EntityId(9), ent("IFCPCURVE", vec![rf(3), rf(6)]));
            ("IFCPCURVE", "reference curve is an IfcPolyline", m)
        },
    ];

    for (family, variant, model) in probes {
        let declared = PARTIAL
            .iter()
            .find(|v| v.family == family && v.variant == variant)
            .unwrap_or_else(|| {
                panic!(
                    "no PARTIAL row for {family} / {variant:?}; the catalog and \
                     its runtime probes have drifted apart"
                )
            });

        let scale = units::resolve(&model);
        let mut session = LoweringSession::new(&model, &scale);
        let outcome = lower_representation_item(&mut session, EntityId(9), Transform::identity());

        match declared.support {
            Support::Admitted => assert!(
                outcome.is_ok(),
                "{family} / {variant:?} is declared Admitted but did not lower: {:?}",
                outcome.err()
            ),
            Support::Refused => {
                let error = outcome.err().unwrap_or_else(|| {
                    panic!("{family} / {variant:?} is declared Refused but lowered successfully")
                });
                assert!(
                    error.is_unsupported(),
                    "{family} / {variant:?} must be a typed gap, not corruption: {error}"
                );
            }
        }
    }
}
