//! Nested complex properties and quantities: cycles, depth, budget and
//! malformed members are reported as anomalies, never silently truncated.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_properties::{
    property_checked, property_set_checked, property_sets_by_object, quantity_set, Property,
    PropertyAnomaly, PropertyValue, Quantity, UnresolvedValue,
};

fn text(value: &str) -> Value {
    Value::Text(value.into())
}

fn refs(ids: &[u64]) -> Value {
    Value::List(ids.iter().map(|id| Value::Ref(EntityId(*id))).collect())
}

/// `IfcComplexProperty`: Name, Description, UsageName, HasProperties.
fn complex_property(model: &mut Model, id: u64, members: &[u64]) {
    model.insert(
        EntityId(id),
        Entity::new(
            "IFCCOMPLEXPROPERTY",
            vec![
                text(&format!("C{id}")),
                Value::Null,
                text("usage"),
                refs(members),
            ],
        ),
    );
}

/// `IfcPropertySingleValue`: Name, Description, NominalValue, Unit.
fn single_property(model: &mut Model, id: u64) {
    model.insert(
        EntityId(id),
        Entity::new(
            "IFCPROPERTYSINGLEVALUE",
            vec![
                text(&format!("S{id}")),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
        ),
    );
}

/// `IfcPhysicalComplexQuantity`: Name, Description, HasQuantities,
/// Discrimination, Quality, Usage.
fn complex_quantity(model: &mut Model, id: u64, members: &[u64]) {
    model.insert(
        EntityId(id),
        Entity::new(
            "IFCPHYSICALCOMPLEXQUANTITY",
            vec![
                text(&format!("Q{id}")),
                Value::Null,
                refs(members),
                text("layer"),
                Value::Null,
                Value::Null,
            ],
        ),
    );
}

/// `IfcQuantityLength`: Name, Description, Unit, LengthValue, Formula.
fn length(model: &mut Model, id: u64, value: Value) {
    model.insert(
        EntityId(id),
        Entity::new(
            "IFCQUANTITYLENGTH",
            vec![
                text(&format!("L{id}")),
                Value::Null,
                Value::Null,
                value,
                Value::Null,
            ],
        ),
    );
}

/// `IfcElementQuantity`: GlobalId, OwnerHistory, Name, Description,
/// MethodOfMeasurement, Quantities.
fn element_quantity(model: &mut Model, id: u64, members: &[u64]) {
    model.insert(
        EntityId(id),
        Entity::new(
            "IFCELEMENTQUANTITY",
            vec![
                text("guid"),
                Value::Null,
                text("Qto"),
                Value::Null,
                Value::Null,
                refs(members),
            ],
        ),
    );
}

/// `IfcPropertySet`: GlobalId, OwnerHistory, Name, Description, HasProperties.
fn property_set(model: &mut Model, id: u64, members: &[u64]) {
    model.insert(
        EntityId(id),
        Entity::new(
            "IFCPROPERTYSET",
            vec![
                text("guid"),
                Value::Null,
                text("Pset"),
                Value::Null,
                refs(members),
            ],
        ),
    );
}

fn members(property: &Property) -> &[Property] {
    match &property.value {
        PropertyValue::Complex { properties, .. } => properties,
        other => panic!("expected a complex property, got {other:?}"),
    }
}

fn nested(quantity: &Quantity) -> &[Quantity] {
    match quantity {
        Quantity::Complex { quantities, .. } => quantities,
        other => panic!("expected a complex quantity, got {other:?}"),
    }
}

fn property_nodes(property: &Property) -> usize {
    match &property.value {
        PropertyValue::Complex { properties, .. } => {
            1 + properties.iter().map(property_nodes).sum::<usize>()
        }
        _ => 1,
    }
}

fn quantity_nodes(quantity: &Quantity) -> usize {
    match quantity {
        Quantity::Complex { quantities, .. } => {
            1 + quantities.iter().map(quantity_nodes).sum::<usize>()
        }
        _ => 1,
    }
}

fn cycle(complex: u64, member: u64) -> PropertyAnomaly {
    PropertyAnomaly::ComplexCycle {
        complex: EntityId(complex),
        member: EntityId(member),
    }
}

// ---- complex properties ---------------------------------------------------

#[test]
fn a_complex_property_listing_itself_is_reported() {
    let mut model = Model::new();
    complex_property(&mut model, 1, &[1, 2]);
    single_property(&mut model, 2);

    let (property, anomalies) = property_checked(&model, EntityId(1)).expect("readable");
    let ids: Vec<_> = members(&property).iter().map(|p| p.id).collect();
    assert_eq!(
        ids,
        [EntityId(2)],
        "the self-member is cut, the sibling kept"
    );
    assert_eq!(anomalies, [cycle(1, 1)]);
}

#[test]
fn a_two_property_cycle_is_reported_where_it_closes() {
    let mut model = Model::new();
    complex_property(&mut model, 1, &[2]);
    complex_property(&mut model, 2, &[1, 3]);
    single_property(&mut model, 3);

    let (property, anomalies) = property_checked(&model, EntityId(1)).expect("readable");
    let inner = &members(&property)[0];
    assert_eq!(inner.id, EntityId(2));
    let ids: Vec<_> = members(inner).iter().map(|p| p.id).collect();
    assert_eq!(ids, [EntityId(3)], "#2 keeps its non-cyclic member");
    assert_eq!(
        anomalies,
        [cycle(2, 1)],
        "a direct-only guard would miss this cycle entirely"
    );
}

#[test]
fn a_shared_member_is_not_mistaken_for_a_cycle() {
    // IfcProperty.PartOfComplex is SET [0:?]: #4 may sit under #2 and #3.
    let mut model = Model::new();
    complex_property(&mut model, 1, &[2, 3]);
    complex_property(&mut model, 2, &[4]);
    complex_property(&mut model, 3, &[4]);
    single_property(&mut model, 4);

    let (property, anomalies) = property_checked(&model, EntityId(1)).expect("readable");
    assert_eq!(
        property_nodes(&property),
        5,
        "#4 resolves under both parents"
    );
    assert!(anomalies.is_empty(), "a diamond is legal: {anomalies:?}");
}

#[test]
fn a_wide_fan_out_cycle_is_bounded_and_reported() {
    // Eight complex properties each listing all eight. A depth-only guard
    // follows 7^16 paths; a path-tracking one still has ~10^5 simple paths.
    let ids: Vec<u64> = (1..=8).collect();
    let mut model = Model::new();
    for &id in &ids {
        complex_property(&mut model, id, &ids);
    }

    let (property, anomalies) = property_checked(&model, EntityId(1)).expect("readable");
    assert!(
        property_nodes(&property) <= 10_001,
        "the member budget bounds the resolved tree"
    );
    assert!(anomalies.contains(&cycle(1, 1)), "{anomalies:?}");
    assert!(
        anomalies
            .iter()
            .any(|a| matches!(a, PropertyAnomaly::ComplexBudgetExceeded { limit, .. } if *limit == 10_000)),
        "the cut is reported, not silent"
    );
}

#[test]
fn nesting_deeper_than_the_bound_is_reported() {
    // #1 -> #2 -> ... -> #20 -> single #21.
    let mut model = Model::new();
    for id in 1..20 {
        complex_property(&mut model, id, &[id + 1]);
    }
    complex_property(&mut model, 20, &[21]);
    single_property(&mut model, 21);

    let (property, anomalies) = property_checked(&model, EntityId(1)).expect("readable");
    assert_eq!(
        anomalies,
        [PropertyAnomaly::ComplexTooDeep {
            complex: EntityId(17),
            limit: 16
        }]
    );
    let mut level = &property;
    for _ in 0..16 {
        level = &members(level)[0];
    }
    assert_eq!(level.id, EntityId(17));
    assert!(members(level).is_empty(), "#17's members are not read");
}

#[test]
fn dangling_property_members_are_reported() {
    let mut model = Model::new();
    complex_property(&mut model, 1, &[2, 99]);
    single_property(&mut model, 2);
    property_set(&mut model, 10, &[1, 98]);

    let (set, anomalies) = property_set_checked(&model, EntityId(10)).expect("readable");
    assert_eq!(set.properties.len(), 1);
    assert_eq!(
        anomalies,
        [
            PropertyAnomaly::MissingMember {
                container: EntityId(1),
                member: EntityId(99)
            },
            PropertyAnomaly::MissingMember {
                container: EntityId(10),
                member: EntityId(98)
            },
        ]
    );
}

#[test]
fn a_set_shared_by_two_objects_reports_its_members_once() {
    let mut model = Model::new();
    complex_property(&mut model, 1, &[1]);
    property_set(&mut model, 10, &[1]);
    model.insert(EntityId(20), Entity::new("IFCWALL", vec![text("w1")]));
    model.insert(EntityId(21), Entity::new("IFCWALL", vec![text("w2")]));
    model.insert(
        EntityId(30),
        Entity::new(
            "IFCRELDEFINESBYPROPERTIES",
            vec![
                text("guid"),
                Value::Null,
                Value::Null,
                Value::Null,
                refs(&[20, 21]),
                Value::Ref(EntityId(10)),
            ],
        ),
    );

    let (by_object, anomalies) = property_sets_by_object(&model);
    assert_eq!(by_object.len(), 2);
    assert_eq!(anomalies, [cycle(1, 1)]);
}

// ---- complex quantities ---------------------------------------------------

#[test]
fn a_complex_quantity_listing_itself_is_reported() {
    let mut model = Model::new();
    complex_quantity(&mut model, 1, &[1, 2]);
    length(&mut model, 2, Value::Real(1.5));
    element_quantity(&mut model, 10, &[1]);

    let (set, anomalies) = quantity_set(&model, EntityId(10)).expect("readable");
    let ids: Vec<_> = nested(&set.quantities[0])
        .iter()
        .map(Quantity::id)
        .collect();
    assert_eq!(ids, [EntityId(2)]);
    assert_eq!(anomalies, [cycle(1, 1)]);
}

#[test]
fn a_two_quantity_cycle_is_reported_where_it_closes() {
    let mut model = Model::new();
    complex_quantity(&mut model, 1, &[2]);
    complex_quantity(&mut model, 2, &[1, 3]);
    length(&mut model, 3, Value::Real(2.0));
    element_quantity(&mut model, 10, &[1]);

    let (set, anomalies) = quantity_set(&model, EntityId(10)).expect("readable");
    let inner = &nested(&set.quantities[0])[0];
    assert_eq!(inner.id(), EntityId(2));
    assert_eq!(nested(inner).len(), 1, "#2 keeps its length");
    assert_eq!(anomalies, [cycle(2, 1)]);
}

#[test]
fn a_wide_fan_out_quantity_cycle_is_bounded_and_reported() {
    let ids: Vec<u64> = (1..=8).collect();
    let mut model = Model::new();
    for &id in &ids {
        complex_quantity(&mut model, id, &ids);
    }
    element_quantity(&mut model, 10, &[1]);

    let (set, anomalies) = quantity_set(&model, EntityId(10)).expect("readable");
    assert!(quantity_nodes(&set.quantities[0]) <= 10_001);
    assert!(anomalies.contains(&cycle(1, 1)));
    assert!(anomalies
        .iter()
        .any(|a| matches!(a, PropertyAnomaly::ComplexBudgetExceeded { .. })));
}

#[test]
fn quantity_nesting_deeper_than_the_bound_is_reported() {
    let mut model = Model::new();
    for id in 1..=20 {
        complex_quantity(&mut model, id, &[id + 1]);
    }
    length(&mut model, 21, Value::Real(1.0));
    element_quantity(&mut model, 100, &[1]);

    let (set, anomalies) = quantity_set(&model, EntityId(100)).expect("readable");
    assert_eq!(
        anomalies,
        [PropertyAnomaly::ComplexTooDeep {
            complex: EntityId(17),
            limit: 16
        }]
    );
    let mut level = &set.quantities[0];
    for _ in 0..16 {
        level = &nested(level)[0];
    }
    assert_eq!(level.id(), EntityId(17));
    assert!(nested(level).is_empty());
}

// ---- malformed quantities -------------------------------------------------

#[test]
fn a_quantity_without_a_value_is_reported() {
    let mut model = Model::new();
    length(&mut model, 1, Value::Null);
    // A record truncated before its value slot.
    model.insert(
        EntityId(2),
        Entity::new("IFCQUANTITYAREA", vec![text("Short"), Value::Null]),
    );
    length(&mut model, 3, Value::Real(4.0));
    element_quantity(&mut model, 10, &[1, 2, 3]);

    let (set, anomalies) = quantity_set(&model, EntityId(10)).expect("readable");
    let ids: Vec<_> = set.quantities.iter().map(Quantity::id).collect();
    assert_eq!(ids, [EntityId(1), EntityId(2), EntityId(3)], "none dropped");
    let missing = |q: &Quantity| matches!(q, Quantity::Unresolved { reason, .. } if *reason == UnresolvedValue::Missing);
    assert!(set.quantities[..2].iter().all(missing), "{set:?}");
    assert_eq!(
        anomalies,
        [
            PropertyAnomaly::QuantityValueMissing {
                quantity: EntityId(1)
            },
            PropertyAnomaly::QuantityValueMissing {
                quantity: EntityId(2)
            },
        ]
    );
}

#[test]
fn a_non_numeric_quantity_value_is_reported() {
    let mut model = Model::new();
    length(&mut model, 1, text("two metres"));
    element_quantity(&mut model, 10, &[1]);

    let (set, anomalies) = quantity_set(&model, EntityId(10)).expect("readable");
    match set.quantities.as_slice() {
        [Quantity::Unresolved {
            reason: UnresolvedValue::NotNumeric { found },
            ..
        }] => assert!(found.contains("two metres"), "{found}"),
        other => panic!("expected one unresolved quantity, got {other:?}"),
    }
    match anomalies.as_slice() {
        [PropertyAnomaly::QuantityValueNotNumeric { quantity, found }] => {
            assert_eq!(*quantity, EntityId(1));
            assert!(found.contains("two metres"), "{found}");
        }
        other => panic!("expected one non-numeric anomaly, got {other:?}"),
    }
}

#[test]
fn a_nested_malformed_quantity_is_reported() {
    let mut model = Model::new();
    length(&mut model, 1, Value::Null);
    complex_quantity(&mut model, 2, &[1]);
    element_quantity(&mut model, 10, &[2]);

    let (set, anomalies) = quantity_set(&model, EntityId(10)).expect("readable");
    assert!(matches!(
        nested(&set.quantities[0]),
        [Quantity::Unresolved {
            id: EntityId(1),
            reason: UnresolvedValue::Missing,
            ..
        }]
    ));
    assert_eq!(
        anomalies,
        [PropertyAnomaly::QuantityValueMissing {
            quantity: EntityId(1)
        }]
    );
}

#[test]
fn dangling_quantity_references_are_reported() {
    let mut model = Model::new();
    complex_quantity(&mut model, 1, &[98]);
    element_quantity(&mut model, 10, &[1, 99]);

    let (set, anomalies) = quantity_set(&model, EntityId(10)).expect("readable");
    assert_eq!(set.quantities.len(), 1, "the complex quantity is kept");
    assert_eq!(
        anomalies,
        [
            PropertyAnomaly::MissingMember {
                container: EntityId(1),
                member: EntityId(98)
            },
            PropertyAnomaly::MissingMember {
                container: EntityId(10),
                member: EntityId(99)
            },
        ]
    );
}
