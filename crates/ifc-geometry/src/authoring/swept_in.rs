//! The directrix-driven sweeps, written in the model's declared release.
//!
//! `StartParam`/`EndParam` are `IfcParameterValue` in IFC4 (written bare) and
//! the SELECT `IfcCurveMeasureSelect` in IFC4X3 (written
//! `IFCPARAMETERVALUE(..)`), so the writers without a model cannot be right
//! in both. These take the model and write what its release declares; see
//! `release.rs` for the evidence and the binding rule (#200).

use ifc_model::{Entity, EntityId, Model, Transaction, Value};

use crate::error::GeometryError;
use crate::solid::swept::directrix_slot;

use super::release::bind;
use super::swept::{directrix_attrs, ParamForm, SweepTrim};

/// Stage an `IfcSurfaceCurveSweptAreaSolid` in `model`'s declared release.
///
/// As [`surface_curve_swept_area_solid`](super::surface_curve_swept_area_solid),
/// but the trim parameters take the form the release declares:
///
/// ```text
/// IFC2X3  StartParam : IfcParameterValue           0.5   (required)
/// IFC4    StartParam : OPTIONAL IfcParameterValue  0.5
/// IFC4X3  StartParam : OPTIONAL IfcCurveMeasureSelect
///                                    IFCPARAMETERVALUE(0.5)
/// ```
///
/// A model without `FILE_SCHEMA` binds IFC4.
///
/// # Errors
///
/// [`GeometryError::InvalidAuthoredValue`] for a non-finite trim parameter,
/// or an attribute the release requires left unset (in IFC2X3, `Position`,
/// `StartParam` and `EndParam`).
/// [`GeometryError::AuthoringSchemaUnbound`] if `FILE_SCHEMA` names an
/// unknown release or several. Nothing is staged on error.
pub fn surface_curve_swept_area_solid_in(
    tx: &mut Transaction,
    model: &Model,
    swept_area: EntityId,
    position: Option<EntityId>,
    directrix: EntityId,
    trim: SweepTrim,
    reference_surface: EntityId,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCSURFACECURVESWEPTAREASOLID";
    let release = bind(model, T)?;
    let form = ParamForm::Release(release);
    let mut attrs = directrix_attrs(T, swept_area, position, directrix, trim, form)?;
    attrs[directrix_slot::REFERENCE_SURFACE] = Value::Ref(reference_surface);
    release.require(T, &attrs)?;
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcFixedReferenceSweptAreaSolid` in `model`'s declared release.
///
/// As [`fixed_reference_swept_area_solid`](super::fixed_reference_swept_area_solid),
/// but the trim parameters are bare in IFC4 and `IFCPARAMETERVALUE(..)` in
/// IFC4X3. IFC2X3 does not declare the entity. A model without
/// `FILE_SCHEMA` binds IFC4.
///
/// # Errors
///
/// [`GeometryError::InvalidAuthoredValue`] for a non-finite trim parameter.
/// [`GeometryError::AuthoringEntityNotInSchema`] in IFC2X3.
/// [`GeometryError::AuthoringSchemaUnbound`] if `FILE_SCHEMA` names an
/// unknown release or several. Nothing is staged on error.
pub fn fixed_reference_swept_area_solid_in(
    tx: &mut Transaction,
    model: &Model,
    swept_area: EntityId,
    position: Option<EntityId>,
    directrix: EntityId,
    trim: SweepTrim,
    fixed_reference: EntityId,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCFIXEDREFERENCESWEPTAREASOLID";
    let release = bind(model, T)?;
    let form = ParamForm::Release(release);
    let mut attrs = directrix_attrs(T, swept_area, position, directrix, trim, form)?;
    attrs[directrix_slot::FIXED_REFERENCE] = Value::Ref(fixed_reference);
    release.require(T, &attrs)?;
    Ok(tx.create(Entity::new(T, attrs)))
}
