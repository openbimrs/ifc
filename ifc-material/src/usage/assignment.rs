//! Borrowed `IfcRelAssociatesMaterial` projection.

use ifc_model::EntityId;

use crate::view::{
    borrowed_entity, optional_text, required_ref, required_refs, required_text, MaterialView,
};
use crate::MaterialResult;

borrowed_entity!(MaterialAssignment, "IFCRELASSOCIATESMATERIAL");

impl<'m> MaterialAssignment<'m> {
    /// `IfcRelAssociatesMaterial.GlobalId`. Required.
    pub fn global_id(self) -> MaterialResult<&'m str> {
        required_text(
            "IFCRELASSOCIATESMATERIAL",
            self.id(),
            self.entity(),
            self.slot("GlobalId")?,
            "GlobalId",
        )
    }

    /// `IfcRelAssociatesMaterial.Name`, if given.
    pub fn name(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCRELASSOCIATESMATERIAL",
            self.id(),
            self.entity(),
            self.slot("Name")?,
            "Name",
        )
    }

    /// `IfcRelAssociatesMaterial.Description`, if given.
    pub fn description(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCRELASSOCIATESMATERIAL",
            self.id(),
            self.entity(),
            self.slot("Description")?,
            "Description",
        )
    }

    /// `IfcRelAssociatesMaterial.RelatedObjects`. Required and must be
    /// non-empty.
    pub fn related_object_ids(self) -> MaterialResult<Vec<EntityId>> {
        required_refs(
            "IFCRELASSOCIATESMATERIAL",
            self.id(),
            self.entity(),
            self.slot("RelatedObjects")?,
            "RelatedObjects",
            1,
        )
    }

    /// `IfcRelAssociatesMaterial.RelatingMaterial`, an `IfcMaterialSelect`
    /// branch reference. Required.
    pub fn relating_material_id(self) -> MaterialResult<EntityId> {
        required_ref(
            "IFCRELASSOCIATESMATERIAL",
            self.id(),
            self.entity(),
            self.slot("RelatingMaterial")?,
            "RelatingMaterial",
        )
    }
}

impl<'m> MaterialView<'m> {
    /// Iterates every `IfcRelAssociatesMaterial` instance in the model.
    pub fn assignments(self) -> impl Iterator<Item = MaterialAssignment<'m>> + 'm {
        let release = self.release();
        self.model()
            .of_type("IFCRELASSOCIATESMATERIAL")
            .map(move |(id, entity)| MaterialAssignment::from_known(id, entity, release))
    }

    /// Collects every `IfcRelAssociatesMaterial` whose `RelatedObjects`
    /// includes `object`. Multiple hits indicate an ambiguous direct
    /// assignment; callers that need a single answer should check the
    /// result length themselves or use `assigned_material`.
    pub fn assignments_for(self, object: EntityId) -> MaterialResult<Vec<MaterialAssignment<'m>>> {
        let mut matches = Vec::new();
        for assignment in self.assignments() {
            if assignment.related_object_ids()?.contains(&object) {
                matches.push(assignment);
            }
        }
        Ok(matches)
    }
}
