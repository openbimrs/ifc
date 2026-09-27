//! Colours and colour-or-factor values.
//!
//! RGB components are validated as normalized `[0, 1]` values. Rendering
//! selects preserve the distinction between a referenced IFC colour and a
//! scalar factor; missing or dangling references are typed errors.
//!
//! Colour-space conversion, profiles, and renderer-specific encodings remain
//! adapter concerns. This module only projects IFC presentation data.

mod rgb;
mod select;

pub use rgb::ColourRgb;
pub(crate) use select::optional_colour_or_factor;
pub use select::ColourOrFactor;
