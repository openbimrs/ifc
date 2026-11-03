//! Why an SI batch (#638) was refused beyond a unit conversion. Plain data,
//! so it exists without `properties`, which the crate root's re-export relies on.

use std::fmt;

use ifc_model::EntityId;

/// Why a batch read in SI cannot be converted or run (`AuthoringFailure::SiInput`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SiInputRefusal {
    /// The release is not one whose tables the conversion is reviewed against
    /// (IFC2X3, IFC4 and IFC4X3 are).
    UnsupportedRelease {
        /// The release the schema names.
        schema: String,
    },
    /// The model's header does not declare the release the batch is planned
    /// against, so its units were read in another release's layout.
    ReleaseMismatch {
        /// The header's `FILE_SCHEMA` tokens.
        declared: String,
        /// The release the batch is planned against.
        schema: String,
    },
    /// An `IfcProjectLibrary` states units of its own, so the units that
    /// apply are not the project's alone.
    ContextUnits {
        /// The library.
        context: EntityId,
    },
    /// The batch changes the units its numbers are converted into.
    UnitsChanged {
        /// The unit, unit assignment or measure the operation changes, when it
        /// names one.
        entity: Option<EntityId>,
    },
    /// A value names a unit the batch creates: units resolve against the
    /// model as it was before the batch.
    UnitCreatedInBatch {
        /// The unit.
        unit: EntityId,
    },
    /// The value's unit is stated by another record (a time series, a table
    /// column), which an operation does not carry.
    UnitStatedElsewhere,
    /// An untyped number where the declared `SELECT` admits several numeric
    /// types: its measure cannot be told.
    UntypedSelectValue {
        /// The select type.
        select: String,
    },
    /// An `IfcParameterValue` whose curve or surface has no single unit for
    /// its parameter.
    ParameterSpace {
        /// The curve or surface, when it exists.
        host: Option<EntityId>,
        /// Its type.
        host_type: Option<String>,
        /// Why.
        detail: String,
    },
    /// An `IfcPcurve` on a surface whose parameters are not both lengths,
    /// written by the batch or already in the model.
    ParameterSpaceCurve {
        /// The surface.
        surface: EntityId,
        /// Its type.
        surface_type: String,
    },
    /// An `IfcPolynomialCurve` coefficient: its unit depends on a
    /// parameterisation IFC4.3 does not state.
    PolynomialCoefficients,
    /// An `IfcTextureCoordinateGenerator` parameter: its meaning depends on
    /// the X3D mode.
    TextureGeneratorParameters,
    /// A map conversion or rigid operation: its numbers are in the target
    /// CRS's units, not the project's.
    MapConversion,
    /// An integer given for a measure whose declared type is `INTEGER`
    /// (`IfcIntegerCountRateMeasure`): a converted number is generally not an
    /// integer. An integer for a `REAL` measure is converted, as a real.
    IntegerMeasure {
        /// The measure type.
        measure: String,
    },
}

impl fmt::Display for SiInputRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedRelease { schema } => write!(
                f,
                "SI input is reviewed for IFC2X3, IFC4 and IFC4X3 only, not {schema}"
            ),
            Self::ReleaseMismatch { declared, schema } => write!(
                f,
                "the model declares {declared} and the batch is planned against {schema}; \
                 SI input reads the model's units in the declared release"
            ),
            Self::ContextUnits { context } => write!(
                f,
                "{context} states units of its own; SI input converts into the project's"
            ),
            Self::UnitsChanged {
                entity: Some(entity),
            } => write!(
                f,
                "the operation changes {entity}, a unit the batch converts into; \
                 change units in a batch that reads numbers raw"
            ),
            Self::UnitsChanged { entity: None } => write!(
                f,
                "the operation sets the project's units, which the batch converts into; \
                 change units in a batch that reads numbers raw"
            ),
            Self::UnitCreatedInBatch { unit } => write!(
                f,
                "{unit} is a unit this batch creates; units resolve against the model \
                 as it was before the batch"
            ),
            Self::UnitStatedElsewhere => f.write_str(
                "the unit of this value is stated by another record, which the operation \
                 does not carry",
            ),
            Self::UntypedSelectValue { select } => write!(
                f,
                "an untyped number for {select}, which admits several numeric types; \
                 give it as a typed value"
            ),
            Self::ParameterSpace {
                host,
                host_type,
                detail,
            } => {
                f.write_str("a parameter value whose space has no single unit")?;
                match (host, host_type) {
                    (Some(host), Some(host_type)) => write!(f, " ({host} {host_type}: {detail})"),
                    _ => write!(f, " ({detail})"),
                }
            }
            Self::ParameterSpaceCurve {
                surface,
                surface_type,
            } => write!(
                f,
                "a p-curve on {surface} ({surface_type}), whose parameters are not both lengths"
            ),
            Self::PolynomialCoefficients => f.write_str(
                "polynomial curve coefficients have a unit that depends on a parameterisation \
                 IFC4.3 does not state",
            ),
            Self::TextureGeneratorParameters => f.write_str(
                "texture generator parameters have a meaning that depends on the X3D mode",
            ),
            Self::MapConversion => f.write_str(
                "map conversions state numbers in the target CRS's units, which SI input \
                 does not convert",
            ),
            Self::IntegerMeasure { measure } => write!(
                f,
                "an integer given for {measure}; SI input converts reals, and the converted \
                 number is generally not an integer"
            ),
        }
    }
}
