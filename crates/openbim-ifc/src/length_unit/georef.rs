//! Georeferencing under a new length unit.
//!
//! IFC4 ADD2 TC1 `IfcMapConversion`: `Eastings`, `Northings` and
//! `OrthogonalHeight` locate the project in "the coordinate system of the
//! target map coordinate reference system", and `Scale` is used "when the
//! units of the CRS are not identical to the units of the engineering
//! coordinate system". The map side is in the target `IfcProjectedCRS`'s
//! `MapUnit`, or, when it states none, the project length unit; this is
//! how `ifc-georef` reads it.
//!
//! - **With a `MapUnit`**: the map coordinates keep their unit and are left
//!   as they are; `Scale`, which takes a project length to a map length, is
//!   divided by the length factor (an absent `Scale` is 1).
//! - **Without one**: the map unit is the project's and changes with it, so
//!   the coordinates are rescaled with every other length (by the general
//!   walk) and `Scale` stays.
//!
//! IFC4X3 `IfcMapConversionScaled` adds unitless axis factors, unchanged.
//! IFC4X3 `IfcRigidOperation` coordinates and `Height` follow the same rule:
//! kept in the target's `MapUnit` (or, for the height on an
//! `IfcGeographicCRS`, its `HeightUnit`), else rescaled. Length coordinates
//! on a geographic CRS mean nothing and are refused, as `ifc-georef`
//! refuses them; so is a map conversion to anything but a projected CRS.
//!
//! IFC2X3 has none of these entities. `ePSet_MapConversion` holds the same
//! values as properties on the project (or a site), and
//! `ePSet_ProjectedCRS` an optional `MapUnit` (the reading of `ifc-georef`,
//! after the buildingSMART Australasia georeferencing guide). With a
//! `MapUnit`, the `Eastings`, `Northings` and `OrthogonalHeight` properties
//! are kept and the `Scale` property is divided by the length factor, added
//! when absent.

use std::collections::HashMap;
use std::sync::Arc;

use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_properties::{SchemaVersion, UnitConversion};
use ifc_schema::Schema;

use super::classify::{Classifier, Measure};
use super::report::{ConvertedKind, KeptReason, LengthUnitRefusal, RefusalReason};
use super::space::{attribute, reference};

/// What georeferencing asks of the walk.
#[derive(Default)]
pub(super) struct GeorefPlan {
    /// Attributes left as they are, by entity and attribute name.
    pub(super) kept_slots: HashMap<(EntityId, Arc<str>), KeptReason>,
    /// Properties (IFC2X3) whose value is left as it is.
    pub(super) kept_entities: HashMap<EntityId, KeptReason>,
    /// Map-scale edits: entity, attribute name, new value.
    pub(super) edits: Vec<(EntityId, Arc<str>, Value)>,
    /// IFC2X3 sets that need a new `Scale` property of this value.
    pub(super) new_scales: Vec<(EntityId, f64)>,
}

/// Plan georeferencing for `length`.
pub(super) fn plan(
    version: SchemaVersion,
    classifier: &mut Classifier,
    model: &Model,
    length: &UnitConversion,
    refusals: &mut Vec<LengthUnitRefusal>,
) -> GeorefPlan {
    let mut plan = GeorefPlan::default();
    let inverse = length.inverse();
    let schema = classifier.schema();
    if version == SchemaVersion::Ifc2x3 {
        property_sets(schema, model, &inverse, &mut plan, refusals);
        return plan;
    }
    for (id, entity) in model.iter() {
        let is_a = |ancestor| schema.is_a(&entity.type_name, ancestor);
        if is_a("IFCMAPCONVERSION") {
            map_conversion(schema, model, id, entity, &inverse, &mut plan, refusals);
        } else if is_a("IFCRIGIDOPERATION") {
            rigid_operation(classifier, model, id, entity, &mut plan, refusals);
        }
    }
    plan
}

fn has_value(value: Option<&Value>) -> bool {
    !matches!(value, None | Some(Value::Null | Value::Derived))
}

fn refuse(
    refusals: &mut Vec<LengthUnitRefusal>,
    id: EntityId,
    entity: &Entity,
    attribute: &str,
    target_type: Arc<str>,
) {
    refusals.push(LengthUnitRefusal {
        entity: id,
        entity_type: entity.type_name.clone(),
        attribute: Some(attribute.into()),
        reason: RefusalReason::MapTarget { target_type },
    });
}

fn target(schema: &Schema, model: &Model, id: EntityId) -> Option<(EntityId, Arc<str>)> {
    let target = reference(schema, model, id, "TargetCRS")?;
    Some((target, model.get(target)?.type_name.clone()))
}

fn map_conversion(
    schema: &Schema,
    model: &Model,
    id: EntityId,
    entity: &Entity,
    inverse: &UnitConversion,
    plan: &mut GeorefPlan,
    refusals: &mut Vec<LengthUnitRefusal>,
) {
    let Some((target, target_type)) = target(schema, model, id) else {
        refuse(refusals, id, entity, "TargetCRS", "(missing)".into());
        return;
    };
    if !schema.is_a(&target_type, "IFCPROJECTEDCRS") {
        refuse(refusals, id, entity, "TargetCRS", target_type);
        return;
    }
    if !has_value(attribute(schema, model, target, "MapUnit")) {
        return;
    }
    for name in ["Eastings", "Northings", "OrthogonalHeight"] {
        plan.kept_slots
            .insert((id, name.into()), KeptReason::MapUnit);
    }
    let scale = match attribute(schema, model, id, "Scale") {
        Some(Value::Real(scale)) => *scale,
        Some(Value::Integer(scale)) => *scale as f64,
        _ => 1.0,
    };
    plan.edits
        .push((id, "Scale".into(), Value::Real(inverse.apply(scale))));
}

fn rigid_operation(
    classifier: &mut Classifier,
    model: &Model,
    id: EntityId,
    entity: &Entity,
    plan: &mut GeorefPlan,
    refusals: &mut Vec<LengthUnitRefusal>,
) {
    let schema = classifier.schema();
    let Some((target, target_type)) = target(schema, model, id) else {
        refuse(refusals, id, entity, "TargetCRS", "(missing)".into());
        return;
    };
    let mut is_length = |name| match attribute(schema, model, id, name) {
        Some(Value::Typed { type_name, .. }) => {
            classifier.measure(type_name) == Some(Measure::Length)
        }
        _ => false,
    };
    let coordinates = ["FirstCoordinate", "SecondCoordinate"];
    if schema.is_a(&target_type, "IFCPROJECTEDCRS") {
        if has_value(attribute(schema, model, target, "MapUnit")) {
            for name in coordinates.iter().chain(&["Height"]) {
                plan.kept_slots
                    .insert((id, (*name).into()), KeptReason::MapUnit);
            }
        }
    } else if schema.is_a(&target_type, "IFCGEOGRAPHICCRS") {
        for name in coordinates {
            if is_length(name) {
                refuse(refusals, id, entity, name, target_type.clone());
            }
        }
        if has_value(attribute(schema, model, target, "HeightUnit")) {
            plan.kept_slots
                .insert((id, "Height".into()), KeptReason::MapUnit);
        }
    } else if coordinates.iter().any(|name| is_length(name))
        || has_value(attribute(schema, model, id, "Height"))
    {
        refuse(refusals, id, entity, "TargetCRS", target_type);
    }
}

/// `ePSet_MapConversion` hosts and whether each states a `MapUnit`.
fn property_sets(
    schema: &Schema,
    model: &Model,
    inverse: &UnitConversion,
    plan: &mut GeorefPlan,
    refusals: &mut Vec<LengthUnitRefusal>,
) {
    // Each host's sets by name, from the property relationships.
    let mut sets_of: HashMap<EntityId, Vec<EntityId>> = HashMap::new();
    let mut hosts_of: HashMap<EntityId, Vec<EntityId>> = HashMap::new();
    for (id, entity) in model.iter() {
        if !schema.is_a(&entity.type_name, "IFCRELDEFINESBYPROPERTIES") {
            continue;
        }
        let Some(set) = reference(schema, model, id, "RelatingPropertyDefinition") else {
            continue;
        };
        if let Some(Value::List(objects)) = attribute(schema, model, id, "RelatedObjects") {
            for object in objects {
                if let Value::Ref(object) = object {
                    sets_of.entry(*object).or_default().push(set);
                    hosts_of.entry(set).or_default().push(*object);
                }
            }
        }
    }
    let named = |set: EntityId, name: &str| matches!(attribute(schema, model, set, "Name"), Some(Value::Text(text)) if text.eq_ignore_ascii_case(name));
    for (&set, hosts) in &hosts_of {
        if !named(set, "ePSet_MapConversion") {
            continue;
        }
        let modes: Vec<bool> = hosts
            .iter()
            .map(|host| {
                sets_of[host].iter().any(|crs| {
                    named(*crs, "ePSet_ProjectedCRS")
                        && property(schema, model, *crs, "MapUnit").is_some()
                })
            })
            .collect();
        if modes.iter().any(|mode| *mode != modes[0]) {
            if let Some(entity) = model.get(set) {
                refusals.push(LengthUnitRefusal {
                    entity: set,
                    entity_type: entity.type_name.clone(),
                    attribute: None,
                    reason: RefusalReason::AmbiguousUnit,
                });
            }
            continue;
        }
        if !modes.first().copied().unwrap_or(false) {
            continue;
        }
        for name in ["Eastings", "Northings", "OrthogonalHeight"] {
            if let Some((value, _)) = property(schema, model, set, name) {
                plan.kept_entities.insert(value, KeptReason::MapUnit);
            }
        }
        match property(schema, model, set, "Scale") {
            Some((
                value,
                Value::Typed {
                    type_name,
                    value: inner,
                },
            )) => {
                let scale = inner.as_f64().unwrap_or(1.0);
                plan.edits.push((
                    value,
                    "NominalValue".into(),
                    Value::Typed {
                        type_name: type_name.clone(),
                        value: Box::new(Value::Real(inverse.apply(scale))),
                    },
                ));
            }
            _ => plan.new_scales.push((set, inverse.apply(1.0))),
        }
    }
}

/// A single-value property of `set` by name: its entity and nominal value.
fn property<'m>(
    schema: &Schema,
    model: &'m Model,
    set: EntityId,
    name: &str,
) -> Option<(EntityId, &'m Value)> {
    let Some(Value::List(properties)) = attribute(schema, model, set, "HasProperties") else {
        return None;
    };
    properties.iter().find_map(|property| {
        let Value::Ref(id) = property else {
            return None;
        };
        let entity = model.get(*id)?;
        if !schema.is_a(&entity.type_name, "IFCPROPERTYSINGLEVALUE") {
            return None;
        }
        match attribute(schema, model, *id, "Name") {
            Some(Value::Text(text)) if text.as_ref() == name => {}
            _ => return None,
        }
        let value = attribute(schema, model, *id, "NominalValue")?;
        has_value(Some(value)).then_some((*id, value))
    })
}

/// Records one converted value: entity type, attribute, kind.
pub(super) type Converted<'a> = dyn FnMut(&Arc<str>, &Arc<str>, ConvertedKind) + 'a;

/// Stage the new IFC2X3 `Scale` properties.
pub(super) fn stage_new_scales(
    schema: &Schema,
    model: &Model,
    plan: &GeorefPlan,
    transaction: &mut Transaction,
    converted: &mut Converted<'_>,
) {
    for (set, scale) in &plan.new_scales {
        let property = transaction.create(Entity::new(
            "IFCPROPERTYSINGLEVALUE",
            vec![
                Value::Text("Scale".into()),
                Value::Null,
                Value::Typed {
                    type_name: "IFCREAL".into(),
                    value: Box::new(Value::Real(*scale)),
                },
                Value::Null,
            ],
        ));
        let Some(record) = model.get(*set) else {
            continue;
        };
        let slot = (0..schema.attribute_count(&record.type_name)).find(|slot| {
            schema
                .attribute_at(&record.type_name, *slot)
                .is_some_and(|a| a.name.eq_ignore_ascii_case("HasProperties"))
        });
        if let (Some(slot), Some(Value::List(properties))) =
            (slot, slot.and_then(|s| record.attributes.get(s)))
        {
            let mut properties = properties.clone();
            properties.push(Value::Ref(property));
            transaction.set_attribute(*set, slot, Value::List(properties));
            converted(
                &"IFCPROPERTYSINGLEVALUE".into(),
                &"NominalValue".into(),
                ConvertedKind::MapScale,
            );
        }
    }
}
