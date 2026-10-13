//! Every project-to-map coordinate operation of a model, resolved (#123).
//!
//! `ifc-georef` resolves one coordinate operation given the project's
//! length scale, and leaves that scale to the caller on purpose: the
//! project's `IfcUnitAssignment` is a property and unit question, owned by
//! `ifc-properties`. The two are siblings under ADR 0003, so the join that
//! answers "where is this model on the map" lives in this orchestration
//! layer. It gates itself on `all(georef, properties)` with an inner
//! `#![cfg]`, so the module declaration compiles to nothing without both.
//!
//! Nothing is guessed. The release is pinned by `ifc-georef`: in IFC4 and
//! IFC4X3 the operations are the `IfcCoordinateOperation` entities, found
//! by the pinned release's subtype test; IFC2X3, which declares none, is
//! read through the `ePSet_MapConversion` and `ePSet_ProjectedCRS`
//! property sets (#451, see `ifc_georef::resolve_property_set_georeferencing`
//! for the source and the rules). The release decides which is read; the
//! two are never merged. The project length unit is resolved exactly by
//! `ifc-properties`. A model with nothing to read resolves to an empty
//! list without asking for its units.
#![cfg(all(feature = "georef", feature = "properties"))]

use std::fmt;

use ifc_georef::{
    has_property_set_georeferencing, resolve_project_to_map_in,
    resolve_property_set_georeferencing, GeorefError, GeorefView, ProjectToMap,
};
use ifc_model::{EntityId, Model};
use ifc_properties::{exact_unit, ExactUnitError};
use ifc_schema::SchemaVersion;

/// Why a model's coordinate operations could not be resolved.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum GeoreferencingError {
    /// `ifc-georef` refused the release or an operation.
    Georef(GeorefError),
    /// The project length unit, which scales every operation, could not be
    /// resolved exactly.
    ProjectLengthUnit(ExactUnitError),
    /// The project length unit is an offset unit, which no length scale can
    /// carry.
    OffsetLengthUnit {
        /// The unit entity.
        unit: Option<EntityId>,
    },
}

impl fmt::Display for GeoreferencingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Georef(error) => write!(f, "{error}"),
            Self::ProjectLengthUnit(error) => write!(f, "project length unit: {error}"),
            Self::OffsetLengthUnit { unit } => {
                write!(f, "project length unit {unit:?} has an offset")
            }
        }
    }
}

impl std::error::Error for GeoreferencingError {}

/// Every `IfcCoordinateOperation` in `model` resolved to project-to-map
/// parameters, in file order.
///
/// The model's `FILE_SCHEMA` must pin IFC2X3, IFC4 or IFC4X3. Under IFC4
/// and IFC4X3 ([`GeorefView::for_model`]) the operations are the entities
/// the pinned release declares `IfcCoordinateOperation` subtypes, each
/// resolved by [`resolve_project_to_map_in`]. Under IFC2X3 they are the
/// `ePSet_MapConversion` and `ePSet_ProjectedCRS` pairs
/// ([`resolve_property_set_georeferencing`]): the `IfcProject`'s, or else
/// one per `IfcSite` carrying them, ordered by host. Property sets of those
/// names in an IFC4 or IFC4X3 file are not read. Both use the project
/// length unit's exact SI scale ([`exact_unit`] of `IFCLENGTHMEASURE`).
///
/// # Errors
///
/// [`GeoreferencingError::Georef`] when the release is not IFC2X3, IFC4 or
/// IFC4X3, an IFC2X3 property set is malformed or incomplete, or any
/// operation cannot be resolved (an IFC4X3 plane-angle
/// `IfcRigidOperation` onto an `IfcGeographicCRS` among them: it has no
/// project-to-map form); [`GeoreferencingError::ProjectLengthUnit`] or
/// [`GeoreferencingError::OffsetLengthUnit`] when an operation exists but
/// the project length unit cannot scale it.
pub fn georeferencing(model: &Model) -> Result<Vec<ProjectToMap>, GeoreferencingError> {
    if declares_ifc2x3(model) {
        if !has_property_set_georeferencing(model) {
            return Ok(Vec::new());
        }
        let scale = project_length_scale(model)?;
        return resolve_property_set_georeferencing(model, scale)
            .map_err(GeoreferencingError::Georef);
    }
    let view = GeorefView::for_model(model).map_err(GeoreferencingError::Georef)?;
    let schema = view.schema();
    let types: Vec<&str> = model
        .type_histogram()
        .into_iter()
        .filter(|(name, _)| schema.is_a(name, "IFCCOORDINATEOPERATION"))
        .map(|(name, _)| name)
        .collect();
    let mut operations: Vec<EntityId> = types
        .iter()
        .flat_map(|name| model.ids_of_type(name).iter().copied())
        .collect();
    if operations.is_empty() {
        return Ok(Vec::new());
    }
    // File order, whatever the histogram's order.
    let position: std::collections::HashMap<EntityId, usize> =
        model.ids().enumerate().map(|(i, id)| (id, i)).collect();
    operations.sort_by_key(|id| position.get(id).copied().unwrap_or(usize::MAX));

    let scale = project_length_scale(model)?;
    operations
        .into_iter()
        .map(|id| resolve_project_to_map_in(&view, id, scale))
        .collect::<Result<_, _>>()
        .map_err(GeoreferencingError::Georef)
}

/// Whether the header declares exactly IFC2X3, the release read through
/// property sets. Any other header, missing or ambiguous ones included,
/// goes to [`GeorefView::for_model`], which refuses it with its reason.
fn declares_ifc2x3(model: &Model) -> bool {
    matches!(
        model.header().schema.as_slice(),
        [token] if SchemaVersion::from_header_token(token) == Some(SchemaVersion::Ifc2x3)
    )
}

/// The project length unit's exact metre scale.
fn project_length_scale(model: &Model) -> Result<f64, GeoreferencingError> {
    let unit = exact_unit(model, "IFCLENGTHMEASURE", None)
        .map_err(GeoreferencingError::ProjectLengthUnit)?;
    if unit.offset != 0.0 {
        return Err(GeoreferencingError::OffsetLengthUnit { unit: unit.unit });
    }
    Ok(unit.scale)
}
