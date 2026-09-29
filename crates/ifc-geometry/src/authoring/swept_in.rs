//! The sweeps whose record depends on the model's declared release.
//!
//! `StartParam`/`EndParam` of the directrix-driven sweeps are
//! `IfcParameterValue` in IFC2X3 and IFC4 (written bare) and the SELECT
//! `IfcCurveMeasureSelect` in IFC4X3 (written `IFCPARAMETERVALUE(..)`), so a
//! writer without a model cannot be right in every release (#200). The swept
//! disk's trim keeps its form but not its optionality: IFC2X3 requires it,
//! later releases do not (#210). These writers take the model and write what
//! its release declares; see `release.rs` for the evidence and the binding
//! rule. They are the only writers of these three entities.

use ifc_model::{Entity, EntityId, Model, Transaction, Value};

use crate::error::GeometryError;
use crate::solid::swept::directrix_slot;

use super::release::bind;
use super::swept::{directrix_attrs, disk_attrs, ParamForm, SweepTrim};

/// Stage an `IfcSurfaceCurveSweptAreaSolid` in `model`'s declared release.
///
/// The profile is swept along `directrix` while staying on
/// `reference_surface`, which is what fixes its orientation. The trim
/// parameters take the form the release declares:
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
/// Like the surface-curve sweep, but the profile's orientation is fixed by a
/// direction rather than by a surface. The trim parameters are bare in IFC4
/// and `IFCPARAMETERVALUE(..)` in IFC4X3. IFC2X3 does not declare the
/// entity. A model without `FILE_SCHEMA` binds IFC4.
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

/// Stage an `IfcSweptDiskSolid` in `model`'s declared release: a disk swept
/// along a curve.
///
/// Note the slot layout: this subtypes `IfcSolidModel` directly, so
/// `Directrix` is slot 0 and there is no `SweptArea` or `Position`.
///
/// The trim parameters are `IfcParameterValue`, written bare, in every
/// release; what changes is whether they may be left out:
///
/// ```text
/// IFC2X3  StartParam : IfcParameterValue           (required)
/// IFC4    StartParam : OPTIONAL IfcParameterValue
/// IFC4X3  StartParam : OPTIONAL IfcParameterValue
/// ```
///
/// An IFC2X3 swept disk therefore needs both trim values; the writer does
/// not invent the directrix's full range, which only an evaluator knows. A
/// model without `FILE_SCHEMA` binds IFC4.
///
/// # Errors
///
/// [`GeometryError::InvalidAuthoredValue`] for a non-positive radius or
/// inner radius, an inner radius that is not smaller than the outer one (a
/// bore wider than its tube leaves no solid), a non-finite trim parameter,
/// or, in IFC2X3, an unset `StartParam` or `EndParam`.
/// [`GeometryError::AuthoringSchemaUnbound`] if `FILE_SCHEMA` names an
/// unknown release or several. Nothing is staged on error.
pub fn swept_disk_solid_in(
    tx: &mut Transaction,
    model: &Model,
    directrix: EntityId,
    radius: f64,
    inner_radius: Option<f64>,
    trim: SweepTrim,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCSWEPTDISKSOLID";
    let release = bind(model, T)?;
    let form = ParamForm::Release(release);
    let attrs = disk_attrs(T, directrix, radius, inner_radius, trim, form)?;
    release.require(T, &attrs)?;
    Ok(tx.create(Entity::new(T, attrs)))
}
