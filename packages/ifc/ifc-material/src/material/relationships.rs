//! Material lists, classifications, and resource-level relationships.

use ifc_model::EntityId;

use crate::view::{borrowed_entity, optional_text, required_ref, required_refs, MaterialView};
use crate::MaterialResult;

borrowed_entity!(
    MaterialClassificationRelationship,
    "IFCMATERIALCLASSIFICATIONRELATIONSHIP"
);
borrowed_entity!(MaterialList, "IFCMATERIALLIST");
borrowed_entity!(MaterialRelationship, "IFCMATERIALRELATIONSHIP");

impl MaterialClassificationRelationship<'_> {
    pub fn classification_ids(self) -> MaterialResult<Vec<EntityId>> {
        required_refs(
            "IFCMATERIALCLASSIFICATIONRELATIONSHIP",
            self.id(),
            self.entity(),
            0,
            "MaterialClassifications",
            1,
        )
    }

    pub fn material_id(self) -> MaterialResult<EntityId> {
        required_ref(
            "IFCMATERIALCLASSIFICATIONRELATIONSHIP",
            self.id(),
            self.entity(),
            1,
            "ClassifiedMaterial",
        )
    }
}

impl MaterialList<'_> {
    pub fn material_ids(self) -> MaterialResult<Vec<EntityId>> {
        required_refs(
            "IFCMATERIALLIST",
            self.id(),
            self.entity(),
            0,
            "Materials",
            1,
        )
    }
}

impl<'m> MaterialRelationship<'m> {
    pub fn name(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIALRELATIONSHIP",
            self.id(),
            self.entity(),
            0,
            "Name",
        )
    }

    pub fn description(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIALRELATIONSHIP",
            self.id(),
            self.entity(),
            1,
            "Description",
        )
    }

    pub fn relating_material_id(self) -> MaterialResult<EntityId> {
        required_ref(
            "IFCMATERIALRELATIONSHIP",
            self.id(),
            self.entity(),
            2,
            "RelatingMaterial",
        )
    }

    pub fn related_material_ids(self) -> MaterialResult<Vec<EntityId>> {
        required_refs(
            "IFCMATERIALRELATIONSHIP",
            self.id(),
            self.entity(),
            3,
            "RelatedMaterials",
            1,
        )
    }

    pub fn expression(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIALRELATIONSHIP",
            self.id(),
            self.entity(),
            4,
            "Expression",
        )
    }
}

impl<'m> MaterialView<'m> {
    pub fn classification_relationships(
        self,
    ) -> impl Iterator<Item = MaterialClassificationRelationship<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIALCLASSIFICATIONRELATIONSHIP")
            .map(|(id, entity)| MaterialClassificationRelationship::from_known(id, entity))
    }

    pub fn material_lists(self) -> impl Iterator<Item = MaterialList<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIALLIST")
            .map(|(id, entity)| MaterialList::from_known(id, entity))
    }

    pub fn material_relationships(self) -> impl Iterator<Item = MaterialRelationship<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIALRELATIONSHIP")
            .map(|(id, entity)| MaterialRelationship::from_known(id, entity))
    }
}
