#![cfg(all(
    feature = "structural",
    feature = "geometry-select",
    feature = "schema"
))]

use ifc::geometry::{select_product_representation, RepresentationPurpose};
use ifc::structural::{
    stage_action, stage_activity_assignment, stage_load, ActionDraft, ActionDraftKind,
    ActivityAssignmentDraft, CoordinateSystem, LoadDraft, RelationshipRootDraft,
    StructuralRootDraft, StructuralView,
};
use ifc::{Entity, Model, Value};
use ifc_model::Transaction;
use ifc_schema::ifc4;

#[test]
fn physical_product_structural_assignment_and_geometry_join_on_entity_id() {
    let schema = ifc4();
    let mut model = Model::new();
    let representation = model.push(Entity::new(
        "IFCSHAPEREPRESENTATION",
        vec![
            Value::Null,
            Value::Text("Body".into()),
            Value::Text("SweptSolid".into()),
            Value::Null,
        ],
    ));
    let shape = model.push(Entity::new(
        "IFCPRODUCTDEFINITIONSHAPE",
        vec![
            Value::Null,
            Value::Null,
            Value::List(vec![Value::Ref(representation)]),
        ],
    ));
    let wall = model.push(Entity::new(
        "IFCWALL",
        vec![
            Value::Text("0YvctVUKbD0xjK5xJ8Jg71".into()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Ref(shape),
        ],
    ));

    let mut tx = Transaction::new(&model);
    let load = stage_load(
        &mut tx,
        schema,
        LoadDraft::SingleForce {
            name: Some("wall action".into()),
            force: [Some(1.0), None, None],
            moment: [None; 3],
        },
    )
    .unwrap();
    let action = stage_action(
        &mut tx,
        &model,
        schema,
        ActionDraft {
            root: StructuralRootDraft {
                global_id: "0YvctVUKbD0xjK5xJ8Jg72".into(),
                ..Default::default()
            },
            applied_load: load,
            coordinate_system: CoordinateSystem::Global,
            destabilizing_load: None,
            caused_by: None,
            kind: ActionDraftKind::Point,
        },
    )
    .unwrap();
    stage_activity_assignment(
        &mut tx,
        &model,
        schema,
        ActivityAssignmentDraft {
            root: RelationshipRootDraft {
                global_id: "0YvctVUKbD0xjK5xJ8Jg73".into(),
                ..Default::default()
            },
            relating_element: wall,
            activity: action,
        },
    )
    .unwrap();
    tx.commit(&mut model).unwrap();

    let assignment = StructuralView::new(&model, schema)
        .activities_for(wall)
        .unwrap();
    assert_eq!(assignment[0].target, wall);
    assert_eq!(assignment[0].activity, action);
    assert_eq!(
        select_product_representation(&model, wall, RepresentationPurpose::Body).unwrap(),
        Some(representation)
    );
    assert_eq!(
        select_product_representation(&model, wall, RepresentationPurpose::Plan).unwrap(),
        None,
        "body geometry must not be manufactured into a plan"
    );
}
