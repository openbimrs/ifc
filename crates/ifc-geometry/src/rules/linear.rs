//! Where-rules on the linear-referencing geometry of IFC4X1 on.
//!
//! IFC4X1 introduced the sectioned solids and the triangulated irregular
//! network; IFC4X3 ADD2 added the linear placement, the polynomial curve
//! and the sectioned surface. None of them exists in IFC4 ADD2 TC1, so
//! their rules are outside the IFC4 inventory and are declared per release
//! in `rules/release.rs`.
//!
//! # Reading a three-valued rule
//!
//! A `WHERE` rule is violated only when it evaluates to FALSE; UNKNOWN
//! conforms. An attribute a referenced instance does not have
//! (`temp.Location.OffsetLongitudinal` on a point that is no
//! `IfcPointByDistanceExpression`) is indeterminate, `EXISTS` of it is
//! FALSE, and a comparison with it is UNKNOWN, which `QUERY` does not
//! count. Each rule below states which inputs it cannot decide and leaves
//! them unreported.

use ifc_model::{Entity, EntityId, Value};

use super::dimension::{dim_of, list_refs};
use super::release::Subject;
use super::violation::{RuleViolation, ViolationKind};

/// Run the linear-referencing rules that apply to this entity.
pub(crate) fn check(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    axis2_placement_linear(s, out);
    polynomial_curve(s, out);
    sectioned(s, out);
    tin_not_closed(s, out);
}

/// `IfcAxis2PlacementLinear`, IFC4X3 ADD2.
///
/// - `WR1 : 'IFC4X3_ADD2.IFCPOINTBYDISTANCEEXPRESSION' IN
///   TYPEOF(SELF\IfcPlacement.Location)`;
/// - `WR2 : (NOT (EXISTS (Axis))) OR (NOT (EXISTS (RefDirection))) OR
///   (IfcCrossProduct(Axis,RefDirection).Magnitude > 0.0)`, the text of
///   `IfcAxis2Placement3D.AxisToRefDirPosition`, read the same way
///   ([`super::placement::cross_product_vanishes`]).
///
/// Slots: Location (from `IfcPlacement`), Axis, RefDirection.
fn axis2_placement_linear(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    const TYPE: &str = "IFCAXIS2PLACEMENTLINEAR";
    if let Some(rule) = s.rule(TYPE, "WR1") {
        // An unset Location is TYPEOF(?): UNKNOWN, not a violation.
        if let Some(location) = slot_ref(s.entity, 0) {
            if s.ref_is_none_of(location, &["IFCPOINTBYDISTANCEEXPRESSION"]) {
                out.push(s.violation(
                    rule,
                    ViolationKind::WrongType,
                    format!("Location {location} must be an IfcPointByDistanceExpression"),
                ));
            }
        }
    }
    if let Some(rule) = s.rule(TYPE, "WR2") {
        if let (Some(axis), Some(ref_dir)) = (slot_ref(s.entity, 1), slot_ref(s.entity, 2)) {
            if super::placement::cross_product_vanishes(s.model, axis, ref_dir) {
                out.push(s.violation(
                    rule,
                    ViolationKind::Degenerate,
                    format!("Axis {axis} is parallel to RefDirection {ref_dir}"),
                ));
            }
        }
    }
}

/// `IfcPolynomialCurve`, IFC4X3 ADD2.
///
/// - `CorrectPositionDim : ((Position.Dim=2) AND (NOT EXISTS(CoefficientsZ)))
///   OR (Position.Dim=3)`: a 2D position admits no z coefficients. An
///   undecidable `Position.Dim` makes both disjuncts UNKNOWN at worst, so
///   only a known dimensionality can violate it.
/// - `ValidCoefficients`: at least two of `CoefficientsX`, `CoefficientsY`,
///   `CoefficientsZ` exist (the rule's fourth disjunct, all three, is
///   implied by each of the first three).
///
/// Slots: Position, CoefficientsX, CoefficientsY, CoefficientsZ.
fn polynomial_curve(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    const TYPE: &str = "IFCPOLYNOMIALCURVE";
    if let Some(rule) = s.rule(TYPE, "CorrectPositionDim") {
        let dim = slot_ref(s.entity, 0).and_then(|p| dim_of(s.release, s.model, p));
        let has_z = exists(s.entity, 3);
        match dim {
            Some(2) if has_z => out.push(s.violation(
                rule,
                ViolationKind::Dimensionality,
                "Position is 2D but CoefficientsZ is given",
            )),
            Some(dim) if dim != 2 && dim != 3 => out.push(s.violation(
                rule,
                ViolationKind::Dimensionality,
                format!("Position is {dim}D, must be 2D or 3D"),
            )),
            _ => {}
        }
    }
    if let Some(rule) = s.rule(TYPE, "ValidCoefficients") {
        let given = (1..=3).filter(|slot| exists(s.entity, *slot)).count();
        if given < 2 {
            out.push(s.violation(
                rule,
                ViolationKind::Disagreement,
                format!("{given} of CoefficientsX, CoefficientsY, CoefficientsZ given, need two"),
            ));
        }
    }
}

/// The sectioned solids (IFC4X1 on) and the sectioned surface (IFC4X3
/// ADD2).
///
/// `IfcSectionedSolid` (Directrix, CrossSections):
/// - `ConsistentProfileTypes` and `DirectrixIs3D`, read as
///   `IfcSectionedSpine`'s rules of the same text are
///   ([`super::curve::profile_types_agree`], [`super::curve::fixed_dim_ref`]);
/// - `SectionsSameType : SIZEOF(QUERY(temp <* CrossSections |
///   TYPEOF(CrossSections[1]) :<>: TYPEOF(temp))) = 0`.
///
/// `IfcSectionedSolidHorizontal` adds CrossSectionPositions (slot 2):
/// `CorrespondingSectionPositions` is checked with the other list-length
/// rules (`rules/cardinality.rs`); `NoLongitudinalOffsets` here.
///
/// `IfcSectionedSurface` (Directrix, CrossSectionPositions, CrossSections):
/// `AreaProfileTypes`, `DirectrixIs3D`, `NoOffsets`, `SectionsSameType`
/// here, `CorrespondingSectionPositions` in `rules/cardinality.rs`.
fn sectioned(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    const SOLID: &str = "IFCSECTIONEDSOLID";
    const SURFACE: &str = "IFCSECTIONEDSURFACE";
    if let Some(rule) = s.rule(SOLID, "ConsistentProfileTypes") {
        super::curve::profile_types_agree(s, 1, rule, out);
    }
    for declared_on in [SOLID, SURFACE] {
        if let Some(rule) = s.rule(declared_on, "DirectrixIs3D") {
            super::curve::fixed_dim_ref(s, 0, 3, rule, out);
        }
    }
    for (declared_on, sections) in [(SOLID, 1), (SURFACE, 2)] {
        if let Some(rule) = s.rule(declared_on, "SectionsSameType") {
            sections_same_type(s, sections, rule, out);
        }
    }
    if let Some(rule) = s.rule("IFCSECTIONEDSOLIDHORIZONTAL", "NoLongitudinalOffsets") {
        offsets(s, 2, &[3], rule, out);
    }
    if let Some(rule) = s.rule(SURFACE, "NoOffsets") {
        offsets(s, 1, &[1, 2, 3], rule, out);
    }
    if let Some(rule) = s.rule(SURFACE, "AreaProfileTypes") {
        area_profile_types(s, rule, out);
    }
}

/// `SectionsSameType`: every cross-section is an instance of the first
/// one's type.
///
/// `TYPEOF` of an entity instance is its type and every supertype, so two
/// `TYPEOF` sets are instance-equal exactly when the two instances share
/// one type. An unresolved section's `TYPEOF` is indeterminate and the
/// comparison UNKNOWN, which `QUERY` does not count.
fn sections_same_type(
    s: &Subject<'_>,
    slot: usize,
    rule: &'static str,
    out: &mut Vec<RuleViolation>,
) {
    let sections = list_refs(s.entity, slot);
    let type_of = |id: EntityId| s.model.get(id).map(|e| e.type_name.to_ascii_uppercase());
    let Some(first) = sections.first().and_then(|id| type_of(*id)) else {
        return;
    };
    if let Some((id, other)) = sections
        .iter()
        .skip(1)
        .find_map(|id| type_of(*id).filter(|t| *t != first).map(|t| (*id, t)))
    {
        out.push(s.violation(
            rule,
            ViolationKind::Disagreement,
            format!("cross-section {id} is {other}, but the first is {first}"),
        ));
    }
}

/// `NoLongitudinalOffsets` and `NoOffsets`: no cross-section position
/// sets an offset.
///
/// IFC4X1 and IFC4X2 type the positions as `IfcDistanceExpression` and
/// read `temp.OffsetLongitudinal` on the position itself; IFC4X3 ADD2 types
/// them as `IfcAxis2PlacementLinear` and reads `temp.Location.Offset...`,
/// the offsets of its `IfcPointByDistanceExpression`. Both carry the
/// offsets in the same slots: OffsetLateral 1, OffsetVertical 2,
/// OffsetLongitudinal 3. A position (or location) of another type has no
/// such attribute, so `EXISTS` of it is FALSE.
///
/// IFC4X1/IFC4X2 `NoLongitudinalOffsets : SIZEOF(QUERY(temp <*
/// CrossSectionPositions | EXISTS(temp.OffsetLongitudinal))) = 0`; IFC4X3
/// ADD2 `... EXISTS(temp.Location.OffsetLongitudinal))) = 0`, and
/// `NoOffsets` the same over all three offsets.
fn offsets(
    s: &Subject<'_>,
    positions_slot: usize,
    offset_slots: &[usize],
    rule: &'static str,
    out: &mut Vec<RuleViolation>,
) {
    let carrier = |position: EntityId| -> Option<EntityId> {
        if s.ref_is_a(position, "IFCDISTANCEEXPRESSION") {
            return Some(position);
        }
        if !s.ref_is_a(position, "IFCPLACEMENT") {
            return None;
        }
        let location = slot_ref(s.model.get(position)?, 0)?;
        s.ref_is_a(location, "IFCPOINTBYDISTANCEEXPRESSION")
            .then_some(location)
    };
    for position in list_refs(s.entity, positions_slot) {
        let Some(point) = carrier(position).and_then(|id| s.model.get(id)) else {
            continue;
        };
        if offset_slots.iter().any(|slot| exists(point, *slot)) {
            out.push(s.violation(
                rule,
                ViolationKind::Disagreement,
                format!("cross-section position {position} sets an offset the rule forbids"),
            ));
            return;
        }
    }
}

/// `IfcSectionedSurface.AreaProfileTypes : SIZEOF(QUERY(temp <*
/// CrossSections | temp.ProfileType = IfcProfileTypeEnum.CURVE)) <> 0`.
///
/// The label says area, the text demands at least one `CURVE` profile;
/// the text is normative and is what is checked. Reported only when every
/// section's `ProfileType` is readable: an unreadable one might be the
/// `CURVE` the rule asks for.
fn area_profile_types(s: &Subject<'_>, rule: &'static str, out: &mut Vec<RuleViolation>) {
    let mut kinds = Vec::new();
    for section in list_refs(s.entity, 2) {
        match s
            .model
            .get(section)
            .and_then(|e| e.attribute(0))
            .map(Value::unwrap_typed)
        {
            Some(Value::Enum(kind)) => kinds.push(kind.to_ascii_uppercase()),
            _ => return,
        }
    }
    if !kinds.is_empty() && !kinds.iter().any(|kind| kind == "CURVE") {
        out.push(s.violation(
            rule,
            ViolationKind::Disagreement,
            "no cross-section has ProfileType CURVE",
        ));
    }
}

/// `IfcTriangulatedIrregularNetwork.NotClosed : SELF\IfcTriangulatedFaceSet.
/// Closed = FALSE` (IFC4X1 on).
///
/// `Closed` is OPTIONAL (slot 2: Coordinates, Normals, Closed): an omitted
/// flag compares UNKNOWN and conforms, so only a written TRUE violates.
fn tin_not_closed(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    let Some(rule) = s.rule("IFCTRIANGULATEDIRREGULARNETWORK", "NotClosed") else {
        return;
    };
    if let Some(Value::Bool(true)) = s.entity.attribute(2).map(Value::unwrap_typed) {
        out.push(s.violation(
            rule,
            ViolationKind::Disagreement,
            "a triangulated irregular network declares Closed = TRUE",
        ));
    }
}

/// Entity reference held directly in a slot.
fn slot_ref(entity: &Entity, slot: usize) -> Option<EntityId> {
    match entity.attribute(slot)?.unwrap_typed() {
        Value::Ref(id) => Some(*id),
        _ => None,
    }
}

/// EXPRESS `EXISTS` of an OPTIONAL attribute: written, and not `$`.
fn exists(entity: &Entity, slot: usize) -> bool {
    !matches!(
        entity.attribute(slot).map(Value::unwrap_typed),
        None | Some(Value::Null)
    )
}
