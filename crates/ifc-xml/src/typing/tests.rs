//! Declared types resolved to shapes, and text typed by them.

use super::value::*;
use super::*;
use ifc_model::Value;

fn shape_of(entity: &str, attribute: &str) -> Shape {
    let schema = ifc_schema::ifc4();
    let attribute = schema
        .attributes(entity)
        .into_iter()
        .find(|candidate| candidate.name == attribute)
        .unwrap();
    attribute_shape(schema, attribute).unwrap()
}

#[test]
fn a_label_types_numeric_text_as_a_string() {
    let shape = shape_of("IfcRoot", "Name");
    assert_eq!(
        scalar(&shape.leaf, "1", Lexical::Xsd).unwrap(),
        Value::Text("1".into())
    );
    assert_eq!(
        scalar(&shape.leaf, "i7", Lexical::Native).unwrap(),
        Value::Text("i7".into())
    );
}

#[test]
fn aliased_aggregates_contribute_their_levels() {
    let shape = shape_of("IfcSite", "RefLatitude");
    assert_eq!(shape.levels.len(), 1);
    assert_eq!(shape.leaf, Leaf::Integer);
    let coordinates = shape_of("IfcCartesianPointList3D", "CoordList");
    assert_eq!(coordinates.levels.len(), 2);
    assert_eq!(coordinates.levels[1].fixed, Some(3));
    assert_eq!(coordinates.leaf, Leaf::Real);
}

#[test]
fn a_flat_list_nests_by_fixed_inner_bounds_or_is_refused() {
    let shape = shape_of("IfcCartesianPointList3D", "CoordList");
    let nested = list_text(&shape, "0 0 0 1 2 3", Lexical::Xsd).unwrap();
    assert_eq!(
        nested,
        Value::List(vec![
            Value::List(vec![Value::Real(0.0); 3]),
            Value::List(vec![Value::Real(1.0), Value::Real(2.0), Value::Real(3.0)]),
        ])
    );
    assert!(list_text(&shape, "0 0 0 1", Lexical::Xsd).is_err());
    let open = shape_of("IfcBSplineSurface", "ControlPointsList");
    assert!(matches!(
        nest(&open, vec![Value::Ref(ifc_model::EntityId(1)); 4], None),
        Err(XmlError::Unsupported { .. })
    ));
    assert!(nest(
        &open,
        vec![Value::Ref(ifc_model::EntityId(1)); 4],
        Some(&[2, 2])
    )
    .is_ok());
}

#[test]
fn enumerations_and_booleans_type_by_declaration() {
    let schema = ifc_schema::ifc4();
    let shape = type_shape(schema, "IfcWallTypeEnum").unwrap();
    assert_eq!(
        scalar(&shape.leaf, "notdefined", Lexical::Xsd).unwrap(),
        Value::Enum("NOTDEFINED".into())
    );
    assert!(scalar(&shape.leaf, "door", Lexical::Xsd).is_err());
    let boolean = type_shape(schema, "IfcBoolean").unwrap();
    assert_eq!(
        scalar(&boolean.leaf, "1", Lexical::Xsd).unwrap(),
        Value::Bool(true)
    );
    assert!(scalar(&boolean.leaf, "unknown", Lexical::Xsd).is_err());
    let real = type_shape(schema, "IfcLengthMeasure").unwrap();
    assert!(scalar(&real.leaf, "0,5", Lexical::Xsd).is_err());
    assert!(scalar(&real.leaf, "INF", Lexical::Xsd).is_err());
}
