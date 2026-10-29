//! The material workload of the subset bench (#608): one `IfcMaterial`
//! with a colour, and one `IfcRelAssociatesMaterial` per wall.

use ifc::{name_based_guid, Entity, EntityId, Model, Value};

use super::walls::Scene;

/// Entities the colour chain adds: the material, its colour, shading,
/// surface style, styled item, styled representation and definition.
const CHAIN: usize = 7;

fn text(s: &str) -> Value {
    Value::Text(s.into())
}

fn refs(ids: &[EntityId]) -> Value {
    Value::List(ids.iter().map(|id| Value::Ref(*id)).collect())
}

/// Entities `colour_walls` adds to a model: the chain, and a relationship
/// per wall.
pub(crate) fn added(walls: usize) -> usize {
    CHAIN + walls
}

/// Colour every wall of `model` through one material: grey (0.5) by an
/// item-less styled item on a styled representation in the scene's Body
/// context, defined for the material. Returns the material.
pub(crate) fn colour_walls(model: &mut Model, scene: &Scene) -> EntityId {
    let material = model.push(Entity::new(
        "IFCMATERIAL",
        vec![text("Grey"), Value::Null, Value::Null],
    ));
    let colour = model.push(Entity::new(
        "IFCCOLOURRGB",
        vec![
            text("Grey"),
            Value::Real(0.5),
            Value::Real(0.5),
            Value::Real(0.5),
        ],
    ));
    let shading = model.push(Entity::new(
        "IFCSURFACESTYLESHADING",
        vec![Value::Ref(colour), Value::Real(0.0)],
    ));
    let style = model.push(Entity::new(
        "IFCSURFACESTYLE",
        vec![text("Grey"), Value::Enum("BOTH".into()), refs(&[shading])],
    ));
    let styled = model.push(Entity::new(
        "IFCSTYLEDITEM",
        vec![Value::Null, refs(&[style]), Value::Null],
    ));
    let representation = model.push(Entity::new(
        "IFCSTYLEDREPRESENTATION",
        vec![
            Value::Ref(scene.body),
            text("Style"),
            text("Material"),
            refs(&[styled]),
        ],
    ));
    model.push(Entity::new(
        "IFCMATERIALDEFINITIONREPRESENTATION",
        vec![
            Value::Null,
            Value::Null,
            refs(&[representation]),
            Value::Ref(material),
        ],
    ));
    let walls: Vec<EntityId> = model.ids_of_type("IfcWall").to_vec();
    for (n, wall) in walls.iter().enumerate() {
        model.push(Entity::new(
            "IFCRELASSOCIATESMATERIAL",
            vec![
                Value::Text(
                    name_based_guid(&["material", &n.to_string()])
                        .as_str()
                        .into(),
                ),
                Value::Null,
                Value::Null,
                Value::Null,
                refs(&[*wall]),
                Value::Ref(material),
            ],
        ));
    }
    material
}
