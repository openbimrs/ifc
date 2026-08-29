//! Corpus test: the lint must be quiet on well-formed files and loud on
//! broken ones, measured across every committed fixture.
//!
//! Unit tests prove the rules in isolation; this proves the aggregate is
//! usable. A lint reporting findings on a clean model is one nobody runs
//! twice, so the false-positive count is the assertion that matters most.

#![cfg(all(feature = "step", feature = "spatial", feature = "geometry-select"))]

use std::path::PathBuf;

use ifc::{unreachable_products, Codec, StepCodec, Unreachable};
use ifc_model::{Entity, EntityId, Model, Value};

/// Load a committed fixture.
///
/// Deliberately panics when the file is missing. An earlier version of this
/// test resolved a path outside the repo and returned `Option`, so in CI it
/// skipped every assertion and still reported "ok" -- the exact failure mode
/// a corpus test exists to prevent.
fn fixture(name: &str) -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures")
        .join(name);
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("fixture {} must be committed: {e}", path.display()));
    StepCodec::lenient()
        .read_bytes(&bytes)
        .unwrap_or_else(|e| panic!("fixture {name} must parse leniently: {e:?}"))
}

/// Fixtures that place their products in the spatial tree correctly.
///
/// Measured, not assumed: each one contains at least one
/// `IfcRelContainedInSpatialStructure` and the lint is silent on it.
const WELL_FORMED: [&str; 6] = [
    "ifclite-geometry/bath_csg_solid.ifc",
    "ifclite-geometry/issue_1985_scaled_kinds.ifc",
    "ifclite-geometry/mapped_instances_multi_item.ifc",
    "ifclite-geometry/mapped_instances_nested.ifc",
    "ifclite-geometry/nested_mapped_item.ifc",
    "ifclite-geometry/shared_point_faceted_brep.ifc",
];

/// Fixtures with no containment relationship at all. Minimal geometry test
/// files, not published models -- their products really are unreachable.
const UNCONTAINED: [&str; 4] = [
    "ifclite-geometry/issue_098_wall_W.ifc",
    "ifclite-geometry/issue_1155_halfspace_flyaway.ifc",
    "ifclite-geometry/issue_2019_wall_two_overlapping_openings.ifc",
    "ifclite-geometry/swept_disk_composite_arc_crankbar.ifc",
];

#[test]
fn well_formed_fixtures_produce_no_findings() {
    for name in WELL_FORMED {
        let model = fixture(name);
        let findings = unreachable_products(&model);
        assert!(
            findings.is_empty(),
            "{name} places its products correctly and must be silent, got {findings:?}"
        );
    }
}

#[test]
fn fixtures_without_containment_are_reported() {
    for name in UNCONTAINED {
        let model = fixture(name);
        let findings = unreachable_products(&model);
        assert!(
            !findings.is_empty(),
            "{name} has no IfcRelContainedInSpatialStructure at all, so its \
             product is unreachable and must be reported"
        );
        assert!(
            findings
                .iter()
                .all(|(_, why)| *why == Unreachable::NotContainedInSpatialStructure),
            "{name}: expected containment findings, got {findings:?}"
        );
    }
}

#[test]
fn openings_are_never_reported_across_the_corpus() {
    // The overlapping-openings fixture is the interesting one: its wall is
    // uncontained, but its two IfcOpeningElements must not be added to the
    // count -- they reach the model through IfcRelVoidsElement.
    let model = fixture("ifclite-geometry/issue_2019_wall_two_overlapping_openings.ifc");

    let findings = unreachable_products(&model);

    for (id, _) in &findings {
        let entity = model.get(*id).expect("reported id exists");
        assert!(
            !entity.type_name.eq_ignore_ascii_case("IFCOPENINGELEMENT"),
            "an opening was reported; voided elements are reachable by design"
        );
    }
    assert_eq!(findings.len(), 1, "only the wall itself, got {findings:?}");
}

#[test]
fn adding_containment_silences_the_finding() {
    // Differential: the lint's output must actually respond to the defect it
    // names, not merely correlate with the fixture.
    let mut model = fixture("ifclite-geometry/issue_1155_halfspace_flyaway.ifc");
    let before = unreachable_products(&model);
    assert_eq!(before.len(), 1, "baseline has exactly one finding");
    let product = before[0].0;

    let mut next = model.iter().map(|(id, _)| id.0).max().unwrap_or(0) + 1;
    let mut add = |model: &mut Model, type_name: &str, attributes: Vec<Value>| {
        let id = EntityId(next);
        next += 1;
        model.insert(
            id,
            Entity {
                type_name: type_name.into(),
                attributes,
            },
        );
        id
    };

    let storey = add(
        &mut model,
        "IFCBUILDINGSTOREY",
        vec![
            Value::Text("storey00000000000000001".into()),
            Value::Null,
            Value::Null,
            Value::Null,
        ],
    );
    let mut rel = vec![Value::Null; 6];
    rel[4] = Value::List(vec![Value::Ref(product)]);
    rel[5] = Value::Ref(storey);
    add(&mut model, "IFCRELCONTAINEDINSPATIALSTRUCTURE", rel);

    assert!(
        unreachable_products(&model).is_empty(),
        "adding the one missing relationship must clear the finding"
    );
}

#[test]
fn a_plan_only_annotation_is_caught_on_a_real_model() {
    // Reproduces the original open-signs export defect on a real file: author
    // a product into the Plan context only, contain it correctly, and confirm
    // the lint names it and says what to do.
    let mut model = fixture("ifclite-geometry/bath_csg_solid.ifc");
    assert!(
        unreachable_products(&model).is_empty(),
        "baseline is silent"
    );

    let mut next = model.iter().map(|(id, _)| id.0).max().unwrap_or(0) + 1;
    let mut add = |model: &mut Model, type_name: &str, attributes: Vec<Value>| {
        let id = EntityId(next);
        next += 1;
        model.insert(
            id,
            Entity {
                type_name: type_name.into(),
                attributes,
            },
        );
        id
    };

    let mut ctx = vec![Value::Null; 10];
    ctx[0] = Value::Text("Annotation".into());
    ctx[1] = Value::Text("Plan".into());
    ctx[8] = Value::Enum(".PLAN_VIEW.".into());
    let context = add(&mut model, "IFCGEOMETRICREPRESENTATIONSUBCONTEXT", ctx);

    let mut rep = vec![Value::Null; 4];
    rep[0] = Value::Ref(context);
    rep[1] = Value::Text("Annotation".into());
    rep[2] = Value::Text("Curve2D".into());
    let representation = add(&mut model, "IFCSHAPEREPRESENTATION", rep);

    let mut shp = vec![Value::Null; 3];
    shp[2] = Value::List(vec![Value::Ref(representation)]);
    let shape = add(&mut model, "IFCPRODUCTDEFINITIONSHAPE", shp);

    let mut prod = vec![Value::Null; 7];
    prod[0] = Value::Text("2sign000000000000000001".into());
    prod[6] = Value::Ref(shape);
    let sign = add(&mut model, "IFCANNOTATION", prod);

    // Any spatial container will do; this fixture uses IfcBuilding rather
    // than a storey, and the lint must not care which.
    let container = model
        .iter()
        .find(|(_, e)| {
            let n = e.type_name.to_ascii_uppercase();
            n == "IFCBUILDINGSTOREY" || n == "IFCBUILDING" || n == "IFCSITE" || n == "IFCSPACE"
        })
        .map(|(id, _)| id)
        .expect("the fixture has a spatial container");
    let mut rel = vec![Value::Null; 6];
    rel[4] = Value::List(vec![Value::Ref(sign)]);
    rel[5] = Value::Ref(container);
    add(&mut model, "IFCRELCONTAINEDINSPATIALSTRUCTURE", rel);

    let findings = unreachable_products(&model);

    assert_eq!(
        findings.len(),
        1,
        "exactly the authored sign is reported, got {findings:?}"
    );
    assert_eq!(findings[0].0, sign);
    assert!(
        matches!(
            &findings[0].1,
            Unreachable::NoRepresentationInModelContext { .. }
        ),
        "got {:?}",
        findings[0].1
    );
    assert!(
        findings[0].1.message().contains("model viewer"),
        "the message must say what is wrong: {}",
        findings[0].1.message()
    );
}
