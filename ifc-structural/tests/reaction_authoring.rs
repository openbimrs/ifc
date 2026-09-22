//! Authoring analysis outputs: reactions, result groups, connection
//! conditions, and the eccentric member connection.
//!
//! A reaction is what the analysis returns, as against the action that
//! was applied. The crate could read all of these and author none.

use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_schema::ifc4x3;
use ifc_structural::{
    stage_connection_condition, stage_member_connection, stage_reaction, stage_result_group,
    AxisValues, ConnectionConditionDraft, CoordinateSystem, FailureLimits, MemberConnectionDraft,
    ReactionDraft, ReactionDraftKind, RelationshipRootDraft, ResultGroupDraft, StructuralRootDraft,
};

const GUID: &str = "1hqA$FMcT8$hVvcqsRDBzZ";

fn root() -> StructuralRootDraft {
    StructuralRootDraft {
        global_id: GUID.to_owned(),
        owner_history: None,
        name: Some("Reaction".to_owned()),
        description: None,
        object_type: None,
        object_placement: None,
        representation: None,
    }
}

/// A model holding one load a reaction can point at.
fn seeded() -> (Model, EntityId) {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let load = tx.create(Entity::new(
        "IFCSTRUCTURALLOADSINGLEFORCE",
        vec![Value::Null; 7],
    ));
    tx.commit(&mut model).expect("commit");
    (model, load)
}

/// Every reaction variant stages its own type.
#[test]
fn every_reaction_variant_stages() {
    let (seed, load) = seeded();
    let cases = [
        (ReactionDraftKind::Point, "IFCSTRUCTURALPOINTREACTION"),
        (
            ReactionDraftKind::Curve {
                predefined_type: "DISCRETE",
            },
            "IFCSTRUCTURALCURVEREACTION",
        ),
        (
            ReactionDraftKind::Surface {
                predefined_type: "DISCRETE",
            },
            "IFCSTRUCTURALSURFACEREACTION",
        ),
    ];

    for (kind, expected) in cases {
        let mut model = seed.clone();
        let mut tx = Transaction::new(&model);
        let id = stage_reaction(
            &mut tx,
            &model,
            ifc4x3(),
            ReactionDraft {
                root: root(),
                applied_load: load,
                coordinate_system: CoordinateSystem::Global,
                kind,
            },
        )
        .unwrap_or_else(|error| panic!("{expected} refused: {error:?}"));
        tx.commit(&mut model).expect("commit");
        assert_eq!(model.get(id).expect("staged").type_name.as_ref(), expected);
    }
}

/// The point form has no `PredefinedType` slot at all.
///
/// Curve and surface reactions declare one and require it; the point
/// form's arity stops at `GlobalOrLocal`, so there is no field to set.
#[test]
fn the_point_reaction_has_no_predefined_type() {
    let (mut model, load) = seeded();
    let mut tx = Transaction::new(&model);
    let id = stage_reaction(
        &mut tx,
        &model,
        ifc4x3(),
        ReactionDraft {
            root: root(),
            applied_load: load,
            coordinate_system: CoordinateSystem::Local,
            kind: ReactionDraftKind::Point,
        },
    )
    .expect("point reaction");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.attributes.len(), 9, "point reaction arity");
    assert_eq!(
        staged.attributes[8],
        Value::Enum("LOCAL_COORDS".into()),
        "GlobalOrLocal is the last slot",
    );
    assert_eq!(
        ifc4x3().attribute_names("IfcStructuralPointReaction").len(),
        9,
        "the schema agrees there is no PredefinedType",
    );
}

/// A reaction refuses a load that is not a structural load.
#[test]
fn a_non_load_applied_load_is_refused() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let wrong = tx.create(Entity::new("IFCWALL", vec![Value::Null; 9]));
    tx.commit(&mut model).expect("commit");

    let mut tx = Transaction::new(&model);
    assert!(
        stage_reaction(
            &mut tx,
            &model,
            ifc4x3(),
            ReactionDraft {
                root: root(),
                applied_load: wrong,
                coordinate_system: CoordinateSystem::Global,
                kind: ReactionDraftKind::Point,
            },
        )
        .is_err(),
        "a wall was accepted as an applied load",
    );
}

/// A curve reaction refuses a token its activity enum does not declare.
#[test]
fn a_bogus_reaction_token_is_refused() {
    let (model, load) = seeded();
    let mut tx = Transaction::new(&model);
    assert!(
        stage_reaction(
            &mut tx,
            &model,
            ifc4x3(),
            ReactionDraft {
                root: root(),
                applied_load: load,
                coordinate_system: CoordinateSystem::Global,
                kind: ReactionDraftKind::Curve {
                    predefined_type: "NOT_A_TOKEN",
                },
            },
        )
        .is_err(),
        "an undeclared activity token was accepted",
    );
}

/// A result group stages its theory and linearity.
#[test]
fn a_result_group_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let id = stage_result_group(
        &mut tx,
        &model,
        ifc4x3(),
        ResultGroupDraft {
            global_id: GUID.to_owned(),
            owner_history: None,
            name: Some("Results".to_owned()),
            description: None,
            object_type: None,
            theory_type: "FIRST_ORDER_THEORY".to_owned(),
            result_for_load_group: None,
            is_linear: true,
        },
    )
    .expect("result group");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCSTRUCTURALRESULTGROUP");
    assert_eq!(
        staged.attributes[5],
        Value::Enum("FIRST_ORDER_THEORY".into())
    );
    assert_eq!(staged.attributes[7], Value::Bool(true), "IsLinear");
}

/// `HasObjectType`: a USERDEFINED theory must name itself.
#[test]
fn a_userdefined_theory_without_an_object_type_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    assert!(
        stage_result_group(
            &mut tx,
            &model,
            ifc4x3(),
            ResultGroupDraft {
                global_id: GUID.to_owned(),
                owner_history: None,
                name: None,
                description: None,
                object_type: None,
                theory_type: "USERDEFINED".to_owned(),
                result_for_load_group: None,
                is_linear: false,
            },
        )
        .is_err(),
        "a USERDEFINED theory was accepted with no ObjectType",
    );

    // The same draft passes once the theory is named.
    let mut named = root();
    named.object_type = Some("Custom second-order".to_owned());
    let mut tx = Transaction::new(&model);
    assert!(
        stage_result_group(
            &mut tx,
            &model,
            ifc4x3(),
            ResultGroupDraft {
                global_id: GUID.to_owned(),
                owner_history: None,
                name: None,
                description: None,
                object_type: Some("Custom theory".to_owned()),
                theory_type: "USERDEFINED".to_owned(),
                result_for_load_group: None,
                is_linear: false,
            },
        )
        .is_ok(),
        "a named USERDEFINED theory was refused",
    );
}

/// Both connection-condition families stage their own measures.
#[test]
fn both_connection_condition_families_stage() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);

    let failure = stage_connection_condition(
        &mut tx,
        ifc4x3(),
        Some("Bolt shear"),
        ConnectionConditionDraft::Failure(FailureLimits {
            tension: AxisValues {
                x: Some(1.0),
                y: None,
                z: None,
            },
            compression: AxisValues {
                x: None,
                y: None,
                z: Some(-2.0),
            },
        }),
    )
    .expect("failure condition");
    let slippage = stage_connection_condition(
        &mut tx,
        ifc4x3(),
        None,
        ConnectionConditionDraft::Slippage(AxisValues {
            x: Some(0.003),
            y: None,
            z: None,
        }),
    )
    .expect("slippage condition");
    tx.commit(&mut model).expect("commit");

    let f = model.get(failure).expect("staged");
    assert_eq!(f.type_name.as_ref(), "IFCFAILURECONNECTIONCONDITION");
    assert_eq!(f.attributes[1], Value::Real(1.0), "TensionFailureX");
    assert_eq!(f.attributes[6], Value::Real(-2.0), "CompressionFailureZ");

    let s = model.get(slippage).expect("staged");
    assert_eq!(s.type_name.as_ref(), "IFCSLIPPAGECONNECTIONCONDITION");
    assert_eq!(s.attributes[1], Value::Real(0.003), "SlippageX");
    assert_eq!(s.attributes[3], Value::Null, "SlippageZ unset, not zero");
}

/// A non-finite measure is refused.
#[test]
fn a_non_finite_condition_measure_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    assert!(
        stage_connection_condition(
            &mut tx,
            ifc4x3(),
            None,
            ConnectionConditionDraft::Slippage(AxisValues {
                x: Some(f64::INFINITY),
                y: None,
                z: None,
            }),
        )
        .is_err(),
        "an infinite slippage was accepted",
    );
}

/// Supplying an eccentricity selects the eccentric subtype.
///
/// `IfcRelConnectsWithEccentricity` adds a required `ConnectionConstraint`
/// after the base relation's slots, so the presence of that constraint is
/// what distinguishes the two entities.
#[test]
fn an_eccentricity_selects_the_eccentric_connection() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let member = tx.create(Entity::new(
        "IFCSTRUCTURALCURVEMEMBER",
        vec![Value::Null; 9],
    ));
    let connection = tx.create(Entity::new(
        "IFCSTRUCTURALPOINTCONNECTION",
        vec![Value::Null; 9],
    ));
    let constraint = tx.create(Entity::new(
        "IFCCONNECTIONPOINTECCENTRICITY",
        vec![Value::Null; 6],
    ));
    tx.commit(&mut model).expect("commit");

    let base = {
        let mut tx = Transaction::new(&model);
        let id = stage_member_connection(
            &mut tx,
            &model,
            ifc4x3(),
            MemberConnectionDraft {
                root: RelationshipRootDraft {
                    global_id: GUID.to_owned(),
                    owner_history: None,
                    name: None,
                    description: None,
                },
                member,
                connection,
                applied_condition: None,
                additional_conditions: None,
                supported_length: None,
                condition_coordinate_system: None,
                eccentricity: None,
            },
        )
        .expect("base connection");
        let mut committed = model.clone();
        tx.commit(&mut committed).expect("commit");
        committed
            .get(id)
            .expect("staged")
            .type_name
            .as_ref()
            .to_owned()
    };
    assert_eq!(base, "IFCRELCONNECTSSTRUCTURALMEMBER");

    let mut tx = Transaction::new(&model);
    let id = stage_member_connection(
        &mut tx,
        &model,
        ifc4x3(),
        MemberConnectionDraft {
            root: RelationshipRootDraft {
                global_id: GUID.to_owned(),
                owner_history: None,
                name: None,
                description: None,
            },
            member,
            connection,
            applied_condition: None,
            additional_conditions: None,
            supported_length: None,
            condition_coordinate_system: None,
            eccentricity: Some(constraint),
        },
    )
    .expect("eccentric connection");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCRELCONNECTSWITHECCENTRICITY");
    assert_eq!(
        staged.attributes[10],
        Value::Ref(constraint),
        "ConnectionConstraint is the trailing slot",
    );
}
