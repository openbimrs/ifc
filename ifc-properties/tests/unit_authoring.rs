//! Unit authoring: the project measurement context.
//!
//! A wrong unit is the most dangerous authoring bug in IFC: a mis-declared
//! prefix parses cleanly, validates cleanly, and silently scales every
//! measure in the file. These tests pin the refusals that prevent it.

use ifc_model::{Model, Transaction};
use ifc_properties::{
    add_derived_unit, add_derived_unit_element, add_dimensional_exponents, add_si_unit,
    assign_units, project_units, SiUnitDraft,
};

#[test]
fn a_millimetre_project_context_commits_and_reads_back() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let mm = add_si_unit(
        &mut tx,
        SiUnitDraft {
            unit_type: "LENGTHUNIT",
            name: "METRE",
            prefix: Some("MILLI"),
        },
    )
    .expect("millimetre is a valid SI unit");
    let assignment = assign_units(&mut tx, &[mm]).expect("one unit is enough");
    tx.commit(&mut model).expect("unit context commits");
    assert!(
        model.get(assignment).is_some(),
        "assignment reached the model"
    );
    let units = project_units(&model);
    assert_eq!(units.len(), 1, "exactly the authored unit is visible");
}

#[test]
fn a_prefix_the_schema_does_not_define_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let refused = add_si_unit(
        &mut tx,
        SiUnitDraft {
            unit_type: "LENGTHUNIT",
            name: "METRE",
            prefix: Some("MILLIMETRE"),
        },
    );
    assert!(
        refused.is_err(),
        "MILLIMETRE is an IfcSIUnitName, not an IfcSIPrefix: accepting it would \
         write a unit no reader can scale correctly"
    );
    assert!(tx.is_empty(), "a refused draft stages nothing");
}

#[test]
fn an_empty_unit_assignment_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    assert!(
        assign_units(&mut tx, &[]).is_err(),
        "an empty assignment parses but leaves every measure dimensionless"
    );
}

#[test]
fn a_zero_exponent_derived_unit_element_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let metre = add_si_unit(
        &mut tx,
        SiUnitDraft {
            unit_type: "LENGTHUNIT",
            name: "METRE",
            prefix: None,
        },
    )
    .expect("metre");
    assert!(
        add_derived_unit_element(&mut tx, metre, 0).is_err(),
        "exponent zero contributes nothing and hides an authoring mistake"
    );
}

#[test]
fn a_derived_unit_composes_base_units_with_exponents() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let metre = add_si_unit(
        &mut tx,
        SiUnitDraft {
            unit_type: "LENGTHUNIT",
            name: "METRE",
            prefix: None,
        },
    )
    .expect("metre");
    let cubed = add_derived_unit_element(&mut tx, metre, 3).expect("m^3");
    let volume = add_derived_unit(&mut tx, &[cubed], "VOLUMEUNIT", None)
        .expect("a one-element derived unit is well formed");
    let _dims = add_dimensional_exponents(&mut tx, [3, 0, 0, 0, 0, 0, 0]);
    tx.commit(&mut model).expect("derived unit commits");
    assert!(
        model.get(volume).is_some(),
        "derived unit reached the model"
    );
}

#[test]
fn a_derived_unit_without_elements_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    assert!(
        add_derived_unit(&mut tx, &[], "VOLUMEUNIT", None).is_err(),
        "a derived unit with no elements has no dimension at all"
    );
}
