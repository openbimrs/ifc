//! IFC2X3 varying linear and planar actions (#229).
//!
//! `IfcStructuralLinearActionVarying` carries `SubsequentAppliedLoads :
//! LIST [1:?]`, `IfcStructuralPlanarActionVarying` `LIST [2:?]`; both carry
//! `VaryingAppliedLoadLocation : IfcShapeAspect`. IFC4 and IFC4X3 declare
//! neither entity, so the accessors return `None` there and the writer
//! refuses with `EntityNotInSchema`.

mod support;

use ifc_model::{Codec, EntityId, Model, Transaction, Value};
use ifc_schema::{ifc2x3, ifc4, ifc4x3, Schema};
use ifc_step::StepCodec;
use ifc_structural::{
    stage_action, stage_load, ActionDraft, ActionDraftKind, ActionKind, CoordinateSystem,
    LoadDraft, LoadKind, ProjectedOrTrue, StructuralError, StructuralRootDraft, StructuralView,
    VaryingActionDraft,
};
use support::{enumeration, model, named, GUID};

/// An IFC2X3 model holding an owner history, a shape aspect, two linear
/// forces, three planar forces and a temperature load.
struct Fixture {
    model: Model,
    owner: EntityId,
    location: EntityId,
    linear: [EntityId; 2],
    planar: [EntityId; 3],
    temperature: EntityId,
}

fn fixture() -> Fixture {
    let schema = ifc2x3();
    let mut model = model("IFC2X3");
    let owner = model.push(named(schema, "IfcOwnerHistory", &[]));
    let location = model.push(named(schema, "IfcShapeAspect", &[]));
    let mut tx = Transaction::new(&model);
    let linear = [1.0, 2.0].map(|x| {
        stage_load(
            &mut tx,
            schema,
            LoadDraft::LinearForce {
                name: None,
                force: [Some(x), None, None],
                moment: [None, None, None],
            },
        )
        .unwrap()
    });
    let planar = [3.0, 4.0, 5.0].map(|z| {
        stage_load(
            &mut tx,
            schema,
            LoadDraft::PlanarForce {
                name: None,
                force: [None, None, Some(z)],
            },
        )
        .unwrap()
    });
    let temperature = stage_load(
        &mut tx,
        schema,
        LoadDraft::Temperature {
            name: Some("Gradient".into()),
            delta: [Some(10.0), None, None],
        },
    )
    .unwrap();
    tx.commit(&mut model).unwrap();
    Fixture {
        model,
        owner,
        location,
        linear,
        planar,
        temperature,
    }
}

fn draft(owner: EntityId, applied: EntityId, kind: ActionDraftKind) -> ActionDraft {
    ActionDraft::new(
        StructuralRootDraft::new(GUID).owner_history(owner),
        applied,
        CoordinateSystem::Global,
        kind,
    )
    .destabilizing_load(false)
}

fn linear() -> ActionDraftKind {
    ActionDraftKind::Linear {
        projected_or_true: Some(ProjectedOrTrue::TrueLength),
    }
}

fn planar() -> ActionDraftKind {
    ActionDraftKind::Planar {
        projected_or_true: Some(ProjectedOrTrue::TrueLength),
    }
}

fn step_round_trip(model: &Model) -> Model {
    let codec = StepCodec;
    let bytes = codec.write_bytes(model).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("FILE_SCHEMA(('IFC2X3'))"), "{text}");
    codec.read_bytes(&bytes).unwrap()
}

#[test]
fn ifc2x3_linear_varying_action_reads_back_through_step() {
    let mut f = fixture();
    let schema = ifc2x3();
    let mut tx = Transaction::new(&f.model);
    let id = stage_action(
        &mut tx,
        &f.model,
        schema,
        draft(f.owner, f.linear[0], linear())
            .varying(VaryingActionDraft::new(f.location, vec![f.linear[1]])),
    )
    .unwrap();
    tx.commit(&mut f.model).unwrap();
    assert_eq!(
        f.model.get(id).unwrap().type_name.as_ref(),
        "IFCSTRUCTURALLINEARACTIONVARYING"
    );

    let reread = step_round_trip(&f.model);
    let view = StructuralView::for_model(&reread).unwrap();
    let action = view.action(id).unwrap();
    assert_eq!(action.kind(), ActionKind::Curve);
    assert!(action.is_varying());
    assert_eq!(action.applied_load().unwrap(), f.linear[0]);
    assert_eq!(action.projected_or_true().unwrap(), Some("TRUE_LENGTH"));
    assert_eq!(
        action.varying_applied_load_location().unwrap(),
        Some(f.location)
    );
    let loads = action.subsequent_applied_loads().unwrap().unwrap();
    assert_eq!(loads.len(), 1);
    assert_eq!(loads[0].id(), f.linear[1]);
    assert_eq!(loads[0].kind(), LoadKind::LinearForce);
    assert_eq!(loads[0].components().unwrap()[0], Some(2.0));
}

#[test]
fn ifc2x3_planar_varying_action_reads_back_through_step_in_list_order() {
    let mut f = fixture();
    let schema = ifc2x3();
    let mut tx = Transaction::new(&f.model);
    // A LIST, not a SET: order is kept and a repeat is legal.
    let subsequent = vec![f.planar[2], f.temperature, f.planar[2]];
    let id = stage_action(
        &mut tx,
        &f.model,
        schema,
        draft(f.owner, f.planar[0], planar())
            .varying(VaryingActionDraft::new(f.location, subsequent.clone())),
    )
    .unwrap();
    tx.commit(&mut f.model).unwrap();

    let reread = step_round_trip(&f.model);
    let view = StructuralView::for_model(&reread).unwrap();
    let action = view.action(id).unwrap();
    assert_eq!(action.kind(), ActionKind::Surface);
    assert!(action.is_varying());
    assert_eq!(
        action.varying_applied_load_location().unwrap(),
        Some(f.location)
    );
    let loads = action.subsequent_applied_loads().unwrap().unwrap();
    assert_eq!(
        loads.iter().map(|load| load.id()).collect::<Vec<_>>(),
        subsequent
    );
    assert_eq!(
        loads.iter().map(|load| load.kind()).collect::<Vec<_>>(),
        [
            LoadKind::PlanarForce,
            LoadKind::Temperature,
            LoadKind::PlanarForce
        ]
    );
}

#[test]
fn constant_actions_are_not_varying_in_any_release() {
    let f = fixture();
    let schema = ifc2x3();
    let mut model = f.model.clone();
    let constant = model.push(named(
        schema,
        "IfcStructuralLinearAction",
        &[
            ("GlobalId", Value::Text(GUID.into())),
            ("OwnerHistory", Value::Ref(f.owner)),
            ("AppliedLoad", Value::Ref(f.linear[0])),
            ("GlobalOrLocal", enumeration("GLOBAL_COORDS")),
            ("DestabilizingLoad", Value::Bool(false)),
            ("ProjectedOrTrue", enumeration("TRUE_LENGTH")),
        ],
    ));
    let action = StructuralView::new(&model, schema)
        .action(constant)
        .unwrap();
    assert!(!action.is_varying());
    assert_eq!(action.varying_applied_load_location().unwrap(), None);
    assert!(action.subsequent_applied_loads().unwrap().is_none());

    for (schema, token, entity, load) in [
        (
            ifc4(),
            "IFC4",
            "IfcStructuralLinearAction",
            "IfcStructuralLoadLinearForce",
        ),
        (
            ifc4(),
            "IFC4",
            "IfcStructuralPlanarAction",
            "IfcStructuralLoadPlanarForce",
        ),
        (
            ifc4x3(),
            "IFC4X3_ADD2",
            "IfcStructuralLinearAction",
            "IfcStructuralLoadLinearForce",
        ),
        (
            ifc4x3(),
            "IFC4X3_ADD2",
            "IfcStructuralPlanarAction",
            "IfcStructuralLoadPlanarForce",
        ),
    ] {
        assert!(schema.entity("IfcStructuralLinearActionVarying").is_none());
        assert!(schema.entity("IfcStructuralPlanarActionVarying").is_none());
        let mut model = support::model(token);
        let load = model.push(named(schema, load, &[]));
        let id = model.push(named(
            schema,
            entity,
            &[
                ("GlobalId", Value::Text(GUID.into())),
                ("AppliedLoad", Value::Ref(load)),
                ("GlobalOrLocal", enumeration("GLOBAL_COORDS")),
                ("PredefinedType", enumeration("CONST")),
            ],
        ));
        let action = StructuralView::new(&model, schema).action(id).unwrap();
        assert!(!action.is_varying(), "{token} {entity}");
        assert_eq!(action.varying_applied_load_location().unwrap(), None);
        assert!(action.subsequent_applied_loads().unwrap().is_none());
    }
}

/// Push an IFC2X3 varying action with the given subsequent-load list value.
fn push_varying(
    model: &mut Model,
    f: &Fixture,
    entity: &str,
    applied: EntityId,
    subsequent: Value,
    location: Value,
) -> EntityId {
    model.push(named(
        ifc2x3(),
        entity,
        &[
            ("GlobalId", Value::Text(GUID.into())),
            ("OwnerHistory", Value::Ref(f.owner)),
            ("AppliedLoad", Value::Ref(applied)),
            ("GlobalOrLocal", enumeration("GLOBAL_COORDS")),
            ("DestabilizingLoad", Value::Bool(false)),
            ("ProjectedOrTrue", enumeration("TRUE_LENGTH")),
            ("VaryingAppliedLoadLocation", location),
            ("SubsequentAppliedLoads", subsequent),
        ],
    ))
}

#[test]
fn reader_enforces_list_minimums_and_reference_types() {
    let f = fixture();
    let schema = ifc2x3();
    let mut model = f.model.clone();
    let loc = Value::Ref(f.location);
    let refs = |ids: &[EntityId]| Value::List(ids.iter().copied().map(Value::Ref).collect());
    let linear_empty = push_varying(
        &mut model,
        &f,
        "IfcStructuralLinearActionVarying",
        f.linear[0],
        refs(&[]),
        loc.clone(),
    );
    let planar_one = push_varying(
        &mut model,
        &f,
        "IfcStructuralPlanarActionVarying",
        f.planar[0],
        refs(&[f.planar[1]]),
        loc.clone(),
    );
    let planar_two = push_varying(
        &mut model,
        &f,
        "IfcStructuralPlanarActionVarying",
        f.planar[0],
        refs(&[f.planar[1], f.planar[2]]),
        loc.clone(),
    );
    let dangling = push_varying(
        &mut model,
        &f,
        "IfcStructuralLinearActionVarying",
        f.linear[0],
        refs(&[EntityId(9999)]),
        loc.clone(),
    );
    let not_a_load = push_varying(
        &mut model,
        &f,
        "IfcStructuralLinearActionVarying",
        f.linear[0],
        refs(&[f.location]),
        loc.clone(),
    );
    let unset = push_varying(
        &mut model,
        &f,
        "IfcStructuralLinearActionVarying",
        f.linear[0],
        Value::Null,
        loc,
    );
    let wrong_location = push_varying(
        &mut model,
        &f,
        "IfcStructuralLinearActionVarying",
        f.linear[0],
        refs(&[f.linear[1]]),
        Value::Ref(f.linear[1]),
    );
    let view = StructuralView::new(&model, schema);
    let loads = |id| view.action(id).unwrap().subsequent_applied_loads();

    assert!(matches!(
        loads(linear_empty),
        Err(StructuralError::InvalidCardinality {
            attribute: "SubsequentAppliedLoads",
            minimum: 1,
            actual: 0,
            ..
        })
    ));
    assert!(matches!(
        loads(planar_one),
        Err(StructuralError::InvalidCardinality {
            minimum: 2,
            actual: 1,
            ..
        })
    ));
    assert_eq!(loads(planar_two).unwrap().unwrap().len(), 2);
    assert!(matches!(
        loads(dangling),
        Err(StructuralError::DanglingReference {
            target: EntityId(9999),
            ..
        })
    ));
    assert!(matches!(
        loads(not_a_load),
        Err(StructuralError::WrongReferenceType {
            expected: "IfcStructuralLoad",
            ..
        })
    ));
    assert!(matches!(
        loads(unset),
        Err(StructuralError::InvalidValue {
            attribute: "SubsequentAppliedLoads",
            ..
        })
    ));
    assert!(matches!(
        view.action(wrong_location)
            .unwrap()
            .varying_applied_load_location(),
        Err(StructuralError::WrongReferenceType {
            expected: "IfcShapeAspect",
            ..
        })
    ));
}

fn refusal(schema: &Schema, model: &Model, draft: ActionDraft) -> StructuralError {
    let mut tx = Transaction::new(model);
    let error = stage_action(&mut tx, model, schema, draft).unwrap_err();
    assert!(tx.is_empty(), "a refused varying action stages nothing");
    error
}

#[test]
fn writer_refuses_varying_actions_outside_ifc2x3() {
    for (schema, token) in [(ifc4(), "IFC4"), (ifc4x3(), "IFC4X3_ADD2")] {
        let mut model = model(token);
        let location = model.push(named(schema, "IfcShapeAspect", &[]));
        let mut tx = Transaction::new(&model);
        let loads: Vec<_> = (0..3)
            .map(|_| {
                stage_load(
                    &mut tx,
                    schema,
                    LoadDraft::LinearForce {
                        name: None,
                        force: [Some(1.0), None, None],
                        moment: [None, None, None],
                    },
                )
                .unwrap()
            })
            .collect();
        tx.commit(&mut model).unwrap();
        for (kind, entity) in [
            (linear(), "IfcStructuralLinearActionVarying"),
            (planar(), "IfcStructuralPlanarActionVarying"),
        ] {
            let draft = ActionDraft::new(
                StructuralRootDraft::new(GUID),
                loads[0],
                CoordinateSystem::Global,
                kind,
            )
            .varying(VaryingActionDraft::new(location, loads[1..].to_vec()));
            let error = refusal(schema, &model, draft);
            assert_eq!(
                error,
                StructuralError::EntityNotInSchema {
                    entity,
                    schema: schema.name().to_owned(),
                },
                "{token}"
            );
        }
    }
}

#[test]
fn writer_refuses_short_lists_wrong_kinds_and_wrong_references() {
    let f = fixture();
    let schema = ifc2x3();
    let m = &f.model;

    let error = refusal(
        schema,
        m,
        draft(f.owner, f.linear[0], linear()).varying(VaryingActionDraft::new(f.location, vec![])),
    );
    assert!(matches!(
        error,
        StructuralError::InvalidDraftValue {
            entity_type: "IfcStructuralLinearActionVarying",
            attribute: "SubsequentAppliedLoads",
            ..
        }
    ));

    let error = refusal(
        schema,
        m,
        draft(f.owner, f.planar[0], planar())
            .varying(VaryingActionDraft::new(f.location, vec![f.planar[1]])),
    );
    assert!(matches!(
        error,
        StructuralError::InvalidDraftValue {
            entity_type: "IfcStructuralPlanarActionVarying",
            attribute: "SubsequentAppliedLoads",
            ..
        }
    ));

    let error = refusal(
        schema,
        m,
        draft(f.owner, f.linear[0], ActionDraftKind::Point)
            .varying(VaryingActionDraft::new(f.location, vec![f.linear[1]])),
    );
    assert!(matches!(
        error,
        StructuralError::InvalidDraftValue {
            attribute: "SubsequentAppliedLoads",
            ..
        }
    ));

    let error = refusal(
        schema,
        m,
        draft(f.owner, f.linear[0], linear())
            .varying(VaryingActionDraft::new(f.linear[1], vec![f.linear[1]])),
    );
    assert!(matches!(
        error,
        StructuralError::WrongReferenceType {
            expected: "IfcShapeAspect",
            ..
        }
    ));

    let error = refusal(
        schema,
        m,
        draft(f.owner, f.linear[0], linear())
            .varying(VaryingActionDraft::new(f.location, vec![f.location])),
    );
    assert!(matches!(
        error,
        StructuralError::WrongReferenceType {
            expected: "IfcStructuralLoad",
            ..
        }
    ));

    let error = refusal(
        schema,
        m,
        draft(f.owner, f.linear[0], linear())
            .varying(VaryingActionDraft::new(f.location, vec![EntityId(9999)])),
    );
    assert!(matches!(
        error,
        StructuralError::DanglingReference {
            target: EntityId(9999),
            ..
        }
    ));
}
