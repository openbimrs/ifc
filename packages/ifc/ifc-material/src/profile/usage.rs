//! `IfcMaterialProfileSetUsage` and tapering authored fields.

use ifc_model::EntityId;

use crate::view::{borrowed_entity, optional_integer, optional_number, required_ref, MaterialView};
use crate::{CardinalPointReference, MaterialError, MaterialResult};

borrowed_entity!(MaterialProfileSetUsage, "IFCMATERIALPROFILESETUSAGE");
borrowed_entity!(
    MaterialProfileSetUsageTapering,
    "IFCMATERIALPROFILESETUSAGETAPERING"
);

fn cardinal(
    entity_type: &'static str,
    id: EntityId,
    entity: &ifc_model::Entity,
    slot: usize,
    attribute: &'static str,
) -> MaterialResult<Option<CardinalPointReference>> {
    let Some(value) = optional_integer(entity_type, id, entity, slot, attribute)? else {
        return Ok(None);
    };
    CardinalPointReference::new(value)
        .map(Some)
        .ok_or_else(|| MaterialError::InvalidValue {
            entity: entity_type,
            id,
            attribute,
            value: value.to_string(),
        })
}

fn positive_extent(
    entity_type: &'static str,
    id: EntityId,
    entity: &ifc_model::Entity,
) -> MaterialResult<Option<f64>> {
    let value = optional_number(entity_type, id, entity, 2, "ReferenceExtent")?;
    if value.is_some_and(|value| value <= 0.0) {
        return Err(MaterialError::InvalidValue {
            entity: entity_type,
            id,
            attribute: "ReferenceExtent",
            value: "expected a positive length".to_owned(),
        });
    }
    Ok(value)
}

macro_rules! usage_accessors {
    ($type:ident, $ifc_name:literal) => {
        impl $type<'_> {
            pub fn profile_set_id(self) -> MaterialResult<EntityId> {
                required_ref($ifc_name, self.id(), self.entity(), 0, "ForProfileSet")
            }

            pub fn cardinal_point(self) -> MaterialResult<Option<CardinalPointReference>> {
                cardinal($ifc_name, self.id(), self.entity(), 1, "CardinalPoint")
            }

            pub fn reference_extent(self) -> MaterialResult<Option<f64>> {
                positive_extent($ifc_name, self.id(), self.entity())
            }
        }
    };
}
usage_accessors!(MaterialProfileSetUsage, "IFCMATERIALPROFILESETUSAGE");
usage_accessors!(
    MaterialProfileSetUsageTapering,
    "IFCMATERIALPROFILESETUSAGETAPERING"
);

impl MaterialProfileSetUsageTapering<'_> {
    pub fn end_profile_set_id(self) -> MaterialResult<EntityId> {
        required_ref(
            "IFCMATERIALPROFILESETUSAGETAPERING",
            self.id(),
            self.entity(),
            3,
            "ForProfileEndSet",
        )
    }

    pub fn cardinal_end_point(self) -> MaterialResult<Option<CardinalPointReference>> {
        cardinal(
            "IFCMATERIALPROFILESETUSAGETAPERING",
            self.id(),
            self.entity(),
            4,
            "CardinalEndPoint",
        )
    }
}

impl<'m> MaterialView<'m> {
    pub fn profile_set_usages(self) -> impl Iterator<Item = MaterialProfileSetUsage<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIALPROFILESETUSAGE")
            .map(|(id, entity)| MaterialProfileSetUsage::from_known(id, entity))
    }

    pub fn tapering_profile_set_usages(
        self,
    ) -> impl Iterator<Item = MaterialProfileSetUsageTapering<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIALPROFILESETUSAGETAPERING")
            .map(|(id, entity)| MaterialProfileSetUsageTapering::from_known(id, entity))
    }
}
