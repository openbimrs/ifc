//! The value form of a boundary condition's stiffness, per release (#200, #201).
//!
//! Evidence, `references/ifc-spec/*.exp`:
//!
//! ```text
//!                 IFC2X3                                IFC4, IFC4X3
//! node  trans.    LinearStiffnessX                      TranslationalStiffnessX
//!                   : IfcLinearStiffnessMeasure           : IfcTranslationalStiffnessSelect
//!       rot.      RotationalStiffnessX                  RotationalStiffnessX
//!                   : IfcRotationalStiffnessMeasure       : IfcRotationalStiffnessSelect
//! edge  trans.    LinearStiffnessByLengthX              TranslationalStiffnessByLengthX
//!                   : IfcModulusOfLinearSubgrade-         : IfcModulusOfTranslational-
//!                     ReactionMeasure                       SubgradeReactionSelect
//!       rot.      RotationalStiffnessByLengthX          RotationalStiffnessByLengthX
//!                   : IfcModulusOfRotationalSubgrade-     : IfcModulusOfRotational-
//!                     ReactionMeasure                       SubgradeReactionSelect
//! face  trans.    LinearStiffnessByAreaX                TranslationalStiffnessByAreaX
//!                   : IfcModulusOfSubgradeReactionMeasure : IfcModulusOfSubgradeReactionSelect
//! warping         WarpingStiffness                      WarpingStiffness
//!                   : IfcWarpingMomentMeasure             : IfcWarpingStiffnessSelect
//! ```
//!
//! Every IFC4/IFC4X3 SELECT here is `(IfcBoolean, <the IFC2X3 measure>)`,
//! for example `IfcModulusOfTranslationalSubgradeReactionSelect =
//! SELECT (IfcBoolean, IfcModulusOfLinearSubgradeReactionMeasure)` and
//! `IfcWarpingStiffnessSelect = SELECT (IfcBoolean, IfcWarpingMomentMeasure)`.
//! No release declares an `IfcModulusOfTranslationalSubgradeReactionMeasure`.
//!
//! ISO 10303-21 writes a typed parameter only where the declared type is a
//! SELECT, naming the member the value is. So the form is read from the bound
//! table, never assumed: bare in IFC2X3, the member's wrapper in IFC4 and
//! IFC4X3, and a boolean refused where the declared type admits none.

use ifc_model::Value;
use ifc_schema::{Schema, TypeKind};

use crate::condition::{AxisValues, BoundaryConditionKind, StiffnessValue};
use crate::error::{StructuralError, StructuralResult};

/// The entity each family is written as.
pub(super) fn entity_name(kind: BoundaryConditionKind) -> &'static str {
    match kind {
        BoundaryConditionKind::Edge => "IFCBOUNDARYEDGECONDITION",
        BoundaryConditionKind::Face => "IFCBOUNDARYFACECONDITION",
        BoundaryConditionKind::Node => "IFCBOUNDARYNODECONDITION",
        BoundaryConditionKind::NodeWarping => "IFCBOUNDARYNODECONDITIONWARPING",
    }
}

/// The measure a family's stiffness is. A node condition carries a
/// stiffness, an edge condition a modulus of subgrade reaction per length, a
/// face condition per area: writing the wrong one states a different physical
/// quantity. Each is declared by all three releases.
fn measure_name(kind: BoundaryConditionKind, translational: bool) -> &'static str {
    match (kind, translational) {
        (BoundaryConditionKind::Edge, true) => "IfcModulusOfLinearSubgradeReactionMeasure",
        (BoundaryConditionKind::Edge, false) => "IfcModulusOfRotationalSubgradeReactionMeasure",
        (BoundaryConditionKind::Face, _) => "IfcModulusOfSubgradeReactionMeasure",
        (_, true) => "IfcLinearStiffnessMeasure",
        (_, false) => "IfcRotationalStiffnessMeasure",
    }
}

/// The IFC4/IFC4X3 and IFC2X3 names of one stiffness triple.
fn names(
    kind: BoundaryConditionKind,
    translational: bool,
) -> ([&'static str; 3], [&'static str; 3]) {
    match (kind, translational) {
        (BoundaryConditionKind::Edge, true) => (
            [
                "TranslationalStiffnessByLengthX",
                "TranslationalStiffnessByLengthY",
                "TranslationalStiffnessByLengthZ",
            ],
            [
                "LinearStiffnessByLengthX",
                "LinearStiffnessByLengthY",
                "LinearStiffnessByLengthZ",
            ],
        ),
        (BoundaryConditionKind::Edge, false) => {
            let names = [
                "RotationalStiffnessByLengthX",
                "RotationalStiffnessByLengthY",
                "RotationalStiffnessByLengthZ",
            ];
            (names, names)
        }
        (BoundaryConditionKind::Face, _) => (
            [
                "TranslationalStiffnessByAreaX",
                "TranslationalStiffnessByAreaY",
                "TranslationalStiffnessByAreaZ",
            ],
            [
                "LinearStiffnessByAreaX",
                "LinearStiffnessByAreaY",
                "LinearStiffnessByAreaZ",
            ],
        ),
        (_, true) => (
            [
                "TranslationalStiffnessX",
                "TranslationalStiffnessY",
                "TranslationalStiffnessZ",
            ],
            ["LinearStiffnessX", "LinearStiffnessY", "LinearStiffnessZ"],
        ),
        (_, false) => {
            let names = [
                "RotationalStiffnessX",
                "RotationalStiffnessY",
                "RotationalStiffnessZ",
            ];
            (names, names)
        }
    }
}

/// One stiffness triple as named fields in `schema`'s release.
///
/// The release's own attribute names are used, so the record is laid out by
/// its table: IFC2X3 still calls the translational values `Linear...`.
pub(super) fn triple(
    schema: &Schema,
    kind: BoundaryConditionKind,
    translational: bool,
    values: AxisValues<Option<StiffnessValue>>,
) -> StructuralResult<[(&'static str, Value); 3]> {
    let entity = entity_name(kind);
    let (current, legacy) = names(kind, translational);
    let declared = schema.attribute_names(entity);
    let names = if declared
        .iter()
        .any(|name| name.eq_ignore_ascii_case(current[0]))
    {
        current
    } else {
        legacy
    };
    let measure = measure_name(kind, translational);
    let field = |index: usize, value| {
        declared_form(schema, entity, names[index], value, measure).map(|form| (names[index], form))
    };
    Ok([
        field(0, values.x)?,
        field(1, values.y)?,
        field(2, values.z)?,
    ])
}

/// `WarpingStiffness` as a named field in `schema`'s release.
pub(super) fn warping(
    schema: &Schema,
    value: Option<StiffnessValue>,
) -> StructuralResult<(&'static str, Value)> {
    const ATTRIBUTE: &str = "WarpingStiffness";
    let entity = entity_name(BoundaryConditionKind::NodeWarping);
    let form = declared_form(schema, entity, ATTRIBUTE, value, "IfcWarpingMomentMeasure")?;
    Ok((ATTRIBUTE, form))
}

/// `value` in the form `attribute`'s declared type requires in `schema`.
///
/// A SELECT takes the typed parameter of the member the value is; a defined
/// type takes the bare value. A member the declared type does not admit is
/// refused, never written.
fn declared_form(
    schema: &Schema,
    entity_type: &'static str,
    attribute: &'static str,
    value: Option<StiffnessValue>,
    measure: &'static str,
) -> StructuralResult<Value> {
    let (member, payload) = match value {
        None => return Ok(Value::Null),
        Some(StiffnessValue::Boolean(flag)) => ("IfcBoolean", Value::Bool(flag)),
        Some(StiffnessValue::Measure(number)) => (measure, Value::Real(number)),
    };
    let declared = schema
        .attributes(entity_type)
        .into_iter()
        .find(|declared| declared.name.eq_ignore_ascii_case(attribute))
        .map(|declared| declared.type_name.clone())
        .ok_or_else(|| StructuralError::UnsupportedAttribute {
            entity_type: entity_type.to_owned(),
            attribute: attribute.to_owned(),
        })?;
    if !schema.accepts_type(&declared, member) {
        return Err(StructuralError::InvalidDraftValue {
            entity_type,
            attribute,
            expected:
                "a value the release's declared type admits (IFC2X3 has no boolean stiffness)",
        });
    }
    let select = schema
        .type_def(&declared)
        .is_some_and(|definition| matches!(definition.kind, TypeKind::Select(_)));
    Ok(if select {
        Value::Typed {
            type_name: member.to_ascii_uppercase().into(),
            value: Box::new(payload),
        }
    } else {
        payload
    })
}
