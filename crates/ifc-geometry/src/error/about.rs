//! What a [`GeometryError`] is about: its entity and whether it is unsupported.

use ifc_model::EntityId;

use super::GeometryError;

impl GeometryError {
    /// The entity this error is about, when there is one.
    ///
    /// Lets a caller collect failures per element rather than aborting a whole
    /// file for one bad wall.
    pub fn entity(&self) -> Option<EntityId> {
        match self {
            Self::MissingEntity { referrer, .. } => Some(*referrer),
            Self::MissingAttribute { entity, .. }
            | Self::WrongValueKind { entity, .. }
            | Self::WrongEntityType { entity, .. }
            | Self::Unsupported { entity, .. }
            | Self::CyclicChain { entity, .. }
            | Self::ChainTooDeep { entity, .. }
            | Self::AggregateTooLarge { entity, .. }
            | Self::Degenerate { entity, .. } => Some(*entity),
            Self::GridAxesParallel { intersection, .. }
            | Self::GridAxesDoNotIntersect { intersection, .. }
            | Self::GridAxesInDifferentGrids { intersection, .. } => Some(*intersection),
            Self::GridAxisWithoutGrid { axis } => Some(*axis),
            Self::PlacementRelToConflict { placement, .. } => Some(*placement),
            #[cfg(feature = "compile")]
            Self::CompilationRefused { entity, .. } => Some(*entity),
            #[cfg(feature = "compile")]
            Self::NotASolid { entity, .. } => Some(*entity),
            #[cfg(feature = "compile")]
            Self::CachedPlacementMismatch { placement, .. } => Some(*placement),
            #[cfg(feature = "compile")]
            Self::SeamSideUnsupported { placement, .. } => Some(*placement),
            #[cfg(feature = "compile")]
            Self::CurvePathUnsupported { placement, .. } => Some(*placement),
            #[cfg(feature = "compile")]
            Self::PathPieceLengthUnstated { placement, .. }
            | Self::CurvePathOpenEndUnsupported { placement, .. }
            | Self::PathPointOffCurve { placement, .. }
            | Self::PathPointAmbiguous { placement, .. }
            | Self::CurvePathBoundUnsupported { placement, .. }
            | Self::PathJointDistancesUnsupported { placement, .. }
            | Self::PlacementSeamWindowRefused { placement, .. } => Some(*placement),
            #[cfg(feature = "compile")]
            Self::StationSeamWindowRefused {
                product, stations, ..
            } => Some(stations.first().copied().unwrap_or(*product)),
            #[cfg(feature = "compile")]
            Self::RunSeamWindowRefused { product, runs, .. } => {
                Some(runs.first().copied().unwrap_or(*product))
            }
            // The opening is what failed; `host` stays readable on the variant.
            Self::OpeningNotSubtracted { opening, .. } => Some(*opening),
            Self::LayerNotSplit { product, .. } => Some(*product),
            Self::LengthUnitUndeclared {
                project,
                assignment,
            } => assignment.or(*project),
            Self::AngleUnitUndeclared { entity } => Some(*entity),
            Self::UnitUnreadable { unit, .. } => Some(*unit),
            Self::UnitsAmbiguous { second, .. } => Some(*second),
            Self::UnitRecordDamaged { entity, .. } => Some(*entity),
            Self::Units(_)
            | Self::UnitsIncomplete { .. }
            | Self::InvalidAuthoredValue { .. }
            | Self::AuthoringSchemaUnbound { .. }
            | Self::AuthoringEntityNotInSchema { .. } => None,
        }
    }

    /// Is this "valid IFC we do not handle yet" rather than a broken file?
    ///
    /// Callers building a viewer usually want to skip and count these, while
    /// treating genuine corruption differently.
    pub fn is_unsupported(&self) -> bool {
        match self {
            Self::Unsupported { .. } => true,
            // Valid IFC this evaluator cannot read; one with seam sides can.
            #[cfg(feature = "compile")]
            Self::SeamSideUnsupported { .. } => true,
            // Valid IFC this evaluator cannot read; one with curve paths can.
            #[cfg(feature = "compile")]
            Self::CurvePathUnsupported { .. } => true,
            // Valid IFC whose path needs an execution provider, or an
            // evaluator with open ends, piece bounds or certified joints
            // (#427, #436). A trim point off its curve, or one it passes
            // through twice, is the file failing to name one place.
            #[cfg(feature = "compile")]
            Self::PathPieceLengthUnstated { .. }
            | Self::CurvePathOpenEndUnsupported { .. }
            | Self::CurvePathBoundUnsupported { .. }
            | Self::PathJointDistancesUnsupported { .. } => true,
            // A seam the kernel cannot certify is valid IFC it cannot read;
            // two seams within precision, or a seam on its edge, is the
            // file's precision failing to name one place.
            #[cfg(feature = "compile")]
            Self::StationSeamWindowRefused { refusal, .. }
            | Self::PlacementSeamWindowRefused { refusal, .. }
            | Self::RunSeamWindowRefused { refusal, .. } => matches!(
                refusal,
                crate::compile::SeamWindowRefusal::UncertifiedLength
                    | crate::compile::SeamWindowRefusal::CertificationBudgetExceeded
            ),
            // A net refusal is as supported as the reason behind it.
            Self::OpeningNotSubtracted { cause, .. } | Self::LayerNotSplit { cause, .. } => {
                cause.is_unsupported()
            }
            _ => false,
        }
    }
}
