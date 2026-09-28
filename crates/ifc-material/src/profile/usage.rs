//! `IfcMaterialProfileSetUsage` and tapering authored fields.

use ifc_model::EntityId;

use crate::view::{borrowed_entity, optional_integer, optional_number, required_ref, MaterialView};
use crate::{CardinalPointReference, MaterialError, MaterialResult};

// `IfcMaterialProfileSetUsageTapering` is `SUBTYPE OF
// (IfcMaterialProfileSetUsage)` in IFC4 and IFC4X3 and keeps its three
// attributes at slots 0..2, so the supertype projection accepts it (#136).
borrowed_entity!(
    MaterialProfileSetUsage,
    "IFCMATERIALPROFILESETUSAGE",
    ["IFCMATERIALPROFILESETUSAGETAPERING"]
);
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
    slot: usize,
) -> MaterialResult<Option<f64>> {
    let value = optional_number(entity_type, id, entity, slot, "ReferenceExtent")?;
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
            /// `ForProfileSet`. Required.
            pub fn profile_set_id(self) -> MaterialResult<EntityId> {
                required_ref(
                    $ifc_name,
                    self.id(),
                    self.entity(),
                    self.slot("ForProfileSet")?,
                    "ForProfileSet",
                )
            }

            /// `CardinalPoint`, if given. Must decode to a positive
            /// `IfcCardinalPointReference`.
            pub fn cardinal_point(self) -> MaterialResult<Option<CardinalPointReference>> {
                cardinal(
                    $ifc_name,
                    self.id(),
                    self.entity(),
                    self.slot("CardinalPoint")?,
                    "CardinalPoint",
                )
            }

            /// `ReferenceExtent`, if given. Must be strictly positive when
            /// present.
            pub fn reference_extent(self) -> MaterialResult<Option<f64>> {
                positive_extent(
                    $ifc_name,
                    self.id(),
                    self.entity(),
                    self.slot("ReferenceExtent")?,
                )
            }
        }
    };
}
usage_accessors!(MaterialProfileSetUsage, "IFCMATERIALPROFILESETUSAGE");
usage_accessors!(
    MaterialProfileSetUsageTapering,
    "IFCMATERIALPROFILESETUSAGETAPERING"
);

impl<'m> MaterialProfileSetUsage<'m> {
    /// The tapering projection of this usage when the record is an
    /// `IfcMaterialProfileSetUsageTapering`, giving its `ForProfileEndSet`
    /// and `CardinalEndPoint`; `None` for a plain usage.
    pub fn tapering(self) -> Option<MaterialProfileSetUsageTapering<'m>> {
        self.entity()
            .is_type("IFCMATERIALPROFILESETUSAGETAPERING")
            .then(|| {
                MaterialProfileSetUsageTapering::from_known(
                    self.id(),
                    self.entity(),
                    self.release(),
                )
            })
    }
}

impl MaterialProfileSetUsageTapering<'_> {
    /// `IfcMaterialProfileSetUsageTapering.ForProfileEndSet`. Required.
    pub fn end_profile_set_id(self) -> MaterialResult<EntityId> {
        required_ref(
            "IFCMATERIALPROFILESETUSAGETAPERING",
            self.id(),
            self.entity(),
            self.slot("ForProfileEndSet")?,
            "ForProfileEndSet",
        )
    }

    /// `IfcMaterialProfileSetUsageTapering.CardinalEndPoint`, if given.
    /// Must decode to a positive `IfcCardinalPointReference`.
    pub fn cardinal_end_point(self) -> MaterialResult<Option<CardinalPointReference>> {
        cardinal(
            "IFCMATERIALPROFILESETUSAGETAPERING",
            self.id(),
            self.entity(),
            self.slot("CardinalEndPoint")?,
            "CardinalEndPoint",
        )
    }
}

impl<'m> MaterialView<'m> {
    /// Iterates every `IfcMaterialProfileSetUsage` in the model, including
    /// its subtype `IfcMaterialProfileSetUsageTapering` (#136), in entity-id
    /// order.
    ///
    /// A tapering usage is a profile-set usage, so a caller asking for usages
    /// does not miss it; [`MaterialProfileSetUsage::tapering`] reaches its end
    /// set and end cardinal point. [`Self::tapering_profile_set_usages`]
    /// yields the subtype alone.
    pub fn profile_set_usages(self) -> impl Iterator<Item = MaterialProfileSetUsage<'m>> + 'm {
        let release = self.release();
        let mut usages: Vec<_> = self
            .model()
            .of_type("IFCMATERIALPROFILESETUSAGE")
            .chain(self.model().of_type("IFCMATERIALPROFILESETUSAGETAPERING"))
            .collect();
        usages.sort_unstable_by_key(|(id, _)| *id);
        usages
            .into_iter()
            .map(move |(id, entity)| MaterialProfileSetUsage::from_known(id, entity, release))
    }

    /// Iterates every `IfcMaterialProfileSetUsageTapering` instance in the
    /// model.
    pub fn tapering_profile_set_usages(
        self,
    ) -> impl Iterator<Item = MaterialProfileSetUsageTapering<'m>> + 'm {
        let release = self.release();
        self.model()
            .of_type("IFCMATERIALPROFILESETUSAGETAPERING")
            .map(move |(id, entity)| {
                MaterialProfileSetUsageTapering::from_known(id, entity, release)
            })
    }
}
