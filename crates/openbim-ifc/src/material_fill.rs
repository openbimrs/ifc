//! A material's cut fill as hatch line families, in metres and radians (#597).
//!
//! `ifc-style` resolves a material's `IfcFillAreaStyle` and leaves the
//! project's length and plane angle units to the caller on purpose: the
//! `IfcUnitAssignment` is a unit question, owned by `ifc-properties`. The
//! two are siblings under ADR 0003, so the join lives in this orchestration
//! layer. It gates itself on `all(style, properties)` with an inner
//! `#![cfg]`, so the module declaration compiles to nothing without both.
//!
//! Nothing is guessed. A project that states no length or plane angle unit
//! leaves that factor undeclared, and only a hatch measure that needs it is
//! refused; a unit assignment that cannot be resolved exactly is refused.
#![cfg(all(feature = "style", feature = "properties"))]

use std::fmt;

use ifc_model::{EntityId, Model};
use ifc_properties::{exact_unit, ExactUnitError};
use ifc_schema::SchemaVersion;
use ifc_style::{ContextFill, MaterialFill, StyleError, StyleIndex, StyleUnits, StyleView};

/// Why a material's cut fill could not be resolved.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum MaterialFillError {
    /// The header does not declare exactly one release this build bundles.
    UnsupportedSchema {
        /// The declared schema tokens, comma-joined.
        schema: String,
    },
    /// `ifc-style` refused the fill, or a unit factor.
    Style(StyleError),
    /// The project's length or plane angle unit could not be resolved exactly.
    ProjectUnit {
        /// The measure whose unit was asked for.
        measure: &'static str,
        /// Why `ifc-properties` refused it.
        error: ExactUnitError,
    },
}

impl fmt::Display for MaterialFillError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchema { schema } => {
                write!(f, "schema {schema:?} is not one release this build bundles")
            }
            Self::Style(error) => write!(f, "{error}"),
            Self::ProjectUnit { measure, error } => {
                write!(f, "project unit of {measure}: {error}")
            }
        }
    }
}

impl std::error::Error for MaterialFillError {}

/// The project's metres per length unit and radians per plane angle unit,
/// each resolved exactly ([`exact_unit`] of `IFCLENGTHMEASURE` and
/// `IFCPLANEANGLEMEASURE`).
///
/// A model without an `IfcProject`, or whose project assigns no unit of
/// that type, leaves the factor `None`: [`ifc_style::StyleUnits`] refuses a
/// hatch measure that needs it. Call this once to resolve many materials
/// with [`ifc_style::StyleIndex::material_fill`].
///
/// # Errors
///
/// [`MaterialFillError::ProjectUnit`] when a unit exists but cannot be
/// resolved exactly (several projects, two units of one type, an unsupported
/// release, a malformed assignment); [`MaterialFillError::Style`] for a
/// factor that is not finite and positive.
pub fn project_style_units(model: &Model) -> Result<StyleUnits, MaterialFillError> {
    let factor = |measure: &'static str| match exact_unit(model, measure, None) {
        Ok(unit) => Ok(Some(unit.scale)),
        Err(ExactUnitError::NoProject | ExactUnitError::NoProjectUnit { .. }) => Ok(None),
        Err(error) => Err(MaterialFillError::ProjectUnit { measure, error }),
    };
    StyleUnits::new(factor("IFCLENGTHMEASURE")?, factor("IFCPLANEANGLEMEASURE")?)
        .map_err(MaterialFillError::Style)
}

/// Run `resolve` on a [`StyleIndex`] over `model`, with the project units.
fn with_index<T>(
    model: &Model,
    resolve: impl FnOnce(&StyleIndex<'_, '_>, StyleUnits) -> Result<T, StyleError>,
) -> Result<T, MaterialFillError> {
    let declared = &model.header().schema;
    let unsupported = || MaterialFillError::UnsupportedSchema {
        schema: declared.join(","),
    };
    let [token] = declared.as_slice() else {
        return Err(unsupported());
    };
    let version = SchemaVersion::from_header_token(token).ok_or_else(unsupported)?;
    let schema = ifc_schema::for_version(version).map_err(|_| unsupported())?;
    let index = StyleIndex::new(StyleView::new(model, schema)).map_err(MaterialFillError::Style)?;
    resolve(&index, project_style_units(model)?).map_err(MaterialFillError::Style)
}

/// The cut fill `material` states, as hatch line families in metres and
/// radians, or `None` when it states none ([`StyleIndex::material_fill`]).
///
/// The model's `FILE_SCHEMA` must pin one release this build bundles; the
/// project units come from [`project_style_units`]. A material that states
/// different fills per view is ambiguous here: use [`material_fills`].
///
/// # Errors
///
/// [`MaterialFillError::UnsupportedSchema`] for a header that does not;
/// [`MaterialFillError::ProjectUnit`] as [`project_style_units`];
/// [`MaterialFillError::Style`] for every refusal of the fill itself
/// (ambiguous, unsupported, a unit the project does not state, a malformed
/// style).
pub fn material_fill(
    model: &Model,
    material: EntityId,
) -> Result<Option<MaterialFill>, MaterialFillError> {
    with_index(model, |index, units| index.material_fill(material, units))
}

/// The fills `material` states per representation context, in metres and
/// radians ([`StyleIndex::material_fills`]): a cut fill in a `SECTION_VIEW`
/// sub-context and a projection fill in an `ELEVATION_VIEW` one, say.
///
/// # Errors
///
/// As [`material_fill`]; [`StyleError::AmbiguousFill`] is owned by the
/// representation or context whose fills differ.
pub fn material_fills(
    model: &Model,
    material: EntityId,
) -> Result<Vec<ContextFill>, MaterialFillError> {
    with_index(model, |index, units| index.material_fills(material, units))
}

/// The fill a bare `IfcFillAreaStyle` states, in metres and radians
/// ([`StyleIndex::fill_area_style`]), for a pattern nothing references.
///
/// # Errors
///
/// As [`material_fill`]; [`StyleError::ReferenceType`] when `style` is no
/// `IfcFillAreaStyle`.
pub fn fill_area_style(model: &Model, style: EntityId) -> Result<MaterialFill, MaterialFillError> {
    with_index(model, |index, units| index.fill_area_style(style, units))
}
