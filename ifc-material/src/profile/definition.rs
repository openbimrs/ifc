//! `IfcMaterialProfile` semantics and authored offsets.

use ifc_model::EntityId;

use crate::view::{
    borrowed_entity, optional_integer, optional_ref, optional_text, required_number_array_2,
    required_ref, MaterialView,
};
use crate::{MaterialError, MaterialResult};

borrowed_entity!(MaterialProfile, "IFCMATERIALPROFILE");
borrowed_entity!(MaterialProfileWithOffsets, "IFCMATERIALPROFILEWITHOFFSETS");

macro_rules! profile_accessors {
    ($type:ident, $ifc_name:literal) => {
        impl<'m> $type<'m> {
            /// `Name`, if given.
            pub fn name(self) -> MaterialResult<Option<&'m str>> {
                optional_text($ifc_name, self.id(), self.entity(), 0, "Name")
            }

            /// `Description`, if given.
            pub fn description(self) -> MaterialResult<Option<&'m str>> {
                optional_text($ifc_name, self.id(), self.entity(), 1, "Description")
            }

            /// `Material`, the associated `IfcMaterial`, if given.
            pub fn material_id(self) -> MaterialResult<Option<EntityId>> {
                optional_ref($ifc_name, self.id(), self.entity(), 2, "Material")
            }

            /// `Profile`, the associated `IfcProfileDef`. Required.
            pub fn profile_id(self) -> MaterialResult<EntityId> {
                required_ref($ifc_name, self.id(), self.entity(), 3, "Profile")
            }

            /// `Priority`, if given. Must be in `0..=100`.
            pub fn priority(self) -> MaterialResult<Option<i64>> {
                let value = optional_integer($ifc_name, self.id(), self.entity(), 4, "Priority")?;
                if value.is_some_and(|value| !(0..=100).contains(&value)) {
                    return Err(MaterialError::InvalidValue {
                        entity: $ifc_name,
                        id: self.id(),
                        attribute: "Priority",
                        value: "expected an integer in 0..=100".to_owned(),
                    });
                }
                Ok(value)
            }

            /// `Category`, if given.
            pub fn category(self) -> MaterialResult<Option<&'m str>> {
                optional_text($ifc_name, self.id(), self.entity(), 5, "Category")
            }
        }
    };
}
profile_accessors!(MaterialProfile, "IFCMATERIALPROFILE");
profile_accessors!(MaterialProfileWithOffsets, "IFCMATERIALPROFILEWITHOFFSETS");

impl MaterialProfileWithOffsets<'_> {
    /// `IfcMaterialProfileWithOffsets.OffsetValues`. Required, a 2-element
    /// list.
    pub fn offset_values(self) -> MaterialResult<[f64; 2]> {
        required_number_array_2(
            "IFCMATERIALPROFILEWITHOFFSETS",
            self.id(),
            self.entity(),
            6,
            "OffsetValues",
        )
    }
}

impl<'m> MaterialView<'m> {
    /// Iterates every `IfcMaterialProfile` instance in the model.
    pub fn profiles(self) -> impl Iterator<Item = MaterialProfile<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIALPROFILE")
            .map(|(id, entity)| MaterialProfile::from_known(id, entity))
    }

    /// Iterates every `IfcMaterialProfileWithOffsets` instance in the model.
    pub fn profiles_with_offsets(
        self,
    ) -> impl Iterator<Item = MaterialProfileWithOffsets<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIALPROFILEWITHOFFSETS")
            .map(|(id, entity)| MaterialProfileWithOffsets::from_known(id, entity))
    }
}
