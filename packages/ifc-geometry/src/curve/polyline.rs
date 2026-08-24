//! `IfcPolyline` and `IfcIndexedPolyCurve`: the two vertex-list curves.
//!
//! # Why they are in one module
//!
//! Both describe a chain of points, but they differ in a way that matters for
//! file size and for correctness. `IfcPolyline` holds a list of
//! `IfcCartesianPoint` *entities*, one STEP record per vertex. An
//! `IfcIndexedPolyCurve` holds a single `IfcCartesianPointList` plus integer
//! indices into it, which is how IFC4 made large surveyed geometry tractable.
//!
//! # The 1-based index trap
//!
//! `IfcLineIndex` and `IfcArcIndex` are lists of `IfcPositiveInteger`, indexing
//! the point list the way EXPRESS aggregates are indexed: **from 1**. Reading
//! them as Rust indices shifts every vertex by one and produces geometry that
//! looks almost right, which is the worst kind of wrong. This module never
//! exposes a raw index: [`PolySegment`] hands back zero-based indices already
//! converted and range-checked against the point count.

use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId, Value};

/// `IfcPolyline` attribute slots.
///
/// From IFC4 ADD2 TC1: `IfcBoundedCurve` and its supertypes declare no
/// explicit attributes, so `Points` is slot 0.
mod polyline_slot {
    /// `Points`: `LIST [2:?] OF IfcCartesianPoint`.
    pub const POINTS: usize = 0;
}

/// `IfcIndexedPolyCurve` attribute slots.
///
/// From IFC4 ADD2 TC1.
mod indexed_slot {
    /// `Points`: an `IfcCartesianPointList` (2D or 3D).
    pub const POINTS: usize = 0;
    /// `Segments`: `OPTIONAL LIST [1:?] OF IfcSegmentIndexSelect`.
    pub const SEGMENTS: usize = 1;
    /// `SelfIntersect`: `OPTIONAL IfcBoolean`, informational only.
    pub const SELF_INTERSECT: usize = 2;
}

/// A borrowed view of an `IfcPolyline`.
#[derive(Debug, Clone, Copy)]
pub struct Polyline<'m> {
    slots: Slots<'m>,
}

impl<'m> Polyline<'m> {
    /// Wrap an entity known to be an `IfcPolyline`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcCartesianPoint` references, in order.
    ///
    /// At least two are required by the schema; fewer is
    /// [`crate::GeometryError::Degenerate`] because a one-point polyline has
    /// no edge and would silently contribute nothing to a profile.
    // TODO: `resource::point` will provide a typed point view to resolve these.
    pub fn point_refs(&self) -> GeometryResult<Vec<EntityId>> {
        let points = self.slots.req_ref_list(polyline_slot::POINTS, "Points")?;
        if points.len() < 2 {
            return Err(self.slots.degenerate(format!(
                "Points must hold at least 2 entries, found {}",
                points.len()
            )));
        }
        Ok(points)
    }

    /// Is the last point the same entity as the first?
    ///
    /// IFC closes a polyline by *repeating the point reference*, not with a
    /// flag. A consumer building a face bound must know whether to add a
    /// closing edge, and comparing coordinates instead of references would
    /// need a tolerance the schema never defines.
    ///
    /// Note the converse is not guaranteed: a file may close a polyline with
    /// two distinct `IfcCartesianPoint` records holding identical
    /// coordinates. This reports only the reference-identity case, which is
    /// what conforming exporters emit.
    pub fn closes_by_repeating_first_point(&self) -> GeometryResult<bool> {
        let points = self.point_refs()?;
        Ok(points.first() == points.last())
    }
}

/// One segment of an `IfcIndexedPolyCurve`, with indices already zero-based.
///
/// The variants mirror `IfcSegmentIndexSelect`. Cardinality is enforced on
/// construction so a consumer never has to re-check that an arc really has
/// three points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolySegment {
    /// `IfcLineIndex`: a polyline run of 2 or more points.
    ///
    /// More than two indices means intermediate vertices, not a spline: every
    /// consecutive pair is a straight edge.
    Line(Vec<usize>),
    /// `IfcArcIndex`: exactly start, a point *on* the arc, and end.
    ///
    /// The middle index is a point the arc passes through, **not** a centre.
    /// Treating it as a centre is a common and silently plausible error.
    Arc {
        /// Index of the arc's start point.
        start: usize,
        /// Index of a point lying on the arc between start and end.
        mid: usize,
        /// Index of the arc's end point.
        end: usize,
    },
}

impl PolySegment {
    /// The zero-based point indices this segment touches, in order.
    pub fn indices(&self) -> Vec<usize> {
        match self {
            Self::Line(indices) => indices.clone(),
            Self::Arc { start, mid, end } => vec![*start, *mid, *end],
        }
    }

    /// Is this an arc segment?
    pub fn is_arc(&self) -> bool {
        matches!(self, Self::Arc { .. })
    }
}

/// A borrowed view of an `IfcIndexedPolyCurve`.
#[derive(Debug, Clone, Copy)]
pub struct IndexedPolyCurve<'m> {
    slots: Slots<'m>,
}

impl<'m> IndexedPolyCurve<'m> {
    /// Wrap an entity known to be an `IfcIndexedPolyCurve`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcCartesianPointList2D` or `IfcCartesianPointList3D` reference.
    // TODO: `resource::point` will provide a typed point-list view.
    pub fn points_ref(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(indexed_slot::POINTS, "Points")
    }

    /// The informational `SelfIntersect` flag, `None` when absent or `.U.`.
    pub fn self_intersect(&self) -> Option<bool> {
        self.slots.opt_bool(indexed_slot::SELF_INTERSECT)
    }

    /// Does the file supply an explicit segment list?
    ///
    /// When it does not, the curve is every point joined in list order by
    /// straight lines. That default is not stored anywhere, so a consumer
    /// that only looks at `Segments` will draw nothing for a perfectly valid
    /// curve.
    pub fn has_explicit_segments(&self) -> bool {
        self.slots.opt(indexed_slot::SEGMENTS).is_some()
    }

    /// The segments, with indices converted to zero-based and range-checked.
    ///
    /// `point_count` is the length of the referenced `IfcCartesianPointList`
    /// and is required because the index bound cannot be checked without it;
    /// an out-of-range index would otherwise reach a kernel as a panic.
    ///
    /// Returns an empty vector when `Segments` is absent. Use
    /// [`Self::has_explicit_segments`] to distinguish that from an explicitly
    /// empty list, and see [`Self::implicit_polyline_indices`] for the
    /// implied all-points-in-order curve.
    pub fn segments(&self, point_count: usize) -> GeometryResult<Vec<PolySegment>> {
        let Some(value) = self.slots.opt(indexed_slot::SEGMENTS) else {
            return Ok(Vec::new());
        };
        let items = value.as_list().ok_or_else(|| {
            self.slots
                .degenerate("Segments must be a list of IfcSegmentIndexSelect")
        })?;
        items
            .iter()
            .map(|item| self.parse_segment(item, point_count))
            .collect()
    }

    /// The zero-based indices implied when `Segments` is absent.
    ///
    /// Simply `0..point_count`, but naming it keeps the schema default in one
    /// place instead of re-derived at every call site.
    pub fn implicit_polyline_indices(point_count: usize) -> Vec<usize> {
        (0..point_count).collect()
    }

    fn parse_segment(&self, item: &Value, point_count: usize) -> GeometryResult<PolySegment> {
        // A conforming file writes `IFCLINEINDEX((1,2))`, so the select
        // arrives as a typed wrapper whose name is the only thing telling
        // line from arc. Some writers drop the wrapper on the outer list but
        // keep it here; nothing legitimate omits it entirely.
        let Value::Typed { type_name, value } = item else {
            return Err(self.slots.degenerate(
                "segment is not a typed IfcLineIndex/IfcArcIndex; the select tag is required \
                 to tell a line from an arc",
            ));
        };
        let raw = value.as_list().ok_or_else(|| {
            self.slots
                .degenerate(format!("{type_name} must wrap a list of indices"))
        })?;

        let mut indices = Vec::with_capacity(raw.len());
        for v in raw {
            let n = match v.unwrap_typed() {
                Value::Integer(i) => *i,
                other => {
                    return Err(self
                        .slots
                        .degenerate(format!("{type_name} index is not an integer: {other:?}")));
                }
            };
            // EXPRESS aggregates are 1-based. This subtraction is the entire
            // reason the raw indices are never exposed.
            if n < 1 {
                return Err(self.slots.degenerate(format!(
                    "{type_name} index {n} is not a positive integer; \
                     IfcPositiveInteger starts at 1"
                )));
            }
            let zero_based = (n - 1) as usize;
            if zero_based >= point_count {
                return Err(self.slots.degenerate(format!(
                    "{type_name} index {n} is out of range for a point list of {point_count}"
                )));
            }
            indices.push(zero_based);
        }

        match type_name.to_ascii_uppercase().as_str() {
            "IFCLINEINDEX" => {
                if indices.len() < 2 {
                    return Err(self.slots.degenerate(format!(
                        "IfcLineIndex needs at least 2 indices, found {}",
                        indices.len()
                    )));
                }
                Ok(PolySegment::Line(indices))
            }
            "IFCARCINDEX" => {
                if indices.len() != 3 {
                    return Err(self.slots.degenerate(format!(
                        "IfcArcIndex needs exactly 3 indices (start, on-arc, end), found {}",
                        indices.len()
                    )));
                }
                Ok(PolySegment::Arc {
                    start: indices[0],
                    mid: indices[1],
                    end: indices[2],
                })
            }
            other => Err(self
                .slots
                .degenerate(format!("{other} is not an IfcSegmentIndexSelect member"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refs(ids: &[u64]) -> Value {
        Value::List(ids.iter().map(|i| Value::Ref(EntityId(*i))).collect())
    }

    fn index_select(name: &str, indices: &[i64]) -> Value {
        Value::Typed {
            type_name: name.into(),
            value: Box::new(Value::List(
                indices.iter().map(|i| Value::Integer(*i)).collect(),
            )),
        }
    }

    fn indexed(segments: Value) -> Entity {
        Entity::new(
            "IFCINDEXEDPOLYCURVE",
            vec![Value::Ref(EntityId(100)), segments, Value::Bool(false)],
        )
    }

    #[test]
    fn polyline_reads_its_points_in_file_order() {
        let e = Entity::new("IFCPOLYLINE", vec![refs(&[1, 2, 3])]);
        let view = Polyline::new(EntityId(1), &e);
        assert_eq!(
            view.point_refs().unwrap(),
            vec![EntityId(1), EntityId(2), EntityId(3)]
        );
    }

    #[test]
    fn polyline_with_fewer_than_two_points_is_degenerate() {
        let e = Entity::new("IFCPOLYLINE", vec![refs(&[1])]);
        let err = Polyline::new(EntityId(4), &e).point_refs().unwrap_err();
        assert!(err.to_string().contains("#4"), "got: {err}");
    }

    /// Closure is expressed by repeating the first point reference, and a
    /// consumer must not add a duplicate closing edge when it is already there.
    #[test]
    fn polyline_closure_is_detected_from_a_repeated_first_reference() {
        let closed = Entity::new("IFCPOLYLINE", vec![refs(&[1, 2, 3, 1])]);
        let open = Entity::new("IFCPOLYLINE", vec![refs(&[1, 2, 3])]);
        assert!(Polyline::new(EntityId(1), &closed)
            .closes_by_repeating_first_point()
            .unwrap());
        assert!(!Polyline::new(EntityId(1), &open)
            .closes_by_repeating_first_point()
            .unwrap());
    }

    /// The single most damaging bug in this module would be an off-by-one, so
    /// it is pinned by an explicit expected value rather than a round trip.
    #[test]
    fn one_based_file_indices_become_zero_based_rust_indices() {
        let e = indexed(Value::List(vec![index_select("IFCLINEINDEX", &[1, 2, 3])]));
        let segments = IndexedPolyCurve::new(EntityId(1), &e).segments(5).unwrap();
        assert_eq!(segments, vec![PolySegment::Line(vec![0, 1, 2])]);
    }

    #[test]
    fn arc_index_names_its_middle_point_as_on_arc_not_as_a_centre() {
        let e = indexed(Value::List(vec![index_select("IFCARCINDEX", &[3, 4, 5])]));
        let segments = IndexedPolyCurve::new(EntityId(1), &e).segments(9).unwrap();
        assert_eq!(
            segments,
            vec![PolySegment::Arc {
                start: 2,
                mid: 3,
                end: 4
            }]
        );
        assert!(segments[0].is_arc());
    }

    #[test]
    fn an_index_past_the_end_of_the_point_list_is_rejected() {
        let e = indexed(Value::List(vec![index_select("IFCLINEINDEX", &[1, 9])]));
        let err = IndexedPolyCurve::new(EntityId(2), &e)
            .segments(3)
            .unwrap_err();
        assert!(err.to_string().contains("out of range"), "got: {err}");
    }

    /// Index 0 cannot be a valid IfcPositiveInteger, and accepting it would
    /// underflow the 1-based conversion.
    #[test]
    fn index_zero_is_rejected_rather_than_underflowing() {
        let e = indexed(Value::List(vec![index_select("IFCLINEINDEX", &[0, 1])]));
        assert!(IndexedPolyCurve::new(EntityId(1), &e).segments(3).is_err());
    }

    #[test]
    fn arc_index_with_the_wrong_cardinality_is_rejected() {
        let e = indexed(Value::List(vec![index_select("IFCARCINDEX", &[1, 2])]));
        let err = IndexedPolyCurve::new(EntityId(1), &e)
            .segments(5)
            .unwrap_err();
        assert!(err.to_string().contains("exactly 3"), "got: {err}");
    }

    #[test]
    fn line_index_needs_at_least_two_indices() {
        let e = indexed(Value::List(vec![index_select("IFCLINEINDEX", &[2])]));
        assert!(IndexedPolyCurve::new(EntityId(1), &e).segments(5).is_err());
    }

    /// Without the select tag there is no way to know whether three indices
    /// mean two straight edges or one arc, so guessing is not an option.
    #[test]
    fn an_untagged_segment_is_rejected_rather_than_assumed_to_be_a_line() {
        let e = indexed(Value::List(vec![Value::List(vec![
            Value::Integer(1),
            Value::Integer(2),
        ])]));
        let err = IndexedPolyCurve::new(EntityId(1), &e)
            .segments(5)
            .unwrap_err();
        assert!(err.to_string().contains("select tag"), "got: {err}");
    }

    /// Absent Segments means "all points in order", which is a real curve and
    /// not an empty one.
    #[test]
    fn absent_segments_is_distinguishable_from_an_empty_segment_list() {
        let absent = Entity::new("IFCINDEXEDPOLYCURVE", vec![Value::Ref(EntityId(100))]);
        let view = IndexedPolyCurve::new(EntityId(1), &absent);
        assert!(!view.has_explicit_segments());
        assert!(view.segments(4).unwrap().is_empty());
        assert_eq!(
            IndexedPolyCurve::implicit_polyline_indices(4),
            vec![0, 1, 2, 3]
        );

        let empty = indexed(Value::List(vec![]));
        assert!(IndexedPolyCurve::new(EntityId(1), &empty).has_explicit_segments());
    }

    #[test]
    fn mixed_line_and_arc_segments_keep_their_file_order() {
        let e = indexed(Value::List(vec![
            index_select("IFCLINEINDEX", &[1, 2]),
            index_select("IFCARCINDEX", &[2, 3, 4]),
            index_select("IFCLINEINDEX", &[4, 5]),
        ]));
        let segments = IndexedPolyCurve::new(EntityId(1), &e).segments(5).unwrap();
        assert_eq!(segments.len(), 3);
        assert!(!segments[0].is_arc());
        assert!(segments[1].is_arc());
        assert_eq!(segments[2].indices(), vec![3, 4]);
    }
}
