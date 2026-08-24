#![cfg(feature = "material-templates")]

use ifc::material::MaterialView;
use ifc::property_catalog::definition::CatalogEdition;
use ifc::property_catalog::embedded::official_catalog;
use ifc::{Entity, EntityId, Model, Value};

#[test]
fn facade_exposes_material_template_join() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCMATERIAL",
            vec![
                Value::Text("Steel S355".into()),
                Value::Null,
                Value::Text("Steel".into()),
            ],
        ),
    );
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCMATERIALPROPERTIES",
            vec![
                Value::Text("Pset_MaterialSteel".into()),
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(4))]),
                Value::Ref(EntityId(1)),
            ],
        ),
    );
    model.insert(
        EntityId(3),
        Entity::new(
            "IFCMATERIALPROPERTIES",
            vec![
                Value::Text("Pset_WallCommon".into()),
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(4))]),
                Value::Ref(EntityId(1)),
            ],
        ),
    );
    model.insert(EntityId(4), Entity::new("IFCPROPERTYSINGLEVALUE", vec![]));
    model.insert(
        EntityId(5),
        Entity::new(
            "IFCMATERIAL",
            vec![Value::Null, Value::Null, Value::Text("Steel".into())],
        ),
    );
    let catalog = official_catalog(CatalogEdition::Ifc4Add2Tc1).unwrap();
    let material = MaterialView::new(&model).materials().next().unwrap();
    let generic_names: Vec<_> = ifc::material_templates::applicable_to(material, &catalog)
        .unwrap()
        .into_iter()
        .map(|template| template.name.as_str())
        .collect();
    assert!(generic_names.contains(&"Pset_MaterialCommon"));
    assert!(!generic_names.contains(&"Pset_MaterialSteel"));
    let category_names: Vec<_> =
        ifc::material_templates::applicable_to_category(material, &catalog)
            .unwrap()
            .into_iter()
            .map(|template| template.name.as_str())
            .collect();
    assert!(category_names.contains(&"Pset_MaterialCommon"));
    assert!(category_names.contains(&"Pset_MaterialSteel"));
    assert!(!category_names.contains(&"Pset_MaterialConcrete"));
    let properties = MaterialView::new(&model)
        .material_properties()
        .next()
        .unwrap();
    assert_eq!(
        ifc::material_templates::template_for(properties, &catalog)
            .unwrap()
            .unwrap()
            .name,
        "Pset_MaterialSteel"
    );
    let unrelated = MaterialView::new(&model)
        .material_properties()
        .find(|properties| properties.id() == EntityId(3))
        .unwrap();
    assert!(ifc::material_templates::template_for(unrelated, &catalog)
        .unwrap()
        .is_none());
    let invalid = MaterialView::new(&model)
        .materials()
        .find(|material| material.id() == EntityId(5))
        .unwrap();
    assert!(matches!(
        ifc::material_templates::applicable_to_category(invalid, &catalog),
        Err(ifc::material::MaterialError::MissingAttribute { .. })
    ));
}
