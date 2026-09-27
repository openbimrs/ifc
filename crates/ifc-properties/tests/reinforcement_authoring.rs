//! Authoring the reinforcement, section, and profile property families.
//!
//! These five are the predefined-properties branch that carries no
//! WHERE rules. What constrains them is structural: required slots,
//! set lower bounds of `[1:?]`, closed enumerations, and the measure
//! type each numeric slot declares.

use ifc_model::{Entity, Model, Transaction, Value};
use ifc_properties::{
    add_profile_properties, add_reinforcement_bar_properties,
    add_reinforcement_definition_properties, add_section_properties,
    add_section_reinforcement_properties, ReinforcementBarDraft, SectionReinforcementDraft,
};
use ifc_schema::{ifc4, ifc4x3};

const GUID: &str = "3Ovv5_Gsj7hgnPnCVjCDRV";

fn bar<'a>() -> ReinforcementBarDraft<'a> {
    ReinforcementBarDraft {
        total_cross_section_area: 452.4,
        steel_grade: "B500B",
        bar_surface: Some("PLAIN"),
        effective_depth: None,
        nominal_bar_diameter: Some(12.0),
        bar_count: Some(4),
    }
}

/// `BarCount` is `IfcCountMeasure`, declared INTEGER in EXPRESS.
///
/// A real in that slot has the right arity and the wrong type, which
/// the structural validator does not catch.
#[test]
fn bar_count_is_written_as_an_integer() {
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);
    let id = add_reinforcement_bar_properties(&mut tx, bar()).expect("staged");
    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("entity");
    assert_eq!(entity.attributes.len(), 6, "declared arity");
    assert_eq!(entity.attributes[5], Value::Integer(4), "BarCount");
    assert_eq!(
        entity.attributes[0],
        Value::Real(452.4),
        "area stays a real"
    );
}

/// The two required slots are refused when they carry no reading.
#[test]
fn a_blank_grade_or_negative_area_is_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    let mut draft = bar();
    draft.steel_grade = "   ";
    add_reinforcement_bar_properties(&mut tx, draft).expect_err("blank grade");

    let mut draft = bar();
    draft.total_cross_section_area = -1.0;
    add_reinforcement_bar_properties(&mut tx, draft).expect_err("negative area");

    let mut draft = bar();
    draft.total_cross_section_area = f64::NAN;
    add_reinforcement_bar_properties(&mut tx, draft).expect_err("non-finite area");

    assert!(tx.is_empty(), "nothing is staged when a rule fails");
}

/// `NominalBarDiameter` is a positive length; `EffectiveDepth` is a
/// plain length and may be negative.
#[test]
fn each_measure_slot_keeps_its_own_bound() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    let mut draft = bar();
    draft.nominal_bar_diameter = Some(0.0);
    add_reinforcement_bar_properties(&mut tx, draft).expect_err("diameter must exceed zero");

    let mut draft = bar();
    draft.effective_depth = Some(-25.0);
    add_reinforcement_bar_properties(&mut tx, draft).expect("a depth may be negative");
}

/// Enumeration tokens are closed and case-sensitive.
#[test]
fn tokens_outside_their_enumeration_are_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let profile = tx.create(Entity::new("IFCRECTANGLEPROFILEDEF", vec![]));

    let mut draft = bar();
    draft.bar_surface = Some("SMOOTH");
    add_reinforcement_bar_properties(&mut tx, draft).expect_err("not a bar surface");

    let mut draft = bar();
    draft.bar_surface = Some("plain");
    add_reinforcement_bar_properties(&mut tx, draft).expect_err("case-sensitive");

    add_section_properties(&mut tx, "VARIABLE", profile, None).expect_err("not a section type");
    add_section_properties(&mut tx, "UNIFORM", profile, None).expect("legal token");
}

/// Every `[1:?]` set in this family refuses an empty collection.
///
/// An empty aggregate satisfies the slot's presence and violates its
/// lower bound, so it parses and states nothing.
#[test]
fn empty_required_sets_are_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let profile = tx.create(Entity::new("IFCRECTANGLEPROFILEDEF", vec![]));
    let section = add_section_properties(&mut tx, "UNIFORM", profile, None).expect("section");

    add_section_reinforcement_properties(
        &mut tx,
        SectionReinforcementDraft {
            longitudinal_start_position: 0.0,
            longitudinal_end_position: 1000.0,
            transverse_position: None,
            reinforcement_role: "MAIN",
            section_definition: section,
            cross_section_reinforcement_definitions: &[],
        },
    )
    .expect_err("[1:?] bars");

    add_reinforcement_definition_properties(&mut tx, GUID, None, None, None, &[])
        .expect_err("[1:?] sections");

    add_profile_properties(&mut tx, None, None, &[], profile).expect_err("[1:?] properties");
}

/// A longitudinal station may be negative and the schema states no
/// ordering between start and end.
#[test]
fn longitudinal_positions_admit_negatives_and_any_order() {
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);
    let profile = tx.create(Entity::new("IFCRECTANGLEPROFILEDEF", vec![]));
    let section = add_section_properties(&mut tx, "TAPERED", profile, None).expect("section");
    let rebar = add_reinforcement_bar_properties(&mut tx, bar()).expect("bar");

    let id = add_section_reinforcement_properties(
        &mut tx,
        SectionReinforcementDraft {
            longitudinal_start_position: -250.0,
            longitudinal_end_position: -500.0,
            transverse_position: Some(-30.0),
            reinforcement_role: "SHEAR",
            section_definition: section,
            cross_section_reinforcement_definitions: &[rebar],
        },
    )
    .expect("negative stations are legal");
    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("entity");
    assert_eq!(entity.attributes.len(), 6, "declared arity");
    assert_eq!(entity.attributes[0], Value::Real(-250.0));
    assert_eq!(entity.attributes[3], Value::Enum("SHEAR".into()));
    assert_eq!(
        entity.attributes[5],
        Value::List(vec![Value::Ref(rebar)]),
        "bars land in their own slot"
    );
}

/// The one set-definition member carries IfcRoot slots; the rest do not.
#[test]
fn only_the_definition_set_carries_a_global_id() {
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);
    let profile = tx.create(Entity::new("IFCRECTANGLEPROFILEDEF", vec![]));
    let section = add_section_properties(&mut tx, "UNIFORM", profile, None).expect("section");
    let rebar = add_reinforcement_bar_properties(&mut tx, bar()).expect("bar");
    let reinforcement = add_section_reinforcement_properties(
        &mut tx,
        SectionReinforcementDraft {
            longitudinal_start_position: 0.0,
            longitudinal_end_position: 1000.0,
            transverse_position: None,
            reinforcement_role: "MAIN",
            section_definition: section,
            cross_section_reinforcement_definitions: &[rebar],
        },
    )
    .expect("reinforcement");

    let definition = add_reinforcement_definition_properties(
        &mut tx,
        GUID,
        Some("Deck reinforcement"),
        None,
        Some("Cast in place"),
        &[reinforcement],
    )
    .expect("definition");
    add_reinforcement_definition_properties(
        &mut tx,
        "not-a-guid",
        None,
        None,
        None,
        &[reinforcement],
    )
    .expect_err("malformed GlobalId");

    tx.commit(&mut model).expect("commit");

    let entity = model.get(definition).expect("entity");
    assert_eq!(entity.attributes.len(), 6, "declared arity");
    assert_eq!(entity.attributes[0], Value::Text(GUID.into()), "GlobalId");
    assert_eq!(entity.attributes[4], Value::Text("Cast in place".into()));

    // The abstraction members start at their own first attribute.
    let section_entity = model.get(section).expect("section");
    assert_eq!(section_entity.attributes.len(), 3);
    assert_eq!(section_entity.attributes[0], Value::Enum("UNIFORM".into()));
}

/// The hardcoded arities match both shipped schemas.
#[test]
fn declared_arities_match_the_schema() {
    for schema in [ifc4(), ifc4x3()] {
        for (name, arity) in [
            ("IFCREINFORCEMENTBARPROPERTIES", 6),
            ("IFCSECTIONPROPERTIES", 3),
            ("IFCSECTIONREINFORCEMENTPROPERTIES", 6),
            ("IFCREINFORCEMENTDEFINITIONPROPERTIES", 6),
            ("IFCPROFILEPROPERTIES", 4),
        ] {
            let declared = schema.attributes(name).len();
            assert_eq!(declared, arity, "{name} arity");
        }
    }
}

/// `ReinforcementRole` is closed too, and has its own guard.
///
/// Each entity states its enumeration separately, so one shared test
/// over bar surface and section type leaves this one unchecked.
#[test]
fn a_role_outside_its_enumeration_is_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let profile = tx.create(Entity::new("IFCRECTANGLEPROFILEDEF", vec![]));
    let section = add_section_properties(&mut tx, "UNIFORM", profile, None).expect("section");
    let rebar = add_reinforcement_bar_properties(&mut tx, bar()).expect("bar");

    let mut draft = SectionReinforcementDraft {
        longitudinal_start_position: 0.0,
        longitudinal_end_position: 1000.0,
        transverse_position: None,
        reinforcement_role: "TENSION",
        section_definition: section,
        cross_section_reinforcement_definitions: &[rebar],
    };
    add_section_reinforcement_properties(&mut tx, draft).expect_err("not a bar role");

    draft.reinforcement_role = "main";
    add_section_reinforcement_properties(&mut tx, draft).expect_err("case-sensitive");

    draft.reinforcement_role = "LIGATURE";
    add_section_reinforcement_properties(&mut tx, draft).expect("legal token");
}
