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
//!
//! The directrix sweeps and the plain swept disk are written by the
//! release-bound writers in `swept_in.rs`, because their trim parameters
//! change form or optionality between releases (#200, #210). This module
//! keeps the shared slot layouts and the writers whose record is the same
//! in every release that declares the entity.

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::error::GeometryError;
use crate::solid::swept::{
    directrix_slot, disk_slot, extruded_slot, revolved_slot, swept_area_slot,
};

use super::release::Release;
use super::std_profile::positive;
use super::{invalid, refs, require_finite};

/// Optional trim parameters shared by the directrix-driven sweeps.
///
/// Absent means the sweep runs the whole directrix. Both are parameter
/// values on the directrix. How they are written depends on the declared
/// type of the slot (#200): bare where it is `IfcParameterValue` (the swept
/// disks in every release, the other sweeps in IFC2X3 and IFC4), and
/// `IFCPARAMETERVALUE(..)` where it is the IFC4X3 SELECT
/// `IfcCurveMeasureSelect`. Whether absent is allowed depends on the release
/// too: IFC2X3 requires both on every sweep that declares them, which is why
/// the writers of those sweeps take the model (#210).
#[derive(Debug, Default, Clone, Copy)]
pub struct SweepTrim {
    /// `StartParam`.
    pub start: Option<f64>,
    /// `EndParam`.
    pub end: Option<f64>,
}

/// How a trim parameter is written.
#[derive(Debug, Clone, Copy)]
pub(super) enum ParamForm {
    /// The slot is declared `IfcParameterValue`, a defined type: bare.
    Bare,
    /// The slot is declared `IfcCurveMeasureSelect`, a SELECT: typed.
    Select,
    /// The form the model's declared release requires.
    Release(Release),
}

/// Write an optional trim parameter into its slot in `form`.
fn put_param(
    attrs: &mut [Value],
    index: usize,
    value: Option<f64>,
    form: ParamForm,
    type_name: &'static str,
    attribute: &'static str,
) -> Result<(), GeometryError> {
    if let Some(value) = value {
        require_finite(type_name, attribute, &[value])?;
    }
    attrs[index] = match (form, value) {
        (ParamForm::Release(release), value) => release.parameter(type_name, attribute, value)?,
        (_, None) => Value::Null,
        (ParamForm::Bare, Some(value)) => Value::Real(value),
        (ParamForm::Select, Some(value)) => Value::Typed {
            type_name: "IFCPARAMETERVALUE".into(),
            value: Box::new(Value::Real(value)),
        },
    };
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

/// The six slots the directrix-driven sweeps share, trim parameters in
/// `form`.
///
/// Slot 5 is left for the caller: it is `ReferenceSurface` on one
/// subtype and `FixedReference` on the others.
pub(super) fn directrix_attrs(
    type_name: &'static str,
    swept_area: EntityId,
    position: Option<EntityId>,
    directrix: EntityId,
    trim: SweepTrim,
    form: ParamForm,
) -> Result<Vec<Value>, GeometryError> {
    let mut attrs = vec![Value::Null; 6];
    attrs[swept_area_slot::SWEPT_AREA] = Value::Ref(swept_area);
    attrs[swept_area_slot::POSITION] = position.map_or(Value::Null, Value::Ref);
    attrs[directrix_slot::DIRECTRIX] = Value::Ref(directrix);
    put_param(
        &mut attrs,
        directrix_slot::START_PARAM,
        trim.start,
        form,
        type_name,
        "StartParam",
    )?;
    put_param(
        &mut attrs,
        directrix_slot::END_PARAM,
        trim.end,
        form,
        type_name,
        "EndParam",
    )?;
    Ok(attrs)
}

/// Stage an `IfcSweptDiskSolidPolygonal`.
///
/// The polygonal form approximates the directrix with straight
/// segments; `fillet_radius` rounds the joints between them.
///
/// Only IFC4 and later declare it, and there `StartParam`/`EndParam` are
/// `OPTIONAL IfcParameterValue` in every release, so the record does not
/// depend on the release and this writer needs no model.
///
/// # Errors
///
/// Everything [`swept_disk_solid_in`](super::swept_disk_solid_in) refuses
/// for the radii and trim values, plus a negative fillet radius. Zero is
/// legal: `IfcNonNegativeLengthMeasure` means a sharp joint, not an error.
pub fn swept_disk_solid_polygonal(
    tx: &mut Transaction,
    directrix: EntityId,
    radius: f64,
    inner_radius: Option<f64>,
    trim: SweepTrim,
    fillet_radius: Option<f64>,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCSWEPTDISKSOLIDPOLYGONAL";
    let mut attrs = disk_attrs(T, directrix, radius, inner_radius, trim, ParamForm::Bare)?;
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

/// The five slots both swept disk solids share, trim parameters in `form`.
pub(super) fn disk_attrs(
    type_name: &'static str,
    directrix: EntityId,
    radius: f64,
    inner_radius: Option<f64>,
    trim: SweepTrim,
    form: ParamForm,
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
    // `IfcParameterValue` on both swept disks in every release: bare (#200).
    put_param(
        &mut attrs,
        disk_slot::START_PARAM,
        trim.start,
        form,
        type_name,
        "StartParam",
    )?;
    put_param(
        &mut attrs,
        disk_slot::END_PARAM,
        trim.end,
        form,
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

/// Stage an `IfcDirectrixDerivedReferenceSweptAreaSolid`.
///
/// Shares the fixed-reference layout exactly, but the reference is
/// *derived from* the directrix rather than held constant: the profile
/// rotates with the curve as it sweeps. Same six slots, different
/// meaning, so it is its own entity rather than a flag.
///
/// Only IFC4X3 declares it, and there the trim parameters are the SELECT
/// `IfcCurveMeasureSelect`, so they are written `IFCPARAMETERVALUE(..)`.
///
/// # Errors
///
/// Refuses a non-finite trim parameter.
pub fn directrix_derived_reference_swept_area_solid(
    tx: &mut Transaction,
    swept_area: EntityId,
    position: Option<EntityId>,
    directrix: EntityId,
    trim: SweepTrim,
    fixed_reference: EntityId,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCDIRECTRIXDERIVEDREFERENCESWEPTAREASOLID";
    let mut attrs = directrix_attrs(T, swept_area, position, directrix, trim, ParamForm::Select)?;
    attrs[directrix_slot::FIXED_REFERENCE] = Value::Ref(fixed_reference);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Which sectioned entity to stage.
///
/// The two carry the same three attributes but in *different slot
/// order*: the solid is Directrix, CrossSections, CrossSectionPositions
/// while the surface is Directrix, CrossSectionPositions, CrossSections.
/// Writing one layout under the other type name produces a record that
/// parses and is wrong, so the order is selected here rather than left
/// to the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionedKind {
    /// `IfcSectionedSolidHorizontal`.
    SolidHorizontal,
    /// `IfcSectionedSurface`.
    Surface,
}

/// Stage an `IfcSectionedSolidHorizontal` or `IfcSectionedSurface`.
///
/// Both sweep a series of cross-sections along a directrix, each
/// positioned by an `IfcAxis2PlacementLinear`. Both require at least
/// two sections and one position per section.
///
/// # Errors
///
/// Refuses fewer than two cross-sections or positions (`LIST [2:?]`)
/// and a count mismatch between them
/// (`CorrespondingSectionPositions`).
pub fn sectioned(
    tx: &mut Transaction,
    kind: SectionedKind,
    directrix: EntityId,
    cross_sections: &[EntityId],
    cross_section_positions: &[EntityId],
) -> Result<EntityId, GeometryError> {
    let entity = match kind {
        SectionedKind::SolidHorizontal => "IFCSECTIONEDSOLIDHORIZONTAL",
        SectionedKind::Surface => "IFCSECTIONEDSURFACE",
    };
    if cross_sections.len() < 2 {
        return Err(invalid(entity, "CrossSections", "expected LIST [2:?]"));
    }
    if cross_section_positions.len() < 2 {
        return Err(invalid(
            entity,
            "CrossSectionPositions",
            "expected LIST [2:?]",
        ));
    }
    if cross_sections.len() != cross_section_positions.len() {
        return Err(invalid(
            entity,
            "CrossSectionPositions",
            format!(
                "expected one position per section: {} sections, {} positions",
                cross_sections.len(),
                cross_section_positions.len()
            ),
        ));
    }
    let attrs = match kind {
        SectionedKind::SolidHorizontal => vec![
            Value::Ref(directrix),
            refs(cross_sections),
            refs(cross_section_positions),
        ],
        SectionedKind::Surface => vec![
            Value::Ref(directrix),
            refs(cross_section_positions),
            refs(cross_sections),
        ],
    };
    Ok(tx.create(Entity::new(entity, attrs)))
}
