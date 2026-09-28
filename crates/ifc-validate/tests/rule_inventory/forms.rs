//! Value-form rule ids: a typed parameter where the declared type is not a
//! SELECT, and a bare value where it is (ISO 10303-21:2016 §12.1.6-§12.1.8).
//!
//! Each failing fixture differs from its passing one only in the form or the
//! type named by one value. Where a failing fixture's twin produces a
//! *different* form finding, that is deliberate: it pins the boundary
//! between the two ids.

use ifc_model::{EntityId, Model, Value};
use ifc_validate::Report;

use super::cases::Case;
use super::fixtures::{entity, ifc4, text, typed, wall_with};

/// An `IfcQuantityArea` whose `AreaValue : IfcAreaMeasure` holds `value`.
fn area(value: Value) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.push(entity(
        schema,
        "IFCQUANTITYAREA",
        &[("Name", text("Area")), ("AreaValue", value)],
    ));
    ifc4(&model)
}

/// An `IfcQuantityLength` whose `LengthValue : IfcLengthMeasure` holds
/// `value`.
fn length(value: Value) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.push(entity(
        schema,
        "IFCQUANTITYLENGTH",
        &[("Name", text("Length")), ("LengthValue", value)],
    ));
    ifc4(&model)
}

/// An `IfcCircle` whose `Radius : IfcPositiveLengthMeasure` holds `radius`.
fn circle(radius: Value) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.insert(EntityId(1), entity(schema, "IFCAXIS2PLACEMENT2D", &[]));
    model.push(entity(
        schema,
        "IFCCIRCLE",
        &[("Position", Value::Ref(EntityId(1))), ("Radius", radius)],
    ));
    ifc4(&model)
}

/// An `IfcCartesianPoint` whose `Coordinates : LIST [1:3] OF
/// IfcLengthMeasure` holds `second` after a bare first member.
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

/// An `IfcPropertySingleValue` whose `NominalValue : IfcValue` holds
/// `value`.
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

/// An `IfcPropertyListValue` whose `ListValues : LIST OF IfcValue` holds
/// `second` after a correctly typed first member.
fn list_value(second: Value) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.push(entity(
        schema,
        "IFCPROPERTYLISTVALUE",
        &[
            ("Name", text("p")),
            (
                "ListValues",
                Value::List(vec![typed("IFCLABEL", text("a")), second]),
            ),
        ],
    ));
    ifc4(&model)
}

/// An `IfcPresentationStyleAssignment` whose `Styles : SET OF
/// IfcPresentationStyleSelect` holds `style`. IFC4 lists the enumeration
/// `IfcNullStyle` in that SELECT.
fn style_assignment(style: Value) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.push(entity(
        schema,
        "IFCPRESENTATIONSTYLEASSIGNMENT",
        &[("Styles", Value::List(vec![style]))],
    ));
    ifc4(&model)
}

/// An `IfcSurfaceStyleRendering` whose `DiffuseColour : IfcColourOrFactor`
/// holds `colour`. The SELECT lists `IfcNormalisedRatioMeasure`, a defined
/// type over `IfcRatioMeasure`.
fn rendering(colour: Value) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.insert(EntityId(1), entity(schema, "IFCCOLOURRGB", &[]));
    model.push(entity(
        schema,
        "IFCSURFACESTYLERENDERING",
        &[
            ("SurfaceColour", Value::Ref(EntityId(1))),
            ("DiffuseColour", colour),
        ],
    ));
    ifc4(&model)
}

/// An IFC4X3 `IfcPointByDistanceExpression` whose `DistanceAlong :
/// IfcCurveMeasureSelect` holds `distance`: the defect the committed
/// `synthetic_alignment_layout.ifc` carried.
fn distance_expression(distance: Value) -> Report {
    let schema = ifc_schema::ifc4x3();
    let mut model = Model::new();
    model.insert(EntityId(1), entity(schema, "IFCLINE", &[]));
    model.push(entity(
        schema,
        "IFCPOINTBYDISTANCEEXPRESSION",
        &[
            ("DistanceAlong", distance),
            ("BasisCurve", Value::Ref(EntityId(1))),
        ],
    ));
    ifc_validate::validate(&model, schema)
}

pub const CASES: &[Case] = &[
    Case {
        rule: "type.typed.outside_select",
        form: "IFCAREAMEASURE(12.5) in IfcQuantityArea.AreaValue",
        fails: || area(typed("IFCAREAMEASURE", Value::Real(12.5))),
        passes: || area(Value::Real(12.5)),
    },
    Case {
        rule: "type.typed.outside_select",
        form: "a specialisation's wrapper in a slot of its underlying type",
        fails: || length(typed("IFCPOSITIVELENGTHMEASURE", Value::Real(1.0))),
        passes: || length(Value::Real(1.0)),
    },
    Case {
        rule: "type.typed.outside_select",
        form: "a wrapper inside a LIST OF IfcLengthMeasure",
        fails: || point(typed("IFCLENGTHMEASURE", Value::Real(1.0))),
        passes: || point(Value::Real(1.0)),
    },
    Case {
        rule: "type.typed.outside_select",
        form: "an enumeration wrapper in an enumeration slot",
        fails: || {
            wall_with(
                "PredefinedType",
                typed("IFCWALLTYPEENUM", Value::Enum("STANDARD".into())),
            )
        },
        passes: || wall_with("PredefinedType", Value::Enum("STANDARD".into())),
    },
    Case {
        rule: "type.typed.outside_select",
        form: "a wrapper nested in a SELECT's typed parameter",
        fails: || single_value(typed("IFCLABEL", typed("IFCLABEL", text("x")))),
        passes: || single_value(typed("IFCLABEL", text("x"))),
    },
    Case {
        rule: "type.typed.wrong_type",
        form: "IFCLABEL in IfcQuantityArea.AreaValue; the right wrapper is only the wrong form",
        fails: || area(typed("IFCLABEL", text("x"))),
        passes: || area(typed("IFCAREAMEASURE", Value::Real(12.5))),
    },
    Case {
        rule: "type.typed.wrong_type",
        form: "the underlying type's wrapper where a specialisation is declared",
        fails: || circle(typed("IFCLENGTHMEASURE", Value::Real(1.0))),
        passes: || circle(typed("IFCPOSITIVELENGTHMEASURE", Value::Real(1.0))),
    },
    Case {
        rule: "type.typed.wrong_type",
        form: "a mismatched wrapper inside a LIST OF IfcLengthMeasure",
        fails: || point(typed("IFCAREAMEASURE", Value::Real(1.0))),
        passes: || point(typed("IFCLENGTHMEASURE", Value::Real(1.0))),
    },
    Case {
        rule: "type.select.untyped",
        form: "a bare number in IfcValue",
        fails: || single_value(Value::Real(1.0)),
        passes: || single_value(typed("IFCREAL", Value::Real(1.0))),
    },
    Case {
        rule: "type.select.untyped",
        form: "a bare member of a LIST OF IfcValue",
        fails: || list_value(text("b")),
        passes: || list_value(typed("IFCLABEL", text("b"))),
    },
    Case {
        rule: "type.select.untyped",
        form: "a bare enumeration constant in a SELECT listing the enumeration",
        fails: || style_assignment(Value::Enum("NULL".into())),
        passes: || style_assignment(typed("IFCNULLSTYLE", Value::Enum("NULL".into()))),
    },
    Case {
        rule: "type.select.untyped",
        form: "a bare DistanceAlong in IFC4X3 IfcCurveMeasureSelect",
        fails: || distance_expression(Value::Real(5.0)),
        passes: || distance_expression(typed("IFCLENGTHMEASURE", Value::Real(5.0))),
    },
    Case {
        rule: "type.select.member",
        form: "a listed defined type's underlying type is not itself a member",
        fails: || rendering(typed("IFCRATIOMEASURE", Value::Real(0.5))),
        passes: || rendering(typed("IFCNORMALISEDRATIOMEASURE", Value::Real(0.5))),
    },
];
