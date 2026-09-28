//! `IfcGrid`, the one `IfcRoot` this module authors.
//!
//! A grid is an `IfcProduct`, so it carries `IfcRoot.OwnerHistory`, which
//! IFC2X3 requires and IFC4 made optional, and its attribute count differs
//! by release (IFC2X3 declares no `PredefinedType`):
//!
//! ```text
//! IFC2X3_TC1   OwnerHistory : IfcOwnerHistory;           10 attributes
//! IFC4         OwnerHistory : OPTIONAL IfcOwnerHistory;  11 attributes
//! IFC4X3_ADD2  OwnerHistory : OPTIONAL IfcOwnerHistory;  11 attributes
//! ```
//!
//! [`grid`] takes no model and writes the IFC4 layout.
//! [`grid_with_owner_history`] binds the model's declared release through
//! `release.rs`, as the `_in` sweep writers do (#200), and lays the record
//! out by attribute name from its table (#202).

use ifc_model::guid::Guid;
use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::TypeKind;

use crate::error::GeometryError;

use super::release::bind;
use super::{invalid, refs};

const ENTITY: &str = "IFCGRID";

/// Stage an `IfcGrid`: the U/V/W axis system a plan is dimensioned
/// against.
///
/// `UAxes` and `VAxes` are `LIST [1:?] OF UNIQUE`, so a grid needs at
/// least one axis in each direction and may not list the same axis
/// twice. A repeated axis is not a harmless duplicate: it makes the
/// grid ambiguous about which intersection a gridline names.
///
/// # Release
///
/// Takes no model, so it writes the IFC4 layout (11 attributes) with
/// `OwnerHistory` `$`. That is valid IFC4 and IFC4X3 and never valid
/// IFC2X3, which requires `OwnerHistory` and declares 10 attributes. Use
/// [`grid_with_owner_history`] to write the model's declared release.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty U or V list, and a repeated
/// axis within any one list.
pub fn grid(
    tx: &mut Transaction,
    global_id: &str,
    placement: Option<EntityId>,
    axes: (&[EntityId], &[EntityId], &[EntityId]),
    predefined_type: Option<&str>,
) -> Result<EntityId, GeometryError> {
    check(global_id, axes)?;
    let (u_axes, v_axes, w_axes) = axes;
    let mut attrs = vec![Value::Null; 11];
    attrs[0] = Value::Text(global_id.into());
    attrs[5] = placement.map_or(Value::Null, Value::Ref);
    attrs[7] = refs(u_axes);
    attrs[8] = refs(v_axes);
    if !w_axes.is_empty() {
        attrs[9] = refs(w_axes);
    }
    attrs[10] = predefined_type.map_or(Value::Null, |t| Value::Enum(t.into()));
    Ok(tx.create(Entity::new(ENTITY, attrs)))
}

/// [`grid`] in the model's declared release, with a caller-supplied
/// `IfcOwnerHistory`, which IFC2X3 requires on every `IfcRoot`.
///
/// The record is laid out by attribute name from the bound release's
/// table, so an IFC2X3 grid has its 10 attributes, not IFC4's 11.
/// `owner_history` must be in the model or staged earlier on `tx`, and
/// must be an `IfcOwnerHistory`; one is never invented here (build it with
/// `ifc-author`). In IFC4 and IFC4X3 the reference fills the optional
/// slot, and the record is otherwise the one [`grid`] writes.
///
/// # Errors
///
/// Those of [`grid`], and: [`GeometryError::AuthoringSchemaUnbound`] for a
/// model that binds no single known release;
/// [`GeometryError::InvalidAuthoredValue`] for a `predefined_type` in
/// IFC2X3, which declares none, or outside the release's
/// `IfcGridTypeEnum`, and on `OwnerHistory` for an `owner_history` that
/// does not resolve or is another entity. Nothing is staged on an error.
pub fn grid_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    placement: Option<EntityId>,
    axes: (&[EntityId], &[EntityId], &[EntityId]),
    predefined_type: Option<&str>,
    owner_history: EntityId,
) -> Result<EntityId, GeometryError> {
    let release = bind(model, ENTITY)?;
    check(global_id, axes)?;
    let (schema, version) = (release.schema(), release.version());
    let declared = schema.attributes(ENTITY);
    if let Some(token) = predefined_type {
        let predefined = declared
            .iter()
            .find(|attribute| attribute.name.eq_ignore_ascii_case("PredefinedType"))
            .ok_or_else(|| {
                invalid(
                    ENTITY,
                    "PredefinedType",
                    format!("{version:?} declares no such attribute"),
                )
            })?;
        let members = match schema.type_def(&predefined.type_name).map(|t| &t.kind) {
            Some(TypeKind::Enumeration(members)) => members.as_slice(),
            _ => &[],
        };
        if !members.iter().any(|member| member == token) {
            return Err(invalid(
                ENTITY,
                "PredefinedType",
                format!("{token} is not a {} token", predefined.type_name),
            ));
        }
    }
    require_owner_history(tx, model, owner_history)?;

    let (u_axes, v_axes, w_axes) = axes;
    let w_axes = if w_axes.is_empty() {
        Value::Null
    } else {
        refs(w_axes)
    };
    let values = [
        ("GlobalId", Value::Text(global_id.into())),
        ("OwnerHistory", Value::Ref(owner_history)),
        ("ObjectPlacement", placement.map_or(Value::Null, Value::Ref)),
        ("UAxes", refs(u_axes)),
        ("VAxes", refs(v_axes)),
        ("WAxes", w_axes),
        (
            "PredefinedType",
            predefined_type.map_or(Value::Null, |t| Value::Enum(t.into())),
        ),
    ];
    let mut attrs = vec![Value::Null; declared.len()];
    for (attribute, value) in values {
        match declared
            .iter()
            .position(|found| found.name.eq_ignore_ascii_case(attribute))
        {
            Some(slot) => attrs[slot] = value,
            None if value == Value::Null => {}
            None => {
                return Err(invalid(
                    ENTITY,
                    attribute,
                    format!("{version:?} declares no such attribute"),
                ))
            }
        }
    }
    release.require(ENTITY, &attrs)?;
    Ok(tx.create(Entity::new(ENTITY, attrs)))
}

/// The GlobalId and the axis lists, whatever the release.
fn check(
    global_id: &str,
    (u_axes, v_axes, w_axes): (&[EntityId], &[EntityId], &[EntityId]),
) -> Result<(), GeometryError> {
    if Guid::parse(global_id).is_none() {
        return Err(invalid(ENTITY, "GlobalId", global_id));
    }
    for (values, attribute) in [(u_axes, "UAxes"), (v_axes, "VAxes")] {
        if values.is_empty() {
            return Err(invalid(
                ENTITY,
                attribute,
                "expected at least one axis, per LIST [1:?]",
            ));
        }
    }
    for (values, attribute) in [(u_axes, "UAxes"), (v_axes, "VAxes"), (w_axes, "WAxes")] {
        let mut seen = std::collections::HashSet::new();
        if let Some(repeat) = values.iter().find(|axis| !seen.insert(**axis)) {
            return Err(invalid(
                ENTITY,
                attribute,
                format!("axis {repeat:?} appears twice in a UNIQUE list"),
            ));
        }
    }
    Ok(())
}

/// Fail unless `id` is an `IfcOwnerHistory` in the model or staged on `tx`.
fn require_owner_history(
    tx: &Transaction,
    model: &Model,
    id: EntityId,
) -> Result<(), GeometryError> {
    let mut found = model.get(id).map(|entity| entity.type_name.to_string());
    for edit in tx.edits() {
        match edit {
            Edit::Create { id: target, entity } if *target == id => {
                found = Some(entity.type_name.to_string());
            }
            Edit::Retype {
                id: target,
                type_name,
            } if *target == id => found = Some(type_name.to_string()),
            Edit::Remove { id: target } if *target == id => found = None,
            _ => {}
        }
    }
    match found {
        Some(found) if found.eq_ignore_ascii_case("IFCOWNERHISTORY") => Ok(()),
        Some(found) => Err(invalid(
            ENTITY,
            "OwnerHistory",
            format!("#{} is a {found}, not an IfcOwnerHistory", id.0),
        )),
        None => Err(invalid(
            ENTITY,
            "OwnerHistory",
            format!("#{} does not resolve", id.0),
        )),
    }
}
