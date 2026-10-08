//! Ordering, continuity, and station-query assembly for `IfcAlignmentCant`.
//!
//! This is the "parent/layout curve assembly" `ALIGN-CANT` names: given the
//! parent `IfcAlignmentCant`, resolve its nested segments in authored order,
//! validate that each segment's start cant agrees with the previous
//! segment's end cant (C0 continuity -- cant profiles are not required to be
//! tangent-continuous the way horizontal/vertical curves are), and expose a
//! single station-domain query across the whole profile.

use ifc_model::{EntityId, Model};

use crate::cant::evaluate::{cant_within, CantAtStation};
use crate::cant::segment::{read_cant_segment, CantSegment, CantSegmentType};
use crate::curve::terminal::split_closing;
use crate::curve::SeamTolerance;
use crate::error::{AlignmentError, AlignmentResult};
use crate::horizontal::AlignmentUnits;
use crate::view::AlignmentView;

/// One resolved, ordered, continuity-checked cant profile.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct CantLayout {
    /// The `IfcAlignmentCant` entity this layout was resolved from.
    pub entity: EntityId,
    /// `RailHeadDistance`: track gauge used to convert cant to superelevation
    /// angle, in metres.
    pub rail_head_distance: f64,
    segments: Vec<CantSegment>,
    /// How far a Viennese bend's two rotation points may differ and still
    /// stay put: the model's declared precision, as the banked lowering
    /// compares them.
    pivot_tolerance: SeamTolerance,
}

impl CantLayout {
    /// Resolve `IfcAlignmentCant#entity`'s nested `IfcAlignmentSegment`
    /// chain into an ordered, continuity-checked cant profile.
    ///
    /// The zero-length segment IFC4.3 requires at the end of a layout is
    /// kept in [`Self::segments`]; its station and start cant are checked
    /// against the previous segment's end like any seam. A zero-length
    /// segment anywhere else, or as the only segment, is refused
    /// ([`AlignmentError::SemanticViolation`]).
    ///
    /// A layout with a `VIENNESEBEND` also reads the model's declared
    /// precision ([`SeamTolerance::for_model`]), the tolerance its rotation
    /// points are compared at, and refuses what that refuses.
    pub fn resolve(
        model: &Model,
        entity: EntityId,
        units: AlignmentUnits,
    ) -> AlignmentResult<Self> {
        let view = AlignmentView::for_model(model)?;
        let cant_entity = model
            .get(entity)
            .ok_or(AlignmentError::MissingEntity { entity })?;
        if !view.schema.is_a(&cant_entity.type_name, "IfcAlignmentCant") {
            return Err(AlignmentError::WrongType {
                entity,
                expected: "IfcAlignmentCant",
                actual: cant_entity.type_name.to_string(),
            });
        }
        // IFC4X3_ADD2: IfcAlignmentCant contributes exactly one attribute,
        // RailHeadDistance, past the inherited IfcRoot/IfcObject/IfcProduct/
        // IfcLinearElement slots.
        let rail_head_distance = cant_entity
            .attributes
            .last()
            .and_then(|value| value.unwrap_typed().as_f64())
            .ok_or(AlignmentError::InvalidAttribute {
                entity,
                index: cant_entity.attributes.len().saturating_sub(1),
                name: "RailHeadDistance",
            })?
            * units.length_to_metres;
        if !(rail_head_distance.is_finite() && rail_head_distance > 0.0) {
            return Err(AlignmentError::InvalidAttribute {
                entity,
                index: cant_entity.attributes.len().saturating_sub(1),
                name: "RailHeadDistance",
            });
        }

        let ids = view.segment_chain(entity, "IfcAlignmentCantSegment")?;
        if ids.is_empty() {
            return Err(AlignmentError::SemanticViolation {
                entity: Some(entity),
                rule: "IfcAlignmentCant must nest at least one IfcAlignmentSegment",
            });
        }
        let mut segments = Vec::with_capacity(ids.len());
        for id in ids {
            segments.push(read_cant_segment(model, id, units)?);
        }
        // IFC4.3 closes every layout with a zero-length segment; one
        // anywhere else is refused. The closing segment stays in the layout
        // and its start cant is checked below like any seam.
        split_closing(&segments, |s| s.horizontal_length, |s| s.entity)?;

        // Ordering by nesting order is authoritative (IFC4X3 does not use a
        // numeric sequence field here), but a malformed file could still
        // nest segments out of distance order or with gaps/overlaps; check
        // explicitly rather than trusting nesting order blindly.
        for pair in segments.windows(2) {
            let [previous, next] = pair else {
                unreachable!()
            };
            let previous_end = previous.start_dist_along + previous.horizontal_length;
            if (next.start_dist_along - previous_end).abs() > 1e-6 {
                return Err(AlignmentError::SemanticViolation {
                    entity: Some(next.entity),
                    rule: "cant segments must be contiguous in StartDistAlong order",
                });
            }
            let previous_end_left = previous.end_cant_left.unwrap_or(previous.start_cant_left);
            let previous_end_right = previous.end_cant_right.unwrap_or(previous.start_cant_right);
            if (previous_end_left - next.start_cant_left).abs() > 1e-9
                || (previous_end_right - next.start_cant_right).abs() > 1e-9
            {
                return Err(AlignmentError::SemanticViolation {
                    entity: Some(next.entity),
                    rule: "cant segments must be C0 continuous: end cant must equal the next segment's start cant",
                });
            }
        }

        // Only a Viennese bend compares rotation points, so only a layout
        // with one reads the precision; every other layout resolves as it
        // did before #312.
        let pivot_tolerance = if segments
            .iter()
            .any(|segment| segment.predefined_type == CantSegmentType::VienneseBend)
        {
            SeamTolerance::for_model(model, units)?
        } else {
            SeamTolerance::strict()
        };

        Ok(Self {
            entity,
            rail_head_distance,
            segments,
            pivot_tolerance,
        })
    }

    /// The cant layout of `IfcAlignment#alignment`.
    ///
    /// Unlike the gradient-curve composition, where a road without cant is
    /// ordinary, this is the call for work that NEEDS cant (the section
    /// frames, the `IfcSegmentedReferenceCurve` role), so its absence is a
    /// refusal rather than `None`.
    ///
    /// # Errors
    ///
    /// Refuses a model that is not IFC4X3, an entity that is not an
    /// `IfcAlignment`, an alignment nesting no `IfcAlignmentCant` or several
    /// of them (`SemanticViolation`: picking one would silently choose a
    /// track), and everything [`Self::resolve`] refuses.
    pub fn for_alignment(
        model: &Model,
        alignment: EntityId,
        units: AlignmentUnits,
    ) -> AlignmentResult<Self> {
        let view = AlignmentView::for_model(model)?;
        let entity = model
            .get(alignment)
            .ok_or(AlignmentError::MissingEntity { entity: alignment })?;
        if !view.schema.is_a(&entity.type_name, "IfcAlignment") {
            return Err(AlignmentError::WrongType {
                entity: alignment,
                expected: "IfcAlignment",
                actual: entity.type_name.to_string(),
            });
        }
        match view
            .nested_children(alignment, "IfcAlignmentCant")?
            .as_slice()
        {
            [only] => Self::resolve(model, *only, units),
            [] => Err(AlignmentError::SemanticViolation {
                entity: Some(alignment),
                rule: "the alignment nests no IfcAlignmentCant layout",
            }),
            _ => Err(AlignmentError::SemanticViolation {
                entity: Some(alignment),
                rule: "an alignment with several cant layouts is ambiguous to compose",
            }),
        }
    }

    /// Segments in authored (distance-along) order.
    #[must_use]
    pub fn segments(&self) -> &[CantSegment] {
        &self.segments
    }

    /// Total distance-along span covered by this profile.
    #[must_use]
    pub fn length(&self) -> f64 {
        self.segments
            .last()
            .map(|last| {
                last.start_dist_along + last.horizontal_length - self.segments[0].start_dist_along
            })
            .unwrap_or(0.0)
    }

    /// Cant at an absolute distance-along value within this profile's span.
    ///
    /// Each segment evaluates as [`cant_at`](crate::cant::cant_at) does; a
    /// `VIENNESEBEND`'s rotation points are compared at the model's declared
    /// precision, as the banked lowering compares them.
    ///
    /// # Errors
    ///
    /// Refuses a non-finite distance or one outside the profile's span, and
    /// everything [`cant_at`](crate::cant::cant_at) refuses, including the
    /// inside of a Viennese bend whose rotation point moves.
    pub fn cant_at_distance(&self, distance_along: f64) -> AlignmentResult<CantAtStation> {
        if !distance_along.is_finite() {
            return Err(AlignmentError::InvalidUnits {
                detail: "distance along must be finite",
            });
        }
        let segment = self
            .segments
            .iter()
            .find(|segment| {
                let start = segment.start_dist_along;
                let end = start + segment.horizontal_length;
                distance_along >= start - 1e-9 && distance_along <= end + 1e-9
            })
            .ok_or(AlignmentError::InvalidUnits {
                detail: "distance along lies outside the cant profile's span",
            })?;
        let xi = if segment.horizontal_length > 0.0 {
            ((distance_along - segment.start_dist_along) / segment.horizontal_length)
                .clamp(0.0, 1.0)
        } else {
            0.0
        };
        cant_within(
            segment,
            xi,
            Some(self.rail_head_distance),
            self.pivot_tolerance,
        )
    }
}
