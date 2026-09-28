//! Predefined property sets in exact resolution (#149, #66).
//!
//! A predefined set (`IfcDoorLiningProperties`, `IfcDoorPanelProperties`,
//! their window counterparts, and every other `IfcPropertySetDefinition`
//! that is neither an `IfcPropertySet` nor a quantity set) keeps its values
//! in attributes of its own entity rather than in named properties. Its
//! members are therefore the attributes its entity declares below
//! `IfcPropertySetDefinition`, in the bound release's order and spelling:
//!
//! ```text
//! IfcDoorLiningProperties  4 LiningDepth  5 LiningThickness  ...
//! IfcDoorPanelProperties   4 PanelDepth   5 PanelOperation   6 PanelWidth
//!                          7 PanelPosition  8 ShapeAspectStyle
//! ```
//!
//! Every attribute is read from the release's table, never by a hard-wired
//! slot, so IFC2X3's `LiningThickness : IfcPositiveLengthMeasure` and IFC4's
//! `IfcNonNegativeLengthMeasure`, or IFC4's added `LiningToPanelOffsetX`,
//! come out as each release declares them. A value is typed by its declared
//! type:
//!
//! - a defined type or EXPRESS simple type gives a scalar with that type as
//!   `value_type` (`IFCPOSITIVELENGTHMEASURE`, `IFCNORMALISEDRATIOMEASURE`);
//!   no unit is stated, so the project unit applies (`exact_unit`);
//! - an enumeration gives [`ExactValue::Enum`] checked against the
//!   release's members (`FIXEDPANEL` is an IFC4 addition);
//! - a select gives the typed member it holds, as a single value does;
//! - an entity gives [`ExactValue::Entity`], the target checked but not
//!   followed;
//! - `$` on an `OPTIONAL` attribute is [`ExactValue::Null`] with the declared
//!   type, an exact absence of the value; `$` on a required one is refused.
//!
//! An aggregate attribute (`IfcReinforcementDefinitionProperties.
//! ReinforcementSectionDefinitions`) is not read and refuses when selected,
//! as the whole set did before (#66).

use std::{collections::BTreeSet, sync::Arc};

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::TypeKind;

use super::assignment::assigned_sets;
use super::composite::{entity_target, enum_accepts};
use super::refs::text_at;
use super::release::{validate_model, Release};
use super::set::{load_named, Member, SetKind};
use super::value::{
    exact_value, select_member, simple_payload_matches, typed_payload_matches, ResolvedValue,
};
use super::{
    ExactEntityRef, ExactLogical, ExactProperty, ExactPropertyEntry, ExactPropertyError,
    ExactSource, ExactValue,
};

/// A predefined property set assigned to an object, with every attribute
/// its entity declares itself resolved exactly.
///
/// Returned by [`exact_predefined_sets`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ExactPredefinedSet {
    /// Whether the set is assigned to the occurrence or inherited from its
    /// `IfcTypeObject` (an `IfcDoorType`, or an IFC2X3 `IfcDoorStyle`); a
    /// queried type object's own sets are `Type` of that object.
    pub source: ExactSource,
    /// Entity id of the set.
    pub set_id: EntityId,
    /// The set's entity in the release's spelling, e.g.
    /// `IfcDoorPanelProperties`.
    pub entity: Arc<str>,
    /// The set's `Name`, if stated.
    pub name: Option<Arc<str>>,
    /// Each attribute the entity declares below `IfcPropertySetDefinition`,
    /// in schema order, named as the schema names it (`PanelWidth`). Each
    /// [`ExactProperty`] has the set's id as both `set_id` and
    /// `property_id`, the attribute's declared type as `value_type`, no
    /// `unit_id`, and `property_set` equal to `name` or else `entity`.
    pub attributes: Vec<ExactPropertyEntry>,
}

impl ExactPredefinedSet {
    /// The attribute named `name` (schema spelling, e.g. `LiningDepth`), or
    /// `None` when the entity declares no such attribute in this release.
    #[must_use]
    pub fn attribute(&self, name: &str) -> Option<&ExactProperty> {
        self.attributes
            .iter()
            .find(|entry| entry.name.as_ref() == name)
            .map(|entry| &entry.property)
    }
}

/// Every predefined property set of entity type `entity` (or a subtype)
/// assigned to `object`, each with all its own attributes resolved.
///
/// A door can carry several sets of one type, one
/// `IfcDoorPanelProperties` per leaf, which [`exact_property`] would refuse
/// as ambiguous; this lists them all. Occurrence sets come first, then
/// those of the object's `IfcTypeObject`, each in assignment order. No
/// override is applied between the two sources: every set is reported with
/// its [`ExactSource`], and which one governs is the caller's decision (for
/// example the occurrence's sets when it has any, else the type's). An
/// empty result is a proven absence of such sets. The traversal, model and
/// assignment validation are those of [`exact_property`], so `object` may
/// be a type object (an `IfcDoorType`, an IFC2X3 `IfcDoorStyle`), whose own
/// `HasPropertySets` are listed with [`ExactSource::Type`] of `object`
/// (#193).
///
/// # Door and window geometry
///
/// This is the read path for derived door operation geometry (#148): per
/// leaf, `PanelOperation` and `PanelPosition` come as
/// [`ExactValue::Enum`], and `PanelWidth` as an `IFCNORMALISEDRATIOMEASURE`
/// fraction of the door's `OverallWidth`; `LiningDepth` and
/// `LiningThickness` are lengths in the project length unit, which
/// [`exact_unit`] resolves from their `value_type`.
///
/// ```
/// use ifc_model::{EntityId, Model};
/// use ifc_properties::{exact_predefined_sets, exact_unit, ExactValue};
///
/// /// (operation, position, width fraction) of each leaf of `door`.
/// fn leaves(model: &Model, door: EntityId) -> Option<Vec<(String, String, f64)>> {
///     let panels = exact_predefined_sets(model, door, "IfcDoorPanelProperties").ok()?;
///     panels
///         .iter()
///         .map(|panel| {
///             let text = |name| match &panel.attribute(name)?.value {
///                 ExactValue::Enum(value) => Some(value.to_string()),
///                 _ => None,
///             };
///             let width = match panel.attribute("PanelWidth")?.value {
///                 ExactValue::Real(fraction) => fraction,
///                 _ => return None, // `$`: no width stated
///             };
///             Some((text("PanelOperation")?, text("PanelPosition")?, width))
///         })
///         .collect()
/// }
///
/// /// The lining depth of `door` in metres.
/// fn lining_depth(model: &Model, door: EntityId) -> Option<f64> {
///     let linings = exact_predefined_sets(model, door, "IfcDoorLiningProperties").ok()?;
///     let depth = linings.first()?.attribute("LiningDepth")?;
///     let ExactValue::Real(value) = depth.value else { return None };
///     let unit = exact_unit(model, depth.value_type.as_deref()?, depth.unit_id).ok()?;
///     Some(value * unit.scale)
/// }
/// # let _ = (leaves, lining_depth);
/// ```
///
/// # Errors
///
/// [`ExactPropertyError::NotAPredefinedSet`] when the declared release does
/// not declare `entity` as a predefined property set, and otherwise any
/// [`ExactPropertyError`] of [`exact_property`], including a set attribute
/// that cannot be read exactly (an aggregate refuses as
/// [`ExactPropertyError::UnsupportedDefinition`]).
///
/// [`exact_property`]: super::exact_property
/// [`exact_unit`]: super::exact_unit
pub fn exact_predefined_sets(
    model: &Model,
    object: EntityId,
    entity: &str,
) -> Result<Vec<ExactPredefinedSet>, ExactPropertyError> {
    let release = validate_model(model)?;
    let schema = release.schema;
    let predefined = schema.entity(entity).is_some()
        && schema.is_a(entity, "IFCPROPERTYSETDEFINITION")
        && !schema.is_a("IFCPROPERTYSET", entity)
        && !schema.is_a("IFCELEMENTQUANTITY", entity);
    if !predefined {
        return Err(ExactPropertyError::NotAPredefinedSet {
            name: entity.into(),
            schema: release.version,
        });
    }
    let assigned = assigned_sets(model, release, object)?;
    let mut sources = vec![(ExactSource::Occurrence, &assigned.occurrence_sets)];
    if let Some((type_id, sets)) = &assigned.type_sets {
        sources.push((ExactSource::Type(*type_id), sets));
    }
    let mut found = Vec::new();
    for (source, sets) in sources {
        for &set_id in sets {
            let set = load_named(model, release, set_id)?;
            if set.kind != SetKind::Predefined || !schema.is_a(&set.entity.type_name, entity) {
                continue;
            }
            let attributes = own_attributes(release, set.entity)
                .map(|(slot, name)| {
                    let member = Member::Attribute(slot);
                    let resolved = set.value(model, release, member)?;
                    Ok(ExactPropertyEntry {
                        name: name.into(),
                        property: set.exact(source, member, resolved),
                    })
                })
                .collect::<Result<_, ExactPropertyError>>()?;
            found.push(ExactPredefinedSet {
                source,
                set_id,
                entity: canonical_name(release, set.entity).into(),
                name: set.named.then(|| set.name.into()),
                attributes,
            });
        }
    }
    Ok(found)
}

/// The name a set selector sees for a predefined set, and whether it is a
/// stated `Name`: its `Name`, or its entity name when `Name` is `$`.
///
/// # Errors
///
/// [`ExactPropertyError::MalformedName`] for a `Name` that is neither text
/// nor `$`.
pub(super) fn predefined_key(
    release: Release,
    set_id: EntityId,
    set: &Entity,
) -> Result<(&str, bool), ExactPropertyError> {
    match set.attributes.get(2) {
        Some(Value::Null) => Ok((canonical_name(release, set), false)),
        name => text_at(set_id, name, "Name").map(|name| (name, true)),
    }
}

/// The entity's name as the release spells it (`IfcDoorLiningProperties`).
fn canonical_name(release: Release, set: &Entity) -> &'static str {
    release
        .schema
        .entity(&set.type_name)
        .map_or("", |definition| definition.name.as_str())
}

/// The slot and name of each attribute a predefined set declares below
/// `IfcPropertySetDefinition` (whose `IfcRoot` attributes every set shares).
pub(super) fn own_attributes(
    release: Release,
    set: &Entity,
) -> impl Iterator<Item = (usize, &'static str)> {
    let schema = release.schema;
    let inherited = schema.attribute_names("IFCPROPERTYSETDEFINITION").len();
    schema
        .attributes(set.type_name.as_ref())
        .into_iter()
        .enumerate()
        .skip(inherited)
        .map(|(slot, attribute)| (slot, attribute.name.as_str()))
}

/// The value of a predefined set's attribute at `slot`, typed by its
/// declaration in the bound release.
///
/// # Errors
///
/// [`ExactPropertyError::UnsupportedDefinition`] for an aggregate
/// attribute, [`ExactPropertyError::MissingValueSlot`] for `$` on a required
/// one, and [`ExactPropertyError::UnsupportedValue`] (or a reference or
/// release error) for a value its declared type does not accept.
pub(super) fn attribute_value(
    model: &Model,
    release: Release,
    set_id: EntityId,
    set: &Entity,
    slot: usize,
) -> Result<ResolvedValue, ExactPropertyError> {
    let schema = release.schema;
    let attribute = schema.attributes(&set.type_name)[slot];
    if attribute.aggregate {
        return Err(ExactPropertyError::UnsupportedDefinition {
            entity: set_id,
            type_name: set.type_name.clone(),
        });
    }
    let declared: Arc<str> = attribute.type_name.to_ascii_uppercase().into();
    let value = &set.attributes[slot];
    let scalar = |value, value_type| {
        Ok(ResolvedValue {
            value,
            value_type: Some(value_type),
            unit_id: None,
        })
    };
    let unsupported = || ExactPropertyError::UnsupportedValue { property: set_id };
    match value {
        Value::Null if attribute.optional => return scalar(ExactValue::Null, declared),
        Value::Null => return Err(ExactPropertyError::MissingValueSlot { property: set_id }),
        _ => {}
    }
    let entity_ref = |target: EntityId| {
        let entity = entity_target(model, release, set_id, target, attribute)?;
        Ok(ExactValue::Entity(ExactEntityRef {
            id: target,
            type_name: entity.type_name.clone(),
        }))
    };
    if schema.entity(&declared).is_some() {
        let Value::Ref(target) = value else {
            return Err(unsupported());
        };
        return scalar(entity_ref(*target)?, declared);
    }
    match schema
        .type_def(&declared)
        .map(|definition| &definition.kind)
    {
        Some(TypeKind::Enumeration(_)) => match value {
            Value::Enum(member) if enum_accepts(release, &declared, member) => {
                scalar(ExactValue::Enum(member.clone()), declared)
            }
            _ => Err(unsupported()),
        },
        Some(TypeKind::Select(_)) => match value {
            Value::Ref(target) => scalar(entity_ref(*target)?, declared),
            value => {
                let typed = select_member(release, set_id, &declared, value)?;
                scalar(typed.value, typed.value_type)
            }
        },
        Some(TypeKind::Defined(_))
            if typed_payload_matches(schema, &declared, value, &mut BTreeSet::new()) =>
        {
            let logical = schema
                .resolve_defined(&declared)
                .eq_ignore_ascii_case("LOGICAL");
            scalar(bare(set_id, value, logical)?, declared)
        }
        None if simple_payload_matches(&declared, value) => {
            scalar(bare(set_id, value, &*declared == "LOGICAL")?, declared)
        }
        _ => Err(unsupported()),
    }
}

/// A bare attribute payload, keeping a `LOGICAL` three-state.
fn bare(set_id: EntityId, value: &Value, logical: bool) -> Result<ExactValue, ExactPropertyError> {
    Ok(match (exact_value(set_id, Some(value))?, logical) {
        (ExactValue::Bool(true), true) => ExactValue::Logical(ExactLogical::True),
        (ExactValue::Bool(false), true) => ExactValue::Logical(ExactLogical::False),
        (other, _) => other,
    })
}

#[cfg(test)]
mod tests {
    //! No predefined set of IFC2X3, IFC4 or IFC4X3 declares a `LOGICAL`
    //! attribute, so the three-state reading is pinned on a minimal schema.

    use ifc_model::{Entity, EntityId, Model, Value};
    use ifc_schema::{Schema, SchemaVersion};

    use super::{attribute_value, Release};
    use crate::{ExactLogical, ExactValue};

    const EXPRESS: &str = "SCHEMA IFC4;
TYPE IfcLogical = LOGICAL;
END_TYPE;
ENTITY IfcPropertySetDefinition;
  GlobalId : STRING;
  OwnerHistory : OPTIONAL STRING;
  Name : OPTIONAL STRING;
  Description : OPTIONAL STRING;
END_ENTITY;
ENTITY IfcFlagProperties
 SUBTYPE OF (IfcPropertySetDefinition);
  Flag : OPTIONAL IfcLogical;
  Bare : OPTIONAL LOGICAL;
END_ENTITY;
END_SCHEMA;";

    #[test]
    fn a_logical_attribute_keeps_three_states() {
        let release = Release {
            version: SchemaVersion::Ifc4,
            schema: Box::leak(Box::new(Schema::from_express(EXPRESS))),
        };
        let model = Model::new();
        for (value, expected) in [
            (Value::Bool(true), ExactLogical::True),
            (Value::Bool(false), ExactLogical::False),
            (Value::LogicalUnknown, ExactLogical::Unknown),
        ] {
            for slot in [4, 5] {
                let mut attributes = vec![Value::Null; 6];
                attributes[slot] = value.clone();
                let set = Entity::new("IFCFLAGPROPERTIES", attributes);
                let resolved = attribute_value(&model, release, EntityId(1), &set, slot)
                    .expect("a logical resolves");
                assert_eq!(resolved.value, ExactValue::Logical(expected), "slot {slot}");
            }
        }
    }
}
