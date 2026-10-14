//! Resolution rules on hand-built models: the `FOR` slot, the referencing
//! type, cardinality and the total count. The release fixtures are in
//! `tests/inverse_attributes.rs`. Every case reads IFC4's tables.
#![cfg(feature = "ifc4")]

use super::*;
use crate::named_attribute::{attribute_by_name, set_attribute_by_name};
use ifc_model::Entity;

fn null(count: usize) -> Vec<Value> {
    vec![Value::Null; count]
}

/// `IfcRelVoidsElement(.., RelatingBuildingElement, RelatedOpeningElement)`.
fn voids(model: &mut Model, host: EntityId, opening: EntityId) -> EntityId {
    let mut attributes = null(4);
    attributes.extend([Value::Ref(host), Value::Ref(opening)]);
    model.push(Entity::new("IFCRELVOIDSELEMENT", attributes))
}

fn read(
    model: &Model,
    schema: &Schema,
    id: EntityId,
    name: &str,
) -> Result<InverseValue, NamedAttributeError> {
    let reverse = ReverseIndex::build(model);
    match attribute_value_by_name(model, schema, &reverse, id, name)? {
        AttributeValue::Inverse(value) => Ok(value),
        other => panic!("{name} read as {other:?}"),
    }
}

#[test]
fn a_single_valued_inverse_is_one_entity_none_or_refused() {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    let wall = model.push(Entity::new("IFCWALL", null(9)));
    let opening = model.push(Entity::new("IFCOPENINGELEMENT", null(9)));
    assert_eq!(
        read(&model, schema, opening, "VoidsElements"),
        Ok(InverseValue::Single(None))
    );
    let first = voids(&mut model, wall, opening);
    assert_eq!(
        read(&model, schema, opening, "voidselements"),
        Ok(InverseValue::Single(Some(first)))
    );
    let second = voids(&mut model, wall, opening);
    assert_eq!(
        read(&model, schema, opening, "VoidsElements"),
        Err(NamedAttributeError::InverseCardinality {
            type_name: "IfcOpeningElement".into(),
            name: "VoidsElements".into(),
            referrers: vec![first, second],
        })
    );
    // The aggregate on the other end holds both.
    assert_eq!(
        read(&model, schema, wall, "HasOpenings"),
        Ok(InverseValue::Aggregate(vec![first, second]))
    );
}

#[test]
fn only_the_for_slot_of_the_named_type_counts() {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    let opening = model.push(Entity::new("IFCOPENINGELEMENT", null(9)));
    let other = model.push(Entity::new("IFCOPENINGELEMENT", null(9)));
    // The opening as the *host* of a void: HasOpenings, not VoidsElements.
    let hosted = voids(&mut model, opening, other);
    // The opening in the slot IfcRelVoidsElement's RelatedOpeningElement
    // occupies, but of IfcRelFillsElement (its RelatedBuildingElement).
    let mut fills = null(4);
    fills.extend([Value::Ref(other), Value::Ref(opening)]);
    let filled = model.push(Entity::new("IFCRELFILLSELEMENT", fills));
    assert_eq!(
        read(&model, schema, opening, "VoidsElements"),
        Ok(InverseValue::Single(None))
    );
    assert_eq!(
        read(&model, schema, opening, "HasOpenings"),
        Ok(InverseValue::Aggregate(vec![hosted]))
    );
    assert_eq!(
        read(&model, schema, opening, "FillsVoids"),
        Ok(InverseValue::Aggregate(vec![filled]))
    );
    let reverse = ReverseIndex::build(&model);
    assert_eq!(total_inverses(&reverse, opening), 2);
    assert_eq!(total_inverses(&reverse, other), 2);
}

#[test]
fn an_aggregate_for_attribute_matches_by_membership_once_per_referrer() {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    let wall = model.push(Entity::new("IFCWALL", null(9)));
    let door = model.push(Entity::new("IFCDOOR", null(13)));
    let storey = model.push(Entity::new("IFCBUILDINGSTOREY", null(10)));
    let mut attributes = null(4);
    // The wall listed twice in one relationship is one referrer.
    attributes.extend([
        Value::List(vec![Value::Ref(wall), Value::Ref(door), Value::Ref(wall)]),
        Value::Ref(storey),
    ]);
    let rel = model.push(Entity::new("IFCRELCONTAINEDINSPATIALSTRUCTURE", attributes));
    for element in [wall, door] {
        assert_eq!(
            read(&model, schema, element, "ContainedInStructure"),
            Ok(InverseValue::Aggregate(vec![rel]))
        );
    }
    assert_eq!(
        read(&model, schema, storey, "ContainsElements"),
        Ok(InverseValue::Aggregate(vec![rel]))
    );
    assert_eq!(
        read(&model, schema, storey, "ContainedInStructure"),
        Err(NamedAttributeError::UnknownAttribute {
            schema: "IFC4".into(),
            type_name: "IfcBuildingStorey".into(),
            name: "ContainedInStructure".into(),
        })
    );
    let reverse = ReverseIndex::build(&model);
    assert_eq!(total_inverses(&reverse, wall), 1);
    assert_eq!(total_inverses(&reverse, rel), 0);
}

#[test]
#[cfg(all(feature = "ifc2x3", feature = "ifc4"))]
fn is_grouped_by_is_single_in_ifc2x3_and_a_set_in_ifc4() {
    for (schema, expected_single) in [(ifc_schema::ifc2x3(), true), (ifc_schema::ifc4(), false)] {
        let mut model = Model::new();
        let group = model.push(Entity::new("IFCGROUP", null(5)));
        let mut attributes = null(6);
        attributes.push(Value::Ref(group));
        let rel = model.push(Entity::new("IFCRELASSIGNSTOGROUP", attributes));
        let slot = inverse_slot(schema, "IfcGroup", "IsGroupedBy")
            .unwrap()
            .unwrap();
        assert_eq!(slot.is_aggregate(), !expected_single, "{}", schema.name());
        let expected = if expected_single {
            InverseValue::Single(Some(rel))
        } else {
            InverseValue::Aggregate(vec![rel])
        };
        assert_eq!(read(&model, schema, group, "IsGroupedBy"), Ok(expected));
    }
}

#[test]
fn an_inverse_name_is_refused_by_the_slot_operations() {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    let wall = model.push(Entity::new("IFCWALL", null(9)));
    let refusal = NamedAttributeError::InverseAttribute {
        type_name: "IfcWall".into(),
        name: "IsDefinedBy".into(),
    };
    assert_eq!(
        attribute_by_name(&model, schema, wall, "isdefinedby"),
        Err(refusal.clone())
    );
    let before = model.get(wall).cloned();
    assert_eq!(
        set_attribute_by_name(&mut model, schema, wall, "IsDefinedBy", Value::Null),
        Err(refusal)
    );
    assert_eq!(model.get(wall).cloned(), before);
    // An explicit name still reads its slot through the combined read.
    let reverse = ReverseIndex::build(&model);
    assert_eq!(
        attribute_value_by_name(&model, schema, &reverse, wall, "Name"),
        Ok(AttributeValue::Explicit(Some(&Value::Null)))
    );
    assert_eq!(
        attribute_value_by_name(&model, schema, &reverse, EntityId(99), "Name"),
        Err(NamedAttributeError::MissingEntity(EntityId(99)))
    );
}

#[test]
fn slots_list_inherited_inverses_first_with_bounds() {
    let schema = ifc_schema::ifc4();
    let slots = inverse_slots(schema, "IfcWall").unwrap();
    let names: Vec<_> = slots.iter().map(|slot| slot.name).collect();
    // IfcRoot declares none; IfcObjectDefinition's come first.
    assert_eq!(names[0], "HasAssignments");
    let typed = slots.iter().find(|slot| slot.name == "IsTypedBy").unwrap();
    assert_eq!(
        (typed.entity, typed.for_attribute, typed.declared_by),
        ("IfcRelDefinesByType", "RelatedObjects", "IfcObject")
    );
    assert_eq!((typed.lower(), typed.upper()), (0, Some(1)));
    let defined = slots
        .iter()
        .find(|slot| slot.name == "IsDefinedBy")
        .unwrap();
    assert_eq!((defined.lower(), defined.upper()), (0, None));
    let voids = inverse_slot(schema, "IfcOpeningElement", "VoidsElements")
        .unwrap()
        .unwrap();
    assert!(!voids.is_aggregate());
    assert_eq!((voids.lower(), voids.upper()), (1, Some(1)));
    assert_eq!(inverse_slot(schema, "IfcWall", "Name"), Ok(None));
    assert!(matches!(
        inverse_slots(schema, "IfcNoSuchThing"),
        Err(NamedAttributeError::UnknownEntity { .. })
    ));
}
