//! Borrowed `IfcRelAssociatesMaterial` projection.

use ifc_model::EntityId;

use crate::view::{
    borrowed_entity, optional_text, required_ref, required_refs, required_text, MaterialView,
};
use crate::MaterialResult;

borrowed_entity!(MaterialAssignment, "IFCRELASSOCIATESMATERIAL");

impl<'m> MaterialAssignment<'m> {
    pub fn global_id(self) -> MaterialResult<&'m str> {
        required_text(
            "IFCRELASSOCIATESMATERIAL",
            self.id(),
            self.entity(),
            0,
            "GlobalId",
        )
    }

    pub fn name(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCRELASSOCIATESMATERIAL",
            self.id(),
            self.entity(),
            2,
            "Name",
        )
    }

    pub fn description(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCRELASSOCIATESMATERIAL",
            self.id(),
            self.entity(),
            3,
            "Description",
        )
    }

    pub fn related_object_ids(self) -> MaterialResult<Vec<EntityId>> {
        required_refs(
            "IFCRELASSOCIATESMATERIAL",
            self.id(),
            self.entity(),
            4,
            "RelatedObjects",
            1,
        )
    }

    pub fn relating_material_id(self) -> MaterialResult<EntityId> {
        required_ref(
            "IFCRELASSOCIATESMATERIAL",
            self.id(),
            self.entity(),
            5,
            "RelatingMaterial",
        )
    }
}

impl<'m> MaterialView<'m> {
    pub fn assignments(self) -> impl Iterator<Item = MaterialAssignment<'m>> + 'm {
        self.model()
            .of_type("IFCRELASSOCIATESMATERIAL")
            .map(|(id, entity)| MaterialAssignment::from_known(id, entity))
    }

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
