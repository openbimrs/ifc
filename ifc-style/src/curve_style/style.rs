//! `IfcCurveStyle` projection.

use ifc_model::{EntityId, Value};

use crate::error::StyleResult;
use crate::view::Record;

#[derive(Debug, Clone, Copy)]
pub struct CurveStyle<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> CurveStyle<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn name(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    pub fn curve_font(&self) -> StyleResult<Option<&'m Value>> {
        self.record.optional_raw("CurveFont")
    }

    pub fn curve_width(&self) -> StyleResult<Option<&'m Value>> {
        self.record.optional_raw("CurveWidth")
    }

    pub fn curve_colour(&self) -> StyleResult<Option<EntityId>> {
        self.record.optional_ref_select(
            "CurveColour",
            "IfcColour",
            &["IfcColourSpecification", "IfcPreDefinedColour"],
        )
    }

    pub fn model_or_draughting(&self) -> StyleResult<Option<bool>> {
        self.record.optional_bool("ModelOrDraughting")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CurveStyleFont<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> CurveStyleFont<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn name(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    pub fn patterns(&self) -> StyleResult<Vec<EntityId>> {
        self.record
            .required_refs("PatternList", "IfcCurveStyleFontPattern", 1, None)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CurveStyleFontPattern<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> CurveStyleFontPattern<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn visible_segment_length(&self) -> StyleResult<f64> {
        self.record.required_number("VisibleSegmentLength")
    }

    pub fn invisible_segment_length(&self) -> StyleResult<f64> {
        self.record.required_number("InvisibleSegmentLength")
    }
}
