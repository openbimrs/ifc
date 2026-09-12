//! Object association projections and shared schema-backed validation.

use ifc_model::EntityId;

use crate::view::ClassificationView;
use crate::{ClassificationError, ClassificationResult};

mod classification;
mod document;
mod library;

pub use classification::ClassificationAssignment;
pub use document::DocumentAssignment;
pub use library::LibraryAssignment;

/// Contract describing which `IfcRelAssociates*` relation and target select is being validated.
pub(crate) struct AssociationSchema {
    /// IFC relation entity type (e.g. `IFCRELASSOCIATESCLASSIFICATION`).
    pub relation: &'static str,
    /// Name of the attribute holding the relating target.
    pub target_attribute: &'static str,
    /// IFC types accepted as the relating target.
    pub target_types: &'static [&'static str],
    /// Select name reported in errors when the target has an unexpected type.
    pub target_label: &'static str,
}

/// Check that `related` are all valid `IfcDefinitionSelect` members and `target` matches `contract`; fails on any dangling or mistyped reference.
pub(crate) fn validate_assignment(
    view: ClassificationView<'_>,
    relation_id: EntityId,
    related: &[EntityId],
    target: EntityId,
    contract: AssociationSchema,
) -> ClassificationResult<()> {
    let AssociationSchema {
        relation,
        target_attribute,
        target_types,
        target_label,
    } = contract;
    let schema = ifc_schema::ifc4();
    for &id in related {
        let entity = view
            .model()
            .get(id)
            .ok_or(ClassificationError::DanglingReference {
                entity: relation,
                id: relation_id,
                attribute: "RelatedObjects",
                target: id,
            })?;
        if !schema.is_a(&entity.type_name, "IFCOBJECTDEFINITION")
            && !schema.is_a(&entity.type_name, "IFCPROPERTYDEFINITION")
        {
            return Err(ClassificationError::ReferenceType {
                entity: relation,
                id: relation_id,
                attribute: "RelatedObjects",
                target: id,
                expected: "IfcDefinitionSelect",
                actual: entity.type_name.to_string(),
            });
        }
    }
    let entity = view
        .model()
        .get(target)
        .ok_or(ClassificationError::DanglingReference {
            entity: relation,
            id: relation_id,
            attribute: target_attribute,
            target,
        })?;
    if target_types.iter().any(|expected| entity.is_type(expected)) {
        Ok(())
    } else {
        Err(ClassificationError::ReferenceType {
            entity: relation,
            id: relation_id,
            attribute: target_attribute,
            target,
            expected: target_label,
            actual: entity.type_name.to_string(),
        })
    }
}
