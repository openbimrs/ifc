//! Sweep every variant of the structural draft enums.
//!
//! `stage_action`, `stage_member` and `stage_connection` each dispatch a
//! kind enum to a type name through a tuple. Existing tests exercise a
//! subset of the variants, so the unexercised arms were never proven to
//! stage at all -- a wrong type name or arity in one of them compiles and
//! passes the suite.
//!
//! One test per writer, walking every variant.

use ifc_model::{Entity, Model, Transaction, Value};
use ifc_schema::ifc4x3;
use ifc_structural::{
    stage_action, stage_connection, stage_load, stage_member, ActionDraft, ActionDraftKind,
    ConnectionDraft, ConnectionDraftKind, CoordinateSystem, LoadDraft, MemberDraft,
    MemberDraftKind, MemberPredefinedType, StructuralRootDraft,
};

const GUID: &str = "1hqA$FMcT8$hVvcqsRDBzZ";

fn root() -> StructuralRootDraft {
    StructuralRootDraft {
        global_id: GUID.to_owned(),
        owner_history: None,
        name: Some("Sweep".to_owned()),
        description: None,
        object_type: None,
        object_placement: None,
        representation: None,
    }
}

/// Every `ConnectionDraftKind` variant stages its own type.
#[test]
fn every_connection_variant_stages() {
    let mut seed = Model::default();
    let mut tx = Transaction::new(&seed);
    // IfcStructuralCurveConnection.AxisDirection is required in IFC4X3,
    // so the curve arm cannot be swept with an empty draft.
    let axis = tx.create(Entity::new("IFCDIRECTION", vec![Value::Null; 1]));
    tx.commit(&mut seed).expect("commit");

    let cases = [
        (
            ConnectionDraftKind::Point {
                applied_condition: None,
                condition_coordinate_system: None,
            },
            "IFCSTRUCTURALPOINTCONNECTION",
        ),
        (
            ConnectionDraftKind::Curve {
                applied_condition: None,
                axis: Some(axis),
            },
            "IFCSTRUCTURALCURVECONNECTION",
        ),
        (
            ConnectionDraftKind::Surface {
                applied_condition: None,
            },
            "IFCSTRUCTURALSURFACECONNECTION",
        ),
    ];

    for (kind, expected) in cases {
        let mut model = seed.clone();
        let mut tx = Transaction::new(&model);
        let id = stage_connection(
            &mut tx,
            &model,
            ifc4x3(),
            ConnectionDraft { root: root(), kind },
        )
        .unwrap_or_else(|error| panic!("{expected} refused: {error:?}"));
        tx.commit(&mut model).expect("commit");
        assert_eq!(model.get(id).expect("staged").type_name.as_ref(), expected);
    }
}

/// Every `MemberDraftKind` variant, in both the base and varying forms.
#[test]
fn every_member_variant_stages() {
    let mut seed = Model::default();
    let mut tx = Transaction::new(&seed);
    let axis = tx.create(Entity::new("IFCDIRECTION", vec![Value::Null; 1]));
    tx.commit(&mut seed).expect("commit");

    let cases = [
        (
            MemberDraftKind::Curve {
                predefined_type: MemberPredefinedType::Cable,
                axis: Some(axis),
                varying: false,
            },
            "IFCSTRUCTURALCURVEMEMBER",
        ),
        (
            MemberDraftKind::Curve {
                predefined_type: MemberPredefinedType::Cable,
                axis: Some(axis),
                varying: true,
            },
            "IFCSTRUCTURALCURVEMEMBERVARYING",
        ),
        (
            MemberDraftKind::Surface {
                predefined_type: MemberPredefinedType::Shell,
                thickness: Some(0.2),
                varying: false,
            },
            "IFCSTRUCTURALSURFACEMEMBER",
        ),
        (
            MemberDraftKind::Surface {
                predefined_type: MemberPredefinedType::Shell,
                thickness: Some(0.2),
                varying: true,
            },
            "IFCSTRUCTURALSURFACEMEMBERVARYING",
        ),
    ];

    for (kind, expected) in cases {
        let mut model = seed.clone();
        let mut tx = Transaction::new(&model);
        let id = stage_member(
            &mut tx,
            &model,
            ifc4x3(),
            MemberDraft { root: root(), kind },
        )
        .unwrap_or_else(|error| panic!("{expected} refused: {error:?}"));
        tx.commit(&mut model).expect("commit");
        assert_eq!(model.get(id).expect("staged").type_name.as_ref(), expected);
    }
}

/// Every `ActionDraftKind` variant stages, against a compatible load.
#[test]
fn every_action_variant_stages() {
    let cases = [
        (
            ActionDraftKind::Point,
            "IFCSTRUCTURALLOADSINGLEFORCE",
            "IFCSTRUCTURALPOINTACTION",
        ),
        (
            ActionDraftKind::Linear {
                projected_or_true: None,
            },
            "IFCSTRUCTURALLOADLINEARFORCE",
            "IFCSTRUCTURALLINEARACTION",
        ),
        (
            ActionDraftKind::Planar {
                projected_or_true: None,
            },
            "IFCSTRUCTURALLOADPLANARFORCE",
            "IFCSTRUCTURALPLANARACTION",
        ),
        (
            ActionDraftKind::Curve {
                projected_or_true: None,
                predefined_type: "CONST",
            },
            "IFCSTRUCTURALLOADLINEARFORCE",
            "IFCSTRUCTURALCURVEACTION",
        ),
        (
            ActionDraftKind::Surface {
                projected_or_true: None,
                predefined_type: "CONST",
            },
            "IFCSTRUCTURALLOADPLANARFORCE",
            "IFCSTRUCTURALSURFACEACTION",
        ),
    ];

    for (kind, load_type, expected) in cases {
        let mut model = Model::default();
        let mut tx = Transaction::new(&model);
        let load = tx.create(Entity::new(load_type, vec![Value::Null; 7]));
        let id = stage_action(
            &mut tx,
            &model,
            ifc4x3(),
            ActionDraft {
                root: root(),
                applied_load: load,
                coordinate_system: CoordinateSystem::Global,
                destabilizing_load: Some(false),
                caused_by: None,
                kind,
            },
        )
        .unwrap_or_else(|error| panic!("{expected} refused: {error:?}"));
        tx.commit(&mut model).expect("commit");
        assert_eq!(model.get(id).expect("staged").type_name.as_ref(), expected);
    }
}

/// The two curve forms name their axis slot differently.
///
/// A member carries `Axis`, a connection `AxisDirection`. Both are
/// required and both hold an `IfcDirection`, so a writer that resolves
/// one name for both leaves the other unauthorable: the connection
/// rejected a supplied axis as unsupported and a missing one as absent.
#[test]
fn the_curve_forms_use_their_own_axis_attribute() {
    let mut seed = Model::default();
    let mut tx = Transaction::new(&seed);
    let axis = tx.create(Entity::new("IFCDIRECTION", vec![Value::Null; 1]));
    tx.commit(&mut seed).expect("commit");

    let mut model = seed.clone();
    let mut tx = Transaction::new(&model);
    let connection = stage_connection(
        &mut tx,
        &model,
        ifc4x3(),
        ConnectionDraft {
            root: root(),
            kind: ConnectionDraftKind::Curve {
                applied_condition: None,
                axis: Some(axis),
            },
        },
    )
    .expect("curve connection");
    tx.commit(&mut model).expect("commit");

    let names = ifc4x3().attribute_names("IfcStructuralCurveConnection");
    assert_eq!(names[8], "AxisDirection", "the connection's own slot name");
    assert_eq!(
        model.get(connection).expect("staged").attributes[8],
        Value::Ref(axis),
        "the axis landed in AxisDirection, not a slot named Axis",
    );

    let mut model = seed.clone();
    let mut tx = Transaction::new(&model);
    let member = stage_member(
        &mut tx,
        &model,
        ifc4x3(),
        MemberDraft {
            root: root(),
            kind: MemberDraftKind::Curve {
                predefined_type: MemberPredefinedType::Cable,
                axis: Some(axis),
                varying: false,
            },
        },
    )
    .expect("curve member");
    tx.commit(&mut model).expect("commit");

    assert_eq!(
        ifc4x3().attribute_names("IfcStructuralCurveMember")[8],
        "Axis",
        "the member's own slot name",
    );
    assert_eq!(
        model.get(member).expect("staged").attributes[8],
        Value::Ref(axis),
    );
}

/// The plain single-displacement form stages.
///
/// It is the distortion form minus its trailing slot.
#[test]
fn the_single_displacement_form_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);

    let id = stage_load(
        &mut tx,
        ifc4x3(),
        LoadDraft::SingleDisplacement {
            name: Some("Settlement".to_owned()),
            displacement: [Some(0.0), Some(0.0), Some(-0.012)],
            rotation: [None, None, None],
        },
    )
    .expect("single displacement");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(
        staged.type_name.as_ref(),
        "IFCSTRUCTURALLOADSINGLEDISPLACEMENT",
    );
    assert_eq!(staged.attributes.len(), 7, "no Distortion slot");
    assert_eq!(staged.attributes[3], Value::Real(-0.012));
}
