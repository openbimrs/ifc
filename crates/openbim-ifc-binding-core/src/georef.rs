//! Georeferencing (feature `georef`, #123).
//!
//! The facade's `georeferencing` resolves every coordinate operation of a
//! model (`IfcMapConversion`, and in IFC4X3 `IfcMapConversionScaled` and a
//! length `IfcRigidOperation`) with the project length unit, as
//! `ifc-georef` resolves them: the target projected CRS, the authored
//! eastings, northings and height, the axis direction and scale, and the
//! resolved operation from project metres to map metres. These are
//! resolved parameters, not IFC values, so they cross as host numbers.
//!
//! Only IFC4 and IFC4X3 declare georeferencing: any other release is
//! refused with `unsupported-schema`. A model without a coordinate
//! operation has an empty list.

use crate::record::{Field, Record, ToRecord};
use crate::{BindingError, IfcModel};

/// One coordinate operation, resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct MapConversion {
    /// The coordinate-operation entity.
    pub operation: u64,
    /// `map-conversion`, `map-conversion-scaled` or `rigid-operation`.
    pub kind: String,
    /// `SourceCRS`: a representation context or a CRS.
    pub source: u64,
    /// `context` or `crs`.
    pub source_kind: String,
    /// The target `IfcProjectedCRS`.
    pub target_crs: ProjectedCrs,
    /// `Eastings` as authored, in `map_unit`.
    pub eastings: f64,
    /// `Northings` as authored, in `map_unit`.
    pub northings: f64,
    /// `OrthogonalHeight` as authored, in `map_unit` (a rigid operation's
    /// `Height`, or 0 when it states none).
    pub orthogonal_height: f64,
    /// `(XAxisAbscissa, XAxisOrdinate)`, normalised.
    pub x_axis: (f64, f64),
    /// `Scale` as declared (1 when unset or for a rigid operation).
    pub scale: f64,
    /// IFC4X3 `IfcMapConversionScaled` `(FactorX, FactorY, FactorZ)`.
    pub factors: Option<(f64, f64, f64)>,
    /// The project length unit the operation was resolved with.
    pub project_unit: LengthUnit,
    /// The map unit: the CRS's `MapUnit`, or the project unit when unset.
    pub map_unit: LengthUnit,
    /// Whether the CRS states `MapUnit` itself.
    pub map_unit_declared: bool,
    /// The operation from project metres to map metres as three columns,
    /// the images of the project's X, Y and Z axes.
    pub linear: [[f64; 3]; 3],
    /// Where the project origin lands, in map metres.
    pub translation: [f64; 3],
}

/// An `IfcProjectedCRS`.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedCrs {
    /// Its entity id.
    pub id: u64,
    /// `Name`, e.g. `EPSG:25832`.
    pub name: Option<String>,
    /// `Description`.
    pub description: Option<String>,
    /// `GeodeticDatum`.
    pub geodetic_datum: Option<String>,
    /// `VerticalDatum`.
    pub vertical_datum: Option<String>,
    /// `MapProjection`.
    pub map_projection: Option<String>,
    /// `MapZone`.
    pub map_zone: Option<String>,
    /// IFC4X3 `WellKnownText`, when it defines the CRS.
    pub well_known_text: Option<String>,
}

/// A length unit reduced to metres.
#[derive(Debug, Clone, PartialEq)]
pub struct LengthUnit {
    /// The unit's name as resolved, e.g. `METRE`, `MILLIMETRE`.
    pub name: String,
    /// Metres per one of this unit.
    pub metres_per_unit: f64,
}

impl IfcModel {
    /// Every coordinate operation, resolved, in file order.
    ///
    /// Refused with `unsupported-schema` for a release other than IFC4 or
    /// IFC4X3, `unsupported` for an operation with no project-to-map form,
    /// `invalid-model` for a malformed operation or project length unit,
    /// `missing-reference` for a dangling one, and `feature-disabled`
    /// without the `georef` feature.
    pub fn georeferencing(&self) -> Result<Vec<MapConversion>, BindingError> {
        #[cfg(feature = "georef")]
        {
            read::georeferencing(self)
        }
        #[cfg(not(feature = "georef"))]
        {
            Err(BindingError::FeatureDisabled("georef"))
        }
    }
}

#[cfg(feature = "georef")]
mod read {
    use ifc::georef::{GeorefError, OperationKind, OperationSource};
    use ifc::GeoreferencingError;

    use super::{LengthUnit, MapConversion, ProjectedCrs};
    use crate::{BindingError, IfcModel};

    pub(super) fn georeferencing(model: &IfcModel) -> Result<Vec<MapConversion>, BindingError> {
        let operations = ifc::georeferencing(&model.inner).map_err(error)?;
        Ok(operations
            .into_iter()
            .map(|map| {
                let (kind, factors) = match map.kind {
                    OperationKind::MapConversion => ("map-conversion", None),
                    OperationKind::MapConversionScaled { factors } => {
                        ("map-conversion-scaled", Some(factors))
                    }
                    OperationKind::RigidOperation { .. } => ("rigid-operation", None),
                    _ => ("other", None),
                };
                let (source, source_kind) = match map.source {
                    OperationSource::Context(id) => (id.0, "context"),
                    OperationSource::CoordinateReferenceSystem(id) => (id.0, "crs"),
                    _ => (map.source_crs.0, "other"),
                };
                let unit = |unit: &ifc::georef::LengthUnit| LengthUnit {
                    name: unit.name.clone(),
                    metres_per_unit: unit.metres_per_unit,
                };
                let crs = &map.target_crs;
                MapConversion {
                    operation: map.operation.0,
                    kind: kind.to_owned(),
                    source,
                    source_kind: source_kind.to_owned(),
                    target_crs: ProjectedCrs {
                        id: crs.entity.0,
                        name: crs.name.clone(),
                        description: crs.description.clone(),
                        geodetic_datum: crs.geodetic_datum.clone(),
                        vertical_datum: crs.vertical_datum.clone(),
                        map_projection: crs.map_projection.clone(),
                        map_zone: crs.map_zone.clone(),
                        well_known_text: crs.well_known_text.clone(),
                    },
                    eastings: map.eastings,
                    northings: map.northings,
                    orthogonal_height: map.orthogonal_height,
                    x_axis: map.x_axis_direction,
                    scale: map.declared_scale,
                    factors,
                    project_unit: unit(&map.project_unit),
                    map_unit: unit(&map.map_unit),
                    map_unit_declared: map.declared_map_unit().is_some(),
                    linear: map.linear_part(),
                    translation: map.translation(),
                }
            })
            .collect())
    }

    fn error(error: GeoreferencingError) -> BindingError {
        let detail = error.to_string();
        match error {
            GeoreferencingError::Georef(error) => match error {
                GeorefError::UnsupportedSchema { token } => BindingError::UnsupportedSchema(token),
                GeorefError::MissingSchema => BindingError::UnsupportedSchema(String::new()),
                GeorefError::AmbiguousSchema { .. } => BindingError::UnsupportedSchema(detail),
                GeorefError::MissingEntity { .. } => BindingError::MissingReference(detail),
                GeorefError::UnsupportedOperation { .. }
                | GeorefError::CoordinateMeasureMismatch { .. } => {
                    BindingError::Unsupported(detail)
                }
                GeorefError::UnitCycle { .. } => BindingError::BudgetExceeded(detail),
                _ => BindingError::InvalidModel(detail),
            },
            GeoreferencingError::ProjectLengthUnit(error) => {
                crate::properties::read::unit_error(error)
            }
            GeoreferencingError::OffsetLengthUnit { .. } => BindingError::Unsupported(detail),
            _ => BindingError::InvalidModel(detail),
        }
    }
}

impl ToRecord for MapConversion {
    fn to_record(&self) -> Record {
        let triple =
            |[x, y, z]: [f64; 3]| Field::List(vec![Field::Real(x), Field::Real(y), Field::Real(z)]);
        Record::new(
            "MapConversion",
            vec![
                ("operation", Field::Id(self.operation)),
                ("kind", Field::Text(self.kind.clone())),
                ("source", Field::Id(self.source)),
                ("source_kind", Field::Text(self.source_kind.clone())),
                ("target_crs", Field::Record(self.target_crs.to_record())),
                ("eastings", Field::Real(self.eastings)),
                ("northings", Field::Real(self.northings)),
                ("orthogonal_height", Field::Real(self.orthogonal_height)),
                (
                    "x_axis",
                    Field::List(vec![Field::Real(self.x_axis.0), Field::Real(self.x_axis.1)]),
                ),
                ("scale", Field::Real(self.scale)),
                (
                    "factors",
                    Field::optional(self.factors, |(x, y, z)| triple([x, y, z])),
                ),
                ("project_unit", Field::Record(self.project_unit.to_record())),
                ("map_unit", Field::Record(self.map_unit.to_record())),
                ("map_unit_declared", Field::Bool(self.map_unit_declared)),
                (
                    "linear",
                    Field::List(self.linear.iter().map(|column| triple(*column)).collect()),
                ),
                ("translation", triple(self.translation)),
            ],
        )
    }
}

impl ToRecord for ProjectedCrs {
    fn to_record(&self) -> Record {
        Record::new(
            "ProjectedCrs",
            vec![
                ("id", Field::Id(self.id)),
                ("name", Field::text(self.name.clone())),
                ("description", Field::text(self.description.clone())),
                ("geodetic_datum", Field::text(self.geodetic_datum.clone())),
                ("vertical_datum", Field::text(self.vertical_datum.clone())),
                ("map_projection", Field::text(self.map_projection.clone())),
                ("map_zone", Field::text(self.map_zone.clone())),
                ("well_known_text", Field::text(self.well_known_text.clone())),
            ],
        )
    }
}

impl ToRecord for LengthUnit {
    fn to_record(&self) -> Record {
        Record::new(
            "LengthUnit",
            vec![
                ("name", Field::Text(self.name.clone())),
                ("metres_per_unit", Field::Real(self.metres_per_unit)),
            ],
        )
    }
}
