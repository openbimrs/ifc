//! Unit conversion and a model's length unit (feature `properties`, #469).
//!
//! A unit is named as text, as `ifc::properties::ConversionUnit::parse`
//! reads it: an SI unit with its prefix written before it (`METRE`,
//! `MILLIMETRE`, `MILLI METRE`, `SQUARE_MILLIMETRE`, `KILOGRAM`; `METER`
//! spellings accepted), or a named conversion-based unit of IFC4 Table 673
//! with exact factors (`inch`, `foot`, `yard`, `mile`, `square foot`,
//! `gallon US`, `pound`, `degree` ...) or `US survey foot`.
//!
//! - [`convert_unit`] converts a value between two named units;
//!   [`IfcModel::convert_unit_value`] between two unit entities of the
//!   model (an `IfcSIUnit`, `IfcConversionBasedUnit` or `IfcDerivedUnit`).
//! - [`format_length`] writes a length as decimal text in a unit, or as
//!   feet and inches with fractions.
//! - [`IfcModel::convert_length_unit`] rescales every length-dimensioned
//!   value of the model to a new project length unit as one checked
//!   transaction (the facade's `convert_length_unit`), or with `dry_run`
//!   only reports. Its [`LengthUnitReport`] lists the rescaled attributes,
//!   the values kept with the reason, and the units written and removed.
//!
//! Refusals: `invalid-value` for text that names no unit, an unknown
//! prefix, a target that is not a length unit, or a bad format;
//! `type-mismatch` for units of different dimensions; `unsupported` for a
//! monetary or context-dependent unit, a unit with an offset (degree
//! Fahrenheit, `IfcConversionBasedUnitWithOffset`: not applied until #110
//! is settled), and values the conversion cannot carry (each named in the
//! detail); `invalid-model` when the project length unit cannot be
//! resolved; `unsupported-schema` for a release other than IFC2X3, IFC4 or
//! IFC4X3; `missing-entity` for a unit id the model lacks;
//! `feature-disabled` without `properties`.

use crate::record::{Field, Record, ToRecord};
use crate::{BindingError, IfcModel};

/// What [`IfcModel::convert_length_unit`] does beyond the length unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct LengthUnitOptions {
    /// Replace the area and volume units by the square and cube of the new
    /// length unit, rescaling their values (default `true`).
    pub area_and_volume: bool,
    /// Remove replaced units nothing else uses (default `true`).
    pub remove_replaced_units: bool,
    /// Report what would change without changing the model.
    pub dry_run: bool,
}

impl Default for LengthUnitOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl LengthUnitOptions {
    /// The defaults: areas and volumes follow, replaced units are removed,
    /// applied.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            area_and_volume: true,
            remove_replaced_units: true,
            dry_run: false,
        }
    }
}

/// Values of one attribute of one entity type that were rescaled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertedValues {
    /// The entity type, upper case.
    pub entity_type: String,
    /// The attribute.
    pub attribute: String,
    /// `length`, `area`, `volume`, `inverse-length`, `inverse-area` or
    /// `map-scale`.
    pub kind: String,
    /// Entities with a value rescaled here.
    pub entities: usize,
    /// Numbers rescaled.
    pub values: usize,
}

/// Length-like values of one attribute left as they are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptValues {
    /// The entity type, upper case.
    pub entity_type: String,
    /// The attribute.
    pub attribute: String,
    /// `explicit-unit`, `map-unit`, `unitless-parameter` or
    /// `unit-not-followed`.
    pub reason: String,
    /// Numbers kept.
    pub values: usize,
}

/// What a length-unit conversion did, or, for a dry run, would do.
#[derive(Debug, Clone, PartialEq)]
pub struct LengthUnitReport {
    /// The project length unit before, as named.
    pub from: String,
    /// The project length unit after.
    pub to: String,
    /// The factor lengths were multiplied by.
    pub length_factor: f64,
    /// The factor areas were multiplied by, when the area unit followed.
    pub area_factor: Option<f64>,
    /// The factor volumes were multiplied by, when the volume unit followed.
    pub volume_factor: Option<f64>,
    /// The new length unit entity; `None` when nothing changes.
    pub length_unit: Option<u64>,
    /// The new area unit entity.
    pub area_unit: Option<u64>,
    /// The new volume unit entity.
    pub volume_unit: Option<u64>,
    /// Rescaled values per entity type and attribute.
    pub converted: Vec<ConvertedValues>,
    /// Values left as they are, with the reason.
    pub kept: Vec<KeptValues>,
    /// Entities with at least one rescaled value.
    pub entities_changed: usize,
    /// Numbers rescaled.
    pub values_converted: usize,
    /// Entities created (empty for a dry run).
    pub created: Vec<u64>,
    /// Entities removed, or for a dry run planned for removal.
    pub removed: Vec<u64>,
    /// Whether the model was changed.
    pub applied: bool,
}

impl ToRecord for ConvertedValues {
    fn to_record(&self) -> Record {
        Record::new(
            "ConvertedValues",
            vec![
                ("entity_type", Field::Text(self.entity_type.clone())),
                ("attribute", Field::Text(self.attribute.clone())),
                ("kind", Field::Text(self.kind.clone())),
                ("entities", Field::Count(self.entities)),
                ("values", Field::Count(self.values)),
            ],
        )
    }
}

impl ToRecord for KeptValues {
    fn to_record(&self) -> Record {
        Record::new(
            "KeptValues",
            vec![
                ("entity_type", Field::Text(self.entity_type.clone())),
                ("attribute", Field::Text(self.attribute.clone())),
                ("reason", Field::Text(self.reason.clone())),
                ("values", Field::Count(self.values)),
            ],
        )
    }
}

impl ToRecord for LengthUnitReport {
    fn to_record(&self) -> Record {
        Record::new(
            "LengthUnitReport",
            vec![
                ("from", Field::Text(self.from.clone())),
                ("to", Field::Text(self.to.clone())),
                ("length_factor", Field::Real(self.length_factor)),
                (
                    "area_factor",
                    Field::optional(self.area_factor, Field::Real),
                ),
                (
                    "volume_factor",
                    Field::optional(self.volume_factor, Field::Real),
                ),
                ("length_unit", Field::id(self.length_unit)),
                ("area_unit", Field::id(self.area_unit)),
                ("volume_unit", Field::id(self.volume_unit)),
                ("converted", Field::records(&self.converted)),
                ("kept", Field::records(&self.kept)),
                ("entities_changed", Field::Count(self.entities_changed)),
                ("values_converted", Field::Count(self.values_converted)),
                ("created", Field::ids(self.created.iter().copied())),
                ("removed", Field::ids(self.removed.iter().copied())),
                ("applied", Field::Bool(self.applied)),
            ],
        )
    }
}

/// Convert `value` from the unit named `from` to the unit named `to`.
///
/// # Errors
///
/// See the module docs.
pub fn convert_unit(value: f64, from: &str, to: &str) -> Result<f64, BindingError> {
    #[cfg(feature = "properties")]
    {
        let from = imp::parse(from)?;
        let to = imp::parse(to)?;
        ifc::properties::convert_value(value, &from, &to).map_err(imp::error)
    }
    #[cfg(not(feature = "properties"))]
    {
        let _ = (value, from, to);
        Err(BindingError::FeatureDisabled("properties"))
    }
}

/// Write `value`, a length in the unit named `unit`, as text. `style` is
/// `decimal` (in `display_unit`, default `unit`, with `places` digits after
/// the point), `feet-inches` (rounded to `1/places` of an inch, `5' - 6
/// 1/2"`; with `suppress_zero_inches`, a whole foot is `5'`) or `inches`
/// (`66 1/2"`).
///
/// # Errors
///
/// See the module docs.
pub fn format_length(
    value: f64,
    unit: &str,
    style: &str,
    places: u32,
    display_unit: Option<&str>,
    suppress_zero_inches: bool,
) -> Result<String, BindingError> {
    #[cfg(feature = "properties")]
    {
        use ifc::properties::LengthFormat;
        let unit = imp::parse(unit)?;
        let format = match style {
            "decimal" => {
                let display = match display_unit {
                    Some(display) => imp::parse(display)?,
                    None => unit.clone(),
                };
                let places = u8::try_from(places)
                    .map_err(|_| BindingError::OutOfRange(format!("{places} decimal places")))?;
                LengthFormat::decimal(display, places)
            }
            "feet-inches" => LengthFormat::feet_inches(places, suppress_zero_inches),
            "inches" => LengthFormat::inches(places),
            other => {
                return Err(BindingError::InvalidValue(format!(
                    "{other:?} is not a length style: decimal, feet-inches or inches"
                )))
            }
        };
        ifc::properties::format_length(value, &unit, &format).map_err(imp::error)
    }
    #[cfg(not(feature = "properties"))]
    {
        let _ = (
            value,
            unit,
            style,
            places,
            display_unit,
            suppress_zero_inches,
        );
        Err(BindingError::FeatureDisabled("properties"))
    }
}

impl IfcModel {
    /// Convert `value` from unit entity `from` to unit entity `to` of this
    /// model.
    ///
    /// # Errors
    ///
    /// See the module docs.
    pub fn convert_unit_value(&self, value: f64, from: u64, to: u64) -> Result<f64, BindingError> {
        #[cfg(feature = "properties")]
        {
            for id in [from, to] {
                if self.inner.get(ifc::EntityId(id)).is_none() {
                    return Err(BindingError::MissingEntity(id));
                }
            }
            ifc::properties::convert_between(
                &self.inner,
                value,
                ifc::EntityId(from),
                ifc::EntityId(to),
            )
            .map_err(imp::error)
        }
        #[cfg(not(feature = "properties"))]
        {
            let _ = (value, from, to);
            Err(BindingError::FeatureDisabled("properties"))
        }
    }

    /// Convert the model's length unit to the unit named `target` (an SI
    /// `METRE` with any prefix, or `inch`, `foot`, `yard`, `mile`, `US
    /// survey foot`), rescaling every length, area and volume value, as one
    /// checked transaction; with `options.dry_run`, only report.
    ///
    /// # Errors
    ///
    /// See the module docs.
    pub fn convert_length_unit(
        &mut self,
        target: &str,
        options: LengthUnitOptions,
    ) -> Result<LengthUnitReport, BindingError> {
        #[cfg(feature = "properties")]
        {
            imp::convert_length_unit(self, target, options)
        }
        #[cfg(not(feature = "properties"))]
        {
            let _ = (target, options);
            Err(BindingError::FeatureDisabled("properties"))
        }
    }
}

#[cfg(feature = "properties")]
mod imp {
    use ifc::properties::{ConversionUnit, UnitConversionError};
    use ifc::{ConvertedKind, KeptReason, LengthUnitError, RefusalReason};

    use super::{ConvertedValues, KeptValues, LengthUnitOptions, LengthUnitReport};
    use crate::{BindingError, IfcModel};

    pub(super) fn parse(spec: &str) -> Result<ConversionUnit, BindingError> {
        ConversionUnit::parse(spec).map_err(error)
    }

    pub(super) fn error(error: UnitConversionError) -> BindingError {
        match error {
            UnitConversionError::DimensionMismatch { .. } => {
                BindingError::TypeMismatch(error.to_string())
            }
            UnitConversionError::MonetaryUnit { .. }
            | UnitConversionError::ContextDependentUnit { .. }
            | UnitConversionError::OffsetUnit { .. } => {
                BindingError::Unsupported(error.to_string())
            }
            UnitConversionError::Resolve(_) => BindingError::InvalidModel(error.to_string()),
            _ => BindingError::InvalidValue(error.to_string()),
        }
    }

    fn kind(kind: ConvertedKind) -> &'static str {
        match kind {
            ConvertedKind::Length => "length",
            ConvertedKind::Area => "area",
            ConvertedKind::Volume => "volume",
            ConvertedKind::InverseLength => "inverse-length",
            ConvertedKind::InverseArea => "inverse-area",
            ConvertedKind::MapScale => "map-scale",
            _ => "other",
        }
    }

    fn reason(reason: KeptReason) -> &'static str {
        match reason {
            KeptReason::ExplicitUnit => "explicit-unit",
            KeptReason::MapUnit => "map-unit",
            KeptReason::UnitlessParameter => "unitless-parameter",
            KeptReason::UnitNotFollowed => "unit-not-followed",
            _ => "other",
        }
    }

    fn refusal(reason: &RefusalReason) -> String {
        match reason {
            RefusalReason::ParameterSpace {
                host,
                host_type,
                detail,
            } => format!("a parameter value on #{} {host_type}: {detail}", host.0),
            RefusalReason::ParameterSpaceCurve {
                surface,
                surface_type,
            } => format!(
                "a p-curve in the parameter space of #{} {surface_type}, which is not a plane",
                surface.0
            ),
            RefusalReason::PolynomialCoefficients => {
                "IfcPolynomialCurve coefficients, whose unit IFC4.3 does not state".into()
            }
            RefusalReason::TextureGeneratorParameters => {
                "IfcTextureCoordinateGenerator parameters, whose meaning depends on the mode".into()
            }
            RefusalReason::UntypedSelectValue { select } => {
                format!("an untyped number in a {select}")
            }
            RefusalReason::MapTarget { target_type } => {
                format!("a coordinate operation onto a {target_type}")
            }
            RefusalReason::AmbiguousUnit => "a value two owners give different units".into(),
            RefusalReason::NotInSchema => "an entity the release does not declare".into(),
            RefusalReason::Unresolved { detail } => detail.to_string(),
            RefusalReason::ContextUnits => "a context with units of its own".into(),
            other => format!("{other:?}"),
        }
    }

    fn length_error(error: LengthUnitError) -> BindingError {
        match error {
            LengthUnitError::Target { target, reason } => {
                BindingError::InvalidValue(format!("{target}: {reason}"))
            }
            LengthUnitError::Schema(error) => BindingError::UnsupportedSchema(error.to_string()),
            LengthUnitError::ProjectUnit { unit_type, error } => match error {
                UnitConversionError::Resolve(_) => {
                    BindingError::InvalidModel(format!("the project {unit_type}: {error}"))
                }
                other => super::imp::error(other),
            },
            LengthUnitError::Refused(refusals) => BindingError::Unsupported(format!(
                "{} value(s) cannot be converted: {}",
                refusals.len(),
                refusals
                    .iter()
                    .take(10)
                    .map(|r| {
                        let attribute = r
                            .attribute
                            .as_deref()
                            .map(|a| format!(".{a}"))
                            .unwrap_or_default();
                        format!(
                            "#{} {}{attribute}: {}",
                            r.entity.0,
                            r.entity_type,
                            refusal(&r.reason)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
            )),
            LengthUnitError::Authoring(error) => BindingError::InvalidValue(error.to_string()),
            LengthUnitError::Conflict(conflicts) => {
                BindingError::InvalidModel(format!("{} conflict(s)", conflicts.len()))
            }
            other => BindingError::InvalidModel(other.to_string()),
        }
    }

    pub(super) fn convert_length_unit(
        model: &mut IfcModel,
        target: &str,
        options: LengthUnitOptions,
    ) -> Result<LengthUnitReport, BindingError> {
        let target = parse(target)?;
        let facade = ifc::LengthUnitOptions::new()
            .area_and_volume(options.area_and_volume)
            .remove_replaced_units(options.remove_replaced_units);
        let report = if options.dry_run {
            ifc::stage_length_unit_conversion(&model.inner, &target, &facade)
                .map_err(length_error)?
                .report
        } else {
            ifc::convert_length_unit(&mut model.inner, &target, &facade).map_err(length_error)?
        };
        Ok(LengthUnitReport {
            from: report.from.name.to_string(),
            to: report.to.name.to_string(),
            length_factor: report.length.factor(),
            area_factor: report.area.map(|c| c.factor()),
            volume_factor: report.volume.map(|c| c.factor()),
            length_unit: report.length_unit.map(|id| id.0),
            area_unit: report.area_unit.map(|id| id.0),
            volume_unit: report.volume_unit.map(|id| id.0),
            converted: report
                .converted
                .iter()
                .map(|c| ConvertedValues {
                    entity_type: c.entity_type.to_string(),
                    attribute: c.attribute.to_string(),
                    kind: kind(c.kind).into(),
                    entities: c.entities,
                    values: c.values,
                })
                .collect(),
            kept: report
                .kept
                .iter()
                .map(|k| KeptValues {
                    entity_type: k.entity_type.to_string(),
                    attribute: k.attribute.to_string(),
                    reason: reason(k.reason).into(),
                    values: k.values,
                })
                .collect(),
            entities_changed: report.entities_changed,
            values_converted: report.values_converted,
            created: report.created.iter().map(|id| id.0).collect(),
            removed: report.removed.iter().map(|id| id.0).collect(),
            applied: report.applied,
        })
    }
}
