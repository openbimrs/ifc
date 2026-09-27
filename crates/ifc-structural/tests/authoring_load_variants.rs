//! The four load subtypes that fell through the original batch.

use ifc_model::{Model, Transaction, Value};
use ifc_schema::ifc4x3;
use ifc_structural::{stage_load, LoadDraft, StructuralError};
/// Warping and distortion are plain numeric siblings: seven slots each.
#[test]
fn warping_and_distortion_loads_stage_their_components() {
    let schema = ifc4x3();
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);

    let warping = stage_load(
        &mut tx,
        schema,
        LoadDraft::SingleForceWarping {
            name: Some("Warp".into()),
            force: [Some(1.0), Some(2.0), Some(3.0)],
            moment: [Some(4.0), Some(5.0), Some(6.0)],
            warping_moment: Some(7.0),
        },
    )
    .expect("a finite warping load is accepted");

    let distortion = stage_load(
        &mut tx,
        schema,
        LoadDraft::SingleDisplacementDistortion {
            name: None,
            displacement: [Some(1.0), None, None],
            rotation: [None, None, Some(2.0)],
            distortion: Some(3.0),
        },
    )
    .expect("a finite distortion load is accepted");

    tx.commit(&mut model).expect("commit");

    let w = model.get(warping).expect("staged");
    assert_eq!(w.type_name.as_ref(), "IFCSTRUCTURALLOADSINGLEFORCEWARPING");
    assert_eq!(w.attributes[7], Value::Real(7.0), "WarpingMoment last");

    let d = model.get(distortion).expect("staged");
    assert_eq!(
        d.type_name.as_ref(),
        "IFCSTRUCTURALLOADSINGLEDISPLACEMENTDISTORTION"
    );
    assert_eq!(d.attributes[7], Value::Real(3.0), "Distortion last");
}
/// SurfaceAndOrShearAreaSpecified: at least one area must exist.
///
/// All-null is the failure a caller writing plain optional slots would
/// produce silently, so it is refused rather than staged empty.
#[test]
fn a_reinforcement_area_with_nothing_specified_is_refused() {
    let schema = ifc4x3();
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    let err = stage_load(
        &mut tx,
        schema,
        LoadDraft::SurfaceReinforcementArea {
            name: Some("Empty".into()),
            surface_1: None,
            surface_2: None,
            shear: None,
        },
    )
    .expect_err("WHERE SurfaceAndOrShearAreaSpecified");
    assert!(
        matches!(err, StructuralError::InvalidDraftValue { .. }),
        "{err:?}"
    );
}

/// NonnegativeArea1/2/3: a negative reinforcement area is meaningless.
#[test]
fn a_negative_reinforcement_area_is_refused() {
    let schema = ifc4x3();
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    for draft in [
        LoadDraft::SurfaceReinforcementArea {
            name: None,
            surface_1: Some(vec![-1.0, 2.0]),
            surface_2: None,
            shear: None,
        },
        LoadDraft::SurfaceReinforcementArea {
            name: None,
            surface_1: None,
            surface_2: None,
            shear: Some(-0.5),
        },
    ] {
        let err = stage_load(&mut tx, schema, draft).expect_err("negative area");
        assert!(
            matches!(err, StructuralError::InvalidDraftValue { .. }),
            "{err:?}"
        );
    }
}
/// ValidListSize: Locations must match Values one for one.
///
/// A mismatched pair is the bug this rule exists to catch: three load
/// values against two locations leaves the third unplaced, and nothing
/// downstream can tell which one.
#[test]
fn a_configuration_with_mismatched_locations_is_refused() {
    let schema = ifc4x3();
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);

    let a = stage_load(
        &mut tx,
        schema,
        LoadDraft::PlanarForce {
            name: None,
            force: [Some(1.0), None, None],
        },
    )
    .expect("a planar force stages");
    tx.commit(&mut model).expect("commit");

    let mut tx = Transaction::new(&model);
    let err = stage_load(
        &mut tx,
        schema,
        LoadDraft::Configuration {
            name: None,
            values: vec![a, a],
            locations: Some(vec![[0.0, 0.0]]),
        },
    )
    .expect_err("WHERE ValidListSize");
    assert!(
        matches!(err, StructuralError::InvalidDraftValue { .. }),
        "{err:?}"
    );
}

/// An empty Values list violates LIST [1:?].
#[test]
fn a_configuration_needs_at_least_one_value() {
    let schema = ifc4x3();
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    let err = stage_load(
        &mut tx,
        schema,
        LoadDraft::Configuration {
            name: None,
            values: Vec::new(),
            locations: None,
        },
    )
    .expect_err("LIST [1:?] OF IfcStructuralLoadOrResult");
    assert!(
        matches!(err, StructuralError::InvalidDraftValue { .. }),
        "{err:?}"
    );
}
/// The accepting cases, so the refusals above are not vacuous.
#[test]
fn well_formed_area_and_configuration_are_accepted() {
    let schema = ifc4x3();
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);

    let area = stage_load(
        &mut tx,
        schema,
        LoadDraft::SurfaceReinforcementArea {
            name: None,
            surface_1: Some(vec![1.0, 2.0]),
            surface_2: None,
            shear: Some(0.0),
        },
    )
    .expect("non-negative areas with one specified");

    let value = stage_load(
        &mut tx,
        schema,
        LoadDraft::PlanarForce {
            name: None,
            force: [Some(1.0), None, None],
        },
    )
    .expect("a planar force stages");
    tx.commit(&mut model).expect("commit");

    let mut tx = Transaction::new(&model);
    let config = stage_load(
        &mut tx,
        schema,
        LoadDraft::Configuration {
            name: Some("Pair".into()),
            values: vec![value, value],
            locations: Some(vec![[0.0, 0.0], [1.0, 0.0]]),
        },
    )
    .expect("matched list sizes");
    tx.commit(&mut model).expect("commit");

    assert_eq!(
        model.get(area).expect("staged").type_name.as_ref(),
        "IFCSURFACEREINFORCEMENTAREA"
    );
    let c = model.get(config).expect("staged");
    assert_eq!(c.type_name.as_ref(), "IFCSTRUCTURALLOADCONFIGURATION");
    assert!(matches!(c.attributes[1], Value::List(ref v) if v.len() == 2));
}
