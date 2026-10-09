//! Where-rules verified against models that violate them.
//!
//! # Why these are integration tests
//!
//! A rule is only useful if it fires on a file a real exporter could produce
//! and stays silent on a valid one. Both halves are asserted here: every
//! violation case has a matching valid case, because a checker that flags
//! everything is as useless as one that flags nothing.

use ifc_geometry::rules::{self, ViolationKind};
use ifc_model::{Entity, EntityId, Model, Value};

fn n(x: f64) -> Value {
    Value::Real(x)
}
fn r(id: u64) -> Value {
    Value::Ref(EntityId(id))
}

/// Build a model with a point, two directions and a placement referencing them.
fn placement_model(axis: &[f64], ref_dir: &[f64], location: &[f64]) -> Model {
    let mut m = Model::new();
    m.insert(
        EntityId(1),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(location.iter().copied().map(n).collect())],
        ),
    );
    m.insert(
        EntityId(2),
        Entity::new(
            "IFCDIRECTION",
            vec![Value::List(axis.iter().copied().map(n).collect())],
        ),
    );
    m.insert(
        EntityId(3),
        Entity::new(
            "IFCDIRECTION",
            vec![Value::List(ref_dir.iter().copied().map(n).collect())],
        ),
    );
    m.insert(
        EntityId(4),
        Entity::new("IFCAXIS2PLACEMENT3D", vec![r(1), r(2), r(3)]),
    );
    m
}

/// `AxisToRefDirPosition`: a parallel axis and reference direction give a
/// degenerate basis. This is the failure that silently produces a collapsed
/// transform rather than an error.
#[test]
fn parallel_axis_and_ref_direction_is_reported() {
    let m = placement_model(&[0.0, 0.0, 1.0], &[0.0, 0.0, 5.0], &[0.0, 0.0, 0.0]);
    let violations = rules::validate(&m, EntityId(4));

    let found = violations
        .iter()
        .find(|v| v.rule == "AxisToRefDirPosition")
        .unwrap_or_else(|| panic!("expected the rule to fire, got {violations:?}"));
    assert_eq!(found.kind, ViolationKind::Degenerate);
    assert!(found.detail.contains("parallel"), "{}", found.detail);
}

/// The same placement with independent directions must be silent.
#[test]
fn an_orthogonal_placement_produces_no_violations() {
    let m = placement_model(&[0.0, 0.0, 1.0], &[1.0, 0.0, 0.0], &[0.0, 0.0, 0.0]);
    assert!(
        rules::validate(&m, EntityId(4)).is_empty(),
        "a valid placement must not be flagged"
    );
}

/// A non-perpendicular but independent RefDirection is LEGAL: the schema only
/// forbids parallel. Flagging it would reject conforming files.
#[test]
fn a_skewed_but_independent_ref_direction_is_accepted() {
    let m = placement_model(&[0.0, 0.0, 1.0], &[1.0, 0.0, 0.3], &[0.0, 0.0, 0.0]);
    assert!(
        rules::validate(&m, EntityId(4)).is_empty(),
        "non-perpendicular is legal; only parallel is not"
    );
}

/// `LocationIs3D`: a 2D point on a 3D placement.
#[test]
fn a_2d_location_on_a_3d_placement_is_reported() {
    let m = placement_model(&[0.0, 0.0, 1.0], &[1.0, 0.0, 0.0], &[1.0, 2.0]);
    let violations = rules::validate(&m, EntityId(4));
    assert!(
        violations.iter().any(|v| v.rule == "LocationIs3D"),
        "got {violations:?}"
    );
}

/// `AxisAndRefDirProvision`: one without the other is under-determined.
#[test]
fn providing_only_one_of_axis_and_ref_direction_is_reported() {
    let mut m = placement_model(&[0.0, 0.0, 1.0], &[1.0, 0.0, 0.0], &[0.0, 0.0, 0.0]);
    m.insert(
        EntityId(4),
        Entity::new("IFCAXIS2PLACEMENT3D", vec![r(1), r(2), Value::Null]),
    );
    let violations = rules::validate(&m, EntityId(4));
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "AxisAndRefDirProvision"),
        "got {violations:?}"
    );
}

/// Both absent is the documented default (global axes), not a violation.
#[test]
fn omitting_both_axis_and_ref_direction_is_the_documented_default() {
    let mut m = placement_model(&[0.0, 0.0, 1.0], &[1.0, 0.0, 0.0], &[0.0, 0.0, 0.0]);
    m.insert(
        EntityId(4),
        Entity::new("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    assert!(rules::validate(&m, EntityId(4)).is_empty());
}

/// `MagnitudeGreaterZero`: a zero direction has no orientation.
#[test]
fn a_zero_length_direction_is_reported() {
    let mut m = Model::new();
    m.insert(
        EntityId(1),
        Entity::new(
            "IFCDIRECTION",
            vec![Value::List(vec![n(0.0), n(0.0), n(0.0)])],
        ),
    );
    let violations = rules::validate(&m, EntityId(1));
    assert!(
        violations.iter().any(|v| v.rule == "MagnitudeGreaterZero"),
        "got {violations:?}"
    );
}

/// `ValidExtrusionDirection`: extruding along the profile plane sweeps no
/// volume. The schema states it as a dot product against the z axis.
#[test]
fn an_extrusion_parallel_to_its_profile_plane_is_reported() {
    let mut m = Model::new();
    // Direction lying entirely in XY -> zero z component.
    m.insert(
        EntityId(1),
        Entity::new(
            "IFCDIRECTION",
            vec![Value::List(vec![n(1.0), n(0.0), n(0.0)])],
        ),
    );
    m.insert(
        EntityId(2),
        Entity::new("IFCEXTRUDEDAREASOLID", vec![r(90), r(91), r(1), n(2.5)]),
    );
    let violations = rules::validate(&m, EntityId(2));
    let found = violations
        .iter()
        .find(|v| v.rule == "ValidExtrusionDirection")
        .unwrap_or_else(|| panic!("expected the rule to fire, got {violations:?}"));
    assert_eq!(found.kind, ViolationKind::Degenerate);
}

/// A normal vertical extrusion is valid.
#[test]
fn a_vertical_extrusion_is_accepted() {
    let mut m = Model::new();
    m.insert(
        EntityId(1),
        Entity::new(
            "IFCDIRECTION",
            vec![Value::List(vec![n(0.0), n(0.0), n(1.0)])],
        ),
    );
    m.insert(
        EntityId(2),
        Entity::new("IFCEXTRUDEDAREASOLID", vec![r(90), r(91), r(1), n(2.5)]),
    );
    assert!(rules::validate(&m, EntityId(2)).is_empty());
}

/// A tilted extrusion is legal: the schema forbids only perpendicularity.
#[test]
fn a_tilted_extrusion_is_accepted() {
    let mut m = Model::new();
    m.insert(
        EntityId(1),
        Entity::new(
            "IFCDIRECTION",
            vec![Value::List(vec![n(0.3), n(0.0), n(1.0)])],
        ),
    );
    m.insert(
        EntityId(2),
        Entity::new("IFCEXTRUDEDAREASOLID", vec![r(90), r(91), r(1), n(2.5)]),
    );
    assert!(rules::validate(&m, EntityId(2)).is_empty());
}

/// A non-positive depth is not a length.
#[test]
fn a_zero_depth_extrusion_is_reported() {
    let mut m = Model::new();
    m.insert(
        EntityId(1),
        Entity::new(
            "IFCDIRECTION",
            vec![Value::List(vec![n(0.0), n(0.0), n(1.0)])],
        ),
    );
    m.insert(
        EntityId(2),
        Entity::new("IFCEXTRUDEDAREASOLID", vec![r(90), r(91), r(1), n(0.0)]),
    );
    let violations = rules::validate(&m, EntityId(2));
    assert!(
        violations
            .iter()
            .any(|v| v.kind == ViolationKind::OutOfRange),
        "got {violations:?}"
    );
}

/// `IfcBooleanClippingResult` must use DIFFERENCE with a half space.
#[test]
fn a_clipping_result_with_the_wrong_operator_is_reported() {
    let mut m = Model::new();
    m.insert(
        EntityId(1),
        Entity::new("IFCEXTRUDEDAREASOLID", vec![r(90), r(91), r(92), n(1.0)]),
    );
    m.insert(
        EntityId(2),
        Entity::new("IFCHALFSPACESOLID", vec![r(93), Value::Bool(true)]),
    );
    m.insert(
        EntityId(3),
        Entity::new(
            "IFCBOOLEANCLIPPINGRESULT",
            vec![Value::Enum("UNION".into()), r(1), r(2)],
        ),
    );
    let violations = rules::validate(&m, EntityId(3));
    assert!(
        violations.iter().any(|v| v.rule == "OperatorType"),
        "clipping must be DIFFERENCE, got {violations:?}"
    );
}

/// A correct clipping result is silent.
#[test]
fn a_valid_clipping_result_is_accepted() {
    let mut m = Model::new();
    m.insert(
        EntityId(1),
        Entity::new("IFCEXTRUDEDAREASOLID", vec![r(90), r(91), r(92), n(1.0)]),
    );
    m.insert(
        EntityId(2),
        Entity::new("IFCHALFSPACESOLID", vec![r(93), Value::Bool(true)]),
    );
    m.insert(
        EntityId(3),
        Entity::new(
            "IFCBOOLEANCLIPPINGRESULT",
            vec![Value::Enum("DIFFERENCE".into()), r(1), r(2)],
        ),
    );
    assert!(
        rules::validate(&m, EntityId(3)).is_empty(),
        "{:?}",
        rules::validate(&m, EntityId(3))
    );
}

/// `BoundaryType`: the polygonal boundary must be a polyline or composite.
#[test]
fn a_half_space_bounded_by_a_circle_is_reported() {
    let mut m = Model::new();
    m.insert(EntityId(1), Entity::new("IFCCIRCLE", vec![r(90), n(2.0)]));
    m.insert(
        EntityId(2),
        Entity::new(
            "IFCPOLYGONALBOUNDEDHALFSPACE",
            vec![r(91), Value::Bool(true), r(92), r(1)],
        ),
    );
    let violations = rules::validate(&m, EntityId(2));
    assert!(
        violations.iter().any(|v| v.rule == "BoundaryType"),
        "got {violations:?}"
    );
}

/// A half-space bounded by `boundary` (one record of `type_name`) in a model
/// declaring `schema` in `FILE_SCHEMA`, or none.
fn bounded_half_space(schema: Option<&str>, type_name: &str) -> Model {
    let mut m = Model::new();
    if let Some(schema) = schema {
        m.header_mut().schema = vec![schema.to_owned()];
    }
    m.insert(EntityId(1), Entity::new(type_name, vec![]));
    m.insert(
        EntityId(2),
        Entity::new(
            "IFCPOLYGONALBOUNDEDHALFSPACE",
            vec![r(91), Value::Bool(true), r(92), r(1)],
        ),
    );
    m
}

/// `BoundaryType` violations, under IFC4's name or IFC2X3 TC1's `WR42`.
fn boundary_type_violations(m: &Model) -> Vec<rules::RuleViolation> {
    rules::validate(m, EntityId(2))
        .into_iter()
        .filter(|v| v.rule == "BoundaryType" || v.rule == "WR42")
        .collect()
}

/// `BoundaryType` is the declared release's (#397): IFC4X3 ADD2 admits
/// `IfcIndexedPolyCurve`; IFC2X3 TC1, IFC4 ADD2 TC1, IFC4X1 and IFC4X2 admit
/// only `IfcPolyline` and `IfcCompositeCurve`, as does a file that declares
/// no release this crate knows (read as IFC4).
#[test]
fn an_indexed_half_space_boundary_is_admitted_only_in_ifc4x3() {
    for schema in ["IFC4X3_ADD2", "IFC4X3"] {
        let m = bounded_half_space(Some(schema), "IFCINDEXEDPOLYCURVE");
        assert!(boundary_type_violations(&m).is_empty(), "{schema}");
    }
    for schema in [
        Some("IFC2X3"),
        Some("IFC4"),
        Some("IFC4X1"),
        Some("IFC4X2"),
        None,
    ] {
        let m = bounded_half_space(schema, "IFCINDEXEDPOLYCURVE");
        let found = boundary_type_violations(&m);
        assert_eq!(found.len(), 1, "{schema:?}: {found:?}");
        assert_eq!(found[0].kind, ViolationKind::WrongType);
    }
    for schema in [Some("IFC2X3"), Some("IFC4"), Some("IFC4X3_ADD2"), None] {
        for curve in ["IFCPOLYLINE", "IFCCOMPOSITECURVE"] {
            let m = bounded_half_space(schema, curve);
            assert!(
                boundary_type_violations(&m).is_empty(),
                "{schema:?} {curve}"
            );
        }
        let m = bounded_half_space(schema, "IFCTRIMMEDCURVE");
        assert_eq!(boundary_type_violations(&m).len(), 1, "{schema:?}");
    }
}

/// `TYPEOF` carries every supertype, so a subtype of `IfcCompositeCurve`
/// satisfies `BoundaryType` as the composite itself does.
#[test]
fn a_composite_curve_subtype_satisfies_boundary_type() {
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let m = bounded_half_space(Some(schema), "IFCBOUNDARYCURVE");
        assert!(boundary_type_violations(&m).is_empty(), "{schema}");
    }
}

/// Validating a whole model reports every violation, not the first.
#[test]
fn model_validation_finds_every_violation() {
    let mut m = placement_model(&[0.0, 0.0, 1.0], &[0.0, 0.0, 2.0], &[0.0, 0.0, 0.0]);
    m.insert(
        EntityId(10),
        Entity::new(
            "IFCDIRECTION",
            vec![Value::List(vec![n(0.0), n(0.0), n(0.0)])],
        ),
    );
    let all = rules::validate_model(&m);
    assert!(
        all.len() >= 2,
        "expected the parallel axis AND the zero direction, got {all:?}"
    );
}

/// A model with no geometry problems yields nothing.
#[test]
fn a_clean_model_yields_no_violations() {
    let m = placement_model(&[0.0, 0.0, 1.0], &[1.0, 0.0, 0.0], &[0.0, 0.0, 0.0]);
    assert!(rules::validate_model(&m).is_empty());
}
