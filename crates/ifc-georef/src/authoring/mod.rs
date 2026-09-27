//! Transactional authoring of the georeferencing entities.
//!
//! # Derived attributes are written as `*`, not `$`
//!
//! `IfcGeometricRepresentationSubContext` redeclares four inherited
//! attributes as DERIVE: they are taken from the parent context, not
//! supplied here. STEP spells a derived attribute as an asterisk, and
//! `$` would instead claim the value is absent -- a different
//! statement, and one that loses the parent's precision and world
//! coordinate system.

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::error::{GeorefError, GeorefResult};

mod operation;

pub use operation::{
    create_geographic_crs, create_map_conversion, create_map_conversion_scaled,
    create_rigid_operation, create_well_known_text, GeographicCrsDraft, MapConversionDraft,
};

fn invalid(entity: &'static str, attribute: &'static str, value: impl Into<String>) -> GeorefError {
    GeorefError::AuthoringInvalid {
        entity,
        attribute,
        value: value.into(),
    }
}

fn optional_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |t| Value::Text(t.into()))
}

/// Stage an `IfcDirection`.
///
/// # Errors
///
/// Refuses ratios that are not 2 or 3 long, which the schema types as
/// `LIST [2:3]`, and any non-finite or all-zero ratio: a direction of
/// zero length has no direction.
pub fn create_direction(tx: &mut Transaction, ratios: &[f64]) -> GeorefResult<EntityId> {
    if !(2..=3).contains(&ratios.len()) {
        return Err(invalid(
            "IFCDIRECTION",
            "DirectionRatios",
            format!("{} ratios", ratios.len()),
        ));
    }
    if ratios.iter().any(|r| !r.is_finite()) {
        return Err(invalid("IFCDIRECTION", "DirectionRatios", "not finite"));
    }
    if ratios.iter().all(|r| *r == 0.0) {
        return Err(invalid("IFCDIRECTION", "DirectionRatios", "zero length"));
    }
    Ok(tx.create(Entity::new(
        "IFCDIRECTION",
        vec![Value::List(
            ratios.iter().copied().map(Value::Real).collect(),
        )],
    )))
}

/// Authored fields for `IfcProjectedCRS`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProjectedCrsDraft<'a> {
    /// `Name`, the CRS identifier such as `EPSG:25832`.
    pub name: &'a str,
    /// `Description`.
    pub description: Option<&'a str>,
    /// `GeodeticDatum`.
    pub geodetic_datum: Option<&'a str>,
    /// `VerticalDatum`.
    pub vertical_datum: Option<&'a str>,
    /// `MapProjection`.
    pub map_projection: Option<&'a str>,
    /// `MapZone`.
    pub map_zone: Option<&'a str>,
    /// `MapUnit`, an `IfcNamedUnit` reference.
    pub map_unit: Option<EntityId>,
}

/// Stage an `IfcProjectedCRS`.
///
/// `Name` is optional in the schema but required here: the reader
/// resolves a CRS by name, and an unnamed one cannot be matched to
/// the projection it claims to use.
///
/// # Errors
///
/// Refuses a blank name.
pub fn create_projected_crs(
    tx: &mut Transaction,
    draft: ProjectedCrsDraft<'_>,
) -> GeorefResult<EntityId> {
    if draft.name.trim().is_empty() {
        return Err(invalid("IFCPROJECTEDCRS", "Name", draft.name));
    }
    Ok(tx.create(Entity::new(
        "IFCPROJECTEDCRS",
        vec![
            Value::Text(draft.name.into()),
            optional_text(draft.description),
            optional_text(draft.geodetic_datum),
            optional_text(draft.vertical_datum),
            optional_text(draft.map_projection),
            optional_text(draft.map_zone),
            draft.map_unit.map_or(Value::Null, Value::Ref),
        ],
    )))
}

/// Stage an `IfcGeometricRepresentationContext`.
///
/// # Errors
///
/// Refuses a coordinate space dimension outside 1..=3 and a
/// non-finite precision.
pub fn create_representation_context(
    tx: &mut Transaction,
    context_type: Option<&str>,
    dimension: i64,
    precision: Option<f64>,
    world_coordinate_system: EntityId,
    true_north: Option<EntityId>,
) -> GeorefResult<EntityId> {
    if !(1..=3).contains(&dimension) {
        return Err(invalid(
            "IFCGEOMETRICREPRESENTATIONCONTEXT",
            "CoordinateSpaceDimension",
            dimension.to_string(),
        ));
    }
    if precision.is_some_and(|p| !p.is_finite()) {
        return Err(invalid(
            "IFCGEOMETRICREPRESENTATIONCONTEXT",
            "Precision",
            "not finite",
        ));
    }
    Ok(tx.create(Entity::new(
        "IFCGEOMETRICREPRESENTATIONCONTEXT",
        vec![
            // ContextIdentifier at 0, ContextType at 1.
            Value::Null,
            optional_text(context_type),
            Value::Integer(dimension),
            precision.map_or(Value::Null, Value::Real),
            Value::Ref(world_coordinate_system),
            true_north.map_or(Value::Null, Value::Ref),
        ],
    )))
}

/// Stage an `IfcGeometricRepresentationSubContext`.
///
/// The four inherited geometry attributes are written as
/// `Value::Derived`: the schema computes them from `ParentContext`,
/// and writing `$` instead would claim the parent's precision and
/// world coordinate system are simply absent.
///
/// # Errors
///
/// Enforces the schema's two WHERE rules. ParentNoSub: a subcontext
/// cannot parent another subcontext, since the derived attributes
/// resolve one level only. UserTargetProvided: a USERDEFINED target
/// view without a name states a custom view and then fails to name
/// it, leaving nothing for a consumer to match on.
pub fn create_representation_subcontext(
    tx: &mut Transaction,
    parent: EntityId,
    parent_is_subcontext: bool,
    context_identifier: Option<&str>,
    target_view: &str,
    user_defined_target_view: Option<&str>,
) -> GeorefResult<EntityId> {
    if parent_is_subcontext {
        return Err(invalid(
            "IFCGEOMETRICREPRESENTATIONSUBCONTEXT",
            "ParentContext",
            "a subcontext cannot parent a subcontext",
        ));
    }
    if target_view.trim().is_empty() {
        return Err(invalid(
            "IFCGEOMETRICREPRESENTATIONSUBCONTEXT",
            "TargetView",
            target_view,
        ));
    }
    let named = user_defined_target_view.is_some_and(|v| !v.trim().is_empty());
    if target_view == "USERDEFINED" && !named {
        return Err(invalid(
            "IFCGEOMETRICREPRESENTATIONSUBCONTEXT",
            "UserDefinedTargetView",
            "required when TargetView is USERDEFINED",
        ));
    }
    Ok(tx.create(Entity::new(
        "IFCGEOMETRICREPRESENTATIONSUBCONTEXT",
        vec![
            optional_text(context_identifier),
            Value::Null,
            // CoordinateSpaceDimension, Precision, WorldCoordinateSystem
            // and TrueNorth: all DERIVE from ParentContext.
            Value::Derived,
            Value::Derived,
            Value::Derived,
            Value::Derived,
            Value::Ref(parent),
            Value::Null,
            Value::Enum(target_view.into()),
            optional_text(user_defined_target_view),
        ],
    )))
}
