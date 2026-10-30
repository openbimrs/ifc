//! Deterministic material-select and occurrence/type resolution.
//!
//! Every hop reads against the model's release: `IfcRelDefinesByType` slots
//! and the legal `IfcTypeObject` subtypes come from its bundled table, and a
//! select member the release does not declare is refused.

use ifc_model::{Entity, EntityId};

use crate::release::{is_type_object, Release};
use crate::usage::index::{Lookup, DEFINES_BY_TYPE};
use crate::view::required_ref;
use crate::{
    Material, MaterialAssignment, MaterialConstituent, MaterialConstituentSet, MaterialError,
    MaterialLayer, MaterialLayerSet, MaterialLayerSetUsage, MaterialLayerWithOffsets, MaterialList,
    MaterialProfile, MaterialProfileSet, MaterialProfileSetUsage, MaterialProfileSetUsageTapering,
    MaterialProfileWithOffsets, MaterialResult, MaterialView,
};

/// A resolved branch of the abstract `IfcMaterialDefinition` select.
///
/// IFC2X3 has no `IfcMaterialDefinition` supertype; its `IfcMaterialSelect`
/// members `IfcMaterial`, `IfcMaterialLayer` and `IfcMaterialLayerSet`
/// resolve to the same branches here. The constituent and profile branches
/// only occur for IFC4 and IFC4X3 models.
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
///
/// IFC2X3 has no `IfcMaterialUsageDefinition` supertype; its
/// `IfcMaterialLayerSetUsage` resolves to [`Self::LayerSet`].
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
#[non_exhaustive]
pub struct ResolvedAssignment<'m> {
    /// The `IfcRelAssociatesMaterial` that produced this resolution.
    pub assignment: MaterialAssignment<'m>,
    /// The resolved `IfcMaterialSelect` branch.
    pub material: ResolvedMaterialSelect<'m>,
    /// Whether the assignment applied directly or via the object's type.
    pub source: AssignmentSource,
}

/// Every concrete `IfcMaterialSelect` member of any bundled release that
/// [`classify`] dispatches. `release_layout` tests pin, per release, that
/// the release's own select closure is exactly the instantiable subset.
pub(crate) const SELECT_MEMBERS: &[&str] = &[
    "IFCMATERIAL",
    "IFCMATERIALCONSTITUENT",
    "IFCMATERIALCONSTITUENTSET",
    "IFCMATERIALLAYER",
    "IFCMATERIALLAYERWITHOFFSETS",
    "IFCMATERIALLAYERSET",
    "IFCMATERIALPROFILE",
    "IFCMATERIALPROFILEWITHOFFSETS",
    "IFCMATERIALPROFILESET",
    "IFCMATERIALLIST",
    "IFCMATERIALLAYERSETUSAGE",
    "IFCMATERIALPROFILESETUSAGE",
    "IFCMATERIALPROFILESETUSAGETAPERING",
];

/// Dispatch a record to its select branch by type name alone, bound to
/// `release`. `None` for a type that is no select member of any release.
fn classify<'m>(
    id: EntityId,
    entity: &'m Entity,
    release: Release<'m>,
) -> Option<(&'static str, ResolvedMaterialSelect<'m>)> {
    use MaterialDefinition as D;
    use MaterialUsageDefinition as U;
    use ResolvedMaterialSelect as S;
    let name = SELECT_MEMBERS
        .iter()
        .copied()
        .find(|name| entity.is_type(name))?;
    let resolved = match name {
        "IFCMATERIAL" => S::Definition(D::Material(Material::from_known(id, entity, release))),
        "IFCMATERIALCONSTITUENT" => S::Definition(D::Constituent(MaterialConstituent::from_known(
            id, entity, release,
        ))),
        "IFCMATERIALCONSTITUENTSET" => S::Definition(D::ConstituentSet(
            MaterialConstituentSet::from_known(id, entity, release),
        )),
        "IFCMATERIALLAYER" => {
            S::Definition(D::Layer(MaterialLayer::from_known(id, entity, release)))
        }
        "IFCMATERIALLAYERWITHOFFSETS" => S::Definition(D::LayerWithOffsets(
            MaterialLayerWithOffsets::from_known(id, entity, release),
        )),
        "IFCMATERIALLAYERSET" => S::Definition(D::LayerSet(MaterialLayerSet::from_known(
            id, entity, release,
        ))),
        "IFCMATERIALPROFILE" => {
            S::Definition(D::Profile(MaterialProfile::from_known(id, entity, release)))
        }
        "IFCMATERIALPROFILEWITHOFFSETS" => S::Definition(D::ProfileWithOffsets(
            MaterialProfileWithOffsets::from_known(id, entity, release),
        )),
        "IFCMATERIALPROFILESET" => S::Definition(D::ProfileSet(MaterialProfileSet::from_known(
            id, entity, release,
        ))),
        "IFCMATERIALLIST" => S::List(MaterialList::from_known(id, entity, release)),
        "IFCMATERIALLAYERSETUSAGE" => S::Usage(U::LayerSet(MaterialLayerSetUsage::from_known(
            id, entity, release,
        ))),
        "IFCMATERIALPROFILESETUSAGE" => S::Usage(U::ProfileSet(
            MaterialProfileSetUsage::from_known(id, entity, release),
        )),
        _ => S::Usage(U::ProfileSetTapering(
            MaterialProfileSetUsageTapering::from_known(id, entity, release),
        )),
    };
    Some((name, resolved))
}

impl<'m> MaterialView<'m> {
    /// Every concrete subtype of the abstract `IfcMaterialDefinition`.
    ///
    /// Yields by type name; a record the model's release does not declare
    /// (an IFC2X3 constituent set, say) is still yielded, and its accessors
    /// fail with [`MaterialError::EntityNotInSchema`].
    pub fn material_definitions(self) -> impl Iterator<Item = MaterialDefinition<'m>> + 'm {
        let release = self.release();
        self.model()
            .iter()
            .filter_map(move |(id, entity)| match classify(id, entity, release)?.1 {
                ResolvedMaterialSelect::Definition(definition) => Some(definition),
                _ => None,
            })
    }

    /// Every concrete subtype of the abstract `IfcMaterialUsageDefinition`.
    ///
    /// Yields by type name, as [`Self::material_definitions`] does.
    pub fn material_usage_definitions(
        self,
    ) -> impl Iterator<Item = MaterialUsageDefinition<'m>> + 'm {
        let release = self.release();
        self.model()
            .iter()
            .filter_map(move |(id, entity)| match classify(id, entity, release)?.1 {
                ResolvedMaterialSelect::Usage(usage) => Some(usage),
                _ => None,
            })
    }

    /// Resolves `id` as an `IfcMaterialSelect` branch of the model's release
    /// by dispatching on its concrete IFC entity type.
    ///
    /// # Errors
    ///
    /// [`MaterialError::WrongEntityType`] if `id` names an entity that is no
    /// `MaterialResource` select member;
    /// [`MaterialError::EntityNotInSchema`] if it is a member of another
    /// release only, such as an `IfcMaterialConstituentSet` or
    /// `IfcMaterialProfileSet` in an IFC2X3 model; and the release-binding
    /// errors when the header binds no release.
    pub fn resolve_material_select(
        self,
        id: EntityId,
    ) -> MaterialResult<ResolvedMaterialSelect<'m>> {
        let entity = self.entity(id, id)?;
        let wrong = || MaterialError::WrongEntityType {
            expected: "IfcMaterialSelect",
            actual: entity.type_name.to_string(),
        };
        let (name, resolved) = classify(id, entity, self.release()).ok_or_else(wrong)?;
        let (_, schema) = self.release().require_entity(name, Some(id))?;
        if !schema.accepts_type("IfcMaterialSelect", name) {
            return Err(wrong());
        }
        Ok(resolved)
    }

    /// Resolves the single material that applies to `object`, preferring a
    /// direct `IfcRelAssociatesMaterial` on the object itself and falling
    /// back to the material assigned to its `IfcTypeObject` via
    /// `IfcRelDefinesByType`. Returns `Ok(None)` if neither exists. Fails
    /// with [`crate::MaterialError::AmbiguousAssignment`] or
    /// [`crate::MaterialError::AmbiguousType`] for more than one distinct
    /// material, or more than one `IfcRelDefinesByType`, and with
    /// `crate::MaterialError::UnknownEntity` if `object` is not in the
    /// model.
    ///
    /// Several `IfcRelAssociatesMaterial` on one object, or on its type,
    /// resolve when they name one `RelatingMaterial` (compared by entity id,
    /// never by content); `assignment` is then the lowest-id relationship.
    /// `AmbiguousAssignment::count` is the number of distinct materials.
    ///
    /// Every record is read against the model's release, so an IFC2X3 model
    /// resolves with IFC2X3 slots and type objects.
    ///
    /// The type fallback is exactly one `IfcRelDefinesByType` hop, and two
    /// such relations are ambiguous even when they name the same type: no
    /// release allows it (IFC2X3 `IfcObject` WR1; IFC4 and IFC4X3
    /// `IsTypedBy SET [0:1]`), so the file states the typing twice and this
    /// lookup does not guess which statement was meant. (`ifc-resource`
    /// deliberately accepts a repeated relation naming the same type.)
    /// `HasAssociations` is
    /// `SET [0:?]` in every release, which is why several material
    /// relationships are accepted. For many objects use
    /// [`crate::MaterialIndex`].
    pub fn assigned_material(
        self,
        object: EntityId,
    ) -> MaterialResult<Option<ResolvedAssignment<'m>>> {
        self.assigned_material_in(self.lookup(), object)
    }

    pub(crate) fn assigned_material_in(
        self,
        lookup: Lookup<'_>,
        object: EntityId,
    ) -> MaterialResult<Option<ResolvedAssignment<'m>>> {
        self.model()
            .get(object)
            .ok_or(MaterialError::UnknownEntity { id: object })?;

        let direct = self.assignments_in(lookup, object)?;
        if let Some(assignment) = self.single_material(object, &direct)? {
            return self
                .resolve_assignment(assignment, AssignmentSource::Occurrence)
                .map(Some);
        }

        let Some((relation_id, type_id)) = self.type_relation(lookup, object)? else {
            return Ok(None);
        };
        let type_entity = self.entity(relation_id, type_id)?;
        if !is_type_object(self.release(), &type_entity.type_name)? {
            return Err(MaterialError::ReferenceType {
                source_id: relation_id,
                target: type_id,
                expected: "IFCTYPEOBJECT subtype",
                actual: type_entity.type_name.to_string(),
            });
        }

        let assigned = self.assignments_in(lookup, type_id)?;
        self.single_material(type_id, &assigned)?
            .map(|assignment| self.resolve_assignment(assignment, AssignmentSource::Type(type_id)))
            .transpose()
    }

    /// The lowest-id of `holder`'s relationships when they all name one
    /// `RelatingMaterial` (by entity id; content is never compared), `None`
    /// when there are none. Every relationship's `RelatingMaterial` is read,
    /// so a missing or non-reference one refuses with its own error.
    fn single_material(
        self,
        holder: EntityId,
        assignments: &[MaterialAssignment<'m>],
    ) -> MaterialResult<Option<MaterialAssignment<'m>>> {
        let Some(first) = assignments.first().copied() else {
            return Ok(None);
        };
        let mut materials = Vec::new();
        for assignment in assignments {
            let material = assignment.relating_material_id()?;
            if !materials.contains(&material) {
                materials.push(material);
            }
        }
        if materials.len() > 1 {
            return Err(MaterialError::AmbiguousAssignment {
                object: holder,
                count: materials.len(),
            });
        }
        Ok(Some(first))
    }

    /// The single `IfcRelDefinesByType` typing `object`, as
    /// `(relation, type)`.
    fn type_relation(
        self,
        lookup: Lookup<'_>,
        object: EntityId,
    ) -> MaterialResult<Option<(EntityId, EntityId)>> {
        let release = self.release();
        let mut type_relations = Vec::new();
        for (relation_id, relation) in
            self.naming(lookup.referrers, DEFINES_BY_TYPE, "RelatedObjects", object)?
        {
            if self.names(lookup, DEFINES_BY_TYPE, relation_id, relation, object)? {
                let type_slot = release.slot(DEFINES_BY_TYPE, relation_id, "RelatingType")?;
                let type_id = required_ref(
                    DEFINES_BY_TYPE,
                    relation_id,
                    relation,
                    type_slot,
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
        Ok(type_relations.first().copied())
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
