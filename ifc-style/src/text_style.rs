//! Text presentation style projections.

use ifc_model::{EntityId, Value};

use crate::error::StyleResult;
use crate::view::Record;

#[derive(Debug, Clone, Copy)]
pub struct TextStyle<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> TextStyle<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn name(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    pub fn text_character_appearance(&self) -> StyleResult<Option<EntityId>> {
        self.record
            .optional_ref("TextCharacterAppearance", "IfcTextStyleForDefinedFont")
    }

    pub fn text_style(&self) -> StyleResult<Option<EntityId>> {
        self.record.optional_ref_select(
            "TextStyle",
            "IfcTextStyleSelect",
            &[
                "IfcTextStyleTextModel",
                "IfcTextStyleWithBoxCharacteristics",
            ],
        )
    }

    pub fn text_font_style(&self) -> StyleResult<EntityId> {
        self.record.required_ref_select(
            "TextFontStyle",
            "IfcTextFontSelect",
            &["IfcExternallyDefinedTextFont", "IfcPreDefinedTextFont"],
        )
    }

    pub fn model_or_draughting(&self) -> StyleResult<Option<bool>> {
        self.record.optional_bool("ModelOrDraughting")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TextStyleFontModel<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> TextStyleFontModel<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn name(&self) -> StyleResult<&'m str> {
        self.record.required_text("Name")
    }

    pub fn font_family(&self) -> StyleResult<Option<&'m Value>> {
        self.record.optional_raw("FontFamily")
    }

    pub fn font_style(&self) -> StyleResult<Option<&'m Value>> {
        self.record.optional_raw("FontStyle")
    }

    pub fn font_variant(&self) -> StyleResult<Option<&'m Value>> {
        self.record.optional_raw("FontVariant")
    }

    pub fn font_weight(&self) -> StyleResult<Option<&'m Value>> {
        self.record.optional_raw("FontWeight")
    }

    pub fn font_size(&self) -> StyleResult<&'m Value> {
        self.record.value("FontSize")
    }
}
