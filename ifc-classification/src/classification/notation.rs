//! Borrowed IFC2X3 `IfcClassificationNotation` projection.
//!
//! IFC2X3 lets `IfcRelAssociatesClassification.RelatingClassification` name
//! a notation instead of a reference. A notation is a set of facets, and its
//! code is the facets' `NotationValue`s in the order the file lists them.
//! This crate does not join them: the separator is a property of the
//! classification system, not of the file. IFC4 removed both records, so on
//! an IFC4 model every read here is `NotInSchema`.

use ifc_model::EntityId;

use crate::view::{borrowed_entity, required_refs, required_text, ClassificationView};
use crate::{ClassificationError, ClassificationResult};

const NOTATION: &str = "IFCCLASSIFICATIONNOTATION";
const FACET: &str = "IFCCLASSIFICATIONNOTATIONFACET";

borrowed_entity!(ClassificationNotation, "IFCCLASSIFICATIONNOTATION");

impl ClassificationNotation<'_> {
    /// Ids of the `NotationFacets`, in file order; non-empty per schema.
    pub fn facet_ids(self) -> ClassificationResult<Vec<EntityId>> {
        required_refs(
            NOTATION,
            self.id(),
            self.entity(),
            self.slot("NotationFacets")?,
            "NotationFacets",
        )
    }
}

impl<'m> ClassificationView<'m> {
    /// All `IfcClassificationNotation` instances in the model (IFC2X3 only).
    pub fn notations(self) -> impl Iterator<Item = ClassificationNotation<'m>> + 'm {
        self.model()
            .of_type(NOTATION)
            .map(move |(id, e)| ClassificationNotation::from_known(id, e, self.release()))
    }

    /// Strictly project the `IfcClassificationNotation` `id`.
    ///
    /// # Errors
    ///
    /// `UnknownEntity`, `WrongEntityType`, or `NotInSchema` when the bound
    /// release (IFC4) does not define notations.
    pub fn notation(self, id: EntityId) -> ClassificationResult<ClassificationNotation<'m>> {
        let entity = self
            .model()
            .get(id)
            .ok_or(ClassificationError::UnknownEntity { id })?;
        let notation = ClassificationNotation::try_bound(id, entity, self.release())?;
        notation.slot("NotationFacets")?;
        Ok(notation)
    }

    /// The notation's code: each facet's `NotationValue`, in file order.
    ///
    /// # Errors
    ///
    /// Those of [`Self::notation`], plus a dangling facet, a facet that is not
    /// an `IfcClassificationNotationFacet`, or a facet without a value.
    pub fn notation_values(self, id: EntityId) -> ClassificationResult<Vec<&'m str>> {
        let notation = self.notation(id)?;
        let mut values = Vec::new();
        for facet_id in notation.facet_ids()? {
            let facet =
                self.model()
                    .get(facet_id)
                    .ok_or(ClassificationError::DanglingReference {
                        entity: NOTATION,
                        id,
                        attribute: "NotationFacets",
                        target: facet_id,
                    })?;
            if !facet.is_type(FACET) {
                return Err(ClassificationError::ReferenceType {
                    entity: NOTATION,
                    id,
                    attribute: "NotationFacets",
                    target: facet_id,
                    expected: "IfcClassificationNotationFacet",
                    actual: facet.type_name.to_string(),
                });
            }
            let slot = self
                .release()
                .text_slot(FACET, facet_id, facet, "NotationValue")?;
            values.push(required_text(
                FACET,
                facet_id,
                facet,
                slot,
                "NotationValue",
            )?);
        }
        Ok(values)
    }
}
