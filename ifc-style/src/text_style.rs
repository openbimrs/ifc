//! Text presentation style projections.

use ifc_model::{EntityId, Value};

use crate::error::StyleResult;
use crate::view::Record;

/// Borrowed projection of `IfcTextStyle`.
#[derive(Debug, Clone, Copy)]
pub struct TextStyle<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> TextStyle<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The entity id of this `IfcTextStyle`.
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The `Name` attribute, when authored.
    pub fn name(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    /// The `TextCharacterAppearance` attribute, when authored.
    pub fn text_character_appearance(&self) -> StyleResult<Option<EntityId>> {
        self.record
            .optional_ref("TextCharacterAppearance", "IfcTextStyleForDefinedFont")
    }

    /// The `TextStyle` attribute (`IfcTextStyleSelect`: model or box-characteristics text
    /// style), when authored.
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

    /// The `TextFontStyle` attribute (`IfcTextFontSelect`: externally defined
    /// or predefined text font). Mandatory.
    pub fn text_font_style(&self) -> StyleResult<EntityId> {
        self.record.required_ref_select(
            "TextFontStyle",
            "IfcTextFontSelect",
            &["IfcExternallyDefinedTextFont", "IfcPreDefinedTextFont"],
        )
    }

    /// The `ModelOrDraughting` flag: `true` for model-space geometry,
    /// `false` for a draughting/annotation text style, `None` if unauthored.
    pub fn model_or_draughting(&self) -> StyleResult<Option<bool>> {
        self.record.optional_bool("ModelOrDraughting")
    }
}

/// Borrowed projection of `IfcTextStyleFontModel`.
#[derive(Debug, Clone, Copy)]
pub struct TextStyleFontModel<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> TextStyleFontModel<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The `Name` attribute. Mandatory.
    pub fn name(&self) -> StyleResult<&'m str> {
        self.record.required_text("Name")
    }

    /// The `FontFamily` attribute (a list of candidate family names), when authored.
    pub fn font_family(&self) -> StyleResult<Option<&'m Value>> {
        self.record.optional_raw("FontFamily")
    }

    /// The `FontStyle` attribute (e.g. normal/italic/oblique), when authored.
    pub fn font_style(&self) -> StyleResult<Option<&'m Value>> {
        self.record.optional_raw("FontStyle")
    }

    /// The `FontVariant` attribute (e.g. normal/small-caps), when authored.
    pub fn font_variant(&self) -> StyleResult<Option<&'m Value>> {
        self.record.optional_raw("FontVariant")
    }

    /// The `FontWeight` attribute, when authored.
    pub fn font_weight(&self) -> StyleResult<Option<&'m Value>> {
        self.record.optional_raw("FontWeight")
    }

    /// The `FontSize` attribute (an `IfcSizeSelect`). Mandatory.
    pub fn font_size(&self) -> StyleResult<&'m Value> {
        self.record.value("FontSize")
    }
}
