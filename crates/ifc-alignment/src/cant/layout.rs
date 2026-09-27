//! Ordering, continuity, and station-query assembly for `IfcAlignmentCant`.
//!
//! This is the "parent/layout curve assembly" `ALIGN-CANT` names: given the
//! parent `IfcAlignmentCant`, resolve its nested segments in authored order,
//! validate that each segment's start cant agrees with the previous
//! segment's end cant (C0 continuity -- cant profiles are not required to be
//! tangent-continuous the way horizontal/vertical curves are), and expose a
//! single station-domain query across the whole profile.

use ifc_model::{EntityId, Model};

use crate::cant::evaluate::{cant_at, CantAtStation};
use crate::cant::segment::{read_cant_segment, CantSegment};
use crate::error::{AlignmentError, AlignmentResult};
use crate::horizontal::AlignmentUnits;
use crate::view::AlignmentView;

/// One resolved, ordered, continuity-checked cant profile.
#[derive(Debug, Clone, PartialEq)]
pub struct CantLayout {
    /// The `IfcAlignmentCant` entity this layout was resolved from.
    pub entity: EntityId,
    /// `RailHeadDistance`: track gauge used to convert cant to superelevation
    /// angle, in metres.
    pub rail_head_distance: f64,
    segments: Vec<CantSegment>,
}

impl CantLayout {
    /// Resolve `IfcAlignmentCant#entity`'s nested `IfcAlignmentSegment`
    /// chain into an ordered, continuity-checked cant profile.
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
            .and_then(|value| value.as_f64())
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

        Ok(Self {
            entity,
            rail_head_distance,
            segments,
        })
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
        cant_at(segment, xi, Some(self.rail_head_distance))
    }
}
