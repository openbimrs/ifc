//! What the builder accepts and what it refuses.
//!
//! The schema here is a hand-written subset rather than the shipped IFC4 file,
//! so these tests run in a fresh clone with no reference material. Behaviour
//! against the normative schema is covered by `real_schema.rs`.

use ifc_author::{AuthorError, EntityBuilder};
use ifc_model::{Model, Value};
use ifc_schema::Schema;

/// A subset with the shapes that matter: an inheritance chain, an optional, a
/// required attribute, an enumeration, an aggregate, and a select.
const SUBSET: &str = "\
SCHEMA IFC4;
TYPE IfcGloballyUniqueId = STRING; END_TYPE;
TYPE IfcLabel = STRING; END_TYPE;
TYPE IfcLengthMeasure = REAL; END_TYPE;
TYPE IfcGeometricProjectionEnum = ENUMERATION OF (PLAN_VIEW, MODEL_VIEW); END_TYPE;
ENTITY IfcRoot;
  GlobalId : IfcGloballyUniqueId;
  Name : OPTIONAL IfcLabel;
END_ENTITY;
ENTITY IfcAnnotation
 SUBTYPE OF (IfcRoot);
  ObjectType : OPTIONAL IfcLabel;
END_ENTITY;
ENTITY IfcCartesianPoint;
  Coordinates : LIST [1:3] OF IfcLengthMeasure;
END_ENTITY;
ENTITY IfcSubContext;
  TargetView : IfcGeometricProjectionEnum;
END_ENTITY;
ENTITY IfcPolyline;
  Points : LIST [2:?] OF IfcCartesianPoint;
END_ENTITY;
END_SCHEMA;
";

fn schema() -> Schema {
    Schema::from_express(SUBSET)
}

const GUID: &str = "3vB2YO$MX4xv5uCqZZG05x";

/// The point of the crate: inherited attributes land in the slots STEP expects.
#[test]
fn named_attributes_resolve_to_inherited_first_positional_slots() {
    let schema = schema();
    let entity = EntityBuilder::new(&schema, "IfcAnnotation")
        .text("ObjectType", "Brandwand")
        .text("GlobalId", GUID)
        .text("Name", "Wall symbol")
        .build()
        .expect("a fully specified annotation builds");

    // Declared order is GlobalId, Name (from IfcRoot), then ObjectType --
    // regardless of the order the caller set them in.
    assert_eq!(entity.type_name.as_ref(), "IFCANNOTATION");
    assert_eq!(entity.text(0), Some(GUID));
    assert_eq!(entity.text(1), Some("Wall symbol"));
    assert_eq!(entity.text(2), Some("Brandwand"));
    assert_eq!(entity.attributes.len(), 3, "arity comes from the schema");
}

#[test]
fn an_unset_optional_becomes_null_not_a_missing_slot() {
    let schema = schema();
    let entity = EntityBuilder::new(&schema, "IfcAnnotation")
        .text("GlobalId", GUID)
        .build()
        .expect("optionals may be omitted");

    assert_eq!(entity.attributes.len(), 3, "slots are still positional");
    assert_eq!(entity.attributes[1], Value::Null);
    assert_eq!(entity.attributes[2], Value::Null);
}

#[test]
fn attribute_names_are_case_insensitive() {
    let schema = schema();
    let entity = EntityBuilder::new(&schema, "IfcAnnotation")
        .text("GLOBALID", GUID)
        .build()
        .expect("EXPRESS declares GlobalId; files write GLOBALID");
    assert_eq!(entity.text(0), Some(GUID));
}

#[test]
fn insert_appends_to_the_model_and_returns_its_id() {
    let schema = schema();
    let mut model = Model::new();
    let id = EntityBuilder::new(&schema, "IfcAnnotation")
        .text("GlobalId", GUID)
        .insert(&mut model)
        .expect("insert succeeds");

    assert_eq!(model.len(), 1);
    assert_eq!(model.ids_of_type("IFCANNOTATION"), [id]);
}

#[test]
fn a_missing_required_attribute_is_refused() {
    let schema = schema();
    let error = EntityBuilder::new(&schema, "IfcAnnotation")
        .text("Name", "no id")
        .build()
        .expect_err("GlobalId is not optional");

    assert!(matches!(
        error,
        AuthorError::MissingRequired { ref attribute, .. } if attribute == "GlobalId"
    ));
}

#[test]
fn a_typo_in_the_entity_name_is_refused() {
    let schema = schema();
    let error = EntityBuilder::new(&schema, "IfcAnnotaton")
        .text("GlobalId", GUID)
        .build()
        .expect_err("the schema declares no such entity");
    assert!(matches!(error, AuthorError::UnknownEntity { .. }));
}

#[test]
fn a_typo_in_an_attribute_name_lists_the_real_ones() {
    let schema = schema();
    let error = EntityBuilder::new(&schema, "IfcAnnotation")
        .text("GlobalId", GUID)
        .text("Nmae", "typo")
        .build()
        .expect_err("no such attribute");

    let AuthorError::UnknownAttribute { known, .. } = &error else {
        panic!("expected UnknownAttribute, got {error:?}");
    };
    assert_eq!(known, &["GlobalId", "Name", "ObjectType"]);
    // The message is the whole diagnostic, so it must name the alternatives.
    assert!(error.to_string().contains("ObjectType"), "{error}");
}

#[test]
fn setting_the_same_attribute_twice_is_refused_rather_than_overwritten() {
    let schema = schema();
    let error = EntityBuilder::new(&schema, "IfcAnnotation")
        .text("GlobalId", GUID)
        .text("Name", "first")
        .text("Name", "second")
        .build()
        .expect_err("a silent overwrite hides a caller bug");
    assert!(matches!(error, AuthorError::DuplicateAttribute { .. }));
}

#[test]
fn a_string_where_a_real_is_declared_is_refused() {
    let schema = schema();
    let error = EntityBuilder::new(&schema, "IfcCartesianPoint")
        .set(
            "Coordinates",
            Value::List(vec![Value::Real(0.0), Value::Text("nope".into())]),
        )
        .build()
        .expect_err("IfcLengthMeasure resolves to REAL");
    assert!(
        matches!(error, AuthorError::TypeMismatch { .. }),
        "{error:?}"
    );
}

#[test]
fn a_scalar_where_an_aggregate_is_declared_is_refused() {
    let schema = schema();
    let error = EntityBuilder::new(&schema, "IfcCartesianPoint")
        .real("Coordinates", 1.0)
        .build()
        .expect_err("Coordinates is a LIST");
    assert!(matches!(
        error,
        AuthorError::AggregateMismatch {
            expected_aggregate: true,
            ..
        }
    ));
}

#[test]
fn a_malformed_globalid_is_refused() {
    let schema = schema();
    let error = EntityBuilder::new(&schema, "IfcAnnotation")
        .text("GlobalId", "not-a-guid")
        .build()
        .expect_err("GlobalId must be 22 chars of IFC base-64");
    assert!(matches!(error, AuthorError::InvalidGlobalId { .. }));
}

#[test]
fn an_enumeration_constant_is_accepted_and_a_string_is_not() {
    let schema = schema();
    assert!(EntityBuilder::new(&schema, "IfcSubContext")
        .enumeration("TargetView", "PLAN_VIEW")
        .build()
        .is_ok());

    let error = EntityBuilder::new(&schema, "IfcSubContext")
        .text("TargetView", "PLAN_VIEW")
        .build()
        .expect_err("an enum is not a string");
    assert!(
        matches!(error, AuthorError::TypeMismatch { .. }),
        "{error:?}"
    );
}

#[test]
fn entity_references_are_accepted_where_an_entity_is_declared() {
    let schema = schema();
    let mut model = Model::new();
    let a = EntityBuilder::new(&schema, "IfcCartesianPoint")
        .set("Coordinates", Value::List(vec![Value::Real(0.0)]))
        .insert(&mut model)
        .expect("point builds");
    let b = EntityBuilder::new(&schema, "IfcCartesianPoint")
        .set("Coordinates", Value::List(vec![Value::Real(1.0)]))
        .insert(&mut model)
        .expect("point builds");

    let line = EntityBuilder::new(&schema, "IfcPolyline")
        .set("Points", Value::List(vec![Value::Ref(a), Value::Ref(b)]))
        .insert(&mut model)
        .expect("a polyline of two points builds");

    assert!(model.dangling_references().is_empty());
    assert_eq!(model.get(line).unwrap().references(), vec![a, b]);
}

/// A failed build must not leave a partial record behind.
#[test]
fn a_refused_insert_does_not_touch_the_model() {
    let schema = schema();
    let mut model = Model::new();
    let _ = EntityBuilder::new(&schema, "IfcAnnotation")
        .text("Name", "no id")
        .insert(&mut model)
        .expect_err("refused");
    assert!(model.is_empty(), "nothing was written");
}
