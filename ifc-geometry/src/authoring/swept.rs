//! The swept solids beyond plain extrusion and revolution.
//!
//! Three shapes of sweep, and the slot layouts do not generalise:
//!
//! - **Tapered** extrusion and revolution add `EndSweptArea`, so the
//!   profile changes along the sweep. Both profiles are references and
//!   neither is evaluated here.
//! - **Directrix-driven** sweeps (`IfcSurfaceCurveSweptAreaSolid`,
//!   `IfcFixedReferenceSweptAreaSolid`) share slots 0-4 and differ only
//!   at slot 5, where one names a reference surface and the other a
//!   fixed direction.
//! - **`IfcSweptDiskSolid`** subtypes `IfcSolidModel` *directly*. It has
//!   no inherited `SweptArea`, so `Directrix` is slot 0, not slot 2.
//!   That break in the family resemblance is the easiest thing to get
//!   wrong here.
//!
//! The one invariant worth enforcing is the disk solid's
//! `InnerRadius < Radius`: a hollow tube whose bore is wider than the
//! tube is not a solid, and that is arithmetic rather than geometry.

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::error::GeometryError;
use crate::solid::swept::{
    directrix_slot, disk_slot, extruded_slot, revolved_slot, swept_area_slot,
};

use super::std_profile::positive;
use super::{invalid, require_finite};

/// Optional trim parameters shared by the directrix-driven sweeps.
///
/// Absent means the sweep runs the whole directrix. Both are
/// `IfcParameterValue`, so they carry their measure type when written.
#[derive(Debug, Default, Clone, Copy)]
pub struct SweepTrim {
    /// `StartParam`.
    pub start: Option<f64>,
    /// `EndParam`.
    pub end: Option<f64>,
}

/// Write an optional `IfcParameterValue` into its slot.
fn put_param(
    attrs: &mut [Value],
    index: usize,
    value: Option<f64>,
    type_name: &'static str,
    attribute: &'static str,
) -> Result<(), GeometryError> {
    if let Some(value) = value {
        require_finite(type_name, attribute, &[value])?;
        attrs[index] = Value::Typed {
            type_name: "IFCPARAMETERVALUE".into(),
            value: Box::new(Value::Real(value)),
        };
    }
    Ok(())
}

/// Stage an `IfcExtrudedAreaSolidTapered`.
///
/// # Errors
///
/// Refuses a non-positive or non-finite depth.
pub fn extruded_area_solid_tapered(
    tx: &mut Transaction,
    swept_area: EntityId,
    position: Option<EntityId>,
    extruded_direction: EntityId,
    depth: f64,
    end_swept_area: EntityId,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCEXTRUDEDAREASOLIDTAPERED";
    positive(T, "Depth", depth)?;
    let mut attrs = vec![Value::Null; 5];
    attrs[swept_area_slot::SWEPT_AREA] = Value::Ref(swept_area);
    attrs[swept_area_slot::POSITION] = position.map_or(Value::Null, Value::Ref);
    attrs[extruded_slot::EXTRUDED_DIRECTION] = Value::Ref(extruded_direction);
    attrs[extruded_slot::DEPTH] = Value::Real(depth);
    attrs[extruded_slot::END_SWEPT_AREA] = Value::Ref(end_swept_area);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcRevolvedAreaSolidTapered`.
///
/// `angle` is in the model's plane-angle unit, very often degrees. The
/// writer does not convert: it has no unit context and inventing one
/// would silently rescale the solid.
///
/// # Errors
///
/// Refuses a non-finite angle.
pub fn revolved_area_solid_tapered(
    tx: &mut Transaction,
    swept_area: EntityId,
    position: Option<EntityId>,
    axis: EntityId,
    angle: f64,
    end_swept_area: EntityId,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCREVOLVEDAREASOLIDTAPERED";
    require_finite(T, "Angle", &[angle])?;
    let mut attrs = vec![Value::Null; 5];
    attrs[swept_area_slot::SWEPT_AREA] = Value::Ref(swept_area);
    attrs[swept_area_slot::POSITION] = position.map_or(Value::Null, Value::Ref);
    attrs[revolved_slot::AXIS] = Value::Ref(axis);
    attrs[revolved_slot::ANGLE] = Value::Real(angle);
    attrs[revolved_slot::END_SWEPT_AREA] = Value::Ref(end_swept_area);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcSurfaceCurveSweptAreaSolid`.
///
/// The profile is swept along `directrix` while staying on
/// `reference_surface`, which is what fixes its orientation.
///
/// # Errors
///
/// Refuses a non-finite trim parameter.
pub fn surface_curve_swept_area_solid(
    tx: &mut Transaction,
    swept_area: EntityId,
    position: Option<EntityId>,
    directrix: EntityId,
    trim: SweepTrim,
    reference_surface: EntityId,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCSURFACECURVESWEPTAREASOLID";
    let mut attrs = directrix_attrs(T, swept_area, position, directrix, trim)?;
    attrs[directrix_slot::REFERENCE_SURFACE] = Value::Ref(reference_surface);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcFixedReferenceSweptAreaSolid`.
///
/// Like the surface-curve sweep, but the profile's orientation is fixed
/// by a direction rather than by a surface.
///
/// # Errors
///
/// Refuses a non-finite trim parameter.
pub fn fixed_reference_swept_area_solid(
    tx: &mut Transaction,
    swept_area: EntityId,
    position: Option<EntityId>,
    directrix: EntityId,
    trim: SweepTrim,
    fixed_reference: EntityId,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCFIXEDREFERENCESWEPTAREASOLID";
    let mut attrs = directrix_attrs(T, swept_area, position, directrix, trim)?;
    attrs[directrix_slot::FIXED_REFERENCE] = Value::Ref(fixed_reference);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// The six slots the two directrix-driven sweeps share.
///
/// Slot 5 is left for the caller: it is `ReferenceSurface` on one
/// subtype and `FixedReference` on the other.
fn directrix_attrs(
    type_name: &'static str,
    swept_area: EntityId,
    position: Option<EntityId>,
    directrix: EntityId,
    trim: SweepTrim,
) -> Result<Vec<Value>, GeometryError> {
    let mut attrs = vec![Value::Null; 6];
    attrs[swept_area_slot::SWEPT_AREA] = Value::Ref(swept_area);
    attrs[swept_area_slot::POSITION] = position.map_or(Value::Null, Value::Ref);
    attrs[directrix_slot::DIRECTRIX] = Value::Ref(directrix);
    put_param(
        &mut attrs,
        directrix_slot::START_PARAM,
        trim.start,
        type_name,
        "StartParam",
    )?;
    put_param(
        &mut attrs,
        directrix_slot::END_PARAM,
        trim.end,
        type_name,
        "EndParam",
    )?;
    Ok(attrs)
}

/// Stage an `IfcSweptDiskSolid`: a disk swept along a curve.
///
/// Note the slot layout: this subtypes `IfcSolidModel` directly, so
/// `Directrix` is slot 0 and there is no `SweptArea` or `Position`.
///
/// # Errors
///
/// Refuses a non-positive radius, a non-positive inner radius, a
/// non-finite trim parameter, or an inner radius that is not smaller
/// than the outer one -- a bore wider than its tube leaves no solid.
pub fn swept_disk_solid(
    tx: &mut Transaction,
    directrix: EntityId,
    radius: f64,
    inner_radius: Option<f64>,
    trim: SweepTrim,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCSWEPTDISKSOLID";
    let attrs = disk_attrs(T, directrix, radius, inner_radius, trim)?;
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcSweptDiskSolidPolygonal`.
///
/// The polygonal form approximates the directrix with straight
/// segments; `fillet_radius` rounds the joints between them.
///
/// # Errors
///
/// Everything [`swept_disk_solid`] refuses, plus a negative fillet
/// radius. Zero is legal: `IfcNonNegativeLengthMeasure` means a sharp
/// joint, not an error.
pub fn swept_disk_solid_polygonal(
    tx: &mut Transaction,
    directrix: EntityId,
    radius: f64,
    inner_radius: Option<f64>,
    trim: SweepTrim,
    fillet_radius: Option<f64>,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCSWEPTDISKSOLIDPOLYGONAL";
    let mut attrs = disk_attrs(T, directrix, radius, inner_radius, trim)?;
    attrs.push(Value::Null);
    if let Some(fillet) = fillet_radius {
        require_finite(T, "FilletRadius", &[fillet])?;
        if fillet < 0.0 {
            return Err(invalid(
                T,
                "FilletRadius",
                format!("expected a non-negative length, got {fillet}"),
            ));
        }
        attrs[disk_slot::FILLET_RADIUS] = Value::Real(fillet);
    }
    Ok(tx.create(Entity::new(T, attrs)))
}

/// The five slots both swept disk solids share.
fn disk_attrs(
    type_name: &'static str,
    directrix: EntityId,
    radius: f64,
    inner_radius: Option<f64>,
    trim: SweepTrim,
) -> Result<Vec<Value>, GeometryError> {
    positive(type_name, "Radius", radius)?;
    let mut attrs = vec![Value::Null; 5];
    attrs[disk_slot::DIRECTRIX] = Value::Ref(directrix);
    attrs[disk_slot::RADIUS] = Value::Real(radius);
    if let Some(inner) = inner_radius {
        positive(type_name, "InnerRadius", inner)?;
        // A bore at least as wide as the tube leaves nothing solid. The
        // schema types both as positive lengths but does not relate them,
        // so this is the writer's to catch.
        if inner >= radius {
            return Err(invalid(
                type_name,
                "InnerRadius",
                format!("{inner} is not below the outer radius {radius}"),
            ));
        }
        attrs[disk_slot::INNER_RADIUS] = Value::Real(inner);
    }
    put_param(
        &mut attrs,
        disk_slot::START_PARAM,
        trim.start,
        type_name,
        "StartParam",
    )?;
    put_param(
        &mut attrs,
        disk_slot::END_PARAM,
        trim.end,
        type_name,
        "EndParam",
    )?;
    Ok(attrs)
}

/// Stage an `IfcSurfaceOfLinearExtrusion`: a surface, not a solid.
///
/// `SweptCurve` is an `IfcProfileDef`, and `Depth` is a plain
/// `IfcLengthMeasure` here -- negative extrusion is legal, unlike the
/// solid form which requires a positive depth.
///
/// # Errors
///
/// Refuses a non-finite depth.
pub fn surface_of_linear_extrusion(
    tx: &mut Transaction,
    swept_curve: EntityId,
    position: Option<EntityId>,
    extruded_direction: EntityId,
    depth: f64,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCSURFACEOFLINEAREXTRUSION";
    require_finite(T, "Depth", &[depth])?;
    let attrs = vec![
        Value::Ref(swept_curve),
        position.map_or(Value::Null, Value::Ref),
        Value::Ref(extruded_direction),
        Value::Real(depth),
    ];
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcSurfaceOfRevolution`.
///
/// There is no angle: the surface is the full revolution of the swept
/// curve about `axis_position`.
pub fn surface_of_revolution(
    tx: &mut Transaction,
    swept_curve: EntityId,
    position: Option<EntityId>,
    axis_position: EntityId,
) -> EntityId {
    let attrs = vec![
        Value::Ref(swept_curve),
        position.map_or(Value::Null, Value::Ref),
        Value::Ref(axis_position),
    ];
    tx.create(Entity::new("IFCSURFACEOFREVOLUTION", attrs))
}

/// Stage an `IfcPlane`.
pub fn plane(tx: &mut Transaction, position: EntityId) -> EntityId {
    tx.create(Entity::new("IFCPLANE", vec![Value::Ref(position)]))
}

/// Stage an `IfcCylindricalSurface`.
///
/// # Errors
///
/// Refuses a non-positive or non-finite radius.
pub fn cylindrical_surface(
    tx: &mut Transaction,
    position: EntityId,
    radius: f64,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCCYLINDRICALSURFACE";
    positive(T, "Radius", radius)?;
    let attrs = vec![Value::Ref(position), Value::Real(radius)];
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcAxis1Placement`: a point and an optional axis direction.
///
/// Revolutions need one, and it is the only placement form the other
/// authoring modules do not already cover.
pub fn axis1_placement(
    tx: &mut Transaction,
    location: EntityId,
    axis: Option<EntityId>,
) -> EntityId {
    let attrs = vec![Value::Ref(location), axis.map_or(Value::Null, Value::Ref)];
    tx.create(Entity::new("IFCAXIS1PLACEMENT", attrs))
}
