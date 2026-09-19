//! Material relationships, properties and classifications authored,
//! then read back through this crate's own readers.
//!
//! Slot order comes from the bundled IFC4X3 schema. Asserting through
//! the readers rather than on raw slots is the point: an authored
//! record is only correct if the reader that already existed finds it.

use ifc_material::{
    create_material, create_material_classification_relationship,
    create_material_definition_representation, create_material_properties,
    create_material_relationship, create_profile_with_offsets, MaterialDraft, MaterialView,
    ProfileDraft,
};
use ifc_model::codec::Codec;
use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_step::StepCodec;

/// A material to relate.
fn material(tx: &mut Transaction, name: &str) -> EntityId {
    create_material(
        tx,
        MaterialDraft {
            name,
            description: None,
            category: None,
        },
    )
}

/// A concrete mix relates to its constituent materials.
#[test]
fn an_authored_material_relationship_reads_back() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let concrete = material(&mut tx, "C30/37");
    let cement = material(&mut tx, "CEM I 42.5N");
    let aggregate = material(&mut tx, "Gravel 4/32");
    create_material_relationship(
        &mut tx,
        &model,
        Some("Mix"),
        None,
        concrete,
        &[cement, aggregate],
        Some("1:2:4"),
    )
    .expect("authored relationship");
    tx.commit(&mut model).expect("commit");

    let view = MaterialView::new(&model);
    let found: Vec<_> = view.material_relationships().collect();
    assert_eq!(found.len(), 1, "one relationship in the model");
    let rel = found[0];
    assert_eq!(rel.name().expect("name"), Some("Mix"));
    assert_eq!(rel.relating_material_id().expect("relating"), concrete);
    assert_eq!(
        rel.related_material_ids().expect("related"),
        vec![cement, aggregate],
        "order is preserved as authored"
    );
    assert_eq!(rel.expression().expect("expression"), Some("1:2:4"));
}

/// Properties attach to any material definition, not only IfcMaterial.
#[test]
fn authored_material_properties_read_back() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let steel = material(&mut tx, "S355");
    let density = tx.create(Entity::new(
        "IFCPROPERTYSINGLEVALUE",
        vec![
            Value::Text("MassDensity".into()),
            Value::Null,
            Value::Real(7850.0),
            Value::Null,
        ],
    ));
    create_material_properties(
        &mut tx,
        &model,
        Some("Pset_MaterialCommon"),
        None,
        &[density],
        steel,
    )
    .expect("authored properties");
    tx.commit(&mut model).expect("commit");

    let view = MaterialView::new(&model);
    let found: Vec<_> = view.material_properties().collect();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name().expect("name"), Some("Pset_MaterialCommon"));
    assert_eq!(found[0].material_id().expect("material"), steel);
    assert_eq!(found[0].property_ids().expect("ids"), vec![density]);
}

/// A classified material reads back through the classification reader.
#[test]
fn an_authored_classification_reads_back() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let brick = material(&mut tx, "Clay brick");
    let reference = tx.create(Entity::new(
        "IFCCLASSIFICATIONREFERENCE",
        vec![
            Value::Null,
            Value::Text("Pr_20_93_07".into()),
            Value::Text("Bricks".into()),
            Value::Null,
            Value::Null,
            Value::Null,
        ],
    ));
    create_material_classification_relationship(&mut tx, &model, &[reference], brick)
        .expect("authored classification");
    tx.commit(&mut model).expect("commit");

    let view = MaterialView::new(&model);
    let found: Vec<_> = view.classification_relationships().collect();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].material_id().expect("material"), brick);
    assert_eq!(found[0].classification_ids().expect("ids"), vec![reference]);
}

/// Records that parse but carry no meaning are refused.
///
/// Each SET is `[1:?]` in the schema, so an empty aggregate is not an
/// under-specified record but a malformed one. The self-reference case
/// is worse: it parses, and a reader walking composition cycles.
#[test]
fn meaningless_relationships_are_refused() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let a = material(&mut tx, "A");
    let b = material(&mut tx, "B");
    tx.commit(&mut model).expect("commit");
    let mut tx = Transaction::new(&model);

    assert!(
        create_material_relationship(&mut tx, &model, None, None, a, &[], None).is_err(),
        "an empty related set states no relationship"
    );
    assert!(
        create_material_relationship(&mut tx, &model, None, None, a, &[b, a], None).is_err(),
        "a material cannot be derived from itself"
    );
    assert!(
        create_material_properties(&mut tx, &model, None, None, &[], a).is_err(),
        "a property set with no properties is malformed"
    );
    assert!(
        create_material_classification_relationship(&mut tx, &model, &[], a).is_err(),
        "an unclassified classification relationship is malformed"
    );
}

/// The schema's OnlyStyledRepresentations rule is enforced, not deferred.
///
/// A surface style hung on a plain IfcShapeRepresentation parses and
/// then renders as nothing, which is why the schema states the rule
/// and why refusing it here is worth more than a validator warning.
#[test]
fn only_styled_representations_are_accepted() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let paint = material(&mut tx, "Paint");
    let styled = tx.create(Entity::new(
        "IFCSTYLEDREPRESENTATION",
        vec![Value::Null, Value::Null, Value::Null, Value::List(vec![])],
    ));
    let shape = tx.create(Entity::new(
        "IFCSHAPEREPRESENTATION",
        vec![Value::Null, Value::Null, Value::Null, Value::List(vec![])],
    ));
    tx.commit(&mut model).expect("commit");
    let mut tx = Transaction::new(&model);

    assert!(
        create_material_definition_representation(&mut tx, &model, None, None, &[shape], paint)
            .is_err(),
        "a plain shape representation is not a styled one"
    );
    assert!(
        create_material_definition_representation(&mut tx, &model, None, None, &[], paint).is_err(),
        "an empty representation list is malformed"
    );
    create_material_definition_representation(&mut tx, &model, None, None, &[styled], paint)
        .expect("a styled representation is accepted");
}

/// The offset profile variant survives STEP text with its array intact.
///
/// OffsetValues is an ARRAY [1:2]; a writer that flattened or reordered
/// it would still produce a parseable file with the profile shifted.
#[test]
fn an_offset_profile_survives_step_text() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let steel = material(&mut tx, "S355");
    let profile = tx.create(Entity::new(
        "IFCRECTANGLEPROFILEDEF",
        vec![
            Value::Enum("AREA".into()),
            Value::Text("200x100".into()),
            Value::Null,
            Value::Real(200.0),
            Value::Real(100.0),
        ],
    ));
    tx.commit(&mut model).expect("commit");
    let mut tx = Transaction::new(&model);

    let draft = ProfileDraft {
        name: Some("Flange"),
        description: None,
        material: Some(steel),
        profile,
        priority: Some(40),
        category: Some("Load bearing"),
    };
    let mut over = draft;
    over.priority = Some(101);
    assert!(
        create_profile_with_offsets(&mut tx, &model, over, [1.0, 2.0]).is_err(),
        "Priority is a percentage; 101 is outside 0..=100"
    );
    let mut bad = draft;
    bad.priority = Some(-1);
    assert!(
        create_profile_with_offsets(&mut tx, &model, bad, [1.0, 2.0]).is_err(),
        "a negative priority is outside the range too"
    );
    assert!(
        create_profile_with_offsets(&mut tx, &model, draft, [f64::NAN, 1.0]).is_err(),
        "a non-finite offset is not a length"
    );
    let id = create_profile_with_offsets(&mut tx, &model, draft, [12.5, -7.5])
        .expect("authored offset profile");
    tx.commit(&mut model).expect("commit");

    let mut bytes = Vec::new();
    StepCodec.write(&model, &mut bytes).expect("written");
    let reparsed = StepCodec.read_bytes(&bytes).expect("reparsed");
    let entity = reparsed.get(id).expect("survives the text seam");
    assert_eq!(entity.type_name.as_ref(), "IFCMATERIALPROFILEWITHOFFSETS");
    let Some(Value::List(offsets)) = entity.attributes.get(6) else {
        panic!("OffsetValues is slot 6, after the six inherited profile slots");
    };
    assert_eq!(offsets.len(), 2, "ARRAY [1:2] keeps both bounds");
    assert!(matches!(offsets[0], Value::Real(v) if (v - 12.5).abs() < 1e-9));
    assert!(matches!(offsets[1], Value::Real(v) if (v + 7.5).abs() < 1e-9));
}
