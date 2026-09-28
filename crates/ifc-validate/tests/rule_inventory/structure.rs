//! Structural rule ids: references, slots, derivation, shape, uniqueness.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_validate::Report;

use super::cases::Case;
use super::fixtures::{entity, ifc4, text, wall, wall_with, GUID_A, GUID_B};

/// A wall whose `ObjectPlacement` points at `#2`, which is `target` (or
/// absent when `None`).
fn placed_wall(target: Option<&str>) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        wall(
            schema,
            GUID_A,
            &[("ObjectPlacement", Value::Ref(EntityId(2)))],
        ),
    );
    if let Some(type_name) = target {
        model.insert(EntityId(2), entity(schema, type_name, &[]));
    }
    ifc4(&model)
}

/// An `IfcOrientedEdge` whose derived `EdgeStart` holds `edge_start`.
///
/// IFC4 `IfcOrientedEdge` redeclares the inherited `IfcEdge.EdgeStart` as
/// `DERIVE SELF\IfcEdge.EdgeStart : IfcVertex := ...`, so Part 21 writes
/// that slot as `*`.
fn oriented_edge(edge_start: Value) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.insert(EntityId(1), entity(schema, "IFCVERTEX", &[]));
    model.insert(
        EntityId(2),
        entity(
            schema,
            "IFCORIENTEDEDGE",
            &[
                ("EdgeStart", edge_start),
                ("EdgeEnd", Value::Derived),
                ("Orientation", Value::Bool(true)),
            ],
        ),
    );
    ifc4(&model)
}

/// An `IfcPolyline` whose `Points` (`LIST [2:?] OF IfcCartesianPoint`)
/// holds `points`.
fn polyline(points: Value) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.insert(EntityId(1), entity(schema, "IFCCARTESIANPOINT", &[]));
    model.insert(
        EntityId(2),
        entity(schema, "IFCPOLYLINE", &[("Points", points)]),
    );
    ifc4(&model)
}

/// A storey containing `#1`, which is `member`, through
/// `RelatedElements : SET [1:?] OF IfcProduct`.
fn containment(member: &str) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        entity(schema, member, &[("GlobalId", text(GUID_A))]),
    );
    model.insert(
        EntityId(2),
        entity(schema, "IFCBUILDINGSTOREY", &[("GlobalId", text(GUID_B))]),
    );
    model.push(entity(
        schema,
        "IFCRELCONTAINEDINSPATIALSTRUCTURE",
        &[
            (
                "RelatedElements",
                Value::List(vec![Value::Ref(EntityId(1))]),
            ),
            ("RelatingStructure", Value::Ref(EntityId(2))),
        ],
    ));
    ifc4(&model)
}

/// An `IfcCartesianPoint` (`LIST [1:3] OF IfcLengthMeasure`) with `n`
/// coordinates.
fn point(n: usize) -> Report {
    let mut model = Model::new();
    model.push(entity(
        ifc_schema::ifc4(),
        "IFCCARTESIANPOINT",
        &[("Coordinates", Value::List(vec![Value::Real(0.0); n]))],
    ));
    ifc4(&model)
}

/// An `IfcCartesianPointList3D` (`LIST [1:?] OF LIST [3:3] OF
/// IfcLengthMeasure`) with the given rows.
fn point_list(rows: Vec<Value>) -> Report {
    let mut model = Model::new();
    model.push(entity(
        ifc_schema::ifc4(),
        "IFCCARTESIANPOINTLIST3D",
        &[("CoordList", Value::List(rows))],
    ));
    ifc4(&model)
}

fn row(values: &[f64]) -> Value {
    Value::List(values.iter().map(|&v| Value::Real(v)).collect())
}

/// An `IfcPolyLoop` (`LIST [3:?] OF UNIQUE IfcCartesianPoint`) over the
/// points `order` names, out of three distinct points.
fn poly_loop(order: &[u64]) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    for id in 1..=3 {
        model.insert(
            EntityId(id),
            entity(
                schema,
                "IFCCARTESIANPOINT",
                &[("Coordinates", row(&[id as f64, 0.0, 0.0]))],
            ),
        );
    }
    model.insert(
        EntityId(10),
        entity(
            schema,
            "IFCPOLYLOOP",
            &[(
                "Polygon",
                Value::List(order.iter().map(|&id| Value::Ref(EntityId(id))).collect()),
            )],
        ),
    );
    ifc4(&model)
}

/// Two `IfcApplication`s, checked against `UR1` (unique
/// `ApplicationIdentifier`).
fn applications(first: &str, second: &str) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    for (n, identifier) in [first, second].into_iter().enumerate() {
        model.push(entity(
            schema,
            "IFCAPPLICATION",
            &[
                ("ApplicationIdentifier", text(identifier)),
                ("ApplicationFullName", text(&format!("App {n}"))),
                ("Version", text("1")),
            ],
        ));
    }
    ifc4(&model)
}

pub const CASES: &[Case] = &[
    Case {
        rule: "structure.reference.dangling",
        form: "a reference to an absent instance",
        fails: || placed_wall(None),
        passes: || placed_wall(Some("IFCLOCALPLACEMENT")),
    },
    Case {
        rule: "structure.reference.wrong_type",
        form: "an entity slot pointing at an unrelated entity",
        fails: || placed_wall(Some("IFCWALL")),
        passes: || placed_wall(Some("IFCLOCALPLACEMENT")),
    },
    Case {
        rule: "structure.reference.wrong_type",
        form: "a non-product inside a SET OF IfcProduct",
        fails: || containment("IFCPROPERTYSET"),
        passes: || containment("IFCWALL"),
    },
    Case {
        rule: "structure.required.slot_count",
        form: "a record shorter than the declaration",
        fails: || {
            let mut model = Model::new();
            model.push(Entity::new("IFCWALL", vec![text(GUID_A)]));
            ifc4(&model)
        },
        passes: || wall_with("GlobalId", text(GUID_A)),
    },
    Case {
        rule: "structure.required.missing",
        form: "a mandatory slot written $",
        fails: || wall_with("GlobalId", Value::Null),
        passes: || wall_with("GlobalId", text(GUID_A)),
    },
    Case {
        rule: "structure.required.not_derived",
        form: "* in a slot the schema does not derive",
        fails: || wall_with("Name", Value::Derived),
        passes: || wall_with("Name", Value::Null),
    },
    Case {
        rule: "structure.required.derived_as_null",
        form: "$ in a redeclared DERIVE slot",
        fails: || oriented_edge(Value::Null),
        passes: || oriented_edge(Value::Derived),
    },
    Case {
        rule: "structure.required.derived_has_value",
        form: "a value in a redeclared DERIVE slot",
        fails: || oriented_edge(Value::Ref(EntityId(1))),
        passes: || oriented_edge(Value::Derived),
    },
    Case {
        rule: "structure.cardinality.expected_aggregate",
        form: "a scalar where a LIST is declared",
        fails: || polyline(Value::Ref(EntityId(1))),
        passes: || polyline(Value::List(vec![Value::Ref(EntityId(1)); 2])),
    },
    Case {
        rule: "structure.cardinality.unexpected_aggregate",
        form: "a LIST where a scalar is declared",
        fails: || wall_with("Name", Value::List(vec![text("a")])),
        passes: || wall_with("Name", text("a")),
    },
    Case {
        rule: "structure.aggregate.too_few",
        form: "an empty LIST [1:3]",
        fails: || point(0),
        passes: || point(3),
    },
    Case {
        rule: "structure.aggregate.too_few",
        form: "a two-element row of LIST [1:?] OF LIST [3:3]",
        fails: || point_list(vec![row(&[0.0, 0.0, 0.0]), row(&[0.0, 0.0])]),
        passes: || point_list(vec![row(&[0.0, 0.0, 0.0]), row(&[1.0, 0.0, 0.0])]),
    },
    Case {
        rule: "structure.aggregate.too_many",
        form: "four elements in a LIST [1:3]",
        fails: || point(4),
        passes: || point(2),
    },
    Case {
        rule: "structure.aggregate.nesting",
        form: "a scalar where LIST OF LIST nests a row",
        fails: || point_list(vec![row(&[0.0, 0.0, 0.0]), Value::Real(1.0)]),
        passes: || point_list(vec![row(&[0.0, 0.0, 0.0]), row(&[1.0, 0.0, 0.0])]),
    },
    Case {
        rule: "structure.aggregate.duplicate",
        form: "a repeated point in LIST OF UNIQUE",
        fails: || poly_loop(&[1, 2, 1]),
        passes: || poly_loop(&[1, 2, 3]),
    },
    Case {
        rule: "structure.unique.violation",
        form: "two applications sharing ApplicationIdentifier (UR1)",
        fails: || applications("id", "id"),
        passes: || applications("id", "other"),
    },
    Case {
        rule: "type.scalar.mismatch",
        form: "a string inside the inner level of LIST OF LIST OF IfcLengthMeasure",
        fails: || {
            point_list(vec![Value::List(vec![
                Value::Real(0.0),
                text("x"),
                Value::Real(0.0),
            ])])
        },
        passes: || point_list(vec![row(&[0.0, 0.5, 0.0])]),
    },
];
