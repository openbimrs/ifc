//! The IFC4X3 segment enumerations, pinned against the bundled table (#16).
//!
//! The readers turn each `PredefinedType` token into a typed variant. A
//! schema member they did not recognise would degrade to a verbatim
//! `Other`, so every member is read through the public reader here. Slot
//! positions are pinned by the unit tests in `src/slot/tests.rs`.

use std::sync::Arc;

use ifc_alignment::{
    read_cant_segment, read_horizontal_segment, read_vertical_segment, AlignmentError,
    AlignmentUnits, CantSegmentType, HorizontalSegmentType, VerticalSegmentType,
};
use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::{ifc4x3, TypeKind};

const UNITS: AlignmentUnits = AlignmentUnits {
    length_to_metres: 1.0,
    angle_to_radians: 1.0,
};

fn members(enumeration: &str) -> Vec<String> {
    match &ifc4x3().type_def(enumeration).expect("declared").kind {
        TypeKind::Enumeration(members) => members.clone(),
        other => panic!("{enumeration} is not an enumeration: {other:?}"),
    }
}

#[test]
fn the_segment_enumerations_are_pinned() {
    assert_eq!(
        members("IfcAlignmentHorizontalSegmentTypeEnum"),
        [
            "BLOSSCURVE",
            "CIRCULARARC",
            "CLOTHOID",
            "COSINECURVE",
            "CUBIC",
            "HELMERTCURVE",
            "LINE",
            "SINECURVE",
            "VIENNESEBEND"
        ]
    );
    assert_eq!(
        members("IfcAlignmentVerticalSegmentTypeEnum"),
        [
            "CIRCULARARC",
            "CLOTHOID",
            "CONSTANTGRADIENT",
            "PARABOLICARC"
        ]
    );
    assert_eq!(
        members("IfcAlignmentCantSegmentTypeEnum"),
        [
            "BLOSSCURVE",
            "CONSTANTCANT",
            "COSINECURVE",
            "HELMERTCURVE",
            "LINEARTRANSITION",
            "SINECURVE",
            "VIENNESEBEND"
        ]
    );
}

fn segment(type_name: &str, values: Vec<Value>) -> Model {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![Value::Real(0.0), Value::Real(0.0)])],
        ),
    );
    model.insert(EntityId(2), Entity::new(type_name, values));
    model
}

#[test]
fn every_vertical_member_reads_to_a_named_variant() {
    for member in members("IfcAlignmentVerticalSegmentTypeEnum") {
        // Whether a family takes a radius is the reader's rule; one of the
        // two forms reads.
        let read = [None, Some(1_000.0)].into_iter().find_map(|radius| {
            let model = segment(
                "IFCALIGNMENTVERTICALSEGMENT",
                vec![
                    Value::Null,
                    Value::Null,
                    Value::Real(0.0),
                    Value::Real(10.0),
                    Value::Real(0.0),
                    Value::Real(0.0),
                    Value::Real(0.0),
                    radius.map_or(Value::Null, Value::Real),
                    Value::Enum(Arc::from(member.as_str())),
                ],
            );
            read_vertical_segment(&model, EntityId(2), UNITS).ok()
        });
        let read = read.unwrap_or_else(|| panic!("{member} reads in neither form"));
        assert!(
            !matches!(read.predefined_type, VerticalSegmentType::Other(_)),
            "{member} is not a named variant"
        );
    }
}

#[test]
fn every_cant_member_reads_to_a_named_variant() {
    for member in members("IfcAlignmentCantSegmentTypeEnum") {
        let model = segment(
            "IFCALIGNMENTCANTSEGMENT",
            vec![
                Value::Null,
                Value::Null,
                Value::Real(0.0),
                Value::Real(10.0),
                Value::Real(0.0),
                Value::Real(0.0),
                Value::Real(0.0),
                Value::Real(0.0),
                Value::Enum(Arc::from(member.as_str())),
            ],
        );
        let read = read_cant_segment(&model, EntityId(2), UNITS).expect("reads");
        assert!(
            !matches!(read.predefined_type, CantSegmentType::Other(_)),
            "{member} is not a named variant"
        );
    }
}

/// `LINE` and `CIRCULARARC` are named; every other member is a transition
/// family, preserved verbatim for the lowering to accept or refuse.
#[test]
fn every_horizontal_member_reads_and_keeps_its_token() {
    for member in members("IfcAlignmentHorizontalSegmentTypeEnum") {
        let model = segment(
            "IFCALIGNMENTHORIZONTALSEGMENT",
            vec![
                Value::Null,
                Value::Null,
                Value::Ref(EntityId(1)),
                Value::Real(0.0),
                Value::Real(0.0),
                Value::Real(0.0),
                Value::Real(10.0),
                Value::Null,
                Value::Enum(Arc::from(member.as_str())),
            ],
        );
        let read = read_horizontal_segment(&model, EntityId(2), UNITS).expect("reads");
        assert_eq!(read.segment_type.source_name(), member);
        let named = matches!(
            read.segment_type,
            HorizontalSegmentType::Line | HorizontalSegmentType::CircularArc
        );
        assert_eq!(
            named,
            member == "LINE" || member == "CIRCULARARC",
            "{member}"
        );
    }
}

/// A record of a type the reader does not serve is refused by name.
#[test]
fn a_parameter_segment_of_the_wrong_family_is_refused() {
    let model = segment("IFCALIGNMENTCANTSEGMENT", vec![Value::Null; 9]);
    assert!(matches!(
        read_vertical_segment(&model, EntityId(2), UNITS),
        Err(AlignmentError::WrongType { .. })
    ));
    assert!(matches!(
        read_horizontal_segment(&model, EntityId(2), UNITS),
        Err(AlignmentError::WrongType { .. })
    ));
}
