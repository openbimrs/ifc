//! Exact, fail-closed resolution of a measure's effective unit.
//!
//! A value's effective unit is its explicit `Unit` if stated, otherwise the
//! project default from `IfcProject.UnitsInContext`. This module resolves
//! either to an exact conversion into SI base units, or refuses. Every table
//! it reads belongs to the release `FILE_SCHEMA` declares.
//!
//! ## Internal split
//!
//! - `resolve.rs`: one unit entity to its SI scale and dimensions, following
//!   conversion chains and derived elements under a depth budget.

mod resolve;

use std::{fmt, sync::Arc};

use ifc_model::{EntityId, Model, Value};
use ifc_schema::{for_version, SchemaVersion};

use super::measure::{measure_unit, MeasureUnit};
use super::refs::{ref_at, refs_at};
use super::release::{validate_model, Release};
use super::value::select_accepts_entity;
use super::ExactPropertyError;
use resolve::{declared_unit_type, Resolver};

use crate::convert::{normalise_decimal, unit_conversion, ConversionUnit, UnitConversionError};

/// A measure's effective unit, resolved to SI base units.
///
/// `value_si = value * scale + offset`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct ExactUnit {
    /// The unit entity that applies, or `None` for a dimensionless measure
    /// (`IFCCOUNTMEASURE`, `IFCRATIOMEASURE`, ...) that takes no unit.
    pub unit: Option<EntityId>,
    /// Whether `unit` is the project default rather than an explicit unit.
    pub from_project: bool,
    /// SI dimensional exponents `[L, M, T, I, Θ, N, J]`.
    pub dimensions: [i32; 7],
    /// Multiplier into the SI base unit (kilogram for mass, kelvin for
    /// temperature, radian for plane angle).
    pub scale: f64,
    /// Added after scaling; nonzero only for `DEGREE_CELSIUS`.
    pub offset: f64,
}

/// Why a measure's unit cannot be resolved exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExactUnitError {
    /// The model, or a record the resolution traversed, failed a check that
    /// [`crate::exact_property`] applies the same way: diagnostics, header,
    /// missing references, slot arity, malformed aggregates, or a construct
    /// the declared release does not define.
    Structure(ExactPropertyError),
    /// The declared release defines no type of this name.
    MeasureNotInSchema {
        /// The measure type as requested.
        measure_type: Arc<str>,
        /// The release the header declares.
        schema: SchemaVersion,
    },
    /// The type is not a measure (`IFCLABEL`, `IFCBOOLEAN`, ...), so no unit
    /// can apply to it.
    NotAMeasure {
        /// The type as requested.
        measure_type: Arc<str>,
    },
    /// A measure this resolver has no verified unit correspondence for,
    /// such as a monetary, descriptive, logarithmic or list-valued measure.
    UnmappedMeasureType {
        /// The measure type as requested.
        measure_type: Arc<str>,
    },
    /// A dimensionless measure was given an explicit unit.
    UnexpectedUnit {
        /// The explicit unit.
        unit: EntityId,
    },
    /// The model has no `IfcProject` to take a default unit from.
    NoProject,
    /// The model has several `IfcProject`s, so no one project default applies.
    MultipleProjects {
        /// The first project.
        first: EntityId,
        /// The second project.
        second: EntityId,
    },
    /// No unit is stated and the project assigns none of the needed type.
    NoProjectUnit {
        /// The unit type the measure needs.
        unit_type: Arc<str>,
    },
    /// The project assigns two units of the needed type, which
    /// `IfcCorrectUnitAssignment` forbids; neither is "the" project unit.
    DuplicateProjectUnit {
        /// The unit type assigned twice.
        unit_type: Arc<str>,
        /// The first unit of that type.
        first: EntityId,
        /// The second unit of that type.
        second: EntityId,
    },
    /// The entity is not a unit this resolver converts: not an `IfcUnit` at
    /// all, or a context-dependent or monetary unit with no SI conversion.
    UnsupportedUnit {
        /// The rejected unit.
        unit: EntityId,
        /// Its IFC type name.
        type_name: Arc<str>,
    },
    /// A unit attribute that must be an enumeration constant, a positive
    /// number or a typed value is not.
    MalformedUnit {
        /// The unit entity or one of its parts.
        entity: EntityId,
        /// The offending attribute.
        attribute: &'static str,
    },
    /// An `IfcSIUnit` prefix that is not an `IfcSIPrefix` constant.
    UnknownPrefix {
        /// The unit.
        unit: EntityId,
        /// The prefix as written.
        prefix: Arc<str>,
    },
    /// An `IfcSIUnit` name that the release's `IfcDimensionsForSiUnit` does
    /// not list, or a unit type its `IfcCorrectDimensions` does not list
    /// (including `USERDEFINED`).
    UnknownUnitName {
        /// The unit.
        unit: EntityId,
        /// The name or unit type as written.
        name: Arc<str>,
    },
    /// The unit declares another unit type than the measure needs.
    UnitTypeMismatch {
        /// The unit.
        unit: EntityId,
        /// The unit type the measure needs.
        expected: Arc<str>,
        /// The unit type the unit declares.
        found: Arc<str>,
    },
    /// A unit's dimensions contradict its declared unit type, its declared
    /// `Dimensions`, or the unit its conversion factor is expressed in.
    DimensionMismatch {
        /// The unit.
        unit: EntityId,
        /// The dimensions required.
        expected: [i32; 7],
        /// The dimensions found.
        found: [i32; 7],
    },
    /// An offset unit: `IfcConversionBasedUnitWithOffset`, or a unit with an
    /// offset used where only a scale can apply (inside a derived unit or as
    /// a conversion factor). IFC4's prose on where the offset applies
    /// contradicts its own Fahrenheit example (factor 1.8,
    /// `f = k * 1.8 - 459.67`), so the offset is not applied until
    /// buildingSMART settles it (IFC4.x-development#1193; tracked in #110).
    UnsupportedOffset {
        /// The unit carrying the offset.
        unit: EntityId,
    },
    /// Conversion-based units form a cycle.
    CyclicConversion {
        /// The units on the cycle, closing on the first.
        cycle: Vec<EntityId>,
    },
    /// The chain of conversion and derived units is deeper than the budget.
    ConversionChainTooDeep {
        /// The unit at which the budget ran out.
        unit: EntityId,
        /// The depth budget.
        max_depth: usize,
    },
}

impl From<ExactPropertyError> for ExactUnitError {
    fn from(error: ExactPropertyError) -> Self {
        Self::Structure(error)
    }
}

impl fmt::Display for ExactUnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "exact IFC unit resolution failed: {self:?}")
    }
}

impl std::error::Error for ExactUnitError {}

/// Resolve the effective unit of a measure to an exact SI conversion.
///
/// `measure_type` is the value's declared type, e.g. `IFCAREAMEASURE`;
/// `explicit_unit` is `IfcPropertySingleValue.Unit` when stated. Without an
/// explicit unit the project default of the needed type applies. The model
/// binds to the release its `FILE_SCHEMA` declares, IFC2X3, IFC4 or IFC4X3,
/// and every table is that release's.
///
/// # Errors
///
/// Any [`ExactUnitError`]: the answer is refused whenever the unit is
/// missing, ambiguous, inconsistent, cyclic, or not convertible exactly.
pub fn exact_unit(
    model: &Model,
    measure_type: &str,
    explicit_unit: Option<EntityId>,
) -> Result<ExactUnit, ExactUnitError> {
    let release = validate_model(model)?;
    let target = measure_unit(release, measure_type)?;
    let Some(expected) = target.unit_type().cloned() else {
        if let Some(unit) = explicit_unit {
            return Err(ExactUnitError::UnexpectedUnit { unit });
        }
        return Ok(ExactUnit {
            unit: None,
            from_project: false,
            dimensions: [0; 7],
            scale: 1.0,
            offset: 0.0,
        });
    };
    let (unit, from_project) = match explicit_unit {
        Some(unit) => (unit, false),
        None => (project_unit(model, release, &expected)?, true),
    };
    let resolved = Resolver::new(model, release).resolve(unit)?;
    if !resolved.unit_type.eq_ignore_ascii_case(&expected) {
        return Err(ExactUnitError::UnitTypeMismatch {
            unit,
            expected,
            found: resolved.unit_type,
        });
    }
    Ok(ExactUnit {
        unit: Some(unit),
        from_project,
        dimensions: resolved.dimensions,
        scale: resolved.scale,
        offset: resolved.offset,
    })
}

/// The one unit of `unit_type` in the single project's unit assignment.
fn project_unit(
    model: &Model,
    release: Release,
    unit_type: &Arc<str>,
) -> Result<EntityId, ExactUnitError> {
    let project = match model.ids_of_type("IFCPROJECT") {
        [] => return Err(ExactUnitError::NoProject),
        [project] => *project,
        [first, second, ..] => {
            return Err(ExactUnitError::MultipleProjects {
                first: *first,
                second: *second,
            })
        }
    };
    let entity = model.get(project).expect("type index is current");
    release.require_exact_slots(project, entity)?;
    let slot = release
        .schema
        .attribute_names("IFCPROJECT")
        .iter()
        .position(|name| name.eq_ignore_ascii_case("UnitsInContext"))
        .expect("IfcProject declares UnitsInContext in every bundled release");
    let no_unit = || ExactUnitError::NoProjectUnit {
        unit_type: unit_type.clone(),
    };
    // Optional in IFC4, mandatory in IFC2X3; a `$` states no units either way.
    if matches!(entity.attributes.get(slot), Some(ifc_model::Value::Null)) {
        return Err(no_unit());
    }
    let assignment = ref_at(project, entity.attributes.get(slot), "UnitsInContext")?;
    let assignment_entity = model
        .get(assignment)
        .ok_or(ExactPropertyError::MissingReference {
            from: project,
            to: assignment,
        })?;
    release.require_exact_slots(assignment, assignment_entity)?;
    if !assignment_entity.is_type("IFCUNITASSIGNMENT") {
        return Err(ExactUnitError::UnsupportedUnit {
            unit: assignment,
            type_name: assignment_entity.type_name.clone(),
        });
    }
    let units = refs_at(assignment, assignment_entity.attributes.first(), "Units")?;
    let mut found = None;
    for unit in units {
        let unit_entity = model
            .get(unit)
            .ok_or(ExactPropertyError::MissingReference {
                from: assignment,
                to: unit,
            })?;
        release.require_exact_slots(unit, unit_entity)?;
        if !select_accepts_entity(release.schema, "IFCUNIT", &unit_entity.type_name) {
            return Err(ExactUnitError::UnsupportedUnit {
                unit,
                type_name: unit_entity.type_name.clone(),
            });
        }
        let declared = declared_unit_type(release, unit, unit_entity)?;
        if declared
            .as_deref()
            .is_some_and(|declared| declared.eq_ignore_ascii_case(unit_type))
        {
            if let Some(first) = found.replace(unit) {
                return Err(ExactUnitError::DuplicateProjectUnit {
                    unit_type: unit_type.clone(),
                    first,
                    second: unit,
                });
            }
        }
    }
    found.ok_or_else(no_unit)
}

/// The one unit of `unit_type` (an `IfcUnitEnum` or `IfcDerivedUnitEnum`
/// constant such as `LENGTHUNIT`) the single project assigns, unresolved.
///
/// # Errors
///
/// [`ExactUnitError::NoProject`], [`ExactUnitError::MultipleProjects`],
/// [`ExactUnitError::NoProjectUnit`] when the assignment has none of that
/// type, [`ExactUnitError::DuplicateProjectUnit`], and the structural
/// refusals of [`exact_unit`].
pub fn exact_project_unit(model: &Model, unit_type: &str) -> Result<EntityId, ExactUnitError> {
    let release = validate_model(model)?;
    project_unit(model, release, &Arc::from(unit_type.to_ascii_uppercase()))
}

/// What unit a measure type takes in a release: none, a named unit of an
/// `IfcUnitEnum` type, or a derived unit of an `IfcDerivedUnitEnum` type.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MeasureUnitType {
    /// A dimensionless measure (`IfcRatioMeasure`, `IfcCountMeasure` ...).
    Dimensionless,
    /// A named unit of this `IfcUnitEnum` constant: `IfcLengthMeasure`
    /// and `IfcPositiveLengthMeasure` take a `LENGTHUNIT`.
    Named(Arc<str>),
    /// A derived unit of this `IfcDerivedUnitEnum` constant.
    Derived(Arc<str>),
}

/// The unit a measure type takes in `schema`, derived from the release's
/// own tables as [`exact_unit`] derives it (`IfcAreaMeasure` takes an
/// `AREAUNIT`, `IfcNonNegativeLengthMeasure` a `LENGTHUNIT`).
///
/// # Errors
///
/// [`ExactUnitError::Structure`] with `UnsupportedSchema` for a release
/// the exact resolver does not bind (IFC4X1, IFC4X2) or the build leaves
/// out; [`ExactUnitError::MeasureNotInSchema`],
/// [`ExactUnitError::NotAMeasure`] and
/// [`ExactUnitError::UnmappedMeasureType`] as [`exact_unit`] refuses them.
pub fn measure_unit_type(
    schema: SchemaVersion,
    measure_type: &str,
) -> Result<MeasureUnitType, ExactUnitError> {
    let unsupported = || {
        ExactUnitError::Structure(ExactPropertyError::UnsupportedSchema {
            schema: format!("{schema:?}"),
        })
    };
    if !matches!(
        schema,
        SchemaVersion::Ifc2x3 | SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3
    ) {
        return Err(unsupported());
    }
    let release = Release {
        version: schema,
        schema: for_version(schema).map_err(|_| unsupported())?,
    };
    Ok(match measure_unit(release, measure_type)? {
        MeasureUnit::Dimensionless => MeasureUnitType::Dimensionless,
        MeasureUnit::Named(unit_type) => MeasureUnitType::Named(unit_type),
        MeasureUnit::Derived(unit_type) => MeasureUnitType::Derived(unit_type),
    })
}

/// A model's unit entity reduced to a [`ConversionUnit`] (#469): an
/// `IfcSIUnit` with its prefix, an `IfcConversionBasedUnit` through its
/// `ConversionFactor` chain, or an `IfcDerivedUnit` by its elements'
/// exponents, resolved exactly against the release the header declares.
///
/// # Errors
///
/// [`UnitConversionError::MonetaryUnit`],
/// [`UnitConversionError::ContextDependentUnit`],
/// [`UnitConversionError::OffsetUnit`] for an
/// `IfcConversionBasedUnitWithOffset` (#110), and
/// [`UnitConversionError::Resolve`] for every other refusal of the exact
/// resolver (missing, malformed or cyclic units, an unknown prefix ...).
pub fn resolve_unit(model: &Model, unit: EntityId) -> Result<ConversionUnit, UnitConversionError> {
    let release = validate_model(model).map_err(ExactUnitError::from)?;
    let entity = model.get(unit).ok_or(ExactUnitError::Structure(
        ExactPropertyError::MissingReference {
            from: unit,
            to: unit,
        },
    ))?;
    let is_a = |ancestor| release.schema.is_a(&entity.type_name, ancestor);
    if is_a("IFCMONETARYUNIT") {
        return Err(UnitConversionError::MonetaryUnit { unit });
    }
    if is_a("IFCCONTEXTDEPENDENTUNIT") {
        return Err(UnitConversionError::ContextDependentUnit { unit });
    }
    let resolved = Resolver::new(model, release).resolve(unit)?;
    let (mantissa, decimal_exponent) =
        normalise_decimal(resolved.mantissa, resolved.decimal_exponent);
    Ok(ConversionUnit {
        name: unit_label(model, unit).into(),
        unit_type: Some(resolved.unit_type),
        dimensions: resolved.dimensions,
        mantissa,
        decimal_exponent,
        offset: resolved.offset,
    })
}

/// Convert `value` from one of the model's units to another.
///
/// # Errors
///
/// As [`resolve_unit`] for either unit, and as
/// [`crate::unit_conversion`].
pub fn convert_between(
    model: &Model,
    value: f64,
    from: EntityId,
    to: EntityId,
) -> Result<f64, UnitConversionError> {
    if !value.is_finite() {
        return Err(UnitConversionError::NonFinite { value });
    }
    let from = resolve_unit(model, from)?;
    let to = resolve_unit(model, to)?;
    Ok(unit_conversion(&from, &to)?.apply(value))
}

/// `MILLI METRE` for an SI unit, the `Name` of a conversion-based unit,
/// otherwise `#id TYPE`.
fn unit_label(model: &Model, unit: EntityId) -> String {
    let Some(entity) = model.get(unit) else {
        return format!("#{}", unit.0);
    };
    if entity.is_type("IFCSIUNIT") {
        if let (Some(prefix), Some(Value::Enum(name))) =
            (entity.attributes.get(2), entity.attributes.get(3))
        {
            return match prefix {
                Value::Enum(prefix) => format!("{prefix} {name}"),
                _ => name.to_string(),
            };
        }
    }
    if entity.type_name.starts_with("IFCCONVERSIONBASEDUNIT") {
        if let Some(Value::Text(name)) = entity.attributes.get(2) {
            return name.to_string();
        }
    }
    format!("#{} {}", unit.0, entity.type_name)
}
