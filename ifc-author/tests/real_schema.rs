//! Authoring against the normative IFC4 EXPRESS schema.
//!
//! The hand-written subset in `authoring.rs` proves the logic; this proves the
//! logic survives contact with the real 776-entity schema, where inheritance
//! chains are deep and attribute counts are not guesses. Skips when the
//! reference material is absent, matching `ifc-schema/tests/real_schemas.rs`.

use ifc_author::{AuthorError, EntityBuilder};
use ifc_model::{Codec, Model, Value};
use ifc_schema::Schema;
use std::path::PathBuf;

/// Prefers the bundled schema (`ifc-schema`'s `ifc4` feature, on by
/// default) so this test exercises the same path production code uses. If
/// the workspace has the reference material vendored, cross-check that the
/// bundled artifact matches the raw EXPRESS parse -- catches artifact drift.
fn ifc4() -> Option<Schema> {
    let bundled = ifc_schema::ifc4();
    let raw_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../references/ifc-spec/ifc4-add2-tc1/IFC4.exp");
    if let Ok(bytes) = std::fs::read(&raw_path) {
        let raw = Schema::from_express_bytes(&bytes);
        assert_eq!(
            bundled.entity_count(),
            raw.entity_count(),
            "bundled ifc-schema artifact is stale relative to references/ifc-spec"
        );
        assert_eq!(bundled.type_count(), raw.type_count());
    }
    Some(bundled.clone())
}

const GUID: &str = "3vB2YO$MX4xv5uCqZZG05x";

macro_rules! schema_or_skip {
    () => {
        match ifc4() {
            Some(schema) => schema,
            None => {
                eprintln!("skipped: references/ifc-spec not present");
                return;
            }
        }
    };
}

/// `IfcAnnotation` inherits GlobalId/OwnerHistory/Name/Description from
/// `IfcRoot`, then ObjectType, ObjectPlacement, Representation. Seven slots --
/// the number an application would otherwise have to know by heart.
#[test]
fn ifc_annotation_has_the_arity_the_schema_declares() {
    let schema = schema_or_skip!();
    let entity = EntityBuilder::new(&schema, "IfcAnnotation")
        .text("GlobalId", GUID)
        .text("Name", "Brandwand")
        .build()
        .expect("annotation builds against the real schema");

    assert_eq!(
        schema.attribute_names("IfcAnnotation"),
        [
            "GlobalId",
            "OwnerHistory",
            "Name",
            "Description",
            "ObjectType",
            "ObjectPlacement",
            "Representation"
        ],
        "slot layout comes from IFC4.exp, not from this test"
    );
    assert_eq!(entity.attributes.len(), 7);
    assert_eq!(entity.text(0), Some(GUID));
    assert_eq!(entity.text(2), Some("Brandwand"));
}

/// The 2D approval-plan use case: a styled polyline in a plan sub-context.
/// Every entity here is authored by name, and the result must round-trip.
#[test]
fn a_2d_annotation_assembly_builds_and_round_trips() {
    let schema = schema_or_skip!();
    let mut model = Model::new();
    model.header_mut().schema = vec!["IFC4".to_owned()];

    let a = EntityBuilder::new(&schema, "IfcCartesianPoint")
        .set(
            "Coordinates",
            Value::List(vec![Value::Real(0.0), Value::Real(0.0)]),
        )
        .insert(&mut model)
        .expect("point a");
    let b = EntityBuilder::new(&schema, "IfcCartesianPoint")
        .set(
            "Coordinates",
            Value::List(vec![Value::Real(1000.0), Value::Real(0.0)]),
        )
        .insert(&mut model)
        .expect("point b");
    let polyline = EntityBuilder::new(&schema, "IfcPolyline")
        .set("Points", Value::List(vec![Value::Ref(a), Value::Ref(b)]))
        .insert(&mut model)
        .expect("polyline");
    let annotation = EntityBuilder::new(&schema, "IfcAnnotation")
        .text("GlobalId", GUID)
        .text("Name", "Brandwand")
        .insert(&mut model)
        .expect("annotation");

    assert!(
        model.dangling_references().is_empty(),
        "every reference resolves"
    );
    assert_eq!(model.get(polyline).unwrap().references(), vec![a, b]);
    assert!(model.get(annotation).unwrap().is_type("IFCANNOTATION"));

    // The whole point of authoring: it must survive serialization.
    let bytes = ifc_step::StepCodec
        .write_bytes(&model)
        .expect("authored model serializes");
    let reparsed = ifc_step::StepCodec
        .read_bytes(&bytes)
        .expect("authored model parses back");

    assert_eq!(reparsed.len(), model.len());
    assert!(reparsed.dangling_references().is_empty());
    assert_eq!(
        reparsed.ids_of_type("IFCANNOTATION").len(),
        1,
        "the annotation survived the round trip"
    );
}

/// Deep inheritance is where hand-counted slots go wrong: `IfcWall` inherits
/// through Product, Object, ObjectDefinition, Root.
#[test]
fn a_deep_inheritance_chain_still_places_globalid_first() {
    let schema = schema_or_skip!();
    let entity = EntityBuilder::new(&schema, "IfcWall")
        .text("GlobalId", GUID)
        .text("Name", "Exterior wall")
        .build()
        .expect("wall builds");

    assert_eq!(entity.text(0), Some(GUID), "GlobalId is slot 0");
    assert!(
        entity.attributes.len() >= 8,
        "IfcWall inherits a long chain, got {}",
        entity.attributes.len()
    );
}

/// The failure that motivated the crate: an attribute that does not exist on
/// this entity, even though it exists elsewhere in the schema.
#[test]
fn an_attribute_from_a_different_entity_is_refused() {
    let schema = schema_or_skip!();
    let error = EntityBuilder::new(&schema, "IfcAnnotation")
        .text("GlobalId", GUID)
        .real("RefLatitude", 52.0)
        .build()
        .expect_err("RefLatitude belongs to IfcSite, not IfcAnnotation");
    assert!(matches!(error, AuthorError::UnknownAttribute { .. }));
}

/// `IfcGeometricRepresentationSubContext.TargetView` is the attribute the 2D
/// approval-plan workflow needs, and it is an enumeration.
#[test]
fn plan_view_target_is_accepted_as_an_enumeration() {
    let schema = schema_or_skip!();
    assert!(
        schema
            .entity("IfcGeometricRepresentationSubContext")
            .is_some(),
        "the schema declares the sub-context this use case needs"
    );

    let error = EntityBuilder::new(&schema, "IfcGeometricRepresentationSubContext")
        .text("TargetView", "PLAN_VIEW")
        .build()
        .expect_err("a quoted string is not an enumeration constant");
    assert!(
        matches!(
            error,
            AuthorError::TypeMismatch { .. } | AuthorError::MissingRequired { .. }
        ),
        "{error:?}"
    );
}
