//! Deterministic material-select and occurrence/type resolution.

use ifc_model::EntityId;

use crate::{
    Material, MaterialAssignment, MaterialConstituent, MaterialConstituentSet, MaterialError,
    MaterialLayer, MaterialLayerSet, MaterialLayerSetUsage, MaterialLayerWithOffsets, MaterialList,
    MaterialProfile, MaterialProfileSet, MaterialProfileSetUsage, MaterialProfileSetUsageTapering,
    MaterialProfileWithOffsets, MaterialResult, MaterialView,
};

/// A resolved branch of the abstract `IfcMaterialDefinition` select.
#[derive(Debug, Clone, Copy)]
pub enum MaterialDefinition<'m> {
    /// Resolves to an `IfcMaterial`.
    Material(Material<'m>),
    /// Resolves to an `IfcMaterialConstituent`.
    Constituent(MaterialConstituent<'m>),
    /// Resolves to an `IfcMaterialConstituentSet`.
    ConstituentSet(MaterialConstituentSet<'m>),
    /// Resolves to an `IfcMaterialLayer`.
    Layer(MaterialLayer<'m>),
    /// Resolves to an `IfcMaterialLayerWithOffsets`.
    LayerWithOffsets(MaterialLayerWithOffsets<'m>),
    /// Resolves to an `IfcMaterialLayerSet`.
    LayerSet(MaterialLayerSet<'m>),
    /// Resolves to an `IfcMaterialProfile`.
    Profile(MaterialProfile<'m>),
    /// Resolves to an `IfcMaterialProfileWithOffsets`.
    ProfileWithOffsets(MaterialProfileWithOffsets<'m>),
    /// Resolves to an `IfcMaterialProfileSet`.
    ProfileSet(MaterialProfileSet<'m>),
}

/// A resolved branch of the abstract `IfcMaterialUsageDefinition` select.
#[derive(Debug, Clone, Copy)]
pub enum MaterialUsageDefinition<'m> {
    /// Resolves to an `IfcMaterialLayerSetUsage`.
    LayerSet(MaterialLayerSetUsage<'m>),
    /// Resolves to an `IfcMaterialProfileSetUsage`.
    ProfileSet(MaterialProfileSetUsage<'m>),
    /// Resolves to an `IfcMaterialProfileSetUsageTapering`.
    ProfileSetTapering(MaterialProfileSetUsageTapering<'m>),
}

/// A resolved branch of the abstract `IfcMaterialSelect`.
#[derive(Debug, Clone, Copy)]
pub enum ResolvedMaterialSelect<'m> {
    /// Resolves to an `IfcMaterialDefinition` subtype.
    Definition(MaterialDefinition<'m>),
    /// Resolves to an `IfcMaterialList`.
    List(MaterialList<'m>),
    /// Resolves to an `IfcMaterialUsageDefinition` subtype.
    Usage(MaterialUsageDefinition<'m>),
}

/// Whether a resolved material assignment came from the object directly or
/// was inherited from its `IfcTypeObject`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentSource {
    /// The `IfcRelAssociatesMaterial` directly relates the queried object.
    Occurrence,
    /// The `IfcRelAssociatesMaterial` relates the object's `IfcTypeObject`,
    /// identified by this entity id, and was inherited from it.
    Type(EntityId),
}

/// A fully resolved material for one object, with provenance.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedAssignment<'m> {
    /// The `IfcRelAssociatesMaterial` that produced this resolution.
    pub assignment: MaterialAssignment<'m>,
    /// The resolved `IfcMaterialSelect` branch.
    pub material: ResolvedMaterialSelect<'m>,
    /// Whether the assignment applied directly or via the object's type.
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

    /// Resolves `id` as an `IfcMaterialSelect` branch by dispatching on its
    /// concrete IFC entity type. Fails with
    /// [`crate::MaterialError::WrongEntityType`] if `id` names an entity
    /// that is not a `MaterialResource` select member.
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

    /// Resolves the single material that applies to `object`, preferring a
    /// direct `IfcRelAssociatesMaterial` on the object itself and falling
    /// back to the material assigned to its `IfcTypeObject` via
    /// `IfcRelDefinesByType`. Returns `Ok(None)` if neither exists. Fails
    /// with [`crate::MaterialError::AmbiguousAssignment`] or
    /// [`crate::MaterialError::AmbiguousType`] if more than one candidate is
    /// found at either level, and with
    /// `crate::MaterialError::UnknownEntity` if `object` is not in the
    /// model.
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
