//! `IfcPlanarExtent` and its `IfcPlanarBox` subtype.
//!
//! These are `IfcGeometricRepresentationItem` subtypes that carry no shape:
//! they describe the 2D area a presentation element occupies. `ifc-geometry`
//! classifies both `non-shape` with `ifc-style` as owner, so the typed reads
//! live here rather than in a lowering path.

use ifc_model::EntityId;

use crate::error::StyleResult;
use crate::view::Record;

/// Borrowed projection of `IfcPlanarExtent`: a rectangular presentation area.
///
/// `SizeInX`/`SizeInY` are `IfcLengthMeasure`, which unlike the positive
/// measures elsewhere in this crate permits zero and negative values, so they
/// are read as plain numbers without a range guard.
#[derive(Debug, Clone, Copy)]
pub struct PlanarExtent<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> PlanarExtent<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The `SizeInX` attribute. Mandatory.
    pub fn size_in_x(&self) -> StyleResult<f64> {
        self.record.required_number("SizeInX")
    }

    /// The `SizeInY` attribute. Mandatory.
    pub fn size_in_y(&self) -> StyleResult<f64> {
        self.record.required_number("SizeInY")
    }
}

/// Borrowed projection of `IfcPlanarBox`: a planar extent plus the placement
/// that positions it.
#[derive(Debug, Clone, Copy)]
pub struct PlanarBox<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> PlanarBox<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The inherited `IfcPlanarExtent` attributes.
    #[must_use]
    pub fn planar_extent(&self) -> PlanarExtent<'m, 's> {
        PlanarExtent::from_record(self.record)
    }

    /// The `Placement` attribute. Mandatory.
    ///
    /// `IfcAxis2Placement` is a select over the 2D and 3D placements, so both
    /// are admitted; the id is returned for a geometry-side reader to resolve
    /// rather than interpreted here.
    pub fn placement(&self) -> StyleResult<EntityId> {
        self.record.required_ref_select(
            "Placement",
            "IfcAxis2Placement",
            &["IfcAxis2Placement2D", "IfcAxis2Placement3D"],
        )
    }
}
