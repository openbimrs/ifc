//! `IfcCompositeCurve` and its segments.
//!
//! # Why segments are entities, not just curve references
//!
//! A composite curve is a chain of `IfcCompositeCurveSegment`, and each segment
//! carries two facts the parent curve cannot: whether it is traversed forwards
//! or backwards, and what kind of continuity holds at its far end.
//!
//! **`SameSense = .F.` is common, not exotic.** Exporters reuse one
//! `IfcCircle` or `IfcPolyline` for several outlines and flip the sense rather
//! than emitting a mirrored copy. A consumer that ignores `SameSense` produces
//! a chain whose segments do not meet: each reversed segment starts where its
//! neighbour also starts. The failure looks like a tolerance problem and is
//! not one.
//!
//! **`Transition` is a promise about the joint**, checked by nobody. The last
//! segment of a closed composite curve must say `DISCONTINUOUS` only if the
//! curve is open; a closed curve repeats the continuity of the first joint.
//! The code carries useful information for a kernel deciding whether it may
//! fuse two segments into one edge, so it is exposed rather than discarded.

use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId};

/// `IfcCompositeCurve` attribute slots.
///
/// From IFC4 ADD2 TC1. `IfcCompositeCurveOnSurface`, `IfcBoundaryCurve` and
/// `IfcOuterBoundaryCurve` are subtypes that add no explicit attributes, so
/// these same indices apply to all four.
mod curve_slot {
    /// `Segments`: `LIST [1:?] OF IfcCompositeCurveSegment`.
    pub const SEGMENTS: usize = 0;
    /// `SelfIntersect`: `IfcLogical`, informational only.
    pub const SELF_INTERSECT: usize = 1;
}

/// `IfcCompositeCurveSegment` attribute slots.
///
/// From IFC4 ADD2 TC1. `IfcReparametrisedCompositeCurveSegment` adds
/// `ParamLength` at slot 3 and inherits these three unchanged.
mod segment_slot {
    /// `Transition`: `IfcTransitionCode`.
    pub const TRANSITION: usize = 0;
    /// `SameSense`: `IfcBoolean`.
    pub const SAME_SENSE: usize = 1;
    /// `ParentCurve`: the `IfcCurve` this segment is a piece of.
    pub const PARENT_CURVE: usize = 2;
    /// `ParamLength` on `IfcReparametrisedCompositeCurveSegment`.
    pub const PARAM_LENGTH: usize = 3;
}

/// `IfcTransitionCode`: what holds where one segment meets the next.
///
/// The values are ordered from weakest to strongest guarantee, and each
/// implies all the weaker ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionCode {
    /// The segments do not even meet; the curve has a gap at this joint.
    Discontinuous,
    /// The end point of this segment is the start point of the next.
    Continuous,
    /// Positions and tangent *directions* match.
    ///
    /// Tangent magnitude may still jump, so a kernel must not assume the
    /// parameterisation is smooth across the joint.
    ContSameGradient,
    /// Positions, tangent directions and curvature all match.
    ContSameGradientSameCurvature,
}

impl TransitionCode {
    /// Parse the enumeration token.
    ///
    /// Returns `None` for an unrecognised token; defaulting would claim a
    /// continuity the file never asserted.
    pub fn from_token(token: &str) -> Option<Self> {
        match token.to_ascii_uppercase().as_str() {
            "DISCONTINUOUS" => Some(Self::Discontinuous),
            "CONTINUOUS" => Some(Self::Continuous),
            "CONTSAMEGRADIENT" => Some(Self::ContSameGradient),
            "CONTSAMEGRADIENTSAMECURVATURE" => Some(Self::ContSameGradientSameCurvature),
            _ => None,
        }
    }

    /// Do the segments at this joint at least touch?
    ///
    /// The question a kernel actually asks before deciding whether the chain
    /// forms one connected wire.
    pub fn is_connected(&self) -> bool {
        !matches!(self, Self::Discontinuous)
    }
}

/// A borrowed view of an `IfcCompositeCurveSegment`.
///
/// Also covers `IfcReparametrisedCompositeCurveSegment`, whose extra
/// `ParamLength` is read by [`Self::param_length`].
#[derive(Debug, Clone, Copy)]
pub struct CompositeCurveSegment<'m> {
    slots: Slots<'m>,
}

impl<'m> CompositeCurveSegment<'m> {
    /// Wrap an entity known to be an `IfcCompositeCurveSegment` subtype.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The curve this segment is a piece of.
    ///
    /// Frequently shared with other segments and other composite curves, which
    /// is precisely why [`Self::same_sense`] exists.
    pub fn parent_curve_ref(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(segment_slot::PARENT_CURVE, "ParentCurve")
    }

    /// Is the segment traversed along the parent curve's own direction?
    ///
    /// `false` means reverse it before joining it to its neighbours.
    pub fn same_sense(&self) -> GeometryResult<bool> {
        self.slots.req_bool(segment_slot::SAME_SENSE, "SameSense")
    }

    /// The continuity asserted at the joint with the *next* segment.
    ///
    /// Note "next": the code describes the transition at the end of this
    /// segment, so the last segment of an open curve says `DISCONTINUOUS`.
    pub fn transition(&self) -> GeometryResult<TransitionCode> {
        let token = self
            .slots
            .opt_enum(segment_slot::TRANSITION)
            .ok_or_else(|| {
                self.slots
                    .degenerate("Transition is missing or is not an enumeration token")
            })?;
        TransitionCode::from_token(token).ok_or_else(|| {
            self.slots
                .degenerate(format!("unknown IfcTransitionCode .{token}."))
        })
    }

    /// `ParamLength`, present only on `IfcReparametrisedCompositeCurveSegment`.
    ///
    /// Returns `None` for a plain `IfcCompositeCurveSegment`. When present it
    /// rescales the segment's parameter range to `[0, ParamLength]`, so a
    /// consumer that ignores it will evaluate the parent curve at the wrong
    /// parameters. Values must be positive.
    pub fn param_length(&self) -> GeometryResult<Option<f64>> {
        let Some(value) = self.slots.opt_f64(segment_slot::PARAM_LENGTH) else {
            return Ok(None);
        };
        if value > 0.0 {
            Ok(Some(value))
        } else {
            Err(self
                .slots
                .degenerate(format!("ParamLength must be positive, found {value}")))
        }
    }

    /// Is this the reparametrised subtype?
    pub fn is_reparametrised(&self) -> bool {
        self.slots
            .entity()
            .type_name
            .eq_ignore_ascii_case("IFCREPARAMETRISEDCOMPOSITECURVESEGMENT")
    }
}

/// A borrowed view of an `IfcCompositeCurve`.
///
/// Also covers the subtypes that add no attributes:
/// `IfcCompositeCurveOnSurface`, `IfcBoundaryCurve`, `IfcOuterBoundaryCurve`.
#[derive(Debug, Clone, Copy)]
pub struct CompositeCurve<'m> {
    slots: Slots<'m>,
}

impl<'m> CompositeCurve<'m> {
    /// Wrap an entity known to be an `IfcCompositeCurve` subtype.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcCompositeCurveSegment` references, in traversal order.
    ///
    /// Order is significant and must not be sorted: the chain's shape is the
    /// list order combined with each segment's `SameSense`.
    pub fn segment_refs(&self) -> GeometryResult<Vec<EntityId>> {
        let segments = self.slots.req_ref_list(curve_slot::SEGMENTS, "Segments")?;
        if segments.is_empty() {
            return Err(self
                .slots
                .degenerate("Segments is empty; LIST [1:?] requires at least one segment"));
        }
        Ok(segments)
    }

    /// The informational `SelfIntersect` flag, `None` when absent or `.U.`.
    pub fn self_intersect(&self) -> Option<bool> {
        self.slots.opt_bool(curve_slot::SELF_INTERSECT)
    }

    /// Is this curve constrained to lie on a surface?
    ///
    /// True for `IfcCompositeCurveOnSurface` and its `IfcBoundaryCurve` /
    /// `IfcOuterBoundaryCurve` descendants, whose segments must all have
    /// `IfcPcurve` or surface-curve parents.
    pub fn is_on_surface(&self) -> bool {
        matches!(
            self.slots.entity().type_name.to_ascii_uppercase().as_str(),
            "IFCCOMPOSITECURVEONSURFACE" | "IFCBOUNDARYCURVE" | "IFCOUTERBOUNDARYCURVE"
        )
    }

    /// Is this the *outer* boundary of a surface?
    ///
    /// `IfcOuterBoundaryCurve` is the only way a file marks which of a
    /// surface's boundaries is the outline rather than a hole, and the
    /// distinction is carried by the entity type alone.
    pub fn is_outer_boundary(&self) -> bool {
        self.slots
            .entity()
            .type_name
            .eq_ignore_ascii_case("IFCOUTERBOUNDARYCURVE")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::Value;

    fn segment(transition: &str, same_sense: bool) -> Entity {
        Entity::new(
            "IFCCOMPOSITECURVESEGMENT",
            vec![
                Value::Enum(transition.into()),
                Value::Bool(same_sense),
                Value::Ref(EntityId(20)),
            ],
        )
    }

    fn composite(type_name: &str, segments: &[u64]) -> Entity {
        Entity::new(
            type_name,
            vec![
                Value::List(segments.iter().map(|i| Value::Ref(EntityId(*i))).collect()),
                Value::Bool(false),
            ],
        )
    }

    #[test]
    fn a_reversed_segment_reports_same_sense_false_rather_than_being_normalised() {
        let e = segment("CONTINUOUS", false);
        let view = CompositeCurveSegment::new(EntityId(1), &e);
        assert!(!view.same_sense().unwrap());
        assert_eq!(view.parent_curve_ref().unwrap(), EntityId(20));
    }

    #[test]
    fn every_transition_code_round_trips_from_its_file_token() {
        for (token, expected) in [
            ("DISCONTINUOUS", TransitionCode::Discontinuous),
            ("CONTINUOUS", TransitionCode::Continuous),
            ("CONTSAMEGRADIENT", TransitionCode::ContSameGradient),
            (
                "CONTSAMEGRADIENTSAMECURVATURE",
                TransitionCode::ContSameGradientSameCurvature,
            ),
        ] {
            let e = segment(token, true);
            assert_eq!(
                CompositeCurveSegment::new(EntityId(1), &e)
                    .transition()
                    .unwrap(),
                expected,
                "token {token}"
            );
        }
    }

    /// Only DISCONTINUOUS leaves a gap; the other three all mean the segments
    /// touch, which is the question a wire builder asks.
    #[test]
    fn only_discontinuous_reports_a_gap_at_the_joint() {
        assert!(!TransitionCode::Discontinuous.is_connected());
        assert!(TransitionCode::Continuous.is_connected());
        assert!(TransitionCode::ContSameGradient.is_connected());
        assert!(TransitionCode::ContSameGradientSameCurvature.is_connected());
    }

    #[test]
    fn an_unknown_transition_token_is_rejected_rather_than_defaulted() {
        assert_eq!(TransitionCode::from_token("SMOOTHISH"), None);
        let e = segment("SMOOTHISH", true);
        assert!(CompositeCurveSegment::new(EntityId(1), &e)
            .transition()
            .is_err());
    }

    #[test]
    fn param_length_is_absent_on_a_plain_segment_and_read_on_the_reparametrised_one() {
        let plain = segment("CONTINUOUS", true);
        let view = CompositeCurveSegment::new(EntityId(1), &plain);
        assert_eq!(view.param_length().unwrap(), None);
        assert!(!view.is_reparametrised());

        let reparam = Entity::new(
            "IFCREPARAMETRISEDCOMPOSITECURVESEGMENT",
            vec![
                Value::Enum("CONTINUOUS".into()),
                Value::Bool(true),
                Value::Ref(EntityId(20)),
                Value::Typed {
                    type_name: "IFCPARAMETERVALUE".into(),
                    value: Box::new(Value::Real(2.0)),
                },
            ],
        );
        let view = CompositeCurveSegment::new(EntityId(1), &reparam);
        assert_eq!(view.param_length().unwrap(), Some(2.0));
        assert!(view.is_reparametrised());
    }

    #[test]
    fn a_non_positive_param_length_is_degenerate() {
        let e = Entity::new(
            "IFCREPARAMETRISEDCOMPOSITECURVESEGMENT",
            vec![
                Value::Enum("CONTINUOUS".into()),
                Value::Bool(true),
                Value::Ref(EntityId(20)),
                Value::Real(0.0),
            ],
        );
        assert!(CompositeCurveSegment::new(EntityId(1), &e)
            .param_length()
            .is_err());
    }

    #[test]
    fn composite_curve_segments_keep_their_file_order() {
        let e = composite("IFCCOMPOSITECURVE", &[1, 2, 3]);
        assert_eq!(
            CompositeCurve::new(EntityId(1), &e).segment_refs().unwrap(),
            vec![EntityId(1), EntityId(2), EntityId(3)]
        );
    }

    #[test]
    fn a_composite_curve_with_no_segments_is_degenerate() {
        let e = composite("IFCCOMPOSITECURVE", &[]);
        assert!(CompositeCurve::new(EntityId(1), &e).segment_refs().is_err());
    }

    /// Whether a boundary is the outline or a hole is carried by the entity
    /// type alone, so classification cannot be skipped.
    #[test]
    fn boundary_curve_subtypes_are_classified_from_the_type_name() {
        let plain = composite("IFCCOMPOSITECURVE", &[1]);
        let on_surface = composite("IFCCOMPOSITECURVEONSURFACE", &[1]);
        let boundary = composite("IFCBOUNDARYCURVE", &[1]);
        let outer = composite("IFCOUTERBOUNDARYCURVE", &[1]);

        assert!(!CompositeCurve::new(EntityId(1), &plain).is_on_surface());
        assert!(CompositeCurve::new(EntityId(1), &on_surface).is_on_surface());
        assert!(CompositeCurve::new(EntityId(1), &boundary).is_on_surface());
        assert!(!CompositeCurve::new(EntityId(1), &boundary).is_outer_boundary());
        assert!(CompositeCurve::new(EntityId(1), &outer).is_outer_boundary());
    }
}
