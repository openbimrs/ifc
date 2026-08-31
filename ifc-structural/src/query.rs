//! Bounded structural relationship queries.

use std::collections::HashSet;

use ifc_model::EntityId;

use crate::error::StructuralResult;
use crate::view::{Record, StructuralView};

#[derive(Debug, Clone, PartialEq)]
pub struct MemberConnection {
    pub relation: EntityId,
    pub member: EntityId,
    pub connection: EntityId,
    pub applied_condition: Option<EntityId>,
    pub additional_conditions: Option<EntityId>,
    pub supported_length: Option<f64>,
    pub coordinate_system: Option<EntityId>,
}

impl MemberConnection {
    #[must_use]
    pub fn relation_id(&self) -> EntityId {
        self.relation
    }

    pub fn member(&self) -> StructuralResult<EntityId> {
        Ok(self.member)
    }

    pub fn connection(&self) -> StructuralResult<EntityId> {
        Ok(self.connection)
    }

    pub fn supported_length(&self) -> StructuralResult<Option<f64>> {
        Ok(self.supported_length)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityAssignment {
    pub relation: EntityId,
    pub target: EntityId,
    pub activity: EntityId,
}

impl<'m, 's> StructuralView<'m, 's> {
    pub fn analysis_items(&self, analysis_model: EntityId) -> StructuralResult<Vec<EntityId>> {
        self.analysis_model(analysis_model)?;
        let mut items = Vec::new();
        for relation in self.ids_of_ancestor("IfcRelAssignsToGroup") {
            let record = Record::new(self.model, self.schema, relation, "IfcRelAssignsToGroup")?;
            if record.required_ref("RelatingGroup", "IfcGroup")? != analysis_model {
                continue;
            }
            let related = record.required_set_refs_select(
                "RelatedObjects",
                "IfcObjectDefinition",
                &["IfcObjectDefinition"],
                1,
            )?;
            if related.contains(&analysis_model) {
                return Err(crate::StructuralError::SemanticViolation {
                    entity: Some(relation),
                    rule: "IfcRelAssignsToGroup must not assign its RelatingGroup to itself",
                });
            }
            items.extend(related);
        }
        Ok(items)
    }

    pub fn member_connections(&self, member: EntityId) -> StructuralResult<Vec<MemberConnection>> {
        self.member(member)?;
        let mut connections = Vec::new();
        for relation in self.ids_of_ancestor("IfcRelConnectsStructuralMember") {
            let record = Record::new(
                self.model,
                self.schema,
                relation,
                "IfcRelConnectsStructuralMember",
            )?;
            let relating =
                record.required_ref("RelatingStructuralMember", "IfcStructuralMember")?;
            if relating != member {
                continue;
            }
            connections.push(MemberConnection {
                relation,
                member: relating,
                connection: record
                    .required_ref("RelatedStructuralConnection", "IfcStructuralConnection")?,
                applied_condition: record
                    .optional_ref("AppliedCondition", "IfcBoundaryCondition")?,
                additional_conditions: record
                    .optional_ref("AdditionalConditions", "IfcStructuralConnectionCondition")?,
                supported_length: record.optional_number("SupportedLength")?,
                coordinate_system: record
                    .optional_ref("ConditionCoordinateSystem", "IfcAxis2Placement3D")?,
            });
        }
        Ok(connections)
    }

    pub fn activities_for(&self, target: EntityId) -> StructuralResult<Vec<ActivityAssignment>> {
        let target_entity = self
            .model
            .get(target)
            .ok_or(crate::StructuralError::EntityNotFound { id: target })?;
        if !self.schema.is_a(&target_entity.type_name, "IfcElement")
            && !self
                .schema
                .is_a(&target_entity.type_name, "IfcStructuralItem")
        {
            return Err(crate::StructuralError::WrongType {
                id: target,
                expected: "IfcStructuralActivityAssignmentSelect",
                actual: target_entity.type_name.to_string(),
            });
        }
        let mut assignments = Vec::new();
        let mut attached_activities = HashSet::new();
        for relation in self.ids_of_ancestor("IfcRelConnectsStructuralActivity") {
            let record = Record::new(
                self.model,
                self.schema,
                relation,
                "IfcRelConnectsStructuralActivity",
            )?;
            let relating = record.required_ref_select(
                "RelatingElement",
                "IfcStructuralActivityAssignmentSelect",
                &["IfcElement", "IfcStructuralItem"],
            )?;
            let activity =
                record.required_ref("RelatedStructuralActivity", "IfcStructuralActivity")?;
            if !attached_activities.insert(activity) {
                return Err(crate::StructuralError::SemanticViolation {
                    entity: Some(relation),
                    rule: "IfcStructuralActivity must have at most one attachment relation",
                });
            }
            if relating != target {
                continue;
            }
            assignments.push(ActivityAssignment {
                relation,
                target: relating,
                activity,
            });
        }
        Ok(assignments)
    }
}
