//! Borrowed `IfcRelAssociatesMaterial` projection.

use ifc_model::EntityId;

use crate::usage::index::{Lookup, ASSOCIATES_MATERIAL};
use crate::view::{
    borrowed_entity, optional_text, required_ref, required_refs, required_text, MaterialView,
};
use crate::MaterialResult;

borrowed_entity!(MaterialAssignment, "IFCRELASSOCIATESMATERIAL");

impl<'m> MaterialAssignment<'m> {
    /// `IfcRelAssociatesMaterial.GlobalId`. Required.
    pub fn global_id(self) -> MaterialResult<&'m str> {
        required_text(
            "IFCRELASSOCIATESMATERIAL",
            self.id(),
            self.entity(),
            self.slot("GlobalId")?,
            "GlobalId",
        )
    }

    /// `IfcRelAssociatesMaterial.Name`, if given.
    pub fn name(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCRELASSOCIATESMATERIAL",
            self.id(),
            self.entity(),
            self.slot("Name")?,
            "Name",
        )
    }

    /// `IfcRelAssociatesMaterial.Description`, if given.
    pub fn description(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCRELASSOCIATESMATERIAL",
            self.id(),
            self.entity(),
            self.slot("Description")?,
            "Description",
        )
    }

    /// `IfcRelAssociatesMaterial.RelatedObjects`. Required and must be
    /// non-empty.
    pub fn related_object_ids(self) -> MaterialResult<Vec<EntityId>> {
        required_refs(
            "IFCRELASSOCIATESMATERIAL",
            self.id(),
            self.entity(),
            self.slot("RelatedObjects")?,
            "RelatedObjects",
            1,
        )
    }

    /// `IfcRelAssociatesMaterial.RelatingMaterial`, an `IfcMaterialSelect`
    /// branch reference. Required.
    pub fn relating_material_id(self) -> MaterialResult<EntityId> {
        required_ref(
            "IFCRELASSOCIATESMATERIAL",
            self.id(),
            self.entity(),
            self.slot("RelatingMaterial")?,
            "RelatingMaterial",
        )
    }
}

impl<'m> MaterialView<'m> {
    /// Iterates every `IfcRelAssociatesMaterial` instance in the model.
    pub fn assignments(self) -> impl Iterator<Item = MaterialAssignment<'m>> + 'm {
        let release = self.release();
        self.model()
            .of_type("IFCRELASSOCIATESMATERIAL")
            .map(move |(id, entity)| MaterialAssignment::from_known(id, entity, release))
    }

    /// Collects every `IfcRelAssociatesMaterial` whose `RelatedObjects`
    /// includes `object`, ascending by id. Several hits naming one
    /// material resolve; see `assigned_material`.
    ///
    /// Reads only the relationships that reference `object` in
    /// `RelatedObjects` (through the model's reverse index when built), so
    /// it costs those and not every relationship in the model; a malformed
    /// relationship elsewhere does not refuse it (#608). For many objects
    /// use [`crate::MaterialIndex`].
    pub fn assignments_for(self, object: EntityId) -> MaterialResult<Vec<MaterialAssignment<'m>>> {
        self.assignments_in(self.lookup(), object)
    }

    pub(crate) fn assignments_in(
        self,
        lookup: Lookup<'_>,
        object: EntityId,
    ) -> MaterialResult<Vec<MaterialAssignment<'m>>> {
        let release = self.release();
        let mut matches = Vec::new();
        for (id, entity) in self.naming(
            lookup.referrers,
            ASSOCIATES_MATERIAL,
            "RelatedObjects",
            object,
        )? {
            if self.names(lookup, ASSOCIATES_MATERIAL, id, entity, object)? {
                matches.push(MaterialAssignment::from_known(id, entity, release));
            }
        }
        Ok(matches)
    }
}
