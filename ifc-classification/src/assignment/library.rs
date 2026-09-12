//! Borrowed `Ifcrelassociateslibrary` association projection.
use crate::assignment::{validate_assignment, AssociationSchema};
use crate::view::{
    borrowed_entity, optional_text, required_ref, required_refs, required_text, ClassificationView,
};
use crate::ClassificationResult;
use ifc_model::EntityId;
borrowed_entity!(LibraryAssignment, "IFCRELASSOCIATESLIBRARY");
impl<'m> LibraryAssignment<'m> {
    /// The `GlobalId` of this `IfcRelAssociatesLibrary`.
    pub fn global_id(self) -> ClassificationResult<&'m str> {
        required_text(
            "IFCRELASSOCIATESLIBRARY",
            self.id(),
            self.entity(),
            0,
            "GlobalId",
        )
    }
    /// Optional `Name` of the association relationship.
    pub fn name(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCRELASSOCIATESLIBRARY",
            self.id(),
            self.entity(),
            2,
            "Name",
        )
    }
    /// Optional `Description` of the association relationship.
    pub fn description(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCRELASSOCIATESLIBRARY",
            self.id(),
            self.entity(),
            3,
            "Description",
        )
    }
    /// Ids of the `RelatedObjects` the library entry applies to; non-empty per schema.
    pub fn related_object_ids(self) -> ClassificationResult<Vec<EntityId>> {
        required_refs(
            "IFCRELASSOCIATESLIBRARY",
            self.id(),
            self.entity(),
            4,
            "RelatedObjects",
        )
    }
    /// Id of the `RelatingLibrary` (an `IfcLibraryInformation` or `IfcLibraryReference`).
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
    /// All `IfcRelAssociatesLibrary` instances in the model.
    pub fn library_assignments(self) -> impl Iterator<Item = LibraryAssignment<'m>> + 'm {
        self.model()
            .of_type("IFCRELASSOCIATESLIBRARY")
            .map(|(id, e)| LibraryAssignment::from_known(id, e))
    }
    /// Library assignments naming `object` among their `RelatedObjects`, with the relating library checked against `IfcLibrarySelect`; fails if `object` is unknown or a reference does not resolve.
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
