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

/// Two walls with the given GlobalIds, checked by the whole-file index
/// that `structure` exports but `validate` does not run.
fn duplicate_ids(first: &str, second: &str) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.push(wall(schema, first, &[]));
    model.push(wall(schema, second, &[]));
    let mut report = Report::new();
    ifc_validate::structure::duplicate_global_ids(&model, schema, &mut report);
    report
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
        rule: "structure.unique.duplicate_global_id",
        form: "two roots sharing a GlobalId",
        fails: || duplicate_ids(GUID_A, GUID_A),
        passes: || duplicate_ids(GUID_A, GUID_B),
    },
];
