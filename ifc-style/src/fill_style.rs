//! Fill-area presentation styles and hatch/tile definitions.

use ifc_model::{EntityId, Value};

use crate::error::StyleResult;
use crate::view::Record;

#[derive(Debug, Clone, Copy)]
pub struct FillAreaStyle<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> FillAreaStyle<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn name(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    pub fn fill_styles(&self) -> StyleResult<Vec<EntityId>> {
        self.record.required_refs_select(
            "FillStyles",
            "IfcFillStyleSelect",
            &[
                "IfcColour",
                "IfcExternallyDefinedHatchStyle",
                "IfcFillAreaStyleHatching",
                "IfcFillAreaStyleTiles",
            ],
            1,
            None,
        )
    }

    pub fn model_or_draughting(&self) -> StyleResult<Option<bool>> {
        self.record.optional_bool("ModelOrDraughting")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FillAreaStyleHatching<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> FillAreaStyleHatching<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn hatch_line_appearance(&self) -> StyleResult<EntityId> {
        self.record
            .required_ref("HatchLineAppearance", "IfcCurveStyle")
    }

    pub fn start_of_next_hatch_line(&self) -> StyleResult<&'m Value> {
        self.record.value("StartOfNextHatchLine")
    }

    pub fn point_of_reference_hatch_line(&self) -> StyleResult<Option<EntityId>> {
        self.record
            .optional_ref("PointOfReferenceHatchLine", "IfcCartesianPoint")
    }

    pub fn pattern_start(&self) -> StyleResult<Option<EntityId>> {
        self.record
            .optional_ref("PatternStart", "IfcCartesianPoint")
    }

    pub fn hatch_line_angle(&self) -> StyleResult<f64> {
        self.record.required_number("HatchLineAngle")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FillAreaStyleTiles<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> FillAreaStyleTiles<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn tiling_pattern(&self) -> StyleResult<Vec<EntityId>> {
        self.record
            .required_refs("TilingPattern", "IfcVector", 2, Some(2))
    }
}
