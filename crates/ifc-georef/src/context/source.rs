//! The source side of a coordinate operation, and its inverse.
//!
//! `IfcCoordinateOperation.SourceCRS` is typed
//! `IfcCoordinateReferenceSystemSelect = SELECT
//! (IfcCoordinateReferenceSystem, IfcGeometricRepresentationContext)` in
//! both IFC4 ADD2 TC1 and IFC4X3 ADD2. Both releases declare the matching
//! inverse `HasCoordinateOperation : SET [0:1] OF IfcCoordinateOperation
//! FOR SourceCRS` on `IfcCoordinateReferenceSystem` and on
//! `IfcGeometricRepresentationContext`, and
//! `IfcGeometricRepresentationSubContext` adds
//! `WHERE NoCoordOperation : SIZEOF(...HasCoordinateOperation) = 0`.
//!
//! What differs is which CRS subtypes exist: IFC4 declares only
//! `IfcProjectedCRS`; IFC4X3 adds `IfcGeographicCRS`. A schema-pinned read
//! asks the pinned table; an unpinned read (no usable header) accepts the
//! union and cannot tell the two apart.
//!
//! The inverse is not stored in a STEP file: it is recovered by scanning
//! every `IfcCoordinateOperation` subtype for a `SourceCRS` naming the
//! entity. More than one breaks the `SET [0:1]` and is refused rather than
//! one operation being picked.

use ifc_model::value::Value;
use ifc_model::{EntityId, Model};

use crate::error::{GeorefError, GeorefResult};
use crate::slot::map_conversion as slot;
use crate::view::GeorefView;

/// The rule label reported when a source has more than one operation.
pub(crate) const HAS_COORDINATE_OPERATION: &str =
    "HasCoordinateOperation : SET [0:1] OF IfcCoordinateOperation";

/// Coordinate-operation types an unpinned read recognizes: the IFC4X3 set,
/// which contains IFC4's.
const UNPINNED_OPERATIONS: &[&str] = &[
    "IFCMAPCONVERSION",
    "IFCMAPCONVERSIONSCALED",
    "IFCRIGIDOPERATION",
];

/// CRS types an unpinned read recognizes as a source.
const UNPINNED_CRS: &[&str] = &["IFCPROJECTEDCRS", "IFCGEOGRAPHICCRS"];

/// The validated `SourceCRS` of a coordinate operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OperationSource {
    /// An `IfcGeometricRepresentationContext` (never a sub-context, which
    /// `NoCoordOperation` forbids): the usual case, the project's own
    /// engineering frame.
    Context(EntityId),
    /// An `IfcCoordinateReferenceSystem` subtype: an operation chained from
    /// another CRS.
    CoordinateReferenceSystem(EntityId),
}

impl OperationSource {
    /// The source entity, whichever select member it is.
    #[must_use]
    pub const fn entity(&self) -> EntityId {
        match self {
            Self::Context(id) | Self::CoordinateReferenceSystem(id) => *id,
        }
    }
}

/// Read and validate the `SourceCRS` of the coordinate operation
/// `operation` in a schema-pinned view.
///
/// # Errors
///
/// [`GeorefError::WrongType`] when `operation` is not an
/// `IfcCoordinateOperation`, or its source is not a member of
/// `IfcCoordinateReferenceSystemSelect`;
/// [`GeorefError::UnsupportedOperation`] when either type is not declared
/// in the pinned release (an `IfcGeographicCRS` under IFC4);
/// [`GeorefError::MissingAttribute`] / [`GeorefError::InvalidAttribute`]
/// for an unset or non-reference `SourceCRS`;
/// [`GeorefError::MissingEntity`] for a dangling one;
/// [`GeorefError::RuleViolation`] for a sub-context source
/// (`NoCoordOperation`) or a source with more than one operation
/// (`HasCoordinateOperation`).
pub fn resolve_operation_source(
    view: &GeorefView,
    operation: EntityId,
) -> GeorefResult<OperationSource> {
    let actual = view.require_known_type(operation)?;
    if !view.schema.is_a(actual, "IfcCoordinateOperation") {
        return Err(GeorefError::WrongType {
            entity: operation,
            expected: "IFCCOORDINATEOPERATION",
            actual: actual.to_owned(),
        });
    }
    operation_source(view.model, Some(view), operation)
}

/// The one coordinate operation whose `SourceCRS` is `source`: the
/// `HasCoordinateOperation` inverse, read in a schema-pinned view.
///
/// `Ok(None)` means `source` is a valid source with no operation.
///
/// # Errors
///
/// [`GeorefError::MissingEntity`] when `source` is not in the model;
/// [`GeorefError::WrongType`] / [`GeorefError::UnsupportedOperation`] when
/// it is not an `IfcCoordinateReferenceSystemSelect` member in the pinned
/// release; [`GeorefError::RuleViolation`] when more than one operation
/// names it, or a sub-context has any.
pub fn coordinate_operation_for(
    view: &GeorefView,
    source: EntityId,
) -> GeorefResult<Option<EntityId>> {
    let model = view.model;
    if model.get(source).is_none() {
        return Err(GeorefError::MissingEntity {
            referrer: source,
            missing: source,
        });
    }
    classify(model, Some(view), source)?;
    let operations = operations_from(model, Some(view), source);
    match operations.as_slice() {
        [] => Ok(None),
        [one] => {
            forbid_sub_context(model, source)?;
            Ok(Some(*one))
        }
        _ => Err(GeorefError::RuleViolation {
            entity: source,
            rule: HAS_COORDINATE_OPERATION,
        }),
    }
}

/// Read, type-check and inverse-check `operation`'s `SourceCRS`. `view`
/// pins the release; `None` accepts the IFC4X3 union.
pub(crate) fn operation_source(
    model: &Model,
    view: Option<&GeorefView>,
    operation: EntityId,
) -> GeorefResult<OperationSource> {
    let entity = model.get(operation).ok_or(GeorefError::MissingEntity {
        referrer: operation,
        missing: operation,
    })?;
    let source = match entity.attribute(slot::SOURCE_CRS) {
        None | Some(Value::Null) => {
            return Err(GeorefError::MissingAttribute {
                entity: operation,
                index: slot::SOURCE_CRS,
                name: "SourceCRS",
            })
        }
        Some(value) => value.as_ref_id().ok_or(GeorefError::InvalidAttribute {
            entity: operation,
            index: slot::SOURCE_CRS,
            name: "SourceCRS",
        })?,
    };
    if model.get(source).is_none() {
        return Err(GeorefError::MissingEntity {
            referrer: operation,
            missing: source,
        });
    }
    let classified = classify(model, view, source)?;
    forbid_sub_context(model, source)?;
    if operations_from(model, view, source).len() > 1 {
        return Err(GeorefError::RuleViolation {
            entity: source,
            rule: HAS_COORDINATE_OPERATION,
        });
    }
    Ok(classified)
}

/// Which `IfcCoordinateReferenceSystemSelect` member `id` is.
fn classify(
    model: &Model,
    view: Option<&GeorefView>,
    id: EntityId,
) -> GeorefResult<OperationSource> {
    let entity = model.get(id).ok_or(GeorefError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    let type_name = entity.type_name.as_ref();
    let (is_context, is_crs) = match view {
        Some(view) => {
            let declared = view.require_known_type(id)?;
            (
                view.schema
                    .is_a(declared, "IfcGeometricRepresentationContext"),
                view.schema.is_a(declared, "IfcCoordinateReferenceSystem"),
            )
        }
        None => (
            entity.is_type("IFCGEOMETRICREPRESENTATIONCONTEXT")
                || entity.is_type("IFCGEOMETRICREPRESENTATIONSUBCONTEXT"),
            UNPINNED_CRS.iter().any(|crs| entity.is_type(crs)),
        ),
    };
    if is_context {
        Ok(OperationSource::Context(id))
    } else if is_crs {
        Ok(OperationSource::CoordinateReferenceSystem(id))
    } else {
        Err(GeorefError::WrongType {
            entity: id,
            expected: "IFCCOORDINATEREFERENCESYSTEM or IFCGEOMETRICREPRESENTATIONCONTEXT",
            actual: type_name.to_owned(),
        })
    }
}

/// `IfcGeometricRepresentationSubContext.NoCoordOperation`.
fn forbid_sub_context(model: &Model, source: EntityId) -> GeorefResult<()> {
    match model.get(source) {
        Some(entity) if entity.is_type("IFCGEOMETRICREPRESENTATIONSUBCONTEXT") => {
            Err(GeorefError::RuleViolation {
                entity: source,
                rule: "NoCoordOperation",
            })
        }
        _ => Ok(()),
    }
}

/// Every coordinate operation whose `SourceCRS` references `source`, in
/// ascending id order.
fn operations_from(model: &Model, view: Option<&GeorefView>, source: EntityId) -> Vec<EntityId> {
    let types: Vec<&str> = match view {
        Some(view) => view.schema.subtypes("IfcCoordinateOperation"),
        None => UNPINNED_OPERATIONS.to_vec(),
    };
    let mut found: Vec<EntityId> = types
        .into_iter()
        .flat_map(|type_name| model.of_type(type_name))
        .filter(|(_, operation)| operation.reference(slot::SOURCE_CRS) == Some(source))
        .map(|(id, _)| id)
        .collect();
    found.sort_unstable();
    found.dedup();
    found
}
