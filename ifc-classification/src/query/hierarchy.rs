//! Bounded classification hierarchy and explicit occurrence/type lookup.

use std::collections::HashMap;

use ifc_model::{Budget, EntityId};

use crate::view::{required_ref, required_refs};
use crate::{
    ClassificationAssignment, ClassificationError, ClassificationReference, ClassificationResult,
    ClassificationSystem, ClassificationView,
};

#[derive(Debug, Clone)]
pub struct ClassificationHierarchy<'m> {
    pub references: Vec<ClassificationReference<'m>>,
    pub system: Option<ClassificationSystem<'m>>,
}

#[derive(Debug, Clone)]
pub struct EffectiveClassifications<'m> {
    pub occurrence: Vec<ClassificationAssignment<'m>>,
    pub type_object: Option<EntityId>,
    pub inherited: Vec<ClassificationAssignment<'m>>,
}

fn require_select(
    view: ClassificationView<'_>,
    source: EntityId,
    target: EntityId,
) -> ClassificationResult<()> {
    let entity = view
        .model()
        .get(target)
        .ok_or(ClassificationError::DanglingReference {
            entity: "IFCRELASSOCIATESCLASSIFICATION",
            id: source,
            attribute: "RelatingClassification",
            target,
        })?;
    if entity.is_type("IFCCLASSIFICATION") || entity.is_type("IFCCLASSIFICATIONREFERENCE") {
        Ok(())
    } else {
        Err(ClassificationError::ReferenceType {
            entity: "IFCRELASSOCIATESCLASSIFICATION",
            id: source,
            attribute: "RelatingClassification",
            target,
            expected: "IfcClassificationSelect",
            actual: entity.type_name.to_string(),
        })
    }
}

impl<'m> ClassificationView<'m> {
    pub fn hierarchy_from(
        self,
        leaf: EntityId,
        budget: Budget,
    ) -> ClassificationResult<ClassificationHierarchy<'m>> {
        let mut current = leaf;
        let mut from_reference = None;
        let mut references: Vec<ClassificationReference<'m>> = Vec::new();
        let mut positions = HashMap::new();
        let mut visited_nodes = 0usize;
        let mut edges_followed = 0usize;

        loop {
            if let Some(&start) = positions.get(&current) {
                let mut path: Vec<_> = references[start..].iter().map(|r| r.id()).collect();
                path.push(current);
                return Err(ClassificationError::Cycle { path });
            }
            if visited_nodes >= budget.max_nodes {
                return Err(ClassificationError::BudgetExceeded {
                    max_depth: budget.max_depth,
                    max_nodes: budget.max_nodes,
                });
            }
            let entity = self.model().get(current).ok_or_else(|| {
                from_reference.map_or(
                    ClassificationError::UnknownEntity { id: current },
                    |source| ClassificationError::DanglingReference {
                        entity: "IFCCLASSIFICATIONREFERENCE",
                        id: source,
                        attribute: "ReferencedSource",
                        target: current,
                    },
                )
            })?;
            visited_nodes += 1;

            if entity.is_type("IFCCLASSIFICATION") && from_reference.is_some() {
                return Ok(ClassificationHierarchy {
                    references,
                    system: Some(ClassificationSystem::try_new(current, entity)?),
                });
            }
            if !entity.is_type("IFCCLASSIFICATIONREFERENCE") {
                if let Some(source) = from_reference {
                    return Err(ClassificationError::ReferenceType {
                        entity: "IFCCLASSIFICATIONREFERENCE",
                        id: source,
                        attribute: "ReferencedSource",
                        target: current,
                        expected: "IfcClassificationReferenceSelect",
                        actual: entity.type_name.to_string(),
                    });
                }
                return Err(ClassificationError::WrongEntityType {
                    expected: "IFCCLASSIFICATIONREFERENCE",
                    actual: entity.type_name.to_string(),
                });
            }

            positions.insert(current, references.len());
            let reference = ClassificationReference::try_new(current, entity)?;
            let source = reference.referenced_source_id()?;
            references.push(reference);
            let Some(source) = source else {
                return Ok(ClassificationHierarchy {
                    references,
                    system: None,
                });
            };
            if edges_followed >= budget.max_depth {
                return Err(ClassificationError::BudgetExceeded {
                    max_depth: budget.max_depth,
                    max_nodes: budget.max_nodes,
                });
            }
            edges_followed += 1;
            from_reference = Some(current);
            current = source;
        }
    }

    pub fn children_of(
        self,
        source: EntityId,
    ) -> ClassificationResult<Vec<ClassificationReference<'m>>> {
        let source_entity = self
            .model()
            .get(source)
            .ok_or(ClassificationError::UnknownEntity { id: source })?;
        if !(source_entity.is_type("IFCCLASSIFICATION")
            || source_entity.is_type("IFCCLASSIFICATIONREFERENCE"))
        {
            return Err(ClassificationError::ReferenceType {
                entity: "IFCCLASSIFICATIONREFERENCE",
                id: source,
                attribute: "ReferencedSource",
                target: source,
                expected: "IfcClassificationReferenceSelect",
                actual: source_entity.type_name.to_string(),
            });
        }
        let mut out = Vec::new();
        for reference in self.references() {
            if reference.referenced_source_id()? == Some(source) {
                out.push(reference);
            }
        }
        out.sort_by_key(|reference| reference.id());
        Ok(out)
    }

    pub fn effective_classifications(
        self,
        object: EntityId,
    ) -> ClassificationResult<EffectiveClassifications<'m>> {
        if self.model().get(object).is_none() {
            return Err(ClassificationError::UnknownEntity { id: object });
        }
        let occurrence = self.classification_assignments_for(object)?;
        for assignment in &occurrence {
            require_select(
                self,
                assignment.id(),
                assignment.relating_classification_id()?,
            )?;
        }

        let mut types = Vec::new();
        for (relationship_id, entity) in self.model().of_type("IFCRELDEFINESBYTYPE") {
            if required_refs(
                "IFCRELDEFINESBYTYPE",
                relationship_id,
                entity,
                4,
                "RelatedObjects",
            )?
            .contains(&object)
            {
                let type_id = required_ref(
                    "IFCRELDEFINESBYTYPE",
                    relationship_id,
                    entity,
                    5,
                    "RelatingType",
                )?;
                self.model()
                    .get(type_id)
                    .ok_or(ClassificationError::DanglingReference {
                        entity: "IFCRELDEFINESBYTYPE",
                        id: relationship_id,
                        attribute: "RelatingType",
                        target: type_id,
                    })?;
                types.push(type_id);
            }
        }
        if types.len() > 1 {
            return Err(ClassificationError::AmbiguousType {
                object,
                count: types.len(),
            });
        }
        let type_id = types.first().copied();
        let inherited_from_type = if let Some(type_id) = type_id {
            let assignments = self.classification_assignments_for(type_id)?;
            for assignment in &assignments {
                require_select(
                    self,
                    assignment.id(),
                    assignment.relating_classification_id()?,
                )?;
            }
            assignments
        } else {
            Vec::new()
        };
        Ok(EffectiveClassifications {
            occurrence,
            type_object: type_id,
            inherited: inherited_from_type,
        })
    }
}
