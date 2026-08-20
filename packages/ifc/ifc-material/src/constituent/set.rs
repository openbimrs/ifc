//! Source-order-preserving projection of the `IfcMaterialConstituentSet` SET.

use ifc_model::EntityId;

use crate::view::{borrowed_entity, optional_refs, optional_text, MaterialView};
use crate::MaterialResult;

borrowed_entity!(MaterialConstituentSet, "IFCMATERIALCONSTITUENTSET");

impl<'m> MaterialConstituentSet<'m> {
    pub fn name(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIALCONSTITUENTSET",
            self.id(),
            self.entity(),
            0,
            "Name",
        )
    }

    pub fn description(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIALCONSTITUENTSET",
            self.id(),
            self.entity(),
            1,
            "Description",
        )
    }

    pub fn constituent_ids(self) -> MaterialResult<Option<Vec<EntityId>>> {
        optional_refs(
            "IFCMATERIALCONSTITUENTSET",
            self.id(),
            self.entity(),
            2,
            "MaterialConstituents",
            1,
        )
    }
}

impl<'m> MaterialView<'m> {
    pub fn constituent_sets(self) -> impl Iterator<Item = MaterialConstituentSet<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIALCONSTITUENTSET")
            .map(|(id, entity)| MaterialConstituentSet::from_known(id, entity))
    }
}
