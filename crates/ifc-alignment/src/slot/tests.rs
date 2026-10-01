//! The declaration and slot inventory, pinned against the bundled IFC4X3
//! ADD2 table (#16).
//!
//! The bundled table ships inside `ifc-schema`, so these run without
//! `references/ifc-spec/` and redistribute nothing. A regenerated schema
//! that moves, renames or drops anything this crate indexes fails here
//! rather than as a silently misread file.

use ifc_schema::{ifc2x3, ifc4, ifc4x1, ifc4x2, ifc4x3, Schema};

use super::*;

/// Every IFC4X3 declaration this crate reads or writes.
const DECLARATIONS: &[&str] = &[
    "IfcAlignment",
    "IfcAlignmentHorizontal",
    "IfcAlignmentVertical",
    "IfcAlignmentCant",
    "IfcAlignmentSegment",
    "IfcAlignmentParameterSegment",
    "IfcAlignmentHorizontalSegment",
    "IfcAlignmentVerticalSegment",
    "IfcAlignmentCantSegment",
    "IfcLinearElement",
    "IfcLinearPositioningElement",
    "IfcPositioningElement",
    "IfcReferent",
    "IfcLinearPlacement",
    "IfcAxis2PlacementLinear",
    "IfcPointByDistanceExpression",
    "IfcCartesianPoint",
    "IfcRelNests",
    "IfcRelAggregates",
    "IfcRelPositions",
    "IfcRelDefinesByProperties",
    "IfcPropertySet",
    "IfcPropertySingleValue",
];

/// `(entity, slot, attribute name)` for every named slot in `crate::slot`.
const SLOTS: &[(&str, usize, &str)] = &[
    ("IfcAlignmentHorizontalSegment", 0, "StartTag"),
    ("IfcAlignmentHorizontalSegment", 1, "EndTag"),
    (
        "IfcAlignmentHorizontalSegment",
        horizontal::START_POINT,
        "StartPoint",
    ),
    (
        "IfcAlignmentHorizontalSegment",
        horizontal::START_DIRECTION,
        "StartDirection",
    ),
    (
        "IfcAlignmentHorizontalSegment",
        horizontal::START_RADIUS,
        "StartRadiusOfCurvature",
    ),
    (
        "IfcAlignmentHorizontalSegment",
        horizontal::END_RADIUS,
        "EndRadiusOfCurvature",
    ),
    (
        "IfcAlignmentHorizontalSegment",
        horizontal::SEGMENT_LENGTH,
        "SegmentLength",
    ),
    (
        "IfcAlignmentHorizontalSegment",
        horizontal::GRAVITY_CENTER_LINE_HEIGHT,
        "GravityCenterLineHeight",
    ),
    (
        "IfcAlignmentHorizontalSegment",
        horizontal::PREDEFINED_TYPE,
        "PredefinedType",
    ),
    ("IfcAlignmentVerticalSegment", 0, "StartTag"),
    (
        "IfcAlignmentVerticalSegment",
        vertical::START_DIST_ALONG,
        "StartDistAlong",
    ),
    (
        "IfcAlignmentVerticalSegment",
        vertical::HORIZONTAL_LENGTH,
        "HorizontalLength",
    ),
    (
        "IfcAlignmentVerticalSegment",
        vertical::START_HEIGHT,
        "StartHeight",
    ),
    (
        "IfcAlignmentVerticalSegment",
        vertical::START_GRADIENT,
        "StartGradient",
    ),
    (
        "IfcAlignmentVerticalSegment",
        vertical::END_GRADIENT,
        "EndGradient",
    ),
    (
        "IfcAlignmentVerticalSegment",
        vertical::RADIUS_OF_CURVATURE,
        "RadiusOfCurvature",
    ),
    (
        "IfcAlignmentVerticalSegment",
        vertical::PREDEFINED_TYPE,
        "PredefinedType",
    ),
    ("IfcAlignmentCantSegment", 0, "StartTag"),
    (
        "IfcAlignmentCantSegment",
        cant::START_DIST_ALONG,
        "StartDistAlong",
    ),
    (
        "IfcAlignmentCantSegment",
        cant::HORIZONTAL_LENGTH,
        "HorizontalLength",
    ),
    (
        "IfcAlignmentCantSegment",
        cant::START_CANT_LEFT,
        "StartCantLeft",
    ),
    (
        "IfcAlignmentCantSegment",
        cant::END_CANT_LEFT,
        "EndCantLeft",
    ),
    (
        "IfcAlignmentCantSegment",
        cant::START_CANT_RIGHT,
        "StartCantRight",
    ),
    (
        "IfcAlignmentCantSegment",
        cant::END_CANT_RIGHT,
        "EndCantRight",
    ),
    (
        "IfcAlignmentCantSegment",
        cant::PREDEFINED_TYPE,
        "PredefinedType",
    ),
    ("IfcAlignment", product::GLOBAL_ID, "GlobalId"),
    ("IfcAlignment", product::NAME, "Name"),
    ("IfcAlignment", product::OBJECT_PLACEMENT, "ObjectPlacement"),
    ("IfcAlignment", alignment::PREDEFINED_TYPE, "PredefinedType"),
    (
        "IfcAlignmentHorizontal",
        product::OBJECT_PLACEMENT,
        "ObjectPlacement",
    ),
    (
        "IfcAlignmentCant",
        cant_layout::RAIL_HEAD_DISTANCE,
        "RailHeadDistance",
    ),
    (
        "IfcAlignmentSegment",
        segment::DESIGN_PARAMETERS,
        "DesignParameters",
    ),
    ("IfcReferent", product::OBJECT_PLACEMENT, "ObjectPlacement"),
    ("IfcReferent", referent::PREDEFINED_TYPE, "PredefinedType"),
    (
        "IfcLinearPlacement",
        linear_placement::PLACEMENT_REL_TO,
        "PlacementRelTo",
    ),
    (
        "IfcLinearPlacement",
        linear_placement::RELATIVE_PLACEMENT,
        "RelativePlacement",
    ),
    (
        "IfcLinearPlacement",
        linear_placement::CARTESIAN_POSITION,
        "CartesianPosition",
    ),
    (
        "IfcPointByDistanceExpression",
        point_by_distance::DISTANCE_ALONG,
        "DistanceAlong",
    ),
    (
        "IfcPointByDistanceExpression",
        point_by_distance::OFFSET_LATERAL,
        "OffsetLateral",
    ),
    (
        "IfcPointByDistanceExpression",
        point_by_distance::OFFSET_VERTICAL,
        "OffsetVertical",
    ),
    (
        "IfcPointByDistanceExpression",
        point_by_distance::OFFSET_LONGITUDINAL,
        "OffsetLongitudinal",
    ),
    (
        "IfcPointByDistanceExpression",
        point_by_distance::BASIS_CURVE,
        "BasisCurve",
    ),
    (
        "IfcAxis2PlacementLinear",
        axis2_placement_linear::LOCATION,
        "Location",
    ),
    (
        "IfcAxis2PlacementLinear",
        axis2_placement_linear::AXIS,
        "Axis",
    ),
    (
        "IfcAxis2PlacementLinear",
        axis2_placement_linear::REF_DIRECTION,
        "RefDirection",
    ),
    ("IfcRelNests", decomposes::RELATING_OBJECT, "RelatingObject"),
    ("IfcRelNests", decomposes::RELATED_OBJECTS, "RelatedObjects"),
    (
        "IfcRelAggregates",
        decomposes::RELATING_OBJECT,
        "RelatingObject",
    ),
    (
        "IfcRelAggregates",
        decomposes::RELATED_OBJECTS,
        "RelatedObjects",
    ),
    (
        "IfcRelPositions",
        rel_positions::RELATING_POSITIONING_ELEMENT,
        "RelatingPositioningElement",
    ),
    (
        "IfcRelPositions",
        rel_positions::RELATED_PRODUCTS,
        "RelatedProducts",
    ),
    (
        "IfcRelDefinesByProperties",
        rel_defines_by_properties::RELATED_OBJECTS,
        "RelatedObjects",
    ),
    (
        "IfcRelDefinesByProperties",
        rel_defines_by_properties::RELATING_PROPERTY_DEFINITION,
        "RelatingPropertyDefinition",
    ),
    ("IfcPropertySet", property_set::NAME, "Name"),
    (
        "IfcPropertySet",
        property_set::HAS_PROPERTIES,
        "HasProperties",
    ),
    (
        "IfcPropertySingleValue",
        property_single_value::NAME,
        "Name",
    ),
    (
        "IfcPropertySingleValue",
        property_single_value::NOMINAL_VALUE,
        "NominalValue",
    ),
];

/// `(entity, arity)`: the full inherited attribute count.
const ARITIES: &[(&str, usize)] = &[
    (
        "IfcAlignmentHorizontalSegment",
        horizontal::PREDEFINED_TYPE + 1,
    ),
    ("IfcAlignmentVerticalSegment", vertical::PREDEFINED_TYPE + 1),
    ("IfcAlignmentCantSegment", cant::PREDEFINED_TYPE + 1),
    ("IfcAlignmentHorizontal", product::ARITY),
    ("IfcAlignmentVertical", product::ARITY),
    ("IfcAlignment", alignment::ARITY),
    ("IfcAlignmentCant", cant_layout::ARITY),
    ("IfcAlignmentSegment", segment::ARITY),
    ("IfcReferent", referent::ARITY),
    ("IfcLinearPlacement", linear_placement::ARITY),
    ("IfcPointByDistanceExpression", point_by_distance::ARITY),
    ("IfcAxis2PlacementLinear", axis2_placement_linear::ARITY),
    ("IfcRelNests", decomposes::ARITY),
    ("IfcRelAggregates", decomposes::ARITY),
    ("IfcRelPositions", rel_positions::ARITY),
    (
        "IfcRelDefinesByProperties",
        rel_defines_by_properties::ARITY,
    ),
    ("IfcPropertySet", property_set::ARITY),
    ("IfcPropertySingleValue", property_single_value::ARITY),
];

fn names(schema: &Schema, entity: &str) -> Vec<String> {
    schema
        .attribute_names(entity)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_pinned_profile_is_ifc4x3_add2() {
    assert_eq!(ifc4x3().name(), "IFC4X3_ADD2");
}

#[test]
fn every_read_declaration_is_declared_by_ifc4x3() {
    let schema = ifc4x3();
    for name in DECLARATIONS {
        assert!(schema.entity(name).is_some(), "{name} is not declared");
    }
}

#[test]
fn every_named_slot_matches_the_bundled_ifc4x3_table() {
    let schema = ifc4x3();
    for &(entity, slot, expected) in SLOTS {
        let declared = names(schema, entity);
        assert_eq!(
            declared.get(slot).map(String::as_str),
            Some(expected),
            "{entity} slot {slot}: schema declares {declared:?}",
        );
    }
}

#[test]
fn every_arity_matches_the_bundled_ifc4x3_table() {
    let schema = ifc4x3();
    for &(entity, arity) in ARITIES {
        assert_eq!(
            schema.attribute_names(entity).len(),
            arity,
            "{entity} arity drifted",
        );
    }
}

/// The business-logic layouts do not exist before IFC4X3, which is why
/// `AlignmentView::for_model` refuses those releases outright.
#[test]
fn alignment_layouts_are_absent_before_ifc4x3() {
    for schema in [ifc2x3(), ifc4(), ifc4x1(), ifc4x2()] {
        for name in [
            "IfcAlignmentHorizontal",
            "IfcAlignmentVertical",
            "IfcAlignmentCant",
            "IfcAlignmentSegment",
            "IfcAlignmentHorizontalSegment",
            "IfcPointByDistanceExpression",
        ] {
            assert!(
                schema.entity(name).is_none(),
                "{name} unexpectedly declared by {}",
                schema.name(),
            );
        }
    }
}

/// IFC4X1 and IFC4X2 do declare an `IfcAlignment`, but with a different
/// attribute layout: reading one through these slots would misread it.
/// This is the evidence behind refusing them rather than aliasing.
#[test]
fn intermediate_releases_declare_a_different_ifc_alignment() {
    let pinned = names(ifc4x3(), "IfcAlignment");
    for schema in [ifc4x1(), ifc4x2()] {
        assert!(schema.entity("IfcAlignment").is_some());
        assert_ne!(
            names(schema, "IfcAlignment"),
            pinned,
            "{} IfcAlignment matches IFC4X3; revisit the refusal",
            schema.name(),
        );
    }
}

/// Subtype memberships the traversal relies on through `is_a`.
#[test]
fn inheritance_the_traversal_relies_on_holds() {
    let schema = ifc4x3();
    for (name, ancestor) in [
        ("IfcAlignment", "IfcLinearPositioningElement"),
        ("IfcAlignment", "IfcPositioningElement"),
        ("IfcReferent", "IfcPositioningElement"),
        ("IfcReferent", "IfcProduct"),
        ("IfcAlignmentHorizontal", "IfcLinearElement"),
        ("IfcAlignmentVertical", "IfcLinearElement"),
        ("IfcAlignmentCant", "IfcLinearElement"),
        ("IfcAlignmentSegment", "IfcLinearElement"),
        (
            "IfcAlignmentHorizontalSegment",
            "IfcAlignmentParameterSegment",
        ),
        (
            "IfcAlignmentVerticalSegment",
            "IfcAlignmentParameterSegment",
        ),
        ("IfcAlignmentCantSegment", "IfcAlignmentParameterSegment"),
        ("IfcRelNests", "IfcRelDecomposes"),
        ("IfcRelAggregates", "IfcRelDecomposes"),
        ("IfcRelPositions", "IfcRelConnects"),
    ] {
        assert!(schema.is_a(name, ancestor), "{name} is not an {ancestor}");
    }
    // The three layouts are siblings: a subtype-aware filter can never read
    // a nested horizontal layout as a vertical one.
    for (a, b) in [
        ("IfcAlignmentHorizontal", "IfcAlignmentVertical"),
        ("IfcAlignmentVertical", "IfcAlignmentCant"),
        ("IfcAlignmentCant", "IfcAlignmentHorizontal"),
        ("IfcAlignment", "IfcLinearElement"),
    ] {
        assert!(!schema.is_a(a, b) && !schema.is_a(b, a), "{a} / {b}");
    }
}

/// `DistanceAlong` is the one SELECT the readers discriminate on, and the
/// stationing properties travel in `IfcValue`.
#[test]
fn select_memberships_the_readers_discriminate_on() {
    let schema = ifc4x3();
    for member in ["IfcLengthMeasure", "IfcParameterValue"] {
        assert!(schema.accepts_type("IfcCurveMeasureSelect", member));
    }
    assert!(!schema.accepts_type("IfcCurveMeasureSelect", "IfcRatioMeasure"));
    for member in ["IfcLengthMeasure", "IfcBoolean"] {
        assert!(
            schema.accepts_type("IfcValue", member),
            "IfcValue lacks {member}"
        );
    }
    assert!(schema.accepts_type("IfcPropertySetDefinitionSelect", "IfcPropertySet"));
}
