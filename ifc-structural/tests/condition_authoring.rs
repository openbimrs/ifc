//! Boundary condition authoring.
//!
//! The reader resolves stiffness values by ATTRIBUTE NAME through the
//! schema, and each family names its attributes differently
//! (TranslationalStiffnessX vs ...ByLengthX vs ...ByAreaX). So the entity
//! type chosen when authoring decides whether anything is readable at all.
//! Every test here commits and reads back rather than inspecting slots.

use ifc_model::{Model, Transaction};
use ifc_structural::{
    stage_boundary_condition, AxisValues, BoundaryConditionDraft, BoundaryConditionKind,
    StiffnessValue, StructuralView,
};

fn ifc4_model() -> Model {
    let mut model = Model::default();
    model.header_mut().schema = vec!["IFC4".to_owned()];
    model
}

fn measures(x: f64, y: f64, z: f64) -> AxisValues<Option<StiffnessValue>> {
    AxisValues {
        x: Some(StiffnessValue::Measure(x)),
        y: Some(StiffnessValue::Measure(y)),
        z: Some(StiffnessValue::Measure(z)),
    }
}

#[test]
fn a_node_condition_reads_back_every_authored_stiffness() {
    let mut model = ifc4_model();
    let mut tx = Transaction::new(&model);
    let id = stage_boundary_condition(
        &mut tx,
        BoundaryConditionKind::Node,
        BoundaryConditionDraft {
            name: Some("Pinned"),
            translational: measures(1.0, 2.0, 3.0),
            rotational: AxisValues::default(),
            warping: None,
        },
    )
    .expect("node condition");
    tx.commit(&mut model).expect("commit");
    let view = StructuralView::for_model(&model).expect("view");
    let condition = view.boundary_condition(id).expect("readable");
    assert_eq!(condition.name().expect("name"), Some("Pinned"));
    let t = condition
        .translational_stiffnesses()
        .expect("translational");
    assert_eq!(t.x, Some(StiffnessValue::Measure(1.0)));
    assert_eq!(t.y, Some(StiffnessValue::Measure(2.0)));
    assert_eq!(t.z, Some(StiffnessValue::Measure(3.0)));
}

#[test]
fn each_family_writes_the_attribute_names_its_reader_expects() {
    // An edge condition stores ...ByLengthX, a face condition ...ByAreaX.
    // If stage_boundary_condition picked the wrong entity, the values would
    // be written under names the reader never looks up and every axis would
    // come back None.
    for (kind, rotational) in [
        (BoundaryConditionKind::Edge, measures(4.0, 5.0, 6.0)),
        (BoundaryConditionKind::Face, AxisValues::default()),
    ] {
        let mut model = ifc4_model();
        let mut tx = Transaction::new(&model);
        let id = stage_boundary_condition(
            &mut tx,
            kind,
            BoundaryConditionDraft {
                name: None,
                translational: measures(1.0, 2.0, 3.0),
                rotational,
                warping: None,
            },
        )
        .expect("condition");
        tx.commit(&mut model).expect("commit");
        let expected_entity = match kind {
            BoundaryConditionKind::Edge => "IFCBOUNDARYEDGECONDITION",
            BoundaryConditionKind::Face => "IFCBOUNDARYFACECONDITION",
            BoundaryConditionKind::Node => "IFCBOUNDARYNODECONDITION",
            BoundaryConditionKind::NodeWarping => "IFCBOUNDARYNODECONDITIONWARPING",
        };
        assert_eq!(
            model.get(id).expect("stored").type_name.as_ref(),
            expected_entity,
            "{kind:?} must be written as its own entity, not a sibling family"
        );
        let view = StructuralView::for_model(&model).expect("view");
        let condition = view.boundary_condition(id).expect("readable");
        let t = condition
            .translational_stiffnesses()
            .expect("translational");
        assert_eq!(
            t.x,
            Some(StiffnessValue::Measure(1.0)),
            "{kind:?} must store its translational X where the reader looks"
        );
    }
}

#[test]
fn warping_is_readable_only_on_the_family_that_declares_it() {
    let mut model = ifc4_model();
    let mut tx = Transaction::new(&model);
    let id = stage_boundary_condition(
        &mut tx,
        BoundaryConditionKind::NodeWarping,
        BoundaryConditionDraft {
            name: None,
            translational: measures(1.0, 2.0, 3.0),
            rotational: measures(4.0, 5.0, 6.0),
            warping: Some(StiffnessValue::Measure(7.0)),
        },
    )
    .expect("warping condition");
    tx.commit(&mut model).expect("commit");
    let view = StructuralView::for_model(&model).expect("view");
    let condition = view.boundary_condition(id).expect("readable");
    assert_eq!(
        condition.warping_stiffness().expect("warping"),
        Some(StiffnessValue::Measure(7.0)),
        "warping sits after the six stiffness slots"
    );
    let r = condition.rotational_stiffnesses().expect("rotational");
    assert_eq!(r.z, Some(StiffnessValue::Measure(6.0)));
}

#[test]
fn a_value_the_family_does_not_declare_is_refused() {
    let model = ifc4_model();
    let mut tx = Transaction::new(&model);
    // A face condition has no rotational attributes at all.
    assert!(
        stage_boundary_condition(
            &mut tx,
            BoundaryConditionKind::Face,
            BoundaryConditionDraft {
                name: None,
                translational: measures(1.0, 2.0, 3.0),
                rotational: measures(4.0, 5.0, 6.0),
                warping: None,
            },
        )
        .is_err(),
        "a rotational spring on a face is a modelling error, not a dropped field"
    );
    // Warping belongs to IfcBoundaryNodeConditionWarping only.
    assert!(
        stage_boundary_condition(
            &mut tx,
            BoundaryConditionKind::Node,
            BoundaryConditionDraft {
                name: None,
                translational: AxisValues::default(),
                rotational: AxisValues::default(),
                warping: Some(StiffnessValue::Measure(1.0)),
            },
        )
        .is_err(),
        "a plain node condition declares no warping stiffness"
    );
}
