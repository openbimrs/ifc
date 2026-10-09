//! Where-rules on solids and booleans.
//!
//! The rules here catch the two failures that waste the most time downstream:
//! an extrusion whose direction lies in the plane of its profile (produces a
//! zero-volume solid), and a boolean between operands of different
//! dimensionality (produces a kernel error with no useful location).

use super::release::Subject;
use super::{RuleViolation, ViolationKind};
use crate::resource::direction::Direction;
use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::SchemaVersion;

/// Run the solid rules that apply to this entity.
///
/// The rules are those of the release `model` declares in `FILE_SCHEMA`,
/// named as it names them.
pub fn check(model: &Model, id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    super::run_one(model, id, entity, out, run);
}

/// The solid rules for one entity in its release.
pub(crate) fn run(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    advanced_brep_faces(s, out);
    boolean_operands(s, out);
    extruded_area_solid(s, out);
    boolean_clipping_result(s, out);
    polygonal_bounded_half_space(s, out);
    revolved_area_solid(s, out);
}

/// `IfcExtrudedAreaSolid.ValidExtrusionDirection` (IFC2X3 TC1 `WR31`).
///
/// The schema states it as a dot product: the extrusion direction must not be
/// perpendicular to the z axis of the position coordinate system. Equivalently
/// the direction must not lie in the profile's plane -- sweeping a 2D profile
/// along a direction inside its own plane sweeps out no volume. The text is
/// the same in every bundled release.
fn extruded_area_solid(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    const TYPE: &str = "IFCEXTRUDEDAREASOLID";
    if !s.is_a(TYPE) {
        return;
    }
    let (model, entity) = (s.model, s.entity);

    // Slot 2 is ExtrudedDirection: SweptArea and Position are inherited and
    // occupy slots 0 and 1. See crates/ifc-geometry/data/absolute-slots.txt.
    let direction = entity
        .attributes
        .get(2)
        .and_then(|v| v.as_ref_id())
        .and_then(|dir_id| Some((dir_id, model.get(dir_id)?)))
        .and_then(|(dir_id, dir)| Some((dir_id, Direction::new(dir_id, dir).ratios().ok()?)));
    if let (Some(rule), Some((dir_id, ratios))) =
        (s.rule(TYPE, "ValidExtrusionDirection"), direction)
    {
        // The profile lies in the XY plane of the position system, so the z
        // component is the dot product with the plane normal.
        let z = ratios.get(2).copied().unwrap_or(0.0);
        let magnitude_sq: f64 = ratios.iter().map(|r| r * r).sum();
        // A zero direction is the IfcDirection rule's to report.
        if magnitude_sq > 0.0 && (z * z) / magnitude_sq < 1e-20 {
            out.push(s.violation(
                rule,
                ViolationKind::Degenerate,
                format!(
                    "ExtrudedDirection {dir_id} lies in the profile plane; \
                     the extrusion has zero volume"
                ),
            ));
        }
    }

    // Depth must be positive: IfcPositiveLengthMeasure. A type constraint
    // rather than an entity WHERE rule, so it is reported under the
    // attribute's name in every release.
    if let Some(depth) = entity
        .attributes
        .get(3)
        .and_then(|v| v.unwrap_typed().as_f64())
    {
        if depth <= 0.0 {
            out.push(s.violation(
                "Depth",
                ViolationKind::OutOfRange,
                format!("Depth is {depth}, must be a positive length"),
            ));
        }
    }
}

/// `IfcRevolvedAreaSolid.AxisStartInXY`/`AxisDirectionInXY`, and the
/// angle.
fn revolved_area_solid(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    if !s.is_a("IFCREVOLVEDAREASOLID") {
        return;
    }
    revolution_axis_in_xy(s, out);

    // Slot 3 is Angle; slots 0-1 inherited, slot 2 is Axis. No release
    // states this as a WHERE rule; it is kept under its historical name.
    if let Some(angle) = s
        .entity
        .attributes
        .get(3)
        .and_then(|v| v.unwrap_typed().as_f64())
    {
        if angle <= 0.0 {
            out.push(s.violation(
                "AngleGreaterZero",
                ViolationKind::OutOfRange,
                format!("Angle is {angle}, must be greater than zero"),
            ));
        }
    }
}

/// The `IfcBooleanClippingResult` restrictions, per release.
///
/// `SameDim` is NOT checked here: it lives in `boolean_operands`, which
/// resolves operand dimensionality through `dimension::dim_of` and so reaches
/// curve operands too.
///
/// `FirstOperandType` (IFC2X3 TC1 `WR1`) differs by release:
///
/// - IFC2X3 TC1: `('IFC2X3.IFCSWEPTAREASOLID' IN TYPEOF(FirstOperand)) OR
///   ('IFC2X3.IFCBOOLEANCLIPPINGRESULT' IN TYPEOF(FirstOperand))`;
/// - IFC4 ADD2 TC1, IFC4X1, IFC4X2 and IFC4X3 ADD2 add a third disjunct,
///   spelt `'IFC4.IFCSWEPTDISCSOLID' IN TYPEOF(FirstOperand)` (each under its
///   own schema prefix).
///
/// # The `IFCSWEPTDISCSOLID` typo
///
/// No release declares an entity `IfcSweptDiscSolid`, so read literally the
/// third disjunct is always FALSE and adds nothing to IFC2X3's rule. It is
/// read here as `IfcSweptDiskSolid`, the entity it evidently names:
/// buildingSMART tracks the spelling as a defect
/// (buildingSMART/IFC4.x-development#927, "IfcBooleanClippingResult has
/// spelling mistake in where rule", open) and the proposed correction
/// (buildingSMART/IFC4.x-development#1107, open against `ifc4.4-main`)
/// changes only the spelling, stating that it "now allows IfcSweptDiskSolid
/// as a valid type". A literal reading would flag every IFC4 file that
/// clips a swept disk, which the rule's authors intended to admit. IFC2X3
/// TC1 has no such disjunct, so there a swept disk is refused.
///
/// `SecondOperandType` (`WR2`) and `OperatorType` (`WR3`) read the same in
/// every bundled release.
fn boolean_clipping_result(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    const TYPE: &str = "IFCBOOLEANCLIPPINGRESULT";
    let entity = s.entity;
    let first = entity.attributes.get(1).and_then(|v| v.as_ref_id());
    let second = entity.attributes.get(2).and_then(|v| v.as_ref_id());

    if let Some(rule) = s.rule(TYPE, "OperatorType") {
        if let Some(Value::Enum(op)) = entity.attributes.first() {
            if !op.eq_ignore_ascii_case("DIFFERENCE") {
                out.push(s.violation(
                    rule,
                    ViolationKind::WrongType,
                    format!("clipping must use DIFFERENCE, found {op}"),
                ));
            }
        }
    }
    if let (Some(rule), Some(a)) = (s.rule(TYPE, "FirstOperandType"), first) {
        if let Some(e) = s.model.get(a) {
            let (admitted, names): (&[&str], _) = match s.release.version() {
                SchemaVersion::Ifc2x3 => (
                    &["IFCSWEPTAREASOLID", "IFCBOOLEANCLIPPINGRESULT"],
                    "a swept area or nested clipping result",
                ),
                _ => (
                    &[
                        "IFCSWEPTAREASOLID",
                        "IFCSWEPTDISKSOLID",
                        "IFCBOOLEANCLIPPINGRESULT",
                    ],
                    "a swept area, swept disk or nested clipping result",
                ),
            };
            if s.ref_is_none_of(a, admitted) {
                out.push(s.violation(
                    rule,
                    ViolationKind::WrongType,
                    format!(
                        "clipping requires {names} as FirstOperand in {}, found {}",
                        s.release.version().release_id(),
                        e.type_name
                    ),
                ));
            }
        }
    }
    if let (Some(rule), Some(b)) = (s.rule(TYPE, "SecondOperandType"), second) {
        if let Some(e) = s.model.get(b) {
            if s.ref_is_none_of(b, &["IFCHALFSPACESOLID"]) {
                out.push(s.violation(
                    rule,
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

/// `IfcPolygonalBoundedHalfSpace.BoundaryType`, in the model's own release.
///
/// `BoundaryDim` is checked with the other dimension rules. The admitted
/// boundary types differ by release:
///
/// - IFC4 ADD2 TC1: `SIZEOF(TYPEOF(PolygonalBoundary) *
///   ['IFC4.IFCPOLYLINE', 'IFC4.IFCCOMPOSITECURVE']) = 1`; IFC2X3 TC1
///   (`WR42`), IFC4X1 and IFC4X2 state the same two types under their own
///   schema prefix;
/// - IFC4X3 ADD2: `SIZEOF(TYPEOF(PolygonalBoundary) *
///   ['IFC4X3_ADD2.IFCPOLYLINE', 'IFC4X3_ADD2.IFCCOMPOSITECURVE',
///   'IFC4X3_ADD2.IFCINDEXEDPOLYCURVE']) = 1`.
///
/// `TYPEOF` carries every supertype, and the admitted types are disjoint,
/// so the rule holds exactly when the boundary is one of them or a subtype
/// of one (an `IfcBoundaryCurve` is an `IfcCompositeCurve`), judged in that
/// release's own entity table.
fn polygonal_bounded_half_space(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    let Some(rule) = s.rule("IFCPOLYGONALBOUNDEDHALFSPACE", "BoundaryType") else {
        return;
    };
    // Slot 3 is PolygonalBoundary: BaseSurface and AgreementFlag are
    // inherited (0, 1) and Position is slot 2.
    let Some(boundary) = s.entity.attributes.get(3).and_then(|v| v.as_ref_id()) else {
        return;
    };
    let Some(curve) = s.model.get(boundary) else {
        return;
    };
    let release = s.release.version();
    let admitted: &[&str] = match release {
        SchemaVersion::Ifc4x3 => &["IFCPOLYLINE", "IFCCOMPOSITECURVE", "IFCINDEXEDPOLYCURVE"],
        _ => &["IFCPOLYLINE", "IFCCOMPOSITECURVE"],
    };
    if s.ref_is_none_of(boundary, admitted) {
        let names = match release {
            SchemaVersion::Ifc4x3 => "IfcPolyline, IfcCompositeCurve or IfcIndexedPolyCurve",
            _ => "IfcPolyline or IfcCompositeCurve",
        };
        let name = curve.type_name.to_ascii_uppercase();
        out.push(s.violation(
            rule,
            ViolationKind::WrongType,
            format!(
                "PolygonalBoundary must be {names} in {}, found {name}",
                release.release_id()
            ),
        ));
    }
}

/// `HasAdvancedFaces` and `VoidsHaveAdvancedFaces`, IFC4 ADD2 TC1 on.
///
/// Both demand that every face of a shell be an `IfcAdvancedFace`
/// (`'IFCADVANCEDFACE' IN TYPEOF(Afs)`); they differ only in which shells
/// they walk. IFC2X3 TC1 has no advanced B-rep.
fn advanced_brep_faces(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    // Outer is slot 0, inherited from IfcManifoldSolidBrep.
    if let Some(rule) = s.rule("IFCADVANCEDBREP", "HasAdvancedFaces") {
        if let Some(Value::Ref(outer)) = s.entity.attribute(0).map(|v| v.unwrap_typed()) {
            if let Some(face) = non_advanced_faces(s, *outer).first() {
                out.push(s.violation(
                    rule,
                    ViolationKind::WrongType,
                    format!("the outer shell holds {face}, which is not an IfcAdvancedFace"),
                ));
            }
        }
    }

    // Voids is slot 1 and exists only on IfcAdvancedBrepWithVoids.
    let Some(rule) = s.rule("IFCADVANCEDBREPWITHVOIDS", "VoidsHaveAdvancedFaces") else {
        return;
    };
    for void in super::dimension::list_refs(s.entity, 1) {
        if let Some(face) = non_advanced_faces(s, void).first() {
            out.push(s.violation(
                rule,
                ViolationKind::WrongType,
                format!("void shell {void} holds {face}, which is not an IfcAdvancedFace"),
            ));
            return;
        }
    }
}

/// The faces of `shell` (`CfsFaces`, slot 0) that are not
/// `IfcAdvancedFace`s in the release. An unresolvable face is not evidence
/// of a non-advanced one.
fn non_advanced_faces(s: &Subject<'_>, shell: EntityId) -> Vec<EntityId> {
    let Some(entity) = s.model.get(shell) else {
        return Vec::new();
    };
    super::dimension::list_refs(entity, 0)
        .into_iter()
        .filter(|face| s.model.get(*face).is_some() && !s.ref_is_a(*face, "IFCADVANCEDFACE"))
        .collect()
}

/// The three `IfcBooleanResult` operand rules.
///
/// `SameDim` (IFC2X3 TC1 `WR1`) compares the operands' dimensionality. The
/// two `*Closed` rules, IFC4 ADD2 TC1 on, apply only when an operand is an
/// `IfcTessellatedFaceSet`: such a set may bound a solid only if it declares
/// itself closed. IFC2X3 TC1 declares neither.
fn boolean_operands(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    const TYPE: &str = "IFCBOOLEANRESULT";
    // Operator, FirstOperand, SecondOperand.
    let first = slot_ref(s.entity, 1);
    let second = slot_ref(s.entity, 2);

    if let (Some(rule), Some(a), Some(b)) = (s.rule(TYPE, "SameDim"), first, second) {
        if let (Some(da), Some(db)) = (
            super::dimension::dim_of(s.release, s.model, a),
            super::dimension::dim_of(s.release, s.model, b),
        ) {
            if da != db {
                out.push(s.violation(
                    rule,
                    ViolationKind::Dimensionality,
                    format!("first operand is {da}D but second operand is {db}D"),
                ));
            }
        }
    }

    for (operand, label) in [
        (first, "FirstOperandClosed"),
        (second, "SecondOperandClosed"),
    ] {
        let (Some(rule), Some(operand)) = (s.rule(TYPE, label), operand) else {
            continue;
        };
        let Some(target) = s.model.get(operand) else {
            continue;
        };
        // Closed sits at a different slot per subtype: after Normals on
        // IfcTriangulatedFaceSet (and its IFC4X1-on subtype
        // IfcTriangulatedIrregularNetwork), immediately after the inherited
        // Coordinates on IfcPolygonalFaceSet. It is OPTIONAL, and the rule
        // demands EXISTS(Closed) AND Closed, so an omitted flag violates
        // exactly as a false one does.
        let slot = if s.ref_is_a(operand, "IFCTRIANGULATEDFACESET") {
            2
        } else if s.ref_is_a(operand, "IFCPOLYGONALFACESET") {
            1
        } else {
            // Not a tessellated face set, or a subtype whose Closed slot
            // no bundled release fixes: nothing to read.
            continue;
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
            out.push(s.violation(
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

/// `AxisStartInXY` and `AxisDirectionInXY` (IFC2X3 TC1 `WR31`, `WR32`).
///
/// A revolved area solid sweeps its profile about an axis that must lie in
/// the profile's own xy plane: the schema states this as the z component of
/// both the axis location and its direction being zero. An axis leaving
/// that plane would sweep the profile out of its own frame.
///
/// `AxisStartInXY` differs by release:
///
/// - IFC2X3 TC1 to IFC4X2: `Axis.Location.Coordinates[3] = 0.0`, where
///   `Location` is typed `IfcCartesianPoint`;
/// - IFC4X3 ADD2 types `Location` as any `IfcPoint` and states
///   `('IFC4X3_ADD2.IFCCARTESIANPOINT' IN TYPEOF(Axis.Location)) AND
///   (Axis.Location\IfcCartesianPoint.Coordinates[3] = 0.0)`, so a
///   location that is not a Cartesian point violates it.
///
/// `AxisDirectionInXY` reads `Axis.Z.DirectionRatios[3] = 0.0` in every
/// bundled release.
fn revolution_axis_in_xy(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    const TYPE: &str = "IFCREVOLVEDAREASOLID";
    // Axis is slot 2 and is an IfcAxis1Placement: Location, Axis.
    let Some(Value::Ref(axis)) = s.entity.attribute(2).map(|v| v.unwrap_typed()) else {
        return;
    };
    let Some(placement) = s.model.get(*axis) else {
        return;
    };

    for (slot, label, what) in [
        (0usize, "AxisStartInXY", "Location"),
        (1, "AxisDirectionInXY", "Z direction"),
    ] {
        let Some(rule) = s.rule(TYPE, label) else {
            continue;
        };
        let Some(Value::Ref(target)) = placement.attribute(slot).map(|v| v.unwrap_typed()) else {
            continue;
        };
        let Some(target_entity) = s.model.get(*target) else {
            continue;
        };
        if slot == 0
            && s.release.version() == SchemaVersion::Ifc4x3
            && s.ref_is_none_of(*target, &["IFCCARTESIANPOINT"])
        {
            out.push(s.violation(
                rule,
                ViolationKind::WrongType,
                format!(
                    "the revolution axis Location {target} is {}, must be an \
                     IfcCartesianPoint in IFC4X3_ADD2",
                    target_entity.type_name.to_ascii_uppercase()
                ),
            ));
            continue;
        }
        // Coordinates on a point, DirectionRatios on a direction: both are
        // the entity's only attribute, so one read serves both.
        let Some(Value::List(values)) = target_entity.attribute(0).map(|v| v.unwrap_typed()) else {
            continue;
        };
        // A 2D location or direction already lies in the plane; the rule
        // reads index 3, which such a value does not have.
        let Some(z) = values.get(2).and_then(|v| v.unwrap_typed().as_f64()) else {
            continue;
        };
        if z != 0.0 {
            out.push(s.violation(
                rule,
                ViolationKind::OutOfRange,
                format!("the revolution axis {what} has z = {z}, which must be 0"),
            ));
        }
    }
}
