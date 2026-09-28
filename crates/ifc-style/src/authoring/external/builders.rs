//! Constructors and builder setters for the drafts of the parent module,
//! kept apart so the writer stays under the 800-line limit.

#[allow(clippy::wildcard_imports)]
use super::*;

impl<'a> BlobTextureDraft<'a> {
    /// Starts a draft from its required `raster_format`, `raster_code`; every
    /// other field is unset.
    #[must_use]
    pub fn new(raster_format: &'a str, raster_code: &'a str) -> Self {
        Self {
            repeat_s: false,
            repeat_t: false,
            mode: None,
            texture_transform: None,
            parameter: &[],
            raster_format,
            raster_code,
        }
    }

    /// Sets `repeat_s`.
    ///
    /// `RepeatS`: whether the texture tiles along S.
    #[must_use]
    pub fn repeat_s(mut self, value: bool) -> Self {
        self.repeat_s = value;
        self
    }

    /// Sets `repeat_t`.
    ///
    /// `RepeatT`: whether the texture tiles along T.
    #[must_use]
    pub fn repeat_t(mut self, value: bool) -> Self {
        self.repeat_t = value;
        self
    }

    /// Sets `mode`.
    ///
    /// `Mode`: the texture application mode.
    #[must_use]
    pub fn mode(mut self, value: &'a str) -> Self {
        self.mode = Some(value);
        self
    }

    /// Sets `texture_transform`.
    ///
    /// `TextureTransform`: a 2D operator applied to the coordinates.
    #[must_use]
    pub fn texture_transform(mut self, value: EntityId) -> Self {
        self.texture_transform = Some(value);
        self
    }

    /// Sets `parameter`.
    ///
    /// `Parameter`: optional, but non-empty when present.
    #[must_use]
    pub fn parameter(mut self, value: &'a [&'a str]) -> Self {
        self.parameter = value;
        self
    }
}

impl<'a> TextModelDraft<'a> {
    /// Starts a draft with every field unset.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets `text_indent`.
    ///
    /// `TextIndent`.
    #[must_use]
    pub fn text_indent(mut self, value: SizeValue<'a>) -> Self {
        self.text_indent = Some(value);
        self
    }

    /// Sets `text_align`.
    ///
    /// `TextAlign`, one of the closed lower-case alignment words.
    #[must_use]
    pub fn text_align(mut self, value: &'a str) -> Self {
        self.text_align = Some(value);
        self
    }

    /// Sets `text_decoration`.
    ///
    /// `TextDecoration`, one of the closed lower-case decoration words.
    #[must_use]
    pub fn text_decoration(mut self, value: &'a str) -> Self {
        self.text_decoration = Some(value);
        self
    }

    /// Sets `letter_spacing`.
    ///
    /// `LetterSpacing`.
    #[must_use]
    pub fn letter_spacing(mut self, value: SizeValue<'a>) -> Self {
        self.letter_spacing = Some(value);
        self
    }

    /// Sets `word_spacing`.
    ///
    /// `WordSpacing`.
    #[must_use]
    pub fn word_spacing(mut self, value: SizeValue<'a>) -> Self {
        self.word_spacing = Some(value);
        self
    }

    /// Sets `text_transform`.
    ///
    /// `TextTransform`, one of the closed lower-case transform words.
    #[must_use]
    pub fn text_transform(mut self, value: &'a str) -> Self {
        self.text_transform = Some(value);
        self
    }

    /// Sets `line_height`.
    ///
    /// `LineHeight`.
    #[must_use]
    pub fn line_height(mut self, value: SizeValue<'a>) -> Self {
        self.line_height = Some(value);
        self
    }
}
