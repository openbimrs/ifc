//! Unit resolution must name what is wrong, not fall back to metres.
//!
//! Every diagnostic here was reachable in crs/unit.rs but unexercised: a file
//! could carry a broken unit declaration and the failure would surface later,
//! as a silently wrong coordinate rather than a named refusal.

use std::sync::Arc;

use ifc_georef::{resolve_project_to_map, GeorefError};
use ifc_model::value::Value;
use ifc_model::{Entity, EntityId, Model};

fn id(value: u64) -> EntityId {
    EntityId(value)
}

fn r(value: u64) -> Value {
    Value::Ref(id(value))
}

fn e(value: &str) -> Value {
    Value::Enum(Arc::from(value))
}

fn text(value: &str) -> Value {
    Value::Text(Arc::from(value))
}

/// A valid map conversion whose CRS points at whatever unit entity the
/// caller supplies as `#3`. Only the unit declaration varies per test.
fn model_with_unit(unit: Option<Entity>) -> Model {
    let mut model = Model::new();
    model.insert(
        id(1),
        Entity::new("IFCGEOMETRICREPRESENTATIONCONTEXT", vec![]),
    );
    model.insert(
        id(2),
        Entity::new(
            "IFCPROJECTEDCRS",
            vec![
                text("EPSG:25832"),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                r(3),
            ],
        ),
    );
    if let Some(unit) = unit {
        model.insert(id(3), unit);
    }
    model.insert(
        id(4),
        Entity::new(
            "IFCMAPCONVERSION",
            vec![
                r(1),
                r(2),
                Value::Real(0.0),
                Value::Real(0.0),
                Value::Real(0.0),
                Value::Real(1.0),
                Value::Real(0.0),
                Value::Real(1.0),
            ],
        ),
    );
    model
}

#[test]
fn a_unit_reference_with_no_entity_names_the_dangling_id() {
    let model = model_with_unit(None);
    assert!(matches!(
        resolve_project_to_map(&model, id(4), 1.0),
        Err(GeorefError::MissingEntity { referrer, missing })
            if referrer == id(3) && missing == id(3)
    ));
}

#[test]
fn a_unit_that_is_not_a_unit_entity_is_refused_by_type() {
    let model = model_with_unit(Some(Entity::new("IFCCARTESIANPOINT", vec![])));
    assert!(matches!(
        resolve_project_to_map(&model, id(4), 1.0),
        Err(GeorefError::WrongType { entity, expected, .. })
            if entity == id(3) && expected == "IFCSIUNIT or IFCCONVERSIONBASEDUNIT"
    ));
}

#[test]
fn an_unknown_si_prefix_is_refused_rather_than_treated_as_unprefixed() {
    let unit = Entity::new(
        "IFCSIUNIT",
        vec![Value::Derived, e("LENGTHUNIT"), e("SQUILLION"), e("METRE")],
    );
    assert!(matches!(
        resolve_project_to_map(&model_with_unit(Some(unit)), id(4), 1.0),
        Err(GeorefError::InvalidUnit { entity, detail })
            if entity == id(3) && detail == "unknown SI prefix"
    ));
}

#[test]
fn a_conversion_unit_without_a_name_names_the_missing_slot() {
    let unit = Entity::new(
        "IFCCONVERSIONBASEDUNIT",
        vec![Value::Derived, e("LENGTHUNIT"), Value::Null, r(5)],
    );
    assert!(matches!(
        resolve_project_to_map(&model_with_unit(Some(unit)), id(4), 1.0),
        Err(GeorefError::MissingAttribute { entity, index, name })
            if entity == id(3) && index == 2 && name == "Name"
    ));
}

#[test]
fn a_unit_whose_conversion_chain_returns_to_itself_terminates() {
    // Pins the repeat-id check specifically. The depth cap is pinned
    // separately by a_long_chain_of_distinct_units_is_bounded_by_depth,
    // because a self-loop alone is stopped by either guard.
    let unit = Entity::new(
        "IFCCONVERSIONBASEDUNIT",
        vec![Value::Derived, e("LENGTHUNIT"), text("loop"), r(5)],
    );
    let mut model = model_with_unit(Some(unit));
    // #5 measures itself in #3, so resolving #3 needs #3.
    model.insert(
        id(5),
        Entity::new("IFCMEASUREWITHUNIT", vec![Value::Real(2.0), r(3)]),
    );
    assert!(matches!(
        resolve_project_to_map(&model, id(4), 1.0),
        Err(GeorefError::UnitCycle { entity }) if entity == id(3)
    ));
}

/// The refusals above are only meaningful if the same shape succeeds when
/// the file is well formed: a foot resolves through its measure to metres.
#[test]
fn a_well_formed_conversion_unit_resolves_through_its_base() {
    let unit = Entity::new(
        "IFCCONVERSIONBASEDUNIT",
        vec![Value::Derived, e("LENGTHUNIT"), text("FOOT"), r(5)],
    );
    let mut model = model_with_unit(Some(unit));
    model.insert(
        id(5),
        Entity::new("IFCMEASUREWITHUNIT", vec![Value::Real(0.3048), r(6)]),
    );
    model.insert(
        id(6),
        Entity::new(
            "IFCSIUNIT",
            vec![Value::Derived, e("LENGTHUNIT"), Value::Null, e("METRE")],
        ),
    );
    let operation = resolve_project_to_map(&model, id(4), 1.0).expect("foot resolves");
    assert_eq!(operation.map_unit.name, "FOOT");
    assert_eq!(operation.map_unit.metres_per_unit, 0.3048);
}

/// The depth cap, isolated from the repeat-id check.
///
/// Every unit here is distinct, so `chain.contains` never fires; only the
/// length bound can stop this. Without it the walk would run to the end of
/// a chain an author can make arbitrarily long.
#[test]
fn a_long_chain_of_distinct_units_is_bounded_by_depth() {
    let unit = Entity::new(
        "IFCCONVERSIONBASEDUNIT",
        vec![Value::Derived, e("LENGTHUNIT"), text("step0"), r(10)],
    );
    let mut model = model_with_unit(Some(unit));
    // #10..#48: measure -> unit -> measure -> ... , all distinct ids.
    for step in 0..20u64 {
        let measure = 10 + step * 2;
        model.insert(
            id(measure),
            Entity::new("IFCMEASUREWITHUNIT", vec![Value::Real(1.0), r(measure + 1)]),
        );
        model.insert(
            id(measure + 1),
            Entity::new(
                "IFCCONVERSIONBASEDUNIT",
                vec![
                    Value::Derived,
                    e("LENGTHUNIT"),
                    text("step"),
                    r(measure + 2),
                ],
            ),
        );
    }
    assert!(matches!(
        resolve_project_to_map(&model, id(4), 1.0),
        Err(GeorefError::UnitCycle { .. })
    ));
}
