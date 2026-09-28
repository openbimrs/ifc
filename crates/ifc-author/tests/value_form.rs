//! Typed parameter or bare value: the form ISO 10303-21 gives each slot.
//!
//! §12.1.8 writes a value of a SELECT that is not an entity instance as a
//! typed parameter naming a type in the select-list; §12.1.6 and §12.1.7
//! write every other value bare. Authoring refuses the other form, and a
//! wrapper naming the wrong type, against the bundled release tables. Each
//! refusal here is paired with the valid spelling of the same value, which
//! must still build.

use ifc_author::{AuthorError, EntityBuilder, EntityEditor};
use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

const GUID: &str = "3vB2YO$MX4xv5uCqZZG05x";

fn typed(type_name: &str, value: Value) -> Value {
    Value::Typed {
        type_name: type_name.into(),
        value: Box::new(value),
    }
}

fn text(value: &str) -> Value {
    Value::Text(value.into())
}

fn is_form(result: Result<Entity, AuthorError>, typed_required: bool) -> bool {
    matches!(
        result,
        Err(AuthorError::ValueForm { typed_required: required, .. }) if required == typed_required
    )
}

fn is_type_mismatch(result: Result<Entity, AuthorError>) -> bool {
    matches!(result, Err(AuthorError::TypeMismatch { .. }))
}

/// `IfcQuantityArea.AreaValue : IfcAreaMeasure` holding `value`.
fn area(value: Value) -> Result<Entity, AuthorError> {
    EntityBuilder::new(ifc_schema::ifc4(), "IfcQuantityArea")
        .text("Name", "Area")
        .set("AreaValue", value)
        .build()
}

/// `IfcPropertySingleValue.NominalValue : IfcValue` holding `value`.
fn nominal(value: Value) -> Result<Entity, AuthorError> {
    EntityBuilder::new(ifc_schema::ifc4(), "IfcPropertySingleValue")
        .text("Name", "p")
        .set("NominalValue", value)
        .build()
}

/// `IfcPropertyListValue.ListValues : LIST OF IfcValue` holding `values`.
fn list_value(values: Vec<Value>) -> Result<Entity, AuthorError> {
    EntityBuilder::new(ifc_schema::ifc4(), "IfcPropertyListValue")
        .text("Name", "p")
        .set("ListValues", Value::List(values))
        .build()
}

#[test]
fn a_wrapper_where_the_declared_type_is_not_a_select_is_refused() {
    assert!(area(Value::Real(12.5)).is_ok(), "the bare value is valid");
    let refused = area(typed("IFCAREAMEASURE", Value::Real(12.5)));
    assert!(is_form(refused.clone(), false), "{refused:?}");
    let message = refused.unwrap_err().to_string();
    assert!(
        message.contains("AreaValue") && message.contains("bare"),
        "{message}"
    );
    // A specialisation's wrapper is still the right type, in the wrong form.
    let length = EntityBuilder::new(ifc_schema::ifc4(), "IfcQuantityLength")
        .text("Name", "Length")
        .set(
            "LengthValue",
            typed("IFCPOSITIVELENGTHMEASURE", Value::Real(1.0)),
        )
        .build();
    assert!(is_form(length, false));
}

#[test]
fn a_wrapper_of_another_type_is_a_type_mismatch() {
    assert!(is_type_mismatch(area(typed("IFCLABEL", text("x")))));
    // Same primitive, different quantity.
    assert!(is_type_mismatch(area(typed(
        "IFCLENGTHMEASURE",
        Value::Real(1.0)
    ))));
    // An entity-typed slot takes a reference, never a wrapper.
    let sequence = EntityBuilder::new(ifc_schema::ifc4(), "IfcRelSequence")
        .text("GlobalId", GUID)
        .set("RelatingProcess", typed("IFCLABEL", text("task")))
        .reference("RelatedProcess", EntityId(2))
        .build();
    assert!(is_type_mismatch(sequence));
}

#[test]
fn an_enumeration_is_written_bare_outside_a_select() {
    let wall = |value: Value| {
        EntityBuilder::new(ifc_schema::ifc4(), "IfcWall")
            .text("GlobalId", GUID)
            .set("PredefinedType", value)
            .build()
    };
    assert!(wall(Value::Enum("STANDARD".into())).is_ok());
    assert!(is_form(
        wall(typed("IFCWALLTYPEENUM", Value::Enum("STANDARD".into()))),
        false
    ));
}

#[test]
fn a_select_slot_takes_a_typed_member() {
    for valid in [
        typed("IFCLABEL", text("x")),
        // Two SELECTs down: IfcValue -> IfcDerivedMeasureValue.
        typed("IFCMONETARYMEASURE", Value::Real(1.0)),
        // A member aliasing an aggregate.
        typed(
            "IFCCOMPLEXNUMBER",
            Value::List(vec![Value::Real(1.0), Value::Real(2.0)]),
        ),
    ] {
        assert!(nominal(valid.clone()).is_ok(), "{valid:?} must build");
    }
    // Bare: the reader cannot tell a label from an identifier or a text.
    assert!(is_form(nominal(text("x")), true));
    assert!(is_form(nominal(Value::Real(1.0)), true));
    // A wrapper naming a type outside the select-list.
    assert!(is_type_mismatch(nominal(typed(
        "IFCGLOBALLYUNIQUEID",
        text(GUID)
    ))));
    // The parameter of a member that is not a SELECT is bare.
    assert!(is_form(
        nominal(typed("IFCLABEL", typed("IFCLABEL", text("x")))),
        false
    ));
}

#[test]
fn a_listed_defined_types_underlying_type_is_not_a_member() {
    let rendering = |colour: Value| {
        EntityBuilder::new(ifc_schema::ifc4(), "IfcSurfaceStyleRendering")
            .reference("SurfaceColour", EntityId(1))
            .set("DiffuseColour", colour)
            .enumeration("ReflectanceMethod", "NOTDEFINED")
            .build()
    };
    assert!(rendering(typed("IFCNORMALISEDRATIOMEASURE", Value::Real(0.5))).is_ok());
    assert!(rendering(Value::Ref(EntityId(1))).is_ok());
    assert!(is_type_mismatch(rendering(typed(
        "IFCRATIOMEASURE",
        Value::Real(0.5)
    ))));
}

#[test]
fn aggregate_members_are_judged_one_by_one() {
    assert!(list_value(vec![
        typed("IFCLABEL", text("a")),
        typed("IFCLABEL", text("b"))
    ])
    .is_ok());
    let refused = list_value(vec![typed("IFCLABEL", text("a")), text("b")]);
    assert!(is_form(refused.clone(), true), "{refused:?}");
    assert!(
        refused.unwrap_err().to_string().contains("a string"),
        "the offending member is named"
    );

    let point = |second: Value| {
        EntityBuilder::new(ifc_schema::ifc4(), "IfcCartesianPoint")
            .set("Coordinates", Value::List(vec![Value::Real(0.0), second]))
            .build()
    };
    assert!(point(Value::Real(1.0)).is_ok());
    assert!(is_form(
        point(typed("IFCLENGTHMEASURE", Value::Real(1.0))),
        false
    ));
    assert!(is_type_mismatch(point(typed("IFCLABEL", text("1")))));
}

#[test]
fn a_select_of_entities_still_takes_a_bare_reference() {
    let actor = |value: Value| {
        EntityBuilder::new(ifc_schema::ifc4(), "IfcActor")
            .text("GlobalId", GUID)
            .set("TheActor", value)
            .build()
    };
    assert!(actor(Value::Ref(EntityId(1))).is_ok());
    // Not a missing wrapper: nothing but a reference is a member.
    assert!(is_type_mismatch(actor(text("me"))));
}

#[test]
fn the_form_follows_the_declared_release() {
    // IFC4X3 declares `DistanceAlong : IfcCurveMeasureSelect`.
    let distance = |value: Value| {
        EntityBuilder::new(ifc_schema::ifc4x3(), "IfcPointByDistanceExpression")
            .set("DistanceAlong", value)
            .reference("BasisCurve", EntityId(1))
            .build()
    };
    assert!(distance(typed("IFCLENGTHMEASURE", Value::Real(5.0))).is_ok());
    assert!(distance(typed("IFCPARAMETERVALUE", Value::Real(0.5))).is_ok());
    assert!(is_form(distance(Value::Real(5.0)), true));
}

/// §12.1.8, EXAMPLE 2: a defined type aliasing a SELECT is encoded as that
/// SELECT, so a slot declared `Computed_Load` takes the typed form. A declared type the
/// tables cannot resolve accepts either form.
#[test]
fn aliases_are_followed_and_unresolvable_types_stay_permissive() {
    let schema = Schema::from_express(
        "SCHEMA ALIASED_SELECT;\n\
         TYPE Computed_Load = Number_Or_Flag; END_TYPE;\n\
         TYPE Number_Or_Flag = SELECT (Plain_Number, Flag); END_TYPE;\n\
         TYPE Plain_Number = REAL; END_TYPE;\n\
         TYPE Flag = ENUMERATION OF (unknown); END_TYPE;\n\
         ENTITY Beam;\n\
           computed : Computed_Load;\n\
           opaque : Undeclared_Type;\n\
         END_ENTITY;\n\
         END_SCHEMA;",
    );
    let bar = |computed: Value, opaque: Value| {
        EntityBuilder::new(&schema, "Beam")
            .set("computed", computed)
            .set("opaque", opaque)
            .build()
    };
    let float = || typed("PLAIN_NUMBER", Value::Real(1.0));
    assert!(bar(float(), Value::Real(1.0)).is_ok());
    assert!(bar(float(), typed("WHATEVER", Value::Real(1.0))).is_ok());
    assert!(is_form(bar(Value::Real(1.0), Value::Real(1.0)), true));
}

#[test]
fn an_edit_is_held_to_the_same_form() {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    let id = model.push(
        EntityBuilder::new(schema, "IfcQuantityArea")
            .text("Name", "Area")
            .real("AreaValue", 1.0)
            .build()
            .expect("the bare value is valid"),
    );
    let mut tx = Transaction::new(&model);
    let refused = EntityEditor::new(schema, &model, id)
        .expect("entity exists")
        .set("AreaValue", typed("IFCAREAMEASURE", Value::Real(2.0)))
        .stage(&mut tx);
    assert!(
        matches!(
            refused,
            Err(AuthorError::ValueForm {
                typed_required: false,
                ..
            })
        ),
        "{refused:?}"
    );
    assert!(tx.is_empty(), "a refused edit stages nothing");
    EntityEditor::new(schema, &model, id)
        .expect("entity exists")
        .real("AreaValue", 2.0)
        .stage(&mut tx)
        .expect("the bare value is valid");
}
