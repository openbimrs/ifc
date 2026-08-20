//! `IfcMaterialConstituent` semantics.

use ifc_model::EntityId;

use crate::view::{borrowed_entity, optional_number, optional_text, required_ref, MaterialView};
use crate::{MaterialError, MaterialResult};

borrowed_entity!(MaterialConstituent, "IFCMATERIALCONSTITUENT");

impl<'m> MaterialConstituent<'m> {
    pub fn name(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIALCONSTITUENT",
            self.id(),
            self.entity(),
            0,
            "Name",
        )
    }

    pub fn description(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIALCONSTITUENT",
            self.id(),
            self.entity(),
            1,
            "Description",
        )
    }

    pub fn material_id(self) -> MaterialResult<EntityId> {
        required_ref(
            "IFCMATERIALCONSTITUENT",
            self.id(),
            self.entity(),
            2,
            "Material",
        )
    }

    pub fn fraction(self) -> MaterialResult<Option<f64>> {
        let value = optional_number(
            "IFCMATERIALCONSTITUENT",
            self.id(),
            self.entity(),
            3,
            "Fraction",
        )?;
        if value.is_some_and(|value| !(0.0..=1.0).contains(&value)) {
            return Err(MaterialError::InvalidValue {
                entity: "IFCMATERIALCONSTITUENT",
                id: self.id(),
                attribute: "Fraction",
                value: "expected a normalized ratio in 0..=1".to_owned(),
            });
        }
        Ok(value)
    }

    pub fn category(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIALCONSTITUENT",
            self.id(),
            self.entity(),
            4,
            "Category",
        )
    }
}

impl<'m> MaterialView<'m> {
    pub fn constituents(self) -> impl Iterator<Item = MaterialConstituent<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIALCONSTITUENT")
            .map(|(id, entity)| MaterialConstituent::from_known(id, entity))
    }
}
