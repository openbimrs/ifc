//! Borrowed `Ifcrelassociatesclassification` association projection.
use crate::assignment::{validate_assignment, AssociationSchema};
use crate::view::{
    borrowed_entity, optional_text, required_ref, required_refs, required_text, ClassificationView,
};
use crate::ClassificationResult;
use ifc_model::EntityId;
borrowed_entity!(ClassificationAssignment, "IFCRELASSOCIATESCLASSIFICATION");
impl<'m> ClassificationAssignment<'m> {
    /// The `GlobalId` of this `IfcRelAssociatesClassification`.
    pub fn global_id(self) -> ClassificationResult<&'m str> {
        required_text(
            "IFCRELASSOCIATESCLASSIFICATION",
            self.id(),
            self.entity(),
            self.text_slot("GlobalId")?,
            "GlobalId",
        )
    }
    /// Optional `Name` of the association relationship.
    pub fn name(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCRELASSOCIATESCLASSIFICATION",
            self.id(),
            self.entity(),
            self.text_slot("Name")?,
            "Name",
        )
    }
    /// Optional `Description` of the association relationship.
    pub fn description(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCRELASSOCIATESCLASSIFICATION",
            self.id(),
            self.entity(),
            self.text_slot("Description")?,
            "Description",
        )
    }
    /// Ids of the `RelatedObjects` being classified; non-empty per schema.
    pub fn related_object_ids(self) -> ClassificationResult<Vec<EntityId>> {
        required_refs(
            "IFCRELASSOCIATESCLASSIFICATION",
            self.id(),
            self.entity(),
            self.slot("RelatedObjects")?,
            "RelatedObjects",
        )
    }
    /// Id of the `RelatingClassification` (an `IfcClassification` or `IfcClassificationReference`).
    pub fn relating_classification_id(self) -> ClassificationResult<EntityId> {
        required_ref(
            "IFCRELASSOCIATESCLASSIFICATION",
            self.id(),
            self.entity(),
            self.slot("RelatingClassification")?,
            "RelatingClassification",
        )
    }
}
impl<'m> ClassificationView<'m> {
    /// All `IfcRelAssociatesClassification` instances in the model.
    pub fn classification_assignments(
        self,
    ) -> impl Iterator<Item = ClassificationAssignment<'m>> + 'm {
        self.model()
            .of_type("IFCRELASSOCIATESCLASSIFICATION")
            .map(move |(id, e)| ClassificationAssignment::from_known(id, e, self.release()))
    }
    /// Classification assignments naming `object` among their `RelatedObjects`, with the relating classification checked against `IfcClassificationSelect`; fails if `object` is unknown or a reference does not resolve.
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
                    },
                )?;
                out.push(assignment);
            }
        }
        Ok(out)
    }
}
