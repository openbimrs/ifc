//! `IfcRelAssignsToGroup` membership for `IfcInventory`.

use ifc_model::EntityId;

use crate::error::ResourceResult;
use crate::view::ResourceView;

impl<'m, 's> ResourceView<'m, 's> {
    /// Members of an `IfcInventory` group, authored order, via
    /// `IfcRelAssignsToGroup.RelatingGroup`.
    ///
    /// `IfcGroup.IsGroupedBy` is `SET [0:?]`: an inventory may be named by
    /// several assignment relations. Members from every authored relation
    /// are concatenated in relation-then-list order; duplicates across
    /// relations are preserved verbatim rather than deduplicated, since the
    /// schema does not require assignment relations to be disjoint.
    pub fn inventory_items(&self, inventory: EntityId) -> ResourceResult<Vec<EntityId>> {
        self.inventory(inventory)?;
        let mut items = Vec::new();
        for relation in self.ids_of_ancestor("IfcRelAssignsToGroup") {
            let record = self.record(relation, "IfcRelAssignsToGroup")?;
            let relating_group = record.required_ref("RelatingGroup", "IfcGroup")?;
            if relating_group != inventory {
                continue;
            }
            items.extend(record.refs("RelatedObjects", "IfcObjectDefinition", 1, false, false)?);
        }
        Ok(items)
    }
}
