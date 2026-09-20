//! Coordinate operations: how a local engineering grid relates to a map.
//!
//! # The rule that makes a map conversion meaningful
//!
//! `IfcMapConversion` carries `TargetCRSOnlyProjected`:
//!
//! ```text
//! 'IFC4X3_ADD2.IFCPROJECTEDCRS' IN TYPEOF(SELF\IfcCoordinateOperation.TargetCRS)
//! ```
//!
//! Eastings, northings and an orthogonal height are coordinates *on a
//! projection plane*. Pointing them at an `IfcGeographicCRS` -- whose
//! axes are latitude and longitude in degrees -- produces numbers that
//! parse but denote nothing: a 400000.0 "easting" in a degree-based
//! system is off the planet. So the target is checked here rather than
//! left to a validator.
//!
//! # Why the X axis is two reals and not an angle
//!
//! `XAxisAbscissa` and `XAxisOrdinate` are the components of a
//! direction vector, not a bearing. Supplying only one leaves the
//! rotation underdetermined, and supplying `(0, 0)` gives a zero-length
//! vector with no direction at all. Both are refused.

use ifc_model::{Entity, EntityId, Model, Transaction, Value};

use crate::authoring::invalid;
use crate::GeorefResult;

/// What to stage for an `IfcGeographicCRS`.
#[derive(Debug, Clone, Copy, Default)]
pub struct GeographicCrsDraft<'a> {
    /// `Name`: the CRS identifier, e.g. `EPSG:4326`.
    pub name: Option<&'a str>,
    /// `Description`.
    pub description: Option<&'a str>,
    /// `GeodeticDatum`.
    pub geodetic_datum: Option<&'a str>,
    /// `PrimeMeridian`.
    pub prime_meridian: Option<&'a str>,
    /// `AngleUnit`: an `IfcNamedUnit` for latitude and longitude.
    pub angle_unit: Option<EntityId>,
    /// `HeightUnit`: an `IfcNamedUnit` for ellipsoidal height.
    pub height_unit: Option<EntityId>,
}

/// Stage an `IfcGeographicCRS`.
///
/// Every attribute is optional in the schema, but a CRS with no name
/// identifies nothing, so a blank name is refused rather than written
/// as `$`.
///
/// # Errors
///
/// Refuses a name that is present but blank.
pub fn create_geographic_crs(
    tx: &mut Transaction,
    draft: GeographicCrsDraft<'_>,
) -> GeorefResult<EntityId> {
    if let Some(name) = draft.name {
        if name.trim().is_empty() {
            return Err(invalid("IFCGEOGRAPHICCRS", "Name", name));
        }
    }

    Ok(tx.create(Entity::new(
        "IFCGEOGRAPHICCRS",
        vec![
            draft.name.map_or(Value::Null, |v| Value::Text(v.into())),
            draft
                .description
                .map_or(Value::Null, |v| Value::Text(v.into())),
            draft
                .geodetic_datum
                .map_or(Value::Null, |v| Value::Text(v.into())),
            draft
                .prime_meridian
                .map_or(Value::Null, |v| Value::Text(v.into())),
            draft.angle_unit.map_or(Value::Null, Value::Ref),
            draft.height_unit.map_or(Value::Null, Value::Ref),
        ],
    )))
}

/// The offset and rotation placing a local grid on a projection.
#[derive(Debug, Clone, Copy)]
pub struct MapConversionDraft {
    /// `SourceCRS`: usually the model's engineering context.
    pub source_crs: EntityId,
    /// `TargetCRS`: must be an `IfcProjectedCRS`.
    pub target_crs: EntityId,
    /// `Eastings`.
    pub eastings: f64,
    /// `Northings`.
    pub northings: f64,
    /// `OrthogonalHeight`.
    pub orthogonal_height: f64,
    /// `XAxisAbscissa` and `XAxisOrdinate`, the rotation vector.
    ///
    /// Both components or neither: a single one underdetermines the
    /// rotation.
    pub x_axis: Option<(f64, f64)>,
    /// `Scale`.
    pub scale: Option<f64>,
}

fn resolved_type(tx: &Transaction, model: &Model, target: EntityId) -> Option<String> {
    tx.edits()
        .iter()
        .rev()
        .find_map(|edit| match edit {
            ifc_model::Edit::Create { id, entity } if *id == target => {
                Some(entity.type_name.as_ref().to_owned())
            }
            _ => None,
        })
        .or_else(|| model.get(target).map(|e| e.type_name.as_ref().to_owned()))
}

fn check_conversion(
    entity: &'static str,
    tx: &Transaction,
    model: &Model,
    draft: &MapConversionDraft,
) -> GeorefResult<()> {
    let finite = [
        ("Eastings", draft.eastings),
        ("Northings", draft.northings),
        ("OrthogonalHeight", draft.orthogonal_height),
    ];
    for (attribute, value) in finite {
        if !value.is_finite() {
            return Err(invalid(entity, attribute, value.to_string()));
        }
    }

    match resolved_type(tx, model, draft.target_crs).as_deref() {
        Some("IFCPROJECTEDCRS") => {}
        Some(other) => {
            return Err(invalid(
                entity,
                "TargetCRS",
                format!("expected IFCPROJECTEDCRS, found {other}"),
            ))
        }
        None => {
            return Err(invalid(
                entity,
                "TargetCRS",
                "target does not resolve to a staged entity",
            ))
        }
    }

    if let Some((abscissa, ordinate)) = draft.x_axis {
        if !abscissa.is_finite() || !ordinate.is_finite() {
            return Err(invalid(
                entity,
                "XAxisAbscissa",
                format!("expected a finite direction, got ({abscissa}, {ordinate})"),
            ));
        }
        if abscissa == 0.0 && ordinate == 0.0 {
            return Err(invalid(
                entity,
                "XAxisAbscissa",
                "a zero-length vector gives no rotation",
            ));
        }
    }

    if let Some(scale) = draft.scale {
        if !scale.is_finite() || scale == 0.0 {
            return Err(invalid(
                entity,
                "Scale",
                format!("expected a non-zero finite scale, got {scale}"),
            ));
        }
    }

    Ok(())
}

fn conversion_attributes(draft: &MapConversionDraft) -> Vec<Value> {
    let (abscissa, ordinate) = match draft.x_axis {
        Some((a, o)) => (Value::Real(a), Value::Real(o)),
        None => (Value::Null, Value::Null),
    };
    vec![
        Value::Ref(draft.source_crs),
        Value::Ref(draft.target_crs),
        Value::Real(draft.eastings),
        Value::Real(draft.northings),
        Value::Real(draft.orthogonal_height),
        abscissa,
        ordinate,
        draft.scale.map_or(Value::Null, Value::Real),
    ]
}

/// Stage an `IfcMapConversion`.
///
/// # Errors
///
/// Refuses a non-finite coordinate, a `TargetCRS` that is not an
/// `IfcProjectedCRS`, a partially specified or zero-length X axis, and
/// a zero or non-finite scale.
pub fn create_map_conversion(
    tx: &mut Transaction,
    model: &Model,
    draft: MapConversionDraft,
) -> GeorefResult<EntityId> {
    check_conversion("IFCMAPCONVERSION", tx, model, &draft)?;
    Ok(tx.create(Entity::new(
        "IFCMAPCONVERSION",
        conversion_attributes(&draft),
    )))
}

/// Stage an `IfcMapConversionScaled`: per-axis scale factors.
///
/// Used where the projection distorts differently along each axis, so
/// one uniform `Scale` cannot describe it.
///
/// # Errors
///
/// Everything [`create_map_conversion`] refuses, plus a zero or
/// non-finite factor on any axis -- a zero factor collapses that axis
/// entirely.
pub fn create_map_conversion_scaled(
    tx: &mut Transaction,
    model: &Model,
    draft: MapConversionDraft,
    factors: (f64, f64, f64),
) -> GeorefResult<EntityId> {
    check_conversion("IFCMAPCONVERSIONSCALED", tx, model, &draft)?;

    let (x, y, z) = factors;
    for (attribute, value) in [("FactorX", x), ("FactorY", y), ("FactorZ", z)] {
        if !value.is_finite() || value == 0.0 {
            return Err(invalid(
                "IFCMAPCONVERSIONSCALED",
                attribute,
                format!("expected a non-zero finite factor, got {value}"),
            ));
        }
    }

    let mut attributes = conversion_attributes(&draft);
    attributes.extend([Value::Real(x), Value::Real(y), Value::Real(z)]);
    Ok(tx.create(Entity::new("IFCMAPCONVERSIONSCALED", attributes)))
}

/// Stage an `IfcRigidOperation`: a translation with no rotation or scale.
///
/// Unlike a map conversion this does not require a projected target:
/// a rigid shift is meaningful between any two systems sharing units.
///
/// # Errors
///
/// Refuses a non-finite coordinate or height.
pub fn create_rigid_operation(
    tx: &mut Transaction,
    source_crs: EntityId,
    target_crs: EntityId,
    coordinates: (f64, f64),
    height: Option<f64>,
) -> GeorefResult<EntityId> {
    let (first, second) = coordinates;
    for (attribute, value) in [("FirstCoordinate", first), ("SecondCoordinate", second)] {
        if !value.is_finite() {
            return Err(invalid("IFCRIGIDOPERATION", attribute, value.to_string()));
        }
    }
    if let Some(value) = height {
        if !value.is_finite() {
            return Err(invalid("IFCRIGIDOPERATION", "Height", value.to_string()));
        }
    }

    Ok(tx.create(Entity::new(
        "IFCRIGIDOPERATION",
        vec![
            Value::Ref(source_crs),
            Value::Ref(target_crs),
            Value::Real(first),
            Value::Real(second),
            height.map_or(Value::Null, Value::Real),
        ],
    )))
}
