//! Borrowed `Ifcrelassociatesdocument` association projection.
use crate::assignment::{validate_assignment, AssociationSchema};
use crate::view::{
    borrowed_entity, optional_text, required_ref, required_refs, required_text, ClassificationView,
};
use crate::ClassificationResult;
use ifc_model::EntityId;
borrowed_entity!(DocumentAssignment, "IFCRELASSOCIATESDOCUMENT");
impl<'m> DocumentAssignment<'m> {
    pub fn global_id(self) -> ClassificationResult<&'m str> {
        required_text(
            "IFCRELASSOCIATESDOCUMENT",
            self.id(),
            self.entity(),
            0,
            "GlobalId",
        )
    }
    pub fn name(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCRELASSOCIATESDOCUMENT",
            self.id(),
            self.entity(),
            2,
            "Name",
        )
    }
    pub fn description(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCRELASSOCIATESDOCUMENT",
            self.id(),
            self.entity(),
            3,
            "Description",
        )
    }
    pub fn related_object_ids(self) -> ClassificationResult<Vec<EntityId>> {
        required_refs(
            "IFCRELASSOCIATESDOCUMENT",
            self.id(),
            self.entity(),
            4,
            "RelatedObjects",
        )
    }
    pub fn relating_document_id(self) -> ClassificationResult<EntityId> {
        required_ref(
            "IFCRELASSOCIATESDOCUMENT",
            self.id(),
            self.entity(),
            5,
            "RelatingDocument",
        )
    }
}
impl<'m> ClassificationView<'m> {
    pub fn document_assignments(self) -> impl Iterator<Item = DocumentAssignment<'m>> + 'm {
        self.model()
            .of_type("IFCRELASSOCIATESDOCUMENT")
            .map(|(id, e)| DocumentAssignment::from_known(id, e))
    }
    pub fn document_assignments_for(
        self,
        object: EntityId,
    ) -> ClassificationResult<Vec<DocumentAssignment<'m>>> {
        if self.model().get(object).is_none() {
            return Err(crate::ClassificationError::UnknownEntity { id: object });
        }
        let mut out = Vec::new();
        for assignment in self.document_assignments() {
            let related = assignment.related_object_ids()?;
            if related.contains(&object) {
                let target = assignment.relating_document_id()?;
                validate_assignment(
                    self,
                    assignment.id(),
                    &related,
                    target,
                    AssociationSchema {
                        relation: "IFCRELASSOCIATESDOCUMENT",
                        target_attribute: "RelatingDocument",
                        target_types: &["IFCDOCUMENTINFORMATION", "IFCDOCUMENTREFERENCE"],
                        target_label: "IfcDocumentSelect",
                    },
                )?;
                out.push(assignment);
            }
        }
        Ok(out)
    }
}
