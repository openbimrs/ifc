//! `IfcMaterialProperties` and its direct material/property links.

use ifc_model::EntityId;

use crate::view::{borrowed_entity, optional_text, required_ref, required_refs, MaterialView};
use crate::MaterialResult;

borrowed_entity!(MaterialProperties, "IFCMATERIALPROPERTIES");

impl<'m> MaterialProperties<'m> {
    pub fn name(self) -> MaterialResult<Option<&'m str>> {
        optional_text("IFCMATERIALPROPERTIES", self.id(), self.entity(), 0, "Name")
    }

    pub fn description(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIALPROPERTIES",
            self.id(),
            self.entity(),
            1,
            "Description",
        )
    }

    pub fn property_ids(self) -> MaterialResult<Vec<EntityId>> {
        required_refs(
            "IFCMATERIALPROPERTIES",
            self.id(),
            self.entity(),
            2,
            "Properties",
            1,
        )
    }

    pub fn material_id(self) -> MaterialResult<EntityId> {
        required_ref(
            "IFCMATERIALPROPERTIES",
            self.id(),
            self.entity(),
            3,
            "Material",
        )
    }
}

impl<'m> MaterialView<'m> {
    pub fn material_properties(self) -> impl Iterator<Item = MaterialProperties<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIALPROPERTIES")
            .map(|(id, entity)| MaterialProperties::from_known(id, entity))
    }

    pub fn properties_for(
        self,
        material: EntityId,
    ) -> impl Iterator<Item = MaterialResult<MaterialProperties<'m>>> + 'm {
        self.material_properties()
            .filter_map(move |properties| match properties.material_id() {
                Ok(id) if id == material => Some(Ok(properties)),
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            })
    }
}
