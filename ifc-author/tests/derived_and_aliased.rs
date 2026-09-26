//! Derived slots (#18) and attributes typed by an aggregate alias (#17),
//! against the bundled IFC4 and IFC2X3 tables.

use ifc_author::{AuthorError, EntityBuilder, EntityEditor};
use ifc_model::{Codec, EntityId, Model, Transaction, Value};

const GUID: &str = "3vB2YO$MX4xv5uCqZZG05x";

/// 50° 34' 6.96" N, as buildingSMART's own IfcSite examples encode it.
fn latitude() -> Value {
    Value::List(vec![
        Value::Integer(50),
        Value::Integer(34),
        Value::Integer(6),
        Value::Integer(960_000),
    ])
}

fn site(schema: &ifc_schema::Schema) -> EntityBuilder<'_> {
    EntityBuilder::new(schema, "IfcSite")
        .text("GlobalId", GUID)
        // Optional in IFC4, required in IFC2X3.
        .reference("OwnerHistory", EntityId(1))
        .enumeration("CompositionType", "ELEMENT")
}

fn written(model: &Model) -> String {
    let mut model = model.clone();
    model.header_mut().schema = vec!["IFC4".into()];
    String::from_utf8(ifc_step::StepCodec.write_bytes(&model).expect("writes")).expect("utf-8")
}

// ---- #17: a defined type that aliases an aggregate ---------------------------

#[test]
fn a_compound_plane_angle_is_an_aggregate_of_integers() {
    for schema in [ifc_schema::ifc4(), ifc_schema::ifc2x3()] {
        let mut model = Model::new();
        site(schema)
            .set("RefLatitude", latitude())
            .set("RefLongitude", latitude())
            .insert(&mut model)
            .expect("a georeferenced site builds");
        assert!(written(&model).contains("(50,34,6,960000),(50,34,6,960000)"));
    }
}

#[test]
fn a_scalar_or_a_wrong_element_is_still_refused() {
    let schema = ifc_schema::ifc4();
    assert_eq!(
        site(schema)
            .set("RefLatitude", Value::Integer(50))
            .build()
            .unwrap_err(),
        AuthorError::AggregateMismatch {
            entity: "IfcSite".into(),
            attribute: "RefLatitude".into(),
            expected_aggregate: true,
        }
    );
    // Elements are checked against the alias's element type, INTEGER.
    let labels = Value::List(vec![Value::Text("50".into()); 3]);
    assert!(matches!(
        site(schema).set("RefLatitude", labels).build(),
        Err(AuthorError::TypeMismatch { attribute, .. }) if attribute == "RefLatitude"
    ));
}

#[test]
fn an_aliased_aggregate_is_editable() {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    let id = site(schema).insert(&mut model).expect("site builds");
    let mut tx = Transaction::new(&model);
    EntityEditor::new(schema, &model, id)
        .expect("exists")
        .set("RefLatitude", latitude())
        .stage(&mut tx)
        .expect("a list is admissible for IfcCompoundPlaneAngleMeasure");
}

// ---- #18: slots a subtype redeclares as DERIVE ------------------------------

#[test]
fn an_si_unit_writes_its_derived_dimensions_as_an_asterisk() {
    for schema in [ifc_schema::ifc4(), ifc_schema::ifc2x3()] {
        let mut model = Model::new();
        let id = EntityBuilder::new(schema, "IfcSIUnit")
            .enumeration("UnitType", "LENGTHUNIT")
            .enumeration("Name", "METRE")
            .insert(&mut model)
            .expect("IfcSIUnit builds without Dimensions");
        assert_eq!(model.get(id).unwrap().attributes[0], Value::Derived);
        assert!(written(&model).contains("IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.)"));
    }
    // Stating `*` explicitly is equally valid.
    EntityBuilder::new(ifc_schema::ifc4(), "IfcSIUnit")
        .set("Dimensions", Value::Derived)
        .enumeration("UnitType", "LENGTHUNIT")
        .enumeration("Name", "METRE")
        .build()
        .expect("explicit `*` is accepted");
}

#[test]
fn a_sub_context_derives_four_inherited_slots() {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    let parent = EntityBuilder::new(schema, "IfcDirection")
        .set(
            "DirectionRatios",
            Value::List(vec![Value::Real(0.0), Value::Real(1.0)]),
        )
        .insert(&mut model)
        .expect("placeholder target");
    let id = EntityBuilder::new(schema, "IfcGeometricRepresentationSubContext")
        .text("ContextIdentifier", "Body")
        .text("ContextType", "Model")
        .reference("ParentContext", parent)
        .enumeration("TargetView", "MODEL_VIEW")
        .insert(&mut model)
        .expect("a Body subcontext builds");
    let attributes = &model.get(id).unwrap().attributes;
    let derived: Vec<_> = attributes
        .iter()
        .enumerate()
        .filter(|(_, value)| **value == Value::Derived)
        .map(|(index, _)| {
            schema.attributes("IfcGeometricRepresentationSubContext")[index]
                .name
                .clone()
        })
        .collect();
    assert_eq!(
        derived,
        [
            "CoordinateSpaceDimension",
            "Precision",
            "WorldCoordinateSystem",
            "TrueNorth"
        ]
    );
}

#[test]
fn a_derived_slot_refuses_a_value_or_null() {
    let schema = ifc_schema::ifc4();
    for (value, found) in [
        (Value::Ref(EntityId(1)), "an entity reference"),
        (Value::Null, "unset ($)"),
    ] {
        assert_eq!(
            EntityBuilder::new(schema, "IfcSIUnit")
                .set("Dimensions", value)
                .enumeration("UnitType", "LENGTHUNIT")
                .enumeration("Name", "METRE")
                .build()
                .unwrap_err(),
            AuthorError::DerivedAttribute {
                entity: "IfcSIUnit".into(),
                attribute: "Dimensions".into(),
                found: found.into(),
            }
        );
    }
}

#[test]
fn an_asterisk_is_refused_where_nothing_is_derived() {
    // IfcNamedUnit.Dimensions is explicit on a conversion-based unit.
    assert_eq!(
        EntityBuilder::new(ifc_schema::ifc4(), "IfcConversionBasedUnit")
            .set("Dimensions", Value::Derived)
            .enumeration("UnitType", "LENGTHUNIT")
            .text("Name", "inch")
            .reference("ConversionFactor", EntityId(1))
            .build()
            .unwrap_err(),
        AuthorError::NotDerived {
            entity: "IfcConversionBasedUnit".into(),
            attribute: "Dimensions".into(),
        }
    );
}

#[test]
fn editing_keeps_a_derived_slot_and_refuses_overwriting_it() {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    let id = EntityBuilder::new(schema, "IfcSIUnit")
        .enumeration("UnitType", "LENGTHUNIT")
        .enumeration("Name", "METRE")
        .insert(&mut model)
        .expect("builds");
    let mut tx = Transaction::new(&model);
    EntityEditor::new(schema, &model, id)
        .expect("exists")
        .enumeration("Prefix", "MILLI")
        .stage(&mut tx)
        .expect("an unrelated edit keeps `*`");
    assert!(matches!(
        EntityEditor::new(schema, &model, id)
            .expect("exists")
            .set("Dimensions", Value::Null)
            .stage(&mut Transaction::new(&model)),
        Err(AuthorError::DerivedAttribute { attribute, .. }) if attribute == "Dimensions"
    ));
}

/// No IFC release derives a slot on an intermediate supertype, but EXPRESS
/// allows it: the subtype inherits the `*`.
#[test]
fn a_derived_redeclaration_on_a_supertype_is_inherited() {
    let schema = ifc_schema::Schema::from_express(
        "SCHEMA TEST;
ENTITY Owner;
  Size : REAL;
  Label : STRING;
END_ENTITY;
ENTITY Middle SUBTYPE OF (Owner);
DERIVE
  SELF\\Owner.Size : REAL := 1.0;
END_ENTITY;
ENTITY Leaf SUBTYPE OF (Middle);
END_ENTITY;
END_SCHEMA;",
    );
    let leaf = EntityBuilder::new(&schema, "Leaf")
        .text("Label", "x")
        .build()
        .expect("the inherited derived slot needs no value");
    assert_eq!(leaf.attributes[0], Value::Derived);
    assert!(matches!(
        EntityBuilder::new(&schema, "Leaf")
            .real("Size", 2.0)
            .text("Label", "x")
            .build(),
        Err(AuthorError::DerivedAttribute { .. })
    ));
}
