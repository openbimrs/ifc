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
//! Nothing is guessed. The release is pinned by `ifc-georef` (IFC4 or
//! IFC4X3; IFC2X3 declares no georeferencing), the operations are found by
//! the pinned release's subtype test, and the project length unit is
//! resolved exactly by `ifc-properties`. A model with no coordinate
//! operation resolves to an empty list without asking for its units.
#![cfg(all(feature = "georef", feature = "properties"))]

use std::fmt;

use ifc_georef::{resolve_project_to_map_in, GeorefError, GeorefView, ProjectToMap};
use ifc_model::{EntityId, Model};
use ifc_properties::{exact_unit, ExactUnitError};

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
/// The model's `FILE_SCHEMA` must pin IFC4 or IFC4X3
/// ([`GeorefView::for_model`]). Operations are the entities the pinned
/// release declares `IfcCoordinateOperation` subtypes; each is resolved by
/// [`resolve_project_to_map_in`] with the project length unit's exact SI
/// scale ([`exact_unit`] of `IFCLENGTHMEASURE`).
///
/// # Errors
///
/// [`GeoreferencingError::Georef`] when the release is not IFC4 or IFC4X3,
/// or any operation cannot be resolved (an IFC4X3 plane-angle
/// `IfcRigidOperation` onto an `IfcGeographicCRS` among them: it has no
/// project-to-map form); [`GeoreferencingError::ProjectLengthUnit`] or
/// [`GeoreferencingError::OffsetLengthUnit`] when an operation exists but
/// the project length unit cannot scale it.
pub fn georeferencing(model: &Model) -> Result<Vec<ProjectToMap>, GeoreferencingError> {
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

    let unit = exact_unit(model, "IFCLENGTHMEASURE", None)
        .map_err(GeoreferencingError::ProjectLengthUnit)?;
    if unit.offset != 0.0 {
        return Err(GeoreferencingError::OffsetLengthUnit { unit: unit.unit });
    }
    operations
        .into_iter()
        .map(|id| resolve_project_to_map_in(&view, id, unit.scale))
        .collect::<Result<_, _>>()
        .map_err(GeoreferencingError::Georef)
}
