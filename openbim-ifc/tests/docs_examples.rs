//! Compile-and-run proof for the code shown in `docs/`.
//!
//! Documentation that ships uncompiled code is a liability: it drifts silently
//! and a coding agent will reproduce the drift. Every Rust snippet published on
//! the documentation site has a counterpart here, so `cargo test` fails when a
//! documented API changes shape.
//!
//! When editing a docs page, update the matching test in this file. The page
//! and the test are a pair.
//!
//! Every example reads or writes STEP, so the file is gated on that feature:
//! the facade's own `--no-default-features` matrix build must still compile.
#![cfg(feature = "step")]

use std::sync::Arc;

use ifc::{Codec, Entity, Model, StepCodec, Value};

/// Minimal schema-valid STEP payload used by the read-side examples.
///
/// Carries one `IfcAnnotation` so the lossless-passthrough claim on the
/// 2D approval-plan page is exercised against a real parse, not asserted.
const SOURCE: &[u8] = b"ISO-10303-21;\n\
HEADER;\n\
FILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('plan.ifc','',(''),(''),'','','');\n\
FILE_SCHEMA(('IFC4'));\n\
ENDSEC;\n\
DATA;\n\
#1= IFCANNOTATION('3vB2YO$MX4xv5uCqZZG05x',$,'Brandwand',$,$,$,$);\n\
#2= IFCPOLYLINE((#3,#4));\n\
#3= IFCCARTESIANPOINT((0.,0.));\n\
#4= IFCCARTESIANPOINT((1000.,0.));\n\
ENDSEC;\n\
END-ISO-10303-21;\n";

/// `docs/index.md` -- the install/overview snippet.
#[test]
fn overview_snippet_reads_a_model() {
    let model = StepCodec.read_bytes(SOURCE).expect("read");
    assert_eq!(model.len(), 4);
}

/// `docs/use-cases/2d-approval-plans.md` -- lossless passthrough.
///
/// The point of the example is that an entity this build does not interpret
/// still survives a read/write cycle. No domain crate is involved.
#[test]
fn unknown_entities_survive_a_round_trip() {
    let model = StepCodec.read_bytes(SOURCE).expect("read");

    // `ids_of_type` takes the upper-case STEP type name, as the page states.
    let annotations = model.ids_of_type("IFCANNOTATION");
    assert_eq!(annotations.len(), 1);

    let out = StepCodec.write_bytes(&model).expect("write");
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("IFCANNOTATION"));
    assert!(text.contains("Brandwand"));
}

/// `docs/use-cases/2d-approval-plans.md` -- the authoring example.
///
/// This test exists to keep the documented attribute order honest: it asserts
/// the arity the page publishes and proves the result serializes.
#[test]
fn authoring_example_builds_a_serializable_annotation() {
    let mut model = Model::new();

    let id = model.push(Entity::new(
        "IFCANNOTATION",
        vec![
            Value::Text(Arc::from("3vB2YO$MX4xv5uCqZZG05x")), // GlobalId
            Value::Null,                                      // OwnerHistory
            Value::Text(Arc::from("Brandwand")),              // Name
            Value::Null,                                      // Description
            Value::Null,                                      // ObjectType
            Value::Null,                                      // ObjectPlacement
            Value::Null,                                      // Representation
        ],
    ));

    let entity = model.get(id).expect("entity present");
    assert!(entity.is_type("IFCANNOTATION"));
    assert_eq!(entity.text(2), Some("Brandwand"));

    let out = StepCodec.write_bytes(&model).expect("write");
    assert!(String::from_utf8_lossy(&out).contains("IFCANNOTATION"));
}

/// `docs/capabilities.md` -- the authoring gap.
///
/// The matrix states that construction is positional and untyped. If a typed
/// builder API ever lands, this test should be replaced by one that uses it,
/// and the capability row updated in the same commit.
#[test]
fn model_exposes_only_positional_construction() {
    let mut model = Model::new();
    let id = model.push(Entity::new("IFCWALL", vec![Value::Null; 8]));
    assert_eq!(
        model.get(id).map(|e| e.attribute(0)),
        Some(Some(&Value::Null))
    );
}
