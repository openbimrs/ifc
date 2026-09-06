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
    let upper = entity.type_name.to_ascii_uppercase();
    advanced_brep_faces(model, id, entity, &upper, out);
    boolean_operands(model, id, entity, &upper, out);
    match upper.as_str() {
        "IFCEXTRUDEDAREASOLID" | "IFCEXTRUDEDAREASOLIDTAPERED" => {
            extruded_area_solid(model, id, entity, out)
        }
        "IFCBOOLEANRESULT" | "IFCBOOLEANCLIPPINGRESULT" => boolean_result(model, id, entity, out),
        "IFCPOLYGONALBOUNDEDHALFSPACE" => polygonal_bounded_half_space(model, id, entity, out),
        "IFCREVOLVEDAREASOLID" | "IFCREVOLVEDAREASOLIDTAPERED" => {
            revolved_area_solid(model, id, entity, out)
        }
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
    // occupy slots 0 and 1. See ifc-geometry/data/absolute-slots.txt.
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
fn revolved_area_solid(model: &Model, id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    revolution_axis_in_xy(model, id, entity, out);

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

/// The `IfcBooleanClippingResult` restrictions.
///
/// `SameDim` is NOT checked here: it lives in `boolean_operands`, which
/// resolves operand dimensionality through `dimension::dim_of` and so reaches
/// curve operands too. An earlier copy of the rule lived here and used the
/// local `operand_dim`, which answered `Some(3)` for every family it
/// recognised and `None` otherwise -- making `da != db` unreachable. It was
/// dead code that still read as enforcement.
fn boolean_result(model: &Model, id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    let type_name = entity.type_name.to_ascii_uppercase();
    let first = entity.attributes.get(1).and_then(|v| v.as_ref_id());
    let second = entity.attributes.get(2).and_then(|v| v.as_ref_id());

    // IfcBooleanClippingResult additionally requires the operation to be
    // DIFFERENCE and the second operand to be a half space.
    if type_name == "IFCBOOLEANCLIPPINGRESULT" {
        if let Some(Value::Enum(op)) = entity.attributes.first() {
            if !op.eq_ignore_ascii_case("DIFFERENCE") {
                out.push(RuleViolation::new(
                    id,
                    type_name.clone(),
                    "OperatorType",
                    ViolationKind::WrongType,
                    format!("clipping must use DIFFERENCE, found {op}"),
                ));
            }
        }
        if let Some(a) = first {
            if let Some(e) = model.get(a) {
                let n = e.type_name.to_ascii_uppercase();
                // The schema names three admissible first operands: a swept
                // area, a swept disc, or another clipping result. Anything
                // else (a brep, a CSG primitive) makes the clip meaningless.
                let ok = crate::select::is_a(&n, "IFCSWEPTAREASOLID")
                    || crate::select::is_a(&n, "IFCSWEPTDISKSOLID")
                    || n == "IFCBOOLEANCLIPPINGRESULT";
                if !ok {
                    out.push(RuleViolation::new(
                        id,
                        type_name.clone(),
                        "FirstOperandType",
                        ViolationKind::WrongType,
                        format!(
                            "clipping requires a swept area, swept disc or nested \
                             clipping result as FirstOperand, found {}",
                            e.type_name
                        ),
                    ));
                }
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

/// `HasAdvancedFaces` and `VoidsHaveAdvancedFaces`.
///
/// Both demand that every face of a shell be an `IfcAdvancedFace`; they
/// differ only in which shells they walk. The traversal itself lives in
/// [`crate::solid::brep::non_advanced_faces`].
fn advanced_brep_faces(
    model: &Model,
    id: EntityId,
    entity: &Entity,
    name: &str,
    out: &mut Vec<RuleViolation>,
) {
    if !crate::select::is_a(name, "IFCADVANCEDBREP") {
        return;
    }

    // Outer is slot 0, inherited from IfcManifoldSolidBrep.
    if let Some(Value::Ref(outer)) = entity.attribute(0).map(|v| v.unwrap_typed()) {
        let plain = crate::solid::brep::non_advanced_faces(model, *outer);
        if let Some(face) = plain.first() {
            out.push(RuleViolation::new(
                id,
                name.to_string(),
                "HasAdvancedFaces",
                ViolationKind::WrongType,
                format!("the outer shell holds {face}, which is not an IfcAdvancedFace"),
            ));
        }
    }

    // Voids is slot 1 and exists only on IfcAdvancedBrepWithVoids.
    if !crate::select::is_a(name, "IFCADVANCEDBREPWITHVOIDS") {
        return;
    }
    for void in super::dimension::list_refs(entity, 1) {
        let plain = crate::solid::brep::non_advanced_faces(model, void);
        if let Some(face) = plain.first() {
            out.push(RuleViolation::new(
                id,
                name.to_string(),
                "VoidsHaveAdvancedFaces",
                ViolationKind::WrongType,
                format!("void shell {void} holds {face}, which is not an IfcAdvancedFace"),
            ));
            return;
        }
    }
}

/// The three `IfcBooleanResult` operand rules.
///
/// `SameDim` compares the operands' dimensionality. The two `*Closed`
/// rules apply only when an operand is an `IfcTessellatedFaceSet`: such a
/// set may bound a solid only if it declares itself closed.
fn boolean_operands(
    model: &Model,
    id: EntityId,
    entity: &Entity,
    name: &str,
    out: &mut Vec<RuleViolation>,
) {
    if !crate::select::is_a(name, "IFCBOOLEANRESULT") {
        return;
    }
    // Operator, FirstOperand, SecondOperand.
    let first = slot_ref(entity, 1);
    let second = slot_ref(entity, 2);

    if let (Some(a), Some(b)) = (first, second) {
        if let (Some(da), Some(db)) = (
            super::dimension::dim_of(model, a),
            super::dimension::dim_of(model, b),
        ) {
            if da != db {
                out.push(RuleViolation::new(
                    id,
                    name.to_string(),
                    "SameDim",
                    ViolationKind::Dimensionality,
                    format!("first operand is {da}D but second operand is {db}D"),
                ));
            }
        }
    }

    for (operand, rule) in [
        (first, "FirstOperandClosed"),
        (second, "SecondOperandClosed"),
    ] {
        let Some(operand) = operand else {
            continue;
        };
        let Some(target) = model.get(operand) else {
            continue;
        };
        if !crate::select::is_a(
            &target.type_name.to_ascii_uppercase(),
            "IFCTESSELLATEDFACESET",
        ) {
            continue;
        }
        // Closed sits at a different slot per subtype: after Normals on
        // IfcTriangulatedFaceSet, immediately after the inherited
        // Coordinates on IfcPolygonalFaceSet. It is OPTIONAL, and the rule
        // demands EXISTS(Closed) AND Closed, so an omitted flag violates
        // exactly as a false one does.
        let slot = match target.type_name.to_ascii_uppercase().as_str() {
            "IFCTRIANGULATEDFACESET" => 2,
            "IFCPOLYGONALFACESET" => 1,
            // An unknown subtype from a newer schema: no slot to trust.
            _ => continue,
        };
        let closed = match target.attribute(slot).map(|v| v.unwrap_typed()) {
            Some(Value::Bool(flag)) => Some(*flag),
            _ => None,
        };
        if closed != Some(true) {
            let detail = match closed {
                Some(false) => "declares Closed = FALSE",
                _ => "does not declare Closed",
            };
            out.push(RuleViolation::new(
                id,
                name.to_string(),
                rule,
                ViolationKind::Disagreement,
                format!("tessellated operand {operand} {detail}, so it cannot bound a solid"),
            ));
        }
    }
}

/// The entity referenced at `slot`, unwrapping any defined-type wrapper.
fn slot_ref(entity: &Entity, slot: usize) -> Option<EntityId> {
    match entity.attribute(slot).map(|v| v.unwrap_typed()) {
        Some(Value::Ref(id)) => Some(*id),
        _ => None,
    }
}

/// `AxisStartInXY` and `AxisDirectionInXY`.
///
/// A revolved area solid sweeps its profile about an axis that must lie in
/// the profile's own xy plane: the schema states this as the z component of
/// both the axis location and its direction being zero. An axis leaving
/// that plane would sweep the profile out of its own frame.
fn revolution_axis_in_xy(
    model: &Model,
    id: EntityId,
    entity: &Entity,
    out: &mut Vec<RuleViolation>,
) {
    // Axis is slot 2 and is an IfcAxis1Placement: Location, Axis.
    let Some(Value::Ref(axis)) = entity.attribute(2).map(|v| v.unwrap_typed()) else {
        return;
    };
    let Some(placement) = model.get(*axis) else {
        return;
    };
    let name = entity.type_name.to_ascii_uppercase();

    for (slot, rule, label) in [
        (0usize, "AxisStartInXY", "Location"),
        (1, "AxisDirectionInXY", "Z direction"),
    ] {
        let Some(Value::Ref(target)) = placement.attribute(slot).map(|v| v.unwrap_typed()) else {
            continue;
        };
        // Coordinates on a point, DirectionRatios on a direction: both are
        // the entity's only attribute, so one read serves both.
        let Some(coords) = model
            .get(*target)
            .and_then(|e| e.attribute(0).map(|v| v.unwrap_typed()))
        else {
            continue;
        };
        let Value::List(values) = coords else {
            continue;
        };
        // A 2D location or direction already lies in the plane; the rule
        // reads index 3, which such a value does not have.
        let Some(z) = values.get(2).and_then(|v| v.unwrap_typed().as_f64()) else {
            continue;
        };
        if z != 0.0 {
            out.push(RuleViolation::new(
                id,
                name.clone(),
                rule,
                ViolationKind::OutOfRange,
                format!("the revolution axis {label} has z = {z}, which must be 0"),
            ));
        }
    }
}
