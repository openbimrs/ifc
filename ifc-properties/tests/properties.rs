//! `ifc-properties` reading: sets, values, units, templates, checks.

mod common;

use common::{fixture, wall_named};
use ifc_model::{EntityId, Model};
use ifc_properties::{
    compare, prefix_exponent, project_unit_for, property_set_templates, property_sets_by_object,
    quantity_sets, resolved_properties, template_of_set, Attachment, Comparison, ComputedQuantity,
    PropertyAnomaly, PropertyValue, Quantity, QuantityKind, Source, Tolerance, UnitKind,
};

// ---- PROP-PSET -----------------------------------------------------------

/// Every property family is read, not just single values.
#[test]
fn every_property_family_is_read() {
    let model = fixture();
    let (by_object, _) = property_sets_by_object(&model);
    let wall_b = wall_named(&model, "Wall B");
    let sets = by_object.get(&wall_b).expect("wall B has properties");
    let families = sets
        .iter()
        .map(|(_, s)| s)
        .find(|s| s.name.as_deref() == Some("Pset_Families"))
        .expect("families set");

    assert_eq!(families.properties.len(), 7, "one property per family");

    let thickness = families.property("Thickness").expect("thickness");
    match &thickness.value {
        PropertyValue::Single { value, .. } => {
            let v = value.as_ref().expect("nominal value");
            assert_eq!(v.as_f64(), Some(0.2));
            // The measure name is the only statement of what 0.2 MEANS.
            assert_eq!(v.measure(), Some("IFCLENGTHMEASURE"));
        }
        other => panic!("expected single value, got {other:?}"),
    }

    match &families.property("Layers").expect("layers").value {
        PropertyValue::List { values, .. } => {
            assert_eq!(values.len(), 3);
            // LIST order is meaningful: inner, core, outer.
            assert_eq!(values[0].as_f64(), Some(0.012));
            assert_eq!(values[1].as_f64(), Some(0.15));
        }
        other => panic!("expected list, got {other:?}"),
    }

    match &families.property("Curve").expect("curve").value {
        PropertyValue::Table {
            defining,
            defined,
            interpolation,
        } => {
            assert_eq!(defining.len(), 2);
            assert_eq!(defined.len(), 2);
            assert_eq!(interpolation.as_deref(), Some("LINEAR"));
        }
        other => panic!("expected table, got {other:?}"),
    }

    match &families.property("Material").expect("material").value {
        PropertyValue::Reference { usage, reference } => {
            assert_eq!(usage.as_deref(), Some("Reference"));
            assert!(reference.is_some(), "the referenced entity is resolved");
        }
        other => panic!("expected reference, got {other:?}"),
    }

    match &families.property("Assembly").expect("assembly").value {
        PropertyValue::Complex { usage, properties } => {
            assert_eq!(usage.as_deref(), Some("Layered"));
            assert_eq!(properties.len(), 2, "nested properties are resolved");
        }
        other => panic!("expected complex, got {other:?}"),
    }
}

/// A bounded value keeps upper and lower where the schema puts them.
///
/// UpperBoundValue is slot 2 and LowerBoundValue slot 3. Swapping them
/// produces a range that still looks plausible, so this pins the order.
#[test]
fn a_bounded_value_is_not_inverted() {
    let model = fixture();
    let (by_object, _) = property_sets_by_object(&model);
    let wall_b = wall_named(&model, "Wall B");
    let sets = by_object.get(&wall_b).expect("properties");
    let families = sets
        .iter()
        .map(|(_, s)| s)
        .find(|s| s.name.as_deref() == Some("Pset_Families"))
        .expect("families");

    match &families.property("Range").expect("range").value {
        PropertyValue::Bounded {
            upper,
            lower,
            set_point,
            ..
        } => {
            assert_eq!(upper.as_ref().and_then(|v| v.as_f64()), Some(10.0));
            assert_eq!(lower.as_ref().and_then(|v| v.as_f64()), Some(2.0));
            assert_eq!(set_point.as_ref().and_then(|v| v.as_f64()), Some(6.0));
        }
        other => panic!("expected bounded, got {other:?}"),
    }
}

/// A type holds its property sets directly, not through the relationship.
#[test]
fn type_property_sets_are_found_on_the_type_itself() {
    let model = fixture();
    let (by_object, anomalies) = property_sets_by_object(&model);
    let wall_type = *model
        .ids_of_type("IFCWALLTYPE")
        .first()
        .expect("wall type in fixture");

    let sets = by_object.get(&wall_type).expect("the type carries sets");
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].0, Attachment::Type, "read from HasPropertySets");

    // The fixture is well-formed, so no type is attached by relationship.
    assert!(
        !anomalies
            .iter()
            .any(|a| matches!(a, PropertyAnomaly::TypeAttachedByRelationship { .. })),
        "the fixture does not break NoRelatedTypeObject"
    );
}

// ---- PROP-QUERY ----------------------------------------------------------

/// An occurrence property overrides its type, and the type value survives.
#[test]
fn an_occurrence_property_overrides_the_type() {
    let model = fixture();
    let (resolved, _) = resolved_properties(&model);
    let wall_a = wall_named(&model, "Wall A");
    let sets = resolved.get(&wall_a).expect("wall A resolves");

    let common = sets
        .iter()
        .find(|r| r.set.name.as_deref() == Some("Pset_WallCommon"))
        .expect("the common set");
    assert_eq!(common.source, Source::Occurrence, "the occurrence wins");

    match &common.set.property("IsExternal").expect("IsExternal").value {
        PropertyValue::Single { value, .. } => {
            assert_eq!(
                value.as_ref().map(|v| v.scalar.clone()),
                Some(ifc_properties::Scalar::Bool(false)),
                "the occurrence states FALSE"
            );
        }
        other => panic!("expected single, got {other:?}"),
    }

    // The type's version is not discarded: a checker must be able to say WHY
    // the effective value differs from the type default.
    let shadowed = common.shadowed.as_ref().expect("the type set is shadowed");
    assert!(matches!(shadowed.source, Source::Type(_)));
    match &shadowed
        .set
        .property("IsExternal")
        .expect("type value")
        .value
    {
        PropertyValue::Single { value, .. } => {
            assert_eq!(
                value.as_ref().map(|v| v.scalar.clone()),
                Some(ifc_properties::Scalar::Bool(true)),
                "the type stated TRUE"
            );
        }
        other => panic!("expected single, got {other:?}"),
    }
}

/// An occurrence with no set of its own inherits the type's.
#[test]
fn a_type_property_is_inherited_when_not_overridden() {
    let model = fixture();
    let (resolved, _) = resolved_properties(&model);
    let wall_b = wall_named(&model, "Wall B");
    let sets = resolved.get(&wall_b).expect("wall B resolves");

    let common = sets
        .iter()
        .find(|r| r.set.name.as_deref() == Some("Pset_WallCommon"))
        .expect("inherited from the type");
    assert!(
        matches!(common.source, Source::Type(_)),
        "wall B states no Pset_WallCommon of its own"
    );
    assert!(common.shadowed.is_none(), "nothing was overridden");
}

/// Resolution is deterministic: sets come back in name order.
#[test]
fn resolved_sets_are_ordered_by_name() {
    let model = fixture();
    let (resolved, _) = resolved_properties(&model);
    let wall_b = wall_named(&model, "Wall B");
    let names: Vec<_> = resolved
        .get(&wall_b)
        .expect("wall B")
        .iter()
        .filter_map(|r| r.set.name.as_deref())
        .collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted, "output order does not depend on file order");
}

// ---- PROP-QTY ------------------------------------------------------------

/// Every simple quantity kind is read with its value and unit.
#[test]
fn quantities_are_read_by_kind() {
    let model = fixture();
    let (sets, anomalies) = quantity_sets(&model);
    assert_eq!(sets.len(), 1, "one element quantity in the fixture");
    let set = &sets[0];
    assert_eq!(set.method.as_deref(), Some("BaseQuantities"));
    assert_eq!(set.quantities.len(), 7);

    let width = set.quantity("Width").expect("width");
    match width {
        Quantity::Simple {
            kind, value, unit, ..
        } => {
            assert_eq!(*kind, QuantityKind::Length);
            // 200 MILLIMETRE, not 200 metres: the quantity states its own unit
            // and the project default must not be substituted.
            assert_eq!(*value, 200.0);
            assert!(unit.is_some(), "the quantity states a unit");
        }
        other => panic!("expected simple, got {other:?}"),
    }

    match set.quantity("Layers").expect("complex") {
        Quantity::Complex {
            quantities,
            discrimination,
            ..
        } => {
            assert_eq!(quantities.len(), 2, "nested quantities are resolved");
            assert_eq!(discrimination.as_deref(), Some("layer"));
        }
        other => panic!("expected complex, got {other:?}"),
    }

    assert!(
        anomalies.is_empty(),
        "the fixture is well formed: {anomalies:?}"
    );
}

// ---- PROP-UNIT -----------------------------------------------------------

/// A prefixed SI unit reports its exponent, not a rounded factor.
#[test]
fn an_si_prefix_is_an_exact_exponent() {
    assert_eq!(prefix_exponent("MILLI"), Some(-3));
    assert_eq!(prefix_exponent("KILO"), Some(3));
    // An unknown constant is not silently treated as unprefixed.
    assert_eq!(prefix_exponent("SQUILLI"), None);
}

/// The project unit for a type is found, and it is the project's not the
/// quantity's.
#[test]
fn the_project_length_unit_is_metre() {
    let model = fixture();
    let (_, kind) = project_unit_for(&model, "LENGTHUNIT").expect("a project length unit");
    match kind {
        UnitKind::Si {
            name,
            prefix,
            prefix_exponent,
            ..
        } => {
            assert_eq!(&*name, "METRE");
            assert!(prefix.is_none(), "the project default is unprefixed");
            assert_eq!(prefix_exponent, 0);
        }
        other => panic!("expected SI, got {other:?}"),
    }
}

/// A conversion-based unit carries its factor and the unit that factor is in.
#[test]
fn a_conversion_unit_keeps_its_factor() {
    let model = fixture();
    let inch = *model
        .ids_of_type("IFCCONVERSIONBASEDUNIT")
        .first()
        .expect("inch in fixture");
    match ifc_properties::unit(&model, inch).expect("readable") {
        UnitKind::Conversion {
            name,
            factor,
            factor_unit,
            ..
        } => {
            assert_eq!(name.as_deref(), Some("inch"));
            assert_eq!(factor, Some(25.4));
            assert!(factor_unit.is_some(), "25.4 of WHAT is part of the fact");
        }
        other => panic!("expected conversion, got {other:?}"),
    }
}

/// A derived unit keeps its elements and their exponents.
#[test]
fn a_derived_unit_keeps_its_exponents() {
    let model = fixture();
    let derived = *model
        .ids_of_type("IFCDERIVEDUNIT")
        .first()
        .expect("derived unit in fixture");
    match ifc_properties::unit(&model, derived).expect("readable") {
        UnitKind::Derived { elements, .. } => {
            assert_eq!(elements.len(), 2);
            let exponents: Vec<_> = elements.iter().map(|(_, e)| *e).collect();
            // m3 / s: a negative exponent is what makes it a rate.
            assert!(exponents.contains(&1) && exponents.contains(&-1));
        }
        other => panic!("expected derived, got {other:?}"),
    }
}

// ---- PROP-TEMPLATE -------------------------------------------------------

/// A template is read, and linked to the set it governs.
#[test]
fn a_template_governs_its_property_set() {
    let model = fixture();
    let templates = property_set_templates(&model);
    assert_eq!(templates.len(), 1);
    let template = &templates[0];
    assert_eq!(template.applicable_entity.as_deref(), Some("IfcWall"));
    let thickness = template.property("Thickness").expect("templated property");
    assert_eq!(thickness.template_type.as_deref(), Some("P_SINGLEVALUE"));
    assert_eq!(
        thickness.primary_measure.as_deref(),
        Some("IfcLengthMeasure")
    );

    let links = template_of_set(&model);
    assert_eq!(links.len(), 1, "one set is templated");
    assert!(
        links.values().any(|t| *t == template.id),
        "the link points at the template"
    );
}

// ---- PROP-CHECK ----------------------------------------------------------

/// An authored quantity agrees with a matching computed value.
#[test]
fn a_computed_value_can_agree_with_the_file() {
    let model = fixture();
    let (sets, _) = quantity_sets(&model);
    let area = sets[0].quantity("NetSideArea").expect("area");
    let result = compare(
        &model,
        area,
        &ComputedQuantity {
            kind: QuantityKind::Area,
            value: 12.5,
            unit: "SQUARE_METRE".into(),
        },
        Tolerance::default(),
    );
    assert!(matches!(result, Comparison::Agrees { .. }), "{result:?}");
}

/// Comparing different kinds is reported, not silently answered.
#[test]
fn comparing_an_area_to_a_volume_is_a_kind_mismatch() {
    let model = fixture();
    let (sets, _) = quantity_sets(&model);
    let area = sets[0].quantity("NetSideArea").expect("area");
    let result = compare(
        &model,
        area,
        &ComputedQuantity {
            kind: QuantityKind::Volume,
            value: 12.5,
            unit: "CUBIC_METRE".into(),
        },
        Tolerance::default(),
    );
    assert!(
        matches!(result, Comparison::KindMismatch { .. }),
        "a caller error is reported, not answered: {result:?}"
    );
}

/// Different units are reported rather than converted.
///
/// The width is 200 MILLIMETRE. A caller computing 0.2 METRE has the same
/// length, but silently converting is how a 1000x error passes a check.
#[test]
fn a_unit_mismatch_is_reported_not_converted() {
    let model = fixture();
    let (sets, _) = quantity_sets(&model);
    let width = sets[0].quantity("Width").expect("width");
    let result = compare(
        &model,
        width,
        &ComputedQuantity {
            kind: QuantityKind::Length,
            value: 0.2,
            unit: "METRE".into(),
        },
        Tolerance::default(),
    );
    match result {
        Comparison::UnitMismatch { authored_unit, .. } => {
            assert_eq!(authored_unit, "MILLIMETRE", "prefix is part of the unit");
        }
        other => panic!("expected unit mismatch, got {other:?}"),
    }
}

/// A genuine disagreement is reported with its magnitude.
#[test]
fn a_wrong_value_disagrees() {
    let model = fixture();
    let (sets, _) = quantity_sets(&model);
    let area = sets[0].quantity("NetSideArea").expect("area");
    let result = compare(
        &model,
        area,
        &ComputedQuantity {
            kind: QuantityKind::Area,
            value: 13.0,
            unit: "SQUARE_METRE".into(),
        },
        Tolerance::default(),
    );
    match result {
        Comparison::Disagrees {
            relative_difference,
            ..
        } => {
            assert!(relative_difference > 0.03, "{relative_difference}");
        }
        other => panic!("expected disagreement, got {other:?}"),
    }
}

/// A negative quantity breaks WR22 and is reported.
///
/// Every `IfcQuantity*` carries `WR22 : Value >= 0.`. IfcOpenShell's
/// validator does not flag it, so a file like this passes external checks
/// while stating a negative area. Built in memory because a COMMITTED
/// fixture must validate clean.
#[test]
fn a_negative_quantity_is_reported() {
    use ifc_model::{Entity, Value};

    let mut model = Model::default();
    // #1 the offending quantity, #2 the set carrying it.
    model.insert(
        EntityId(1),
        Entity {
            type_name: "IFCQUANTITYAREA".into(),
            attributes: vec![
                Value::Text("NetSideArea".into()),
                Value::Null,
                Value::Null,
                Value::Real(-4.0),
                Value::Null,
            ],
        },
    );
    model.insert(
        EntityId(2),
        Entity {
            type_name: "IFCELEMENTQUANTITY".into(),
            attributes: vec![
                Value::Text("".into()),
                Value::Null,
                Value::Text("Qto".into()),
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(1))]),
            ],
        },
    );

    let (sets, anomalies) = quantity_sets(&model);
    assert_eq!(sets.len(), 1, "the set is still read");
    assert!(
        anomalies.iter().any(|a| matches!(
            a,
            PropertyAnomaly::NegativeQuantity { quantity, value }
                if *quantity == EntityId(1) && *value == -4.0
        )),
        "WR22 breach is reported: {anomalies:?}"
    );
}

/// A quantity whose unit contradicts its kind breaks WR21.
#[test]
fn a_unit_of_the_wrong_type_is_reported() {
    use ifc_model::{Entity, Value};

    let mut model = Model::default();
    // A LENGTHUNIT attached to an area quantity.
    model.insert(
        EntityId(1),
        Entity {
            type_name: "IFCSIUNIT".into(),
            attributes: vec![
                Value::Null,
                Value::Enum("LENGTHUNIT".into()),
                Value::Null,
                Value::Enum("METRE".into()),
            ],
        },
    );
    model.insert(
        EntityId(2),
        Entity {
            type_name: "IFCQUANTITYAREA".into(),
            attributes: vec![
                Value::Text("Area".into()),
                Value::Null,
                Value::Ref(EntityId(1)),
                Value::Real(4.0),
                Value::Null,
            ],
        },
    );
    model.insert(
        EntityId(3),
        Entity {
            type_name: "IFCELEMENTQUANTITY".into(),
            attributes: vec![
                Value::Text("".into()),
                Value::Null,
                Value::Text("Qto".into()),
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(2))]),
            ],
        },
    );

    let (_, anomalies) = quantity_sets(&model);
    assert!(
        anomalies.iter().any(|a| matches!(
            a,
            PropertyAnomaly::QuantityUnitMismatch { expected, found, .. }
                if *expected == "AREAUNIT" && found == "LENGTHUNIT"
        )),
        "WR21 breach is reported: {anomalies:?}"
    );
}

/// The millimetre in the fixture reports its prefix exponent.
///
/// Not just `prefix_exponent("MILLI")` in isolation: this proves the READER
/// carries the prefix off a real file, which is what makes mm distinguishable
/// from m downstream.
#[test]
fn a_prefixed_unit_from_the_file_keeps_its_exponent() {
    let model = fixture();
    let millimetre = model
        .ids_of_type("IFCSIUNIT")
        .iter()
        .copied()
        .find(|id| {
            matches!(
                ifc_properties::unit(&model, *id),
                Some(UnitKind::Si { ref prefix, .. }) if prefix.as_deref() == Some("MILLI")
            )
        })
        .expect("the fixture states a millimetre");

    match ifc_properties::unit(&model, millimetre).expect("readable") {
        UnitKind::Si {
            prefix_exponent,
            name,
            ..
        } => {
            assert_eq!(prefix_exponent, -3, "MILLI is 1e-3");
            assert_eq!(&*name, "METRE");
        }
        other => panic!("expected SI, got {other:?}"),
    }

    // And the scale it implies is exact enough to convert with.
    let scale = ifc_properties::unit(&model, millimetre)
        .expect("readable")
        .si_scale()
        .expect("SI units scale");
    assert!((scale - 0.001).abs() < 1e-12, "{scale}");
}
