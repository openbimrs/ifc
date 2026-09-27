//! `IfcCurveStyle` projection.

use ifc_model::{EntityId, Value};

use crate::error::StyleResult;
use crate::view::Record;

/// Borrowed projection of `IfcCurveStyle`.
#[derive(Debug, Clone, Copy)]
pub struct CurveStyle<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> CurveStyle<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The entity id of this `IfcCurveStyle`.
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The `Name` attribute, when authored.
    pub fn name(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    /// The `CurveFont` attribute: a reference to an `IfcCurveStyleFont`, a
    /// predefined curve font name, or `None` if unauthored. Returned raw
    /// because the select is not narrowed to a single entity type here.
    pub fn curve_font(&self) -> StyleResult<Option<&'m Value>> {
        self.record.optional_raw("CurveFont")
    }

    /// The `CurveWidth` attribute (an `IfcSizeSelect`: a fixed measure or a
    /// positive-length select member), when authored.
    pub fn curve_width(&self) -> StyleResult<Option<&'m Value>> {
        self.record.optional_raw("CurveWidth")
    }

    /// The `CurveColour` attribute, resolved through the `IfcColour` select,
    /// when authored.
    pub fn curve_colour(&self) -> StyleResult<Option<EntityId>> {
        self.record.optional_ref_select(
            "CurveColour",
            "IfcColour",
            &["IfcColourSpecification", "IfcPreDefinedColour"],
        )
    }

    /// The `ModelOrDraughting` flag: `true` for model-space geometry,
    /// `false` for a draughting/annotation curve style, `None` if unauthored.
    pub fn model_or_draughting(&self) -> StyleResult<Option<bool>> {
        self.record.optional_bool("ModelOrDraughting")
    }
}

/// Borrowed projection of `IfcCurveStyleFont`: a named list of dash/gap
/// patterns applied along a styled curve.
#[derive(Debug, Clone, Copy)]
pub struct CurveStyleFont<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> CurveStyleFont<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The `Name` attribute, when authored.
    pub fn name(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    /// The `PatternList`: one or more `IfcCurveStyleFontPattern` references
    /// making up this font's repeating dash/gap sequence. At least one
    /// pattern is required by the schema.
    pub fn patterns(&self) -> StyleResult<Vec<EntityId>> {
        self.record
            .required_refs("PatternList", "IfcCurveStyleFontPattern", 1, None)
    }
}

/// Borrowed projection of `IfcCurveStyleFontPattern`: one visible/invisible
/// segment pair in a dashed curve font.
#[derive(Debug, Clone, Copy)]
pub struct CurveStyleFontPattern<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> CurveStyleFontPattern<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The `VisibleSegmentLength` attribute. Mandatory.
    pub fn visible_segment_length(&self) -> StyleResult<f64> {
        self.record.required_number("VisibleSegmentLength")
    }

    /// The `InvisibleSegmentLength` attribute. Mandatory.
    pub fn invisible_segment_length(&self) -> StyleResult<f64> {
        self.record.required_number("InvisibleSegmentLength")
    }
}
