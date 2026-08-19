//! Where-rules on solids and booleans.
//!
//! The rules here catch the two failures that waste the most time downstream:
//! an extrusion whose direction lies in the plane of its profile (produces a
//! zero-volume solid), and a boolean between operands of different
//! dimensionality (produces a kernel error with no useful location).

use super::{RuleViolation, ViolationKind};
use crate::resource::direction::Direction;
use ifc_model::{Entity, EntityId, Model, Value};

/// Run the solid rules that apply to this entity.
pub fn check(model: &Model, id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    match entity.type_name.to_ascii_uppercase().as_str() {
        "IFCEXTRUDEDAREASOLID" | "IFCEXTRUDEDAREASOLIDTAPERED" => {
            extruded_area_solid(model, id, entity, out)
        }
        "IFCBOOLEANRESULT" | "IFCBOOLEANCLIPPINGRESULT" => boolean_result(model, id, entity, out),
        "IFCPOLYGONALBOUNDEDHALFSPACE" => polygonal_bounded_half_space(model, id, entity, out),
        "IFCREVOLVEDAREASOLID" => revolved_area_solid(id, entity, out),
        _ => {}
    }
}

/// `IfcExtrudedAreaSolid.ValidExtrusionDirection`.
///
/// The schema states it as a dot product: the extrusion direction must not be
/// perpendicular to the z axis of the position coordinate system. Equivalently
/// the direction must not lie in the profile's plane -- sweeping a 2D profile
/// along a direction inside its own plane sweeps out no volume.
fn extruded_area_solid(model: &Model, id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    const TYPE: &str = "IFCEXTRUDEDAREASOLID";

    // Slot 2 is ExtrudedDirection: SweptArea and Position are inherited and
    // occupy slots 0 and 1. See references/absolute-slots.txt.
    let Some(dir_id) = entity.attributes.get(2).and_then(|v| v.as_ref_id()) else {
        return;
    };
    let Some(dir_entity) = model.get(dir_id) else {
        return;
    };
    let Ok(ratios) = Direction::new(dir_id, dir_entity).ratios() else {
        return;
    };

    // The profile lies in the XY plane of the position system, so the z
    // component is the dot product with the plane normal.
    let z = ratios.get(2).copied().unwrap_or(0.0);
    let magnitude_sq: f64 = ratios.iter().map(|r| r * r).sum();
    if magnitude_sq <= 0.0 {
        return; // the IfcDirection rule reports this separately
    }
    if (z * z) / magnitude_sq < 1e-20 {
        out.push(RuleViolation::new(
            id,
            TYPE,
            "ValidExtrusionDirection",
            ViolationKind::Degenerate,
            format!(
                "ExtrudedDirection {dir_id} lies in the profile plane; \
                 the extrusion has zero volume"
            ),
        ));
    }

    // Depth must be positive: IfcPositiveLengthMeasure.
    if let Some(depth) = entity
        .attributes
        .get(3)
        .and_then(|v| v.unwrap_typed().as_f64())
    {
        if depth <= 0.0 {
            out.push(RuleViolation::new(
                id,
                TYPE,
                "Depth",
                ViolationKind::OutOfRange,
                format!("Depth is {depth}, must be a positive length"),
            ));
        }
    }
}

/// `IfcRevolvedAreaSolid.AxisLine`/`AngleGreaterZero`.
fn revolved_area_solid(id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    // Slot 3 is Angle; slots 0-1 inherited, slot 2 is Axis.
    if let Some(angle) = entity
        .attributes
        .get(3)
        .and_then(|v| v.unwrap_typed().as_f64())
    {
        if angle <= 0.0 {
            out.push(RuleViolation::new(
                id,
                "IFCREVOLVEDAREASOLID",
                "AngleGreaterZero",
                ViolationKind::OutOfRange,
                format!("Angle is {angle}, must be greater than zero"),
            ));
        }
    }
}

/// `IfcBooleanResult.SameDim`.
///
/// Both operands must have the same dimensionality. Mixing a 2D and a 3D
/// operand is meaningless, and a kernel discovers it only as a failed
/// intersection.
fn boolean_result(model: &Model, id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    let type_name = entity.type_name.to_ascii_uppercase();
    let first = entity.attributes.get(1).and_then(|v| v.as_ref_id());
    let second = entity.attributes.get(2).and_then(|v| v.as_ref_id());

    if let (Some(a), Some(b)) = (first, second) {
        if let (Some(da), Some(db)) = (operand_dim(model, a), operand_dim(model, b)) {
            if da != db {
                out.push(RuleViolation::new(
                    id,
                    type_name.clone(),
                    "SameDim",
                    ViolationKind::Disagreement,
                    format!("FirstOperand {a} is {da}D but SecondOperand {b} is {db}D"),
                ));
            }
        }
    }

    // IfcBooleanClippingResult additionally requires the operation to be
    // DIFFERENCE and the second operand to be a half space.
    if type_name == "IFCBOOLEANCLIPPINGRESULT" {
        if let Some(Value::Enum(op)) = entity.attributes.first() {
            if !op.eq_ignore_ascii_case("DIFFERENCE") {
                out.push(RuleViolation::new(
                    id,
                    type_name.clone(),
                    "FirstOperandType",
                    ViolationKind::WrongType,
                    format!("clipping must use DIFFERENCE, found {op}"),
                ));
            }
        }
        if let Some(b) = second {
            if let Some(e) = model.get(b) {
                if !crate::select::is_a(&e.type_name.to_ascii_uppercase(), "IFCHALFSPACESOLID") {
                    out.push(RuleViolation::new(
                        id,
                        type_name,
                        "SecondOperandType",
                        ViolationKind::WrongType,
                        format!(
                            "clipping requires a half space as SecondOperand, found {}",
                            e.type_name
                        ),
                    ));
                }
            }
        }
    }
}

/// `IfcPolygonalBoundedHalfSpace.BoundaryType` and `BoundaryDim`.
///
/// The boundary must be a 2D polyline or composite curve. Any other curve type
/// cannot bound the extruded region the schema describes.
fn polygonal_bounded_half_space(
    model: &Model,
    id: EntityId,
    entity: &Entity,
    out: &mut Vec<RuleViolation>,
) {
    // Slot 3 is PolygonalBoundary: BaseSurface and AgreementFlag are
    // inherited (0, 1) and Position is slot 2.
    let Some(boundary) = entity.attributes.get(3).and_then(|v| v.as_ref_id()) else {
        return;
    };
    let Some(curve) = model.get(boundary) else {
        return;
    };
    let name = curve.type_name.to_ascii_uppercase();
    if name != "IFCPOLYLINE" && name != "IFCCOMPOSITECURVE" {
        out.push(RuleViolation::new(
            id,
            "IFCPOLYGONALBOUNDEDHALFSPACE",
            "BoundaryType",
            ViolationKind::WrongType,
            format!("PolygonalBoundary must be IfcPolyline or IfcCompositeCurve, found {name}"),
        ));
    }
}

/// Dimensionality of a boolean operand.
///
/// Solids and half spaces are 3D by construction; the interesting case is a
/// tessellated face set or a nested boolean, which are resolved recursively.
fn operand_dim(model: &Model, id: EntityId) -> Option<usize> {
    let entity = model.get(id)?;
    let name = entity.type_name.to_ascii_uppercase();
    if crate::select::is_a(&name, "IFCSOLIDMODEL")
        || crate::select::is_a(&name, "IFCHALFSPACESOLID")
        || crate::select::is_a(&name, "IFCCSGPRIMITIVE3D")
        || crate::select::is_a(&name, "IFCTESSELLATEDFACESET")
    {
        return Some(3);
    }
    if name == "IFCBOOLEANRESULT" || name == "IFCBOOLEANCLIPPINGRESULT" {
        // A boolean's dimensionality is its operands'.
        return entity
            .attributes
            .get(1)
            .and_then(|v| v.as_ref_id())
            .and_then(|first| operand_dim(model, first));
    }
    None
}
