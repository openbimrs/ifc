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
    /// `IfcMaterialClassificationRelationship.MaterialClassifications`.
    /// Required and must be non-empty.
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

    /// `IfcMaterialClassificationRelationship.ClassifiedMaterial`. Required.
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
    /// `IfcMaterialList.Materials`, the ids of the constituent
    /// `IfcMaterial` entities. Required and must be non-empty.
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
    /// `IfcMaterialRelationship.Name`, if given.
    pub fn name(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIALRELATIONSHIP",
            self.id(),
            self.entity(),
            0,
            "Name",
        )
    }

    /// `IfcMaterialRelationship.Description`, if given.
    pub fn description(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIALRELATIONSHIP",
            self.id(),
            self.entity(),
            1,
            "Description",
        )
    }

    /// `IfcMaterialRelationship.RelatingMaterial`. Required.
    pub fn relating_material_id(self) -> MaterialResult<EntityId> {
        required_ref(
            "IFCMATERIALRELATIONSHIP",
            self.id(),
            self.entity(),
            2,
            "RelatingMaterial",
        )
    }

    /// `IfcMaterialRelationship.RelatedMaterials`. Required and must be
    /// non-empty.
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

    /// `IfcMaterialRelationship.Expression`, if given.
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
    /// Iterates every `IfcMaterialClassificationRelationship` in the model.
    pub fn classification_relationships(
        self,
    ) -> impl Iterator<Item = MaterialClassificationRelationship<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIALCLASSIFICATIONRELATIONSHIP")
            .map(|(id, entity)| MaterialClassificationRelationship::from_known(id, entity))
    }

    /// Iterates every `IfcMaterialList` instance in the model.
    pub fn material_lists(self) -> impl Iterator<Item = MaterialList<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIALLIST")
            .map(|(id, entity)| MaterialList::from_known(id, entity))
    }

    /// Iterates every `IfcMaterialRelationship` instance in the model.
    pub fn material_relationships(self) -> impl Iterator<Item = MaterialRelationship<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIALRELATIONSHIP")
            .map(|(id, entity)| MaterialRelationship::from_known(id, entity))
    }
}
