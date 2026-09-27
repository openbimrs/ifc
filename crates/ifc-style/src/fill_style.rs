//! Fill-area presentation styles and hatch/tile definitions.

use ifc_model::{EntityId, Value};

use crate::error::StyleResult;
use crate::view::Record;

/// Borrowed projection of `IfcFillAreaStyle`.
#[derive(Debug, Clone, Copy)]
pub struct FillAreaStyle<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> FillAreaStyle<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The `Name` attribute, when authored.
    pub fn name(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    /// The `FillStyles` select: one or more colour, external, hatching, or
    /// tiling definitions making up this fill.
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

    /// The `ModelOrDraughting` flag: `true` for model-space geometry,
    /// `false` for a draughting/annotation fill style, `None` if unauthored.
    pub fn model_or_draughting(&self) -> StyleResult<Option<bool>> {
        self.record.optional_bool("ModelOrDraughting")
    }
}

/// Borrowed projection of `IfcFillAreaStyleHatching`: a parallel-line hatch pattern.
#[derive(Debug, Clone, Copy)]
pub struct FillAreaStyleHatching<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> FillAreaStyleHatching<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The `HatchLineAppearance` attribute: the curve style drawing each hatch line. Mandatory.
    pub fn hatch_line_appearance(&self) -> StyleResult<EntityId> {
        self.record
            .required_ref("HatchLineAppearance", "IfcCurveStyle")
    }

    /// The `StartOfNextHatchLine` attribute (an `IfcHatchLineDistanceSelect`),
    /// returned raw because the select is not narrowed to one representation here.
    pub fn start_of_next_hatch_line(&self) -> StyleResult<&'m Value> {
        self.record.value("StartOfNextHatchLine")
    }

    /// The `PointOfReferenceHatchLine` attribute, when authored.
    pub fn point_of_reference_hatch_line(&self) -> StyleResult<Option<EntityId>> {
        self.record
            .optional_ref("PointOfReferenceHatchLine", "IfcCartesianPoint")
    }

    /// The `PatternStart` attribute, when authored.
    pub fn pattern_start(&self) -> StyleResult<Option<EntityId>> {
        self.record
            .optional_ref("PatternStart", "IfcCartesianPoint")
    }

    /// The `HatchLineAngle` attribute. Mandatory.
    pub fn hatch_line_angle(&self) -> StyleResult<f64> {
        self.record.required_number("HatchLineAngle")
    }
}

/// Borrowed projection of `IfcFillAreaStyleTiles`: a repeating tile pattern.
#[derive(Debug, Clone, Copy)]
pub struct FillAreaStyleTiles<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> FillAreaStyleTiles<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The `TilingPattern` attribute: exactly two vectors spanning the tile's repeat lattice.
    pub fn tiling_pattern(&self) -> StyleResult<Vec<EntityId>> {
        self.record
            .required_refs("TilingPattern", "IfcVector", 2, Some(2))
    }
}
