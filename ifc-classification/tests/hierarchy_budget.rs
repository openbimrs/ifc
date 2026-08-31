use ifc_classification::{ClassificationError, ClassificationView};
use ifc_model::{Budget, Entity, EntityId, Model, Value};

fn text(value: &str) -> Value {
    Value::Text(value.to_owned().into())
}

fn classification(name: &str) -> Entity {
    Entity::new(
        "IFCCLASSIFICATION",
        vec![
            Value::Null,
            Value::Null,
            Value::Null,
            text(name),
            Value::Null,
            Value::Null,
            Value::Null,
        ],
    )
}

fn reference(identification: &str, source: EntityId) -> Entity {
    Entity::new(
        "IFCCLASSIFICATIONREFERENCE",
        vec![
            Value::Null,
            text(identification),
            Value::Null,
            Value::Ref(source),
            Value::Null,
            Value::Null,
        ],
    )
}

#[test]
fn terminal_edges_and_system_nodes_count_against_budget() {
    let mut model = Model::new();
    let system = model.push(classification("System"));
    let root = model.push(reference("Root", system));
    let leaf = model.push(reference("Leaf", root));
    let view = ClassificationView::new(&model);
    let exceeded = |start, max_depth, max_nodes| {
        matches!(
            view.hierarchy_from(
                start,
                Budget {
                    max_depth,
                    max_nodes,
                },
            ),
            Err(ClassificationError::BudgetExceeded { .. })
        )
    };

    assert!(exceeded(root, 0, 2));
    assert!(exceeded(root, 1, 1));
    assert!(view
        .hierarchy_from(
            root,
            Budget {
                max_depth: 1,
                max_nodes: 2,
            },
        )
        .is_ok());

    assert!(exceeded(leaf, 1, 3));
    assert!(exceeded(leaf, 2, 2));
    assert!(view
        .hierarchy_from(
            leaf,
            Budget {
                max_depth: 2,
                max_nodes: 3,
            },
        )
        .is_ok());
}

#[test]
fn revisits_do_not_consume_another_distinct_node() {
    let mut model = Model::new();
    let first = model.push(reference("A", EntityId(2)));
    let second = model.push(reference("B", first));
    assert_eq!(second, EntityId(2));

    assert!(matches!(
        ClassificationView::new(&model).hierarchy_from(
            first,
            Budget {
                max_depth: 2,
                max_nodes: 2,
            },
        ),
        Err(ClassificationError::Cycle { .. })
    ));
}
