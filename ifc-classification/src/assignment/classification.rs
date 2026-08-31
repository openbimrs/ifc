//! Borrowed `Ifcrelassociatesclassification` association projection.
use crate::assignment::{validate_assignment, AssociationSchema};
use crate::view::{
    borrowed_entity, optional_text, required_ref, required_refs, required_text, ClassificationView,
};
use crate::ClassificationResult;
use ifc_model::EntityId;
borrowed_entity!(ClassificationAssignment, "IFCRELASSOCIATESCLASSIFICATION");
impl<'m> ClassificationAssignment<'m> {
    pub fn global_id(self) -> ClassificationResult<&'m str> {
        required_text(
            "IFCRELASSOCIATESCLASSIFICATION",
            self.id(),
            self.entity(),
            0,
            "GlobalId",
        )
    }
    pub fn name(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCRELASSOCIATESCLASSIFICATION",
            self.id(),
            self.entity(),
            2,
            "Name",
        )
    }
    pub fn description(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCRELASSOCIATESCLASSIFICATION",
            self.id(),
            self.entity(),
            3,
            "Description",
        )
    }
    pub fn related_object_ids(self) -> ClassificationResult<Vec<EntityId>> {
        required_refs(
            "IFCRELASSOCIATESCLASSIFICATION",
            self.id(),
            self.entity(),
            4,
            "RelatedObjects",
        )
    }
    pub fn relating_classification_id(self) -> ClassificationResult<EntityId> {
        required_ref(
            "IFCRELASSOCIATESCLASSIFICATION",
            self.id(),
            self.entity(),
            5,
            "RelatingClassification",
        )
    }
}
impl<'m> ClassificationView<'m> {
    pub fn classification_assignments(
        self,
    ) -> impl Iterator<Item = ClassificationAssignment<'m>> + 'm {
        self.model()
            .of_type("IFCRELASSOCIATESCLASSIFICATION")
            .map(|(id, e)| ClassificationAssignment::from_known(id, e))
    }
    pub fn classification_assignments_for(
        self,
        object: EntityId,
    ) -> ClassificationResult<Vec<ClassificationAssignment<'m>>> {
        if self.model().get(object).is_none() {
            return Err(crate::ClassificationError::UnknownEntity { id: object });
        }
        let mut out = Vec::new();
        for assignment in self.classification_assignments() {
            let related = assignment.related_object_ids()?;
            if related.contains(&object) {
                let target = assignment.relating_classification_id()?;
                validate_assignment(
                    self,
                    assignment.id(),
                    &related,
                    target,
                    AssociationSchema {
                        relation: "IFCRELASSOCIATESCLASSIFICATION",
                        target_attribute: "RelatingClassification",
                        target_types: &["IFCCLASSIFICATION", "IFCCLASSIFICATIONREFERENCE"],
                        target_label: "IfcClassificationSelect",
                    },
                )?;
                out.push(assignment);
            }
        }
        Ok(out)
    }
}
