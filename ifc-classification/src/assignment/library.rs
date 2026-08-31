//! Borrowed `Ifcrelassociateslibrary` association projection.
use crate::assignment::{validate_assignment, AssociationSchema};
use crate::view::{
    borrowed_entity, optional_text, required_ref, required_refs, required_text, ClassificationView,
};
use crate::ClassificationResult;
use ifc_model::EntityId;
borrowed_entity!(LibraryAssignment, "IFCRELASSOCIATESLIBRARY");
impl<'m> LibraryAssignment<'m> {
    pub fn global_id(self) -> ClassificationResult<&'m str> {
        required_text(
            "IFCRELASSOCIATESLIBRARY",
            self.id(),
            self.entity(),
            0,
            "GlobalId",
        )
    }
    pub fn name(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCRELASSOCIATESLIBRARY",
            self.id(),
            self.entity(),
            2,
            "Name",
        )
    }
    pub fn description(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCRELASSOCIATESLIBRARY",
            self.id(),
            self.entity(),
            3,
            "Description",
        )
    }
    pub fn related_object_ids(self) -> ClassificationResult<Vec<EntityId>> {
        required_refs(
            "IFCRELASSOCIATESLIBRARY",
            self.id(),
            self.entity(),
            4,
            "RelatedObjects",
        )
    }
    pub fn relating_library_id(self) -> ClassificationResult<EntityId> {
        required_ref(
            "IFCRELASSOCIATESLIBRARY",
            self.id(),
            self.entity(),
            5,
            "RelatingLibrary",
        )
    }
}
impl<'m> ClassificationView<'m> {
    pub fn library_assignments(self) -> impl Iterator<Item = LibraryAssignment<'m>> + 'm {
        self.model()
            .of_type("IFCRELASSOCIATESLIBRARY")
            .map(|(id, e)| LibraryAssignment::from_known(id, e))
    }
    pub fn library_assignments_for(
        self,
        object: EntityId,
    ) -> ClassificationResult<Vec<LibraryAssignment<'m>>> {
        if self.model().get(object).is_none() {
            return Err(crate::ClassificationError::UnknownEntity { id: object });
        }
        let mut out = Vec::new();
        for assignment in self.library_assignments() {
            let related = assignment.related_object_ids()?;
            if related.contains(&object) {
                let target = assignment.relating_library_id()?;
                validate_assignment(
                    self,
                    assignment.id(),
                    &related,
                    target,
                    AssociationSchema {
                        relation: "IFCRELASSOCIATESLIBRARY",
                        target_attribute: "RelatingLibrary",
                        target_types: &["IFCLIBRARYINFORMATION", "IFCLIBRARYREFERENCE"],
                        target_label: "IfcLibrarySelect",
                    },
                )?;
                out.push(assignment);
            }
        }
        Ok(out)
    }
}
