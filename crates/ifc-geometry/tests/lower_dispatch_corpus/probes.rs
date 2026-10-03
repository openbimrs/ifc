//! Runtime probes for `PARTIAL` rows, built inline rather than from fixtures.

use ifc_model::{Entity, EntityId, Model, Value};

/// An open two-triangle `IfcTriangulatedIrregularNetwork` with `flags`, as
/// `EntityId(9)`, keyed by the `PARTIAL` row it probes.
pub fn irregular_network(
    flags: &[i64],
    variant: &'static str,
) -> (&'static str, &'static str, Model) {
    let row = |values: &[f64]| Value::List(values.iter().map(|v| Value::Real(*v)).collect());
    let ints = |values: &[i64]| Value::List(values.iter().map(|v| Value::Integer(*v)).collect());
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCCARTESIANPOINTLIST3D",
            vec![Value::List(vec![
                row(&[0.0, 0.0, 10.0]),
                row(&[20.0, 0.0, 10.5]),
                row(&[20.0, 20.0, 11.0]),
                row(&[0.0, 20.0, 10.25]),
            ])],
        ),
    );
    model.insert(
        EntityId(9),
        Entity::new(
            "IFCTRIANGULATEDIRREGULARNETWORK",
            vec![
                Value::Ref(EntityId(1)),
                Value::Null,
                Value::Bool(false),
                Value::List(vec![ints(&[1, 2, 3]), ints(&[1, 3, 4])]),
                Value::Null,
                ints(flags),
            ],
        ),
    );
    ("IFCTRIANGULATEDIRREGULARNETWORK", variant, model)
}
