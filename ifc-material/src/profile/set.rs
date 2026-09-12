//! Ordered `IfcMaterialProfileSet` composition.

use ifc_model::EntityId;

use crate::view::{borrowed_entity, optional_ref, optional_text, required_refs, MaterialView};
use crate::MaterialResult;

borrowed_entity!(MaterialProfileSet, "IFCMATERIALPROFILESET");

impl<'m> MaterialProfileSet<'m> {
    /// `IfcMaterialProfileSet.Name`, if given.
    pub fn name(self) -> MaterialResult<Option<&'m str>> {
        optional_text("IFCMATERIALPROFILESET", self.id(), self.entity(), 0, "Name")
    }

    /// `IfcMaterialProfileSet.Description`, if given.
    pub fn description(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIALPROFILESET",
            self.id(),
            self.entity(),
            1,
            "Description",
        )
    }

    /// `IfcMaterialProfileSet.MaterialProfiles`, in set order. Required
    /// and must be non-empty.
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

    /// `IfcMaterialProfileSet.CompositeProfile`, if given.
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
    /// Iterates every `IfcMaterialProfileSet` instance in the model.
    pub fn profile_sets(self) -> impl Iterator<Item = MaterialProfileSet<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIALPROFILESET")
            .map(|(id, entity)| MaterialProfileSet::from_known(id, entity))
    }
}
