//! Ordered `IfcMaterialProfileSet` composition.

use ifc_model::EntityId;

use crate::view::{borrowed_entity, optional_ref, optional_text, required_refs, MaterialView};
use crate::MaterialResult;

borrowed_entity!(MaterialProfileSet, "IFCMATERIALPROFILESET");

impl<'m> MaterialProfileSet<'m> {
    pub fn name(self) -> MaterialResult<Option<&'m str>> {
        optional_text("IFCMATERIALPROFILESET", self.id(), self.entity(), 0, "Name")
    }

    pub fn description(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIALPROFILESET",
            self.id(),
            self.entity(),
            1,
            "Description",
        )
    }

    pub fn profile_ids(self) -> MaterialResult<Vec<EntityId>> {
        required_refs(
            "IFCMATERIALPROFILESET",
            self.id(),
            self.entity(),
            2,
            "MaterialProfiles",
            1,
        )
    }

    pub fn composite_profile_id(self) -> MaterialResult<Option<EntityId>> {
        optional_ref(
            "IFCMATERIALPROFILESET",
            self.id(),
            self.entity(),
            3,
            "CompositeProfile",
        )
    }
}

impl<'m> MaterialView<'m> {
    pub fn profile_sets(self) -> impl Iterator<Item = MaterialProfileSet<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIALPROFILESET")
            .map(|(id, entity)| MaterialProfileSet::from_known(id, entity))
    }
}
