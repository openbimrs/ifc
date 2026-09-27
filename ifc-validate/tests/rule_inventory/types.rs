//! Type rule ids: entity types, scalar forms, enumerations, selects.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_validate::Report;

use super::cases::Case;
use super::fixtures::{entity, ifc4, text, typed, wall, wall_with, GUID_A, GUID_B};

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

/// A material association over `#1` (an `IfcMaterial`) and `#2` (an
/// `IfcWall`): `RelatingMaterial : IfcMaterialSelect` is `#relating`, and
/// `RelatedObjects : SET OF IfcDefinitionSelect` holds `#related`.
fn material_association(relating: u64, related: u64) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        entity(schema, "IFCMATERIAL", &[("Name", text("steel"))]),
    );
    model.insert(EntityId(2), wall(schema, GUID_A, &[]));
    model.push(entity(
        schema,
        "IFCRELASSOCIATESMATERIAL",
        &[
            ("GlobalId", text(GUID_B)),
            (
                "RelatedObjects",
                Value::List(vec![Value::Ref(EntityId(related))]),
            ),
            ("RelatingMaterial", Value::Ref(EntityId(relating))),
        ],
    ));
    ifc4(&model)
}

/// An `IfcCartesianPoint` whose `Coordinates` (`LIST [1:3] OF
/// IfcLengthMeasure`) holds `second` after a valid first member.
fn point(second: Value) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.push(entity(
        schema,
        "IFCCARTESIANPOINT",
        &[("Coordinates", Value::List(vec![Value::Real(0.0), second]))],
    ));
    ifc4(&model)
}

/// A sequence whose `RelatingProcess : IfcProcess` holds `relating`, from
/// task `#1` to task `#2`.
fn sequence_from(relating: Value) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.insert(EntityId(1), entity(schema, "IFCTASK", &[]));
    model.insert(EntityId(2), entity(schema, "IFCTASK", &[]));
    model.push(entity(
        schema,
        "IFCRELSEQUENCE",
        &[
            ("RelatingProcess", relating),
            ("RelatedProcess", Value::Ref(EntityId(2))),
        ],
    ));
    ifc4(&model)
}

/// An aggregation of `#1` (a wall) whose `RelatedObjects : SET OF
/// IfcObjectDefinition` holds `related`.
fn aggregation(related: Value) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.insert(EntityId(1), wall(schema, GUID_A, &[]));
    model.insert(EntityId(2), wall(schema, GUID_B, &[]));
    model.push(entity(
        schema,
        "IFCRELAGGREGATES",
        &[
            ("RelatingObject", Value::Ref(EntityId(1))),
            ("RelatedObjects", Value::List(vec![related])),
        ],
    ));
    ifc4(&model)
}

/// An `IfcActor` whose `TheActor : IfcActorSelect` -- a SELECT of three
/// entities and nothing else -- holds `actor`; `#1` is an `IfcPerson`.
fn actor(actor: Value) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.insert(EntityId(1), entity(schema, "IFCPERSON", &[]));
    model.push(entity(
        schema,
        "IFCACTOR",
        &[("GlobalId", text(GUID_A)), ("TheActor", actor)],
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
        rule: "type.scalar.mismatch",
        form: "a string inside a LIST OF IfcLengthMeasure",
        fails: || point(text("1.0")),
        passes: || point(Value::Real(1.0)),
    },
    Case {
        rule: "type.scalar.mismatch",
        form: "a reference in a STRING slot",
        fails: || wall_with("Name", Value::Ref(EntityId(1))),
        passes: || wall_with("Name", text("#1")),
    },
    Case {
        rule: "type.select.member",
        form: "a typed wrapper outside the SELECT",
        fails: || single_value(typed("IFCGLOBALLYUNIQUEID", text(GUID_A))),
        passes: || single_value(typed("IFCPOSITIVELENGTHMEASURE", Value::Real(1.0))),
    },
    Case {
        rule: "type.select.member",
        form: "a reference to an entity outside the SELECT",
        fails: || material_association(2, 2),
        passes: || material_association(1, 2),
    },
    Case {
        rule: "type.select.member",
        form: "an entity outside the SELECT inside a SET OF a SELECT",
        fails: || material_association(1, 1),
        passes: || material_association(1, 2),
    },
    Case {
        rule: "type.entity.expected_reference",
        form: "a string in IfcRelSequence.RelatingProcess",
        fails: || sequence_from(text("task")),
        passes: || sequence_from(Value::Ref(EntityId(1))),
    },
    Case {
        rule: "type.entity.expected_reference",
        form: "a typed wrapper in an entity-typed slot",
        fails: || sequence_from(typed("IFCLABEL", text("task"))),
        passes: || sequence_from(Value::Ref(EntityId(1))),
    },
    Case {
        rule: "type.entity.expected_reference",
        form: "an integer inside a SET OF IfcObjectDefinition",
        fails: || aggregation(Value::Integer(2)),
        passes: || aggregation(Value::Ref(EntityId(2))),
    },
    Case {
        rule: "type.entity.expected_reference",
        form: "a string in a SELECT of entities",
        fails: || actor(text("me")),
        passes: || actor(Value::Ref(EntityId(1))),
    },
];
