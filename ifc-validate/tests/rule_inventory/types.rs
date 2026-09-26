//! Type rule ids: entity types, scalar forms, enumerations, selects.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_validate::Report;

use super::cases::Case;
use super::fixtures::{entity, ifc4, text, typed, wall, wall_with, GUID_A};

/// One record of `type_name`, alone.
fn lone(type_name: &str, attributes: Vec<Value>) -> Report {
    let mut model = Model::new();
    model.push(Entity::new(type_name, attributes));
    ifc4(&model)
}

/// An `IfcFaceBound` whose `Orientation` holds `orientation`.
///
/// IFC4 declares `Orientation : IfcBoolean`, and `IfcBoolean = BOOLEAN`:
/// `.T.`/`.F.` only, never `.U.`.
fn face_bound(orientation: Value) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.insert(EntityId(1), entity(schema, "IFCPOLYLOOP", &[]));
    model.insert(
        EntityId(2),
        entity(
            schema,
            "IFCFACEBOUND",
            &[
                ("Bound", Value::Ref(EntityId(1))),
                ("Orientation", orientation),
            ],
        ),
    );
    ifc4(&model)
}

/// An `IfcCompositeCurve` whose `SelfIntersect` holds `value`.
///
/// IFC4 declares `SelfIntersect : IfcLogical`, and `IfcLogical = LOGICAL`:
/// `.T.`, `.F.` *and* `.U.` are all legal.
fn composite_curve(value: Value) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.push(entity(
        schema,
        "IFCCOMPOSITECURVE",
        &[
            ("Segments", Value::List(Vec::new())),
            ("SelfIntersect", value),
        ],
    ));
    ifc4(&model)
}

/// An `IfcPropertySingleValue` whose `NominalValue` (an `IfcValue` SELECT)
/// holds `value`.
fn single_value(value: Value) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.push(entity(
        schema,
        "IFCPROPERTYSINGLEVALUE",
        &[("Name", text("p")), ("NominalValue", value)],
    ));
    ifc4(&model)
}

pub const CASES: &[Case] = &[
    Case {
        rule: "type.entity.unknown",
        form: "an entity no bundled release declares",
        fails: || lone("IFCFUTURESUSTAINABILITYMETRIC", vec![Value::Null]),
        passes: || wall_with("GlobalId", text(GUID_A)),
    },
    Case {
        rule: "type.entity.abstract",
        form: "an ABSTRACT supertype instantiated",
        fails: || {
            let schema = ifc_schema::ifc4();
            let mut model = Model::new();
            model.push(entity(schema, "IFCPRODUCT", &[("GlobalId", text(GUID_A))]));
            ifc4(&model)
        },
        passes: || {
            let mut model = Model::new();
            model.push(wall(ifc_schema::ifc4(), GUID_A, &[]));
            ifc4(&model)
        },
    },
    Case {
        rule: "type.scalar.mismatch",
        form: "an integer in a STRING slot",
        fails: || wall_with("Name", Value::Integer(5)),
        passes: || wall_with("Name", text("5")),
    },
    Case {
        rule: "type.scalar.mismatch",
        form: "BOOLEAN slot written .U.",
        fails: || face_bound(Value::LogicalUnknown),
        passes: || face_bound(Value::Bool(false)),
    },
    Case {
        rule: "type.scalar.mismatch",
        form: "BOOLEAN slot written as an integer",
        fails: || face_bound(Value::Integer(1)),
        passes: || face_bound(Value::Bool(true)),
    },
    Case {
        rule: "type.scalar.mismatch",
        form: "LOGICAL slot written as an integer; .U. is legal",
        fails: || composite_curve(Value::Integer(0)),
        passes: || composite_curve(Value::LogicalUnknown),
    },
    Case {
        rule: "type.scalar.mismatch",
        form: "typed IFCBOOLEAN wrapper holding .U.",
        fails: || single_value(typed("IFCBOOLEAN", Value::LogicalUnknown)),
        passes: || single_value(typed("IFCLOGICAL", Value::LogicalUnknown)),
    },
    Case {
        rule: "type.scalar.fixed_width",
        form: "a 21-character GlobalId",
        fails: || wall_with("GlobalId", text("0000000000000000000C1")),
        passes: || wall_with("GlobalId", text(GUID_A)),
    },
    Case {
        rule: "type.enumeration.member",
        form: "a constant the enumeration does not declare",
        fails: || wall_with("PredefinedType", Value::Enum("CURTAIN".into())),
        passes: || wall_with("PredefinedType", Value::Enum("STANDARD".into())),
    },
    Case {
        rule: "type.enumeration.member",
        form: "an undeclared constant inside a typed wrapper",
        fails: || {
            wall_with(
                "PredefinedType",
                typed("IFCWALLTYPEENUM", Value::Enum("CURTAIN".into())),
            )
        },
        passes: || {
            wall_with(
                "PredefinedType",
                typed("IFCWALLTYPEENUM", Value::Enum("STANDARD".into())),
            )
        },
    },
    Case {
        rule: "type.select.member",
        form: "a typed wrapper outside the SELECT",
        fails: || single_value(typed("IFCGLOBALLYUNIQUEID", text(GUID_A))),
        passes: || single_value(typed("IFCPOSITIVELENGTHMEASURE", Value::Real(1.0))),
    },
];
