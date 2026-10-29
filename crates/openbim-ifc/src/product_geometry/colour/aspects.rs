//! The `IfcShapeAspect`s of an item, found through referrers (#608).

use ifc_model::{EntityId, Value};

use super::{slot_named, ColourError, ColourResolver};

/// One `IfcShapeAspect`, as the constituent lookup reads it.
#[derive(Debug, Clone)]
pub(super) struct Aspect {
    /// `PartOfProductDefinitionShape`, when stated.
    pub(super) owner: Option<EntityId>,
    /// `Name`, which names the constituent.
    pub(super) name: Option<String>,
}

impl ColourResolver<'_> {
    /// The `IfcShapeAspect`s that list `item`: those whose
    /// `ShapeRepresentations` include a representation whose `Items` do,
    /// found through the referrers of `item`, then of that representation,
    /// ascending by id. Each is projected whole, so one that is malformed
    /// refuses the lookups that reach it and no others.
    pub(super) fn aspects_of(&self, item: EntityId) -> Result<Vec<Aspect>, ColourError> {
        if self.schema.entity("IfcShapeAspect").is_none() {
            return Ok(Vec::new());
        }
        let index = self.model.reverse_index();
        let aspect_slot = slot_named(self.schema, "IfcShapeAspect", "ShapeRepresentations");
        let mut found = Vec::new();
        for held_in in index.referrers(item) {
            let Some(representation) = self.model.get(held_in.from) else {
                continue;
            };
            if slot_named(self.schema, &representation.type_name, "Items") != Some(held_in.slot) {
                continue;
            }
            for listed_by in index.referrers(held_in.from) {
                let Some(aspect) = self.model.get(listed_by.from) else {
                    continue;
                };
                if aspect.type_name.eq_ignore_ascii_case("IFCSHAPEASPECT")
                    && aspect_slot == Some(listed_by.slot)
                {
                    found.push(listed_by.from);
                }
            }
        }
        found.sort_unstable();
        found.dedup();
        found
            .into_iter()
            .map(|id| self.aspect(id))
            .collect::<Result<_, _>>()
    }

    /// `IfcShapeAspect` `id`, read whole: its name, its product definition
    /// shape, and every representation and item its representations list.
    fn aspect(&self, id: EntityId) -> Result<Aspect, ColourError> {
        let entity = self.entity(id, id)?;
        let name = self
            .value(id, entity, "Name")?
            .and_then(|value| value.unwrap_typed().as_text())
            .map(str::to_owned);
        let owner = self
            .value(id, entity, "PartOfProductDefinitionShape")?
            .and_then(Value::as_ref_id);
        for representation in self.references(id, entity, "ShapeRepresentations")? {
            let representation_entity = self.entity(id, representation)?;
            self.references(representation, representation_entity, "Items")?;
        }
        Ok(Aspect { owner, name })
    }
}
