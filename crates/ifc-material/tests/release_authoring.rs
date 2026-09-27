//! #77: authoring lays records out in the release the model declares.
//!
//! An IFC2X3 model gets IFC2X3 TC1 records or a typed refusal, never an
//! IFC4 layout; IFC4X3 gets its own attribute names.

use ifc_material::{
    associate_material, associate_material_with_owner_history, create_constituent,
    create_constituent_set, create_layer, create_layer_set, create_layer_set_usage,
    create_layer_with_offsets, create_material, create_material_list, create_material_properties,
    create_material_relationship, create_profile, create_profile_set, ConstituentDraft,
    DirectionSense, LayerDraft, LayerSetDirection, LayerSetDraft, MaterialAssignmentDraft,
    MaterialDraft, MaterialError, MaterialView, ProfileDraft, SchemaVersion,
};
use ifc_model::codec::Codec;
use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_step::StepCodec;

fn declared(schemas: &[&str]) -> Model {
    let mut model = Model::new();
    model.header_mut().schema = schemas.iter().map(|s| (*s).to_owned()).collect();
    model
}

fn gid(seed: u8) -> String {
    ifc_model::guid::Guid::from_uuid([seed; 16]).to_string()
}

fn draft(name: &str) -> MaterialDraft<'_> {
    MaterialDraft {
        name,
        description: None,
        category: None,
    }
}

fn layer(material: EntityId, thickness: f64) -> LayerDraft<'static> {
    LayerDraft {
        material: Some(material),
        thickness,
        is_ventilated: None,
        name: None,
        description: None,
        category: None,
        priority: None,
    }
}

fn assignment<'a>(
    global_id: &'a str,
    objects: &'a [EntityId],
    material: EntityId,
) -> MaterialAssignmentDraft<'a> {
    MaterialAssignmentDraft {
        global_id,
        name: None,
        description: None,
        related_objects: objects,
        relating_material: material,
    }
}

fn not_in_schema(entity: &'static str, attribute: &'static str) -> MaterialError {
    MaterialError::AuthoringNotInSchema {
        entity,
        attribute,
        schema: SchemaVersion::Ifc2x3,
    }
}

fn no_entity(entity: &'static str) -> MaterialError {
    MaterialError::EntityNotInSchema {
        entity,
        id: None,
        schema: SchemaVersion::Ifc2x3,
    }
}

/// IFC2X3 records carry exactly the IFC2X3 attributes, read back through
/// the IFC2X3 view and across a STEP round trip.
#[test]
fn ifc2x3_authoring_writes_ifc2x3_layouts() {
    let mut model = declared(&["IFC2X3"]);
    let owner = model.push(Entity::new("IFCOWNERHISTORY", vec![Value::Null; 8]));
    let wall = model.push(Entity::new("IFCWALL", vec![Value::Null; 8]));
    let mut tx = Transaction::new(&model);
    let brick = create_material(&mut tx, &model, draft("Brick")).unwrap();
    let core = create_layer(&mut tx, &model, layer(brick, 0.24)).unwrap();
    let set = create_layer_set(
        &mut tx,
        &model,
        LayerSetDraft {
            layers: &[core],
            name: Some("Wall 240"),
            description: None,
        },
    )
    .unwrap();
    let usage = create_layer_set_usage(
        &mut tx,
        &model,
        set,
        LayerSetDirection::Axis2,
        DirectionSense::Positive,
        -0.12,
        None,
    )
    .unwrap();
    let list = create_material_list(&mut tx, &model, &[brick]).unwrap();
    let relation = associate_material_with_owner_history(
        &mut tx,
        &model,
        assignment(&gid(1), &[wall], usage),
        owner,
    )
    .unwrap();
    tx.commit(&mut model).unwrap();

    let arity = |id: EntityId| model.get(id).unwrap().attributes.len();
    assert_eq!(arity(brick), 1, "IfcMaterial(Name)");
    assert_eq!(
        arity(core),
        3,
        "IfcMaterialLayer(Material, LayerThickness, IsVentilated)"
    );
    assert_eq!(
        arity(set),
        2,
        "IfcMaterialLayerSet(MaterialLayers, LayerSetName)"
    );
    assert_eq!(arity(usage), 4, "no ReferenceExtent before IFC4");
    assert_eq!(arity(list), 1);
    assert_eq!(arity(relation), 6);
    assert_eq!(
        model.get(relation).unwrap().attributes[1],
        Value::Ref(owner),
        "IFC2X3 OwnerHistory is required and supplied"
    );

    let mut bytes = Vec::new();
    StepCodec.write(&model, &mut bytes).unwrap();
    let reread = StepCodec.read_bytes(&bytes).unwrap();
    let view = MaterialView::new(&reread);
    assert_eq!(view.schema(), Ok(SchemaVersion::Ifc2x3));
    let resolved = view.assigned_material(wall).unwrap().unwrap();
    assert!(matches!(
        resolved.material,
        ifc_material::ResolvedMaterialSelect::Usage(_)
    ));
    let layer = view.layers().next().unwrap();
    assert_eq!(layer.thickness().unwrap(), 0.24);
    assert_eq!(
        view.layer_sets().next().unwrap().name().unwrap(),
        Some("Wall 240")
    );
}

/// A draft value IFC2X3 cannot hold is refused, never dropped, and nothing
/// is staged.
#[test]
fn ifc2x3_authoring_refuses_values_it_cannot_hold() {
    let mut model = declared(&["IFC2X3"]);
    let wall = model.push(Entity::new("IFCWALL", vec![Value::Null; 8]));
    let brick = model.push(Entity::new(
        "IFCMATERIAL",
        vec![Value::Text("Brick".into())],
    ));
    let mut tx = Transaction::new(&model);

    let categorised = MaterialDraft {
        category: Some("Masonry"),
        ..draft("Brick")
    };
    assert_eq!(
        create_material(&mut tx, &model, categorised).unwrap_err(),
        not_in_schema("IFCMATERIAL", "Category")
    );
    let described = MaterialDraft {
        description: Some("Clay"),
        ..draft("Brick")
    };
    assert_eq!(
        create_material(&mut tx, &model, described).unwrap_err(),
        not_in_schema("IFCMATERIAL", "Description")
    );
    for (named, attribute) in [
        (
            LayerDraft {
                name: Some("Core"),
                ..layer(brick, 0.2)
            },
            "Name",
        ),
        (
            LayerDraft {
                category: Some("LoadBearing"),
                ..layer(brick, 0.2)
            },
            "Category",
        ),
        (
            LayerDraft {
                priority: Some(10),
                ..layer(brick, 0.2)
            },
            "Priority",
        ),
    ] {
        assert_eq!(
            create_layer(&mut tx, &model, named).unwrap_err(),
            not_in_schema("IFCMATERIALLAYER", attribute)
        );
    }
    // IfcPositiveLengthMeasure: zero is no IFC2X3 thickness.
    assert!(matches!(
        create_layer(&mut tx, &model, layer(brick, 0.0)),
        Err(MaterialError::AuthoringInvalid {
            attribute: "LayerThickness",
            ..
        })
    ));
    let core = model.push(Entity::new(
        "IFCMATERIALLAYER",
        vec![Value::Ref(brick), Value::Real(0.2), Value::Null],
    ));
    assert_eq!(
        create_layer_set(
            &mut tx,
            &model,
            LayerSetDraft {
                layers: &[core],
                name: None,
                description: Some("External"),
            },
        )
        .unwrap_err(),
        not_in_schema("IFCMATERIALLAYERSET", "Description")
    );
    // OwnerHistory is required in IFC2X3 and never invented.
    assert_eq!(
        associate_material(&mut tx, &model, assignment(&gid(1), &[wall], brick)).unwrap_err(),
        MaterialError::AuthoringRequired {
            entity: "IFCRELASSOCIATESMATERIAL",
            attribute: "OwnerHistory",
            schema: SchemaVersion::Ifc2x3,
        }
    );
    assert!(tx.is_empty(), "no refused record was staged");
}

#[test]
fn ifc2x3_authoring_refuses_a_reference_extent() {
    let mut model = declared(&["IFC2X3"]);
    let brick = model.push(Entity::new(
        "IFCMATERIAL",
        vec![Value::Text("Brick".into())],
    ));
    let core = model.push(Entity::new(
        "IFCMATERIALLAYER",
        vec![Value::Ref(brick), Value::Real(0.2), Value::Null],
    ));
    let set = model.push(Entity::new(
        "IFCMATERIALLAYERSET",
        vec![Value::List(vec![Value::Ref(core)]), Value::Null],
    ));
    let mut tx = Transaction::new(&model);
    assert_eq!(
        create_layer_set_usage(
            &mut tx,
            &model,
            set,
            LayerSetDirection::Axis2,
            DirectionSense::Positive,
            0.0,
            Some(3.0),
        )
        .unwrap_err(),
        not_in_schema("IFCMATERIALLAYERSETUSAGE", "ReferenceExtent")
    );
    assert!(tx.is_empty());
}

/// Entities IFC2X3 does not declare cannot be authored into it.
#[test]
fn ifc2x3_authoring_refuses_entities_it_lacks() {
    let mut model = declared(&["IFC2X3"]);
    let brick = model.push(Entity::new(
        "IFCMATERIAL",
        vec![Value::Text("Brick".into())],
    ));
    let profile = model.push(Entity::new("IFCRECTANGLEPROFILEDEF", vec![Value::Null; 5]));
    let mut tx = Transaction::new(&model);
    let constituent = ConstituentDraft {
        name: None,
        description: None,
        material: brick,
        fraction: Some(1.0),
        category: None,
    };
    assert_eq!(
        create_constituent(&mut tx, &model, constituent).unwrap_err(),
        no_entity("IFCMATERIALCONSTITUENT")
    );
    assert_eq!(
        create_constituent_set(&mut tx, &model, &[brick], None, None).unwrap_err(),
        no_entity("IFCMATERIALCONSTITUENTSET")
    );
    let profile_draft = ProfileDraft {
        name: None,
        description: None,
        material: Some(brick),
        profile,
        priority: None,
        category: None,
    };
    assert_eq!(
        create_profile(&mut tx, &model, profile_draft).unwrap_err(),
        no_entity("IFCMATERIALPROFILE")
    );
    assert_eq!(
        create_profile_set(&mut tx, &model, &[profile], None, None, None).unwrap_err(),
        no_entity("IFCMATERIALPROFILESET")
    );
    assert_eq!(
        create_layer_with_offsets(
            &mut tx,
            &model,
            layer(brick, 0.2),
            LayerSetDirection::Axis3,
            [0.0, 0.1],
        )
        .unwrap_err(),
        no_entity("IFCMATERIALLAYERWITHOFFSETS")
    );
    assert_eq!(
        create_material_relationship(&mut tx, &model, None, None, brick, &[profile], None)
            .unwrap_err(),
        no_entity("IFCMATERIALRELATIONSHIP")
    );
    assert_eq!(
        create_material_properties(&mut tx, &model, None, None, &[profile], brick).unwrap_err(),
        no_entity("IFCMATERIALPROPERTIES"),
        "IFC2X3 IfcMaterialProperties is abstract"
    );
    // A layer-with-offsets record is no IFC2X3 layer-set member.
    let offsets = model.push(Entity::new(
        "IFCMATERIALLAYERWITHOFFSETS",
        vec![Value::Null; 9],
    ));
    assert!(matches!(
        create_layer_set(
            &mut tx,
            &model,
            LayerSetDraft {
                layers: &[offsets],
                name: None,
                description: None,
            },
        ),
        Err(MaterialError::AuthoringReferenceType { .. })
    ));
    assert!(tx.is_empty());
}

/// IFC4 keeps the 0.2.0 layouts, and so does a header with no declaration.
#[test]
fn ifc4_authoring_writes_ifc4_layouts() {
    for schemas in [&["IFC4"][..], &[]] {
        let mut model = declared(schemas);
        let wall = model.push(Entity::new("IFCWALL", vec![Value::Null; 8]));
        let mut tx = Transaction::new(&model);
        let brick = create_material(
            &mut tx,
            &model,
            MaterialDraft {
                category: Some("Masonry"),
                ..draft("Brick")
            },
        )
        .unwrap();
        let core = create_layer(
            &mut tx,
            &model,
            LayerDraft {
                name: Some("Core"),
                priority: Some(80),
                ..layer(brick, 0.0)
            },
        )
        .unwrap();
        let set = create_layer_set(
            &mut tx,
            &model,
            LayerSetDraft {
                layers: &[core],
                name: None,
                description: Some("External"),
            },
        )
        .unwrap();
        let usage = create_layer_set_usage(
            &mut tx,
            &model,
            set,
            LayerSetDirection::Axis2,
            DirectionSense::Negative,
            0.0,
            Some(3.0),
        )
        .unwrap();
        // IFC4 OwnerHistory is OPTIONAL: no owner history is needed.
        let relation =
            associate_material(&mut tx, &model, assignment(&gid(1), &[wall], core)).unwrap();
        tx.commit(&mut model).unwrap();
        let arity = |id: EntityId| model.get(id).unwrap().attributes.len();
        assert_eq!(
            [
                arity(brick),
                arity(core),
                arity(set),
                arity(usage),
                arity(relation)
            ],
            [3, 7, 3, 5, 6],
            "{schemas:?}"
        );
        assert_eq!(
            model.get(relation).unwrap().attributes[1],
            Value::Null,
            "no owner history is invented"
        );
        let view = MaterialView::new(&model);
        assert_eq!(
            view.materials().next().unwrap().category().unwrap(),
            Some("Masonry")
        );
        assert_eq!(view.layers().next().unwrap().priority().unwrap(), Some(80));
    }
}

/// IFC4X3 writes the relationship's `MaterialExpression` at slot 4, and
/// IFC4-onwards entities are available.
#[test]
fn ifc4x3_authoring_writes_ifc4x3_layouts() {
    let mut model = declared(&["IFC4X3_ADD2"]);
    let mut tx = Transaction::new(&model);
    let concrete = create_material(&mut tx, &model, draft("C30/37")).unwrap();
    let cement = create_material(&mut tx, &model, draft("CEM I")).unwrap();
    let relationship = create_material_relationship(
        &mut tx,
        &model,
        Some("Mix"),
        None,
        concrete,
        &[cement],
        Some("CEM I 42.5N"),
    )
    .unwrap();
    let constituent = create_constituent(
        &mut tx,
        &model,
        ConstituentDraft {
            name: Some("Binder"),
            description: None,
            material: cement,
            fraction: Some(0.2),
            category: None,
        },
    )
    .unwrap();
    let set = create_constituent_set(&mut tx, &model, &[constituent], Some("Mix"), None).unwrap();
    tx.commit(&mut model).unwrap();
    assert_eq!(
        model.get(relationship).unwrap().attributes[4],
        Value::Text("CEM I 42.5N".into())
    );
    let view = MaterialView::new(&model);
    assert_eq!(view.schema(), Ok(SchemaVersion::Ifc4x3));
    let read = view.material_relationships().next().unwrap();
    assert_eq!(read.expression().unwrap(), Some("CEM I 42.5N"));
    assert_eq!(model.get(set).unwrap().attributes.len(), 3);
}

/// `RelatingMaterial` must be an `IfcMaterialSelect` member of the model's
/// own release: a layer qualifies in every release, a constituent set only
/// from IFC4, and a wall never.
#[test]
fn the_relating_material_is_checked_against_the_release_select() {
    for (schema, constituent_set_accepted) in [("IFC2X3", false), ("IFC4", true), ("IFC4X3", true)]
    {
        let mut model = declared(&[schema]);
        let owner = model.push(Entity::new("IFCOWNERHISTORY", vec![Value::Null; 8]));
        let wall = model.push(Entity::new("IFCWALL", vec![Value::Null; 8]));
        let layer = model.push(Entity::new("IFCMATERIALLAYER", vec![Value::Null; 3]));
        let mix = model.push(Entity::new(
            "IFCMATERIALCONSTITUENTSET",
            vec![Value::Null; 3],
        ));
        let mut tx = Transaction::new(&model);
        let mut associate = |material| {
            associate_material_with_owner_history(
                &mut tx,
                &model,
                assignment(&gid(1), &[wall], material),
                owner,
            )
        };
        assert!(
            associate(layer).is_ok(),
            "{schema}: a layer is a select member"
        );
        assert_eq!(
            associate(mix).is_ok(),
            constituent_set_accepted,
            "{schema}: constituent set"
        );
        assert!(
            matches!(
                associate(wall),
                Err(MaterialError::AuthoringReferenceType {
                    expected: "IfcMaterialSelect",
                    ..
                })
            ),
            "{schema}: a wall is no material"
        );
    }
}

/// Without one bound release, authoring stages nothing.
#[test]
fn authoring_fails_closed_without_one_release() {
    for (schemas, expected) in [
        (
            &["IFC2X3", "IFC4"][..],
            MaterialError::MultipleSchemas { schemas: 2 },
        ),
        (
            &["IFC5"][..],
            MaterialError::UnsupportedSchema {
                schema: "IFC5".to_owned(),
            },
        ),
    ] {
        let model = declared(schemas);
        let mut tx = Transaction::new(&model);
        assert_eq!(
            create_material(&mut tx, &model, draft("Brick")).unwrap_err(),
            expected
        );
        assert!(tx.is_empty());
    }
}
