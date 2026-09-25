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

/// Which `IfcRelAssociates*` relation and relating attribute is being validated.
///
/// The accepted types are not listed here: they are the types the bound
/// release declares for `RelatedObjects` and for `target_attribute`, so an
/// IFC2X3 `IfcClassificationNotation` target and an IFC4 `IfcClassification`
/// target are each accepted exactly where their release allows them.
pub(crate) struct AssociationSchema {
    /// IFC relation entity type (e.g. `IFCRELASSOCIATESCLASSIFICATION`).
    pub relation: &'static str,
    /// Name of the attribute holding the relating target.
    pub target_attribute: &'static str,
}

/// Check that every `related` object and the `target` are legal values of
/// their attributes in the view's bound release; fails on any dangling or
/// mistyped reference.
///
/// `RelatedObjects` is `IfcDefinitionSelect` in IFC4 and `IfcRoot` in
/// IFC2X3; the relating target is the release's select (for classification,
/// `IfcClassificationSelect` in IFC4, `IfcClassificationNotationSelect` in
/// IFC2X3). Error labels are the release's declared type names.
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
    } = contract;
    let release = view.release();
    let related_type = release.declared_type(relation, relation_id, "RelatedObjects")?;
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
        if !release.accepts(relation, relation_id, "RelatedObjects", &entity.type_name)? {
            return Err(ClassificationError::ReferenceType {
                entity: relation,
                id: relation_id,
                attribute: "RelatedObjects",
                target: id,
                expected: related_type,
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
    if release.accepts(relation, relation_id, target_attribute, &entity.type_name)? {
        Ok(())
    } else {
        Err(ClassificationError::ReferenceType {
            entity: relation,
            id: relation_id,
            attribute: target_attribute,
            target,
            expected: release.declared_type(relation, relation_id, target_attribute)?,
            actual: entity.type_name.to_string(),
        })
    }
}
