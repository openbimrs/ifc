//! Deterministic material-select and occurrence/type resolution.

use ifc_model::EntityId;

use crate::{
    Material, MaterialAssignment, MaterialConstituent, MaterialConstituentSet, MaterialError,
    MaterialLayer, MaterialLayerSet, MaterialLayerSetUsage, MaterialLayerWithOffsets, MaterialList,
    MaterialProfile, MaterialProfileSet, MaterialProfileSetUsage, MaterialProfileSetUsageTapering,
    MaterialProfileWithOffsets, MaterialResult, MaterialView,
};

#[derive(Debug, Clone, Copy)]
pub enum MaterialDefinition<'m> {
    Material(Material<'m>),
    Constituent(MaterialConstituent<'m>),
    ConstituentSet(MaterialConstituentSet<'m>),
    Layer(MaterialLayer<'m>),
    LayerWithOffsets(MaterialLayerWithOffsets<'m>),
    LayerSet(MaterialLayerSet<'m>),
    Profile(MaterialProfile<'m>),
    ProfileWithOffsets(MaterialProfileWithOffsets<'m>),
    ProfileSet(MaterialProfileSet<'m>),
}

#[derive(Debug, Clone, Copy)]
pub enum MaterialUsageDefinition<'m> {
    LayerSet(MaterialLayerSetUsage<'m>),
    ProfileSet(MaterialProfileSetUsage<'m>),
    ProfileSetTapering(MaterialProfileSetUsageTapering<'m>),
}

#[derive(Debug, Clone, Copy)]
pub enum ResolvedMaterialSelect<'m> {
    Definition(MaterialDefinition<'m>),
    List(MaterialList<'m>),
    Usage(MaterialUsageDefinition<'m>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentSource {
    Occurrence,
    Type(EntityId),
}

#[derive(Debug, Clone, Copy)]
pub struct ResolvedAssignment<'m> {
    pub assignment: MaterialAssignment<'m>,
    pub material: ResolvedMaterialSelect<'m>,
    pub source: AssignmentSource,
}

impl<'m> MaterialView<'m> {
    /// Every concrete subtype of the abstract `IfcMaterialDefinition`.
    pub fn material_definitions(self) -> impl Iterator<Item = MaterialDefinition<'m>> + 'm {
        self.model().iter().filter_map(move |(id, _)| {
            match self.resolve_material_select(id).ok()? {
                ResolvedMaterialSelect::Definition(definition) => Some(definition),
                _ => None,
            }
        })
    }

    /// Every concrete subtype of the abstract `IfcMaterialUsageDefinition`.
    pub fn material_usage_definitions(
        self,
    ) -> impl Iterator<Item = MaterialUsageDefinition<'m>> + 'm {
        self.model().iter().filter_map(move |(id, _)| {
            match self.resolve_material_select(id).ok()? {
                ResolvedMaterialSelect::Usage(usage) => Some(usage),
                _ => None,
            }
        })
    }

    pub fn resolve_material_select(
        self,
        id: EntityId,
    ) -> MaterialResult<ResolvedMaterialSelect<'m>> {
        let entity = self.entity(id, id)?;
        let entity_type = entity.type_name.to_ascii_uppercase();
        let definition = match entity_type.as_str() {
            "IFCMATERIAL" => MaterialDefinition::Material(Material::from_known(id, entity)),
            "IFCMATERIALCONSTITUENT" => {
                MaterialDefinition::Constituent(MaterialConstituent::from_known(id, entity))
            }
            "IFCMATERIALCONSTITUENTSET" => {
                MaterialDefinition::ConstituentSet(MaterialConstituentSet::from_known(id, entity))
            }
            "IFCMATERIALLAYER" => MaterialDefinition::Layer(MaterialLayer::from_known(id, entity)),
            "IFCMATERIALLAYERWITHOFFSETS" => MaterialDefinition::LayerWithOffsets(
                MaterialLayerWithOffsets::from_known(id, entity),
            ),
            "IFCMATERIALLAYERSET" => {
                MaterialDefinition::LayerSet(MaterialLayerSet::from_known(id, entity))
            }
            "IFCMATERIALPROFILE" => {
                MaterialDefinition::Profile(MaterialProfile::from_known(id, entity))
            }
            "IFCMATERIALPROFILEWITHOFFSETS" => MaterialDefinition::ProfileWithOffsets(
                MaterialProfileWithOffsets::from_known(id, entity),
            ),
            "IFCMATERIALPROFILESET" => {
                MaterialDefinition::ProfileSet(MaterialProfileSet::from_known(id, entity))
            }
            "IFCMATERIALLIST" => {
                return Ok(ResolvedMaterialSelect::List(MaterialList::from_known(
                    id, entity,
                )));
            }
            "IFCMATERIALLAYERSETUSAGE" => {
                return Ok(ResolvedMaterialSelect::Usage(
                    MaterialUsageDefinition::LayerSet(MaterialLayerSetUsage::from_known(
                        id, entity,
                    )),
                ));
            }
            "IFCMATERIALPROFILESETUSAGE" => {
                return Ok(ResolvedMaterialSelect::Usage(
                    MaterialUsageDefinition::ProfileSet(MaterialProfileSetUsage::from_known(
                        id, entity,
                    )),
                ));
            }
            "IFCMATERIALPROFILESETUSAGETAPERING" => {
                return Ok(ResolvedMaterialSelect::Usage(
                    MaterialUsageDefinition::ProfileSetTapering(
                        MaterialProfileSetUsageTapering::from_known(id, entity),
                    ),
                ));
            }
            _ => {
                return Err(MaterialError::WrongEntityType {
                    expected: "IfcMaterialSelect",
                    actual: entity.type_name.to_string(),
                });
            }
        };
        Ok(ResolvedMaterialSelect::Definition(definition))
    }

    pub fn assigned_material(
        self,
        object: EntityId,
    ) -> MaterialResult<Option<ResolvedAssignment<'m>>> {
        self.model()
            .get(object)
            .ok_or(MaterialError::UnknownEntity { id: object })?;

        let direct = self.assignments_for(object)?;
        if direct.len() > 1 {
            return Err(MaterialError::AmbiguousAssignment {
                object,
                count: direct.len(),
            });
        }
        if let Some(assignment) = direct.first().copied() {
            return self
                .resolve_assignment(assignment, AssignmentSource::Occurrence)
                .map(Some);
        }

        let mut type_relations = Vec::new();
        for (relation_id, relation) in self.model().of_type("IFCRELDEFINESBYTYPE") {
            let related = crate::view::required_refs(
                "IFCRELDEFINESBYTYPE",
                relation_id,
                relation,
                4,
                "RelatedObjects",
                1,
            )?;
            if related.contains(&object) {
                let type_id = crate::view::required_ref(
                    "IFCRELDEFINESBYTYPE",
                    relation_id,
                    relation,
                    5,
                    "RelatingType",
                )?;
                type_relations.push((relation_id, type_id));
            }
        }
        if type_relations.len() > 1 {
            return Err(MaterialError::AmbiguousType {
                object,
                count: type_relations.len(),
            });
        }
        let Some((relation_id, type_id)) = type_relations.first().copied() else {
            return Ok(None);
        };
        let type_entity = self.entity(relation_id, type_id)?;
        if !super::ifc4_type_objects::is_concrete_type_object(&type_entity.type_name) {
            return Err(MaterialError::ReferenceType {
                source_id: relation_id,
                target: type_id,
                expected: "IFCTYPEOBJECT subtype",
                actual: type_entity.type_name.to_string(),
            });
        }

        let assigned = self.assignments_for(type_id)?;
        if assigned.len() > 1 {
            return Err(MaterialError::AmbiguousAssignment {
                object: type_id,
                count: assigned.len(),
            });
        }
        assigned
            .first()
            .copied()
            .map(|assignment| self.resolve_assignment(assignment, AssignmentSource::Type(type_id)))
            .transpose()
    }

    fn resolve_assignment(
        self,
        assignment: MaterialAssignment<'m>,
        source: AssignmentSource,
    ) -> MaterialResult<ResolvedAssignment<'m>> {
        let material_id = assignment.relating_material_id()?;
        Ok(ResolvedAssignment {
            assignment,
            material: self.resolve_material_select(material_id)?,
            source,
        })
    }
}
