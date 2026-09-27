//! The IFC4 curve and surface structural actions.
//!
//! These replaced the IFC2x3 linear/planar pair and added a mandatory
//! `PredefinedType`. The writer refuses USERDEFINED without an
//! ObjectType: the token means "look at ObjectType for the real name",
//! and there is nothing there.

use ifc_model::{Model, Transaction, Value};
use ifc_schema::ifc4x3;
use ifc_structural::{
    stage_action, stage_load, ActionDraft, ActionDraftKind, CoordinateSystem, LoadDraft,
    StructuralRootDraft,
};

const GUID: &str = "1A$vWGh2j9nOAcMUCJ3$Ab";

fn root(object_type: Option<&str>) -> StructuralRootDraft {
    StructuralRootDraft {
        global_id: GUID.into(),
        object_type: object_type.map(Into::into),
        ..StructuralRootDraft::default()
    }
}

fn linear_load(tx: &mut Transaction, schema: &ifc_schema::Schema) -> ifc_model::EntityId {
    stage_load(
        tx,
        schema,
        LoadDraft::LinearForce {
            name: None,
            force: [Some(1.0), None, None],
            moment: [None, None, None],
        },
    )
    .expect("a linear force is accepted")
}

/// A curve action carries its PredefinedType at the declared slot.
#[test]
fn a_curve_action_keeps_its_predefined_type() {
    let schema = ifc4x3();
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let load = linear_load(&mut tx, schema);

    let id = stage_action(
        &mut tx,
        &model,
        schema,
        ActionDraft {
            root: root(None),
            applied_load: load,
            coordinate_system: CoordinateSystem::Global,
            destabilizing_load: Some(false),
            caused_by: None,
            kind: ActionDraftKind::Curve {
                projected_or_true: None,
                predefined_type: "CONST",
            },
        },
    )
    .expect("a curve action is accepted");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let e = model.get(id).expect("staged");
    assert_eq!(e.type_name.as_ref(), "IFCSTRUCTURALCURVEACTION");
    assert_eq!(e.attributes[7], Value::Ref(load), "AppliedLoad at slot 7");
    assert_eq!(
        e.attributes[8],
        Value::Enum("GLOBAL_COORDS".into()),
        "GlobalOrLocal at slot 8"
    );
    assert_eq!(
        e.attributes[11],
        Value::Enum("CONST".into()),
        "PredefinedType at slot 11, after the optional pair"
    );
}

/// A surface action stages its own class, not the curve one.
#[test]
fn a_surface_action_is_not_a_curve_action() {
    let schema = ifc4x3();
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    let load = stage_load(
        &mut tx,
        schema,
        LoadDraft::PlanarForce {
            name: None,
            force: [Some(2.0), None, None],
        },
    )
    .expect("a planar force is accepted");

    let id = stage_action(
        &mut tx,
        &model,
        schema,
        ActionDraft {
            root: root(None),
            applied_load: load,
            coordinate_system: CoordinateSystem::Global,
            destabilizing_load: Some(false),
            caused_by: None,
            kind: ActionDraftKind::Surface {
                projected_or_true: None,
                predefined_type: "NOTDEFINED",
            },
        },
    )
    .expect("a surface action is accepted");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let e = model.get(id).expect("staged");
    assert_eq!(e.type_name.as_ref(), "IFCSTRUCTURALSURFACEACTION");
    assert_eq!(e.attributes[11], Value::Enum("NOTDEFINED".into()));
}

/// USERDEFINED without an ObjectType names nothing, and is refused.
#[test]
fn userdefined_without_an_object_type_is_refused() {
    let schema = ifc4x3();
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let load = linear_load(&mut tx, schema);

    let attempt = |tx: &mut Transaction, object_type: Option<&str>| {
        stage_action(
            tx,
            &model,
            schema,
            ActionDraft {
                root: root(object_type),
                applied_load: load,
                coordinate_system: CoordinateSystem::Global,
                destabilizing_load: Some(false),
                caused_by: None,
                kind: ActionDraftKind::Curve {
                    projected_or_true: None,
                    predefined_type: "USERDEFINED",
                },
            },
        )
    };

    assert!(
        attempt(&mut tx, None).is_err(),
        "USERDEFINED with no ObjectType is refused"
    );
    assert!(
        attempt(&mut tx, Some("CableTension")).is_ok(),
        "the same token with an ObjectType is accepted"
    );
}

/// A curve action refuses a load whose dimension it cannot carry.
///
/// The load select is per-subtype: a planar force belongs on a surface
/// action. Accepting it here would produce a file whose analysis is
/// dimensionally meaningless but structurally well formed.
#[test]
fn a_curve_action_refuses_a_planar_load() {
    let schema = ifc4x3();
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    let planar = stage_load(
        &mut tx,
        schema,
        LoadDraft::PlanarForce {
            name: None,
            force: [Some(2.0), None, None],
        },
    )
    .expect("a planar force is accepted on its own");

    let result = stage_action(
        &mut tx,
        &model,
        schema,
        ActionDraft {
            root: root(None),
            applied_load: planar,
            coordinate_system: CoordinateSystem::Global,
            destabilizing_load: Some(false),
            caused_by: None,
            kind: ActionDraftKind::Curve {
                projected_or_true: None,
                predefined_type: "CONST",
            },
        },
    );
    assert!(
        result.is_err(),
        "a planar force is not a load a curve action can carry"
    );
}
