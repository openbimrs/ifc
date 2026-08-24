#![cfg(feature = "material-templates")]

use ifc::material::MaterialView;
use ifc::property_catalog::definition::CatalogEdition;
use ifc::property_catalog::embedded::official_catalog;
use ifc::{Entity, EntityId, Model, Value};
use std::collections::BTreeSet;

#[test]
fn concrete_steel_and_wood_cover_all_fourteen_official_material_psds() {
    let mut model = Model::new();
    for (id, category) in [(1, "Concrete"), (2, "Steel"), (3, "Wood")] {
        model.insert(
            EntityId(id),
            Entity::new(
                "IFCMATERIAL",
                vec![
                    Value::Text(category.into()),
                    Value::Null,
                    Value::Text(category.into()),
                ],
            ),
        );
    }
    let catalog = official_catalog(CatalogEdition::Ifc4Add2Tc1).unwrap();
    let view = MaterialView::new(&model);
    let names: BTreeSet<_> = view
        .materials()
        .flat_map(|material| {
            ifc::material_templates::applicable_to_category(material, &catalog).unwrap()
        })
        .map(|set| set.name.as_str())
        .collect();
    let expected: BTreeSet<_> = [
        "Pset_MaterialCombustion",
        "Pset_MaterialCommon",
        "Pset_MaterialConcrete",
        "Pset_MaterialEnergy",
        "Pset_MaterialFuel",
        "Pset_MaterialHygroscopic",
        "Pset_MaterialMechanical",
        "Pset_MaterialOptical",
        "Pset_MaterialSteel",
        "Pset_MaterialThermal",
        "Pset_MaterialWater",
        "Pset_MaterialWood",
        "Pset_MaterialWoodBasedBeam",
        "Pset_MaterialWoodBasedPanel",
    ]
    .into_iter()
    .collect();
    assert_eq!(names, expected);
}
