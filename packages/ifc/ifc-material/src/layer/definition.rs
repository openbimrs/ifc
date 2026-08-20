//! Material layer definitions, including authored offsets.

use ifc_model::EntityId;

use crate::view::{
    borrowed_entity, optional_integer, optional_logical, optional_ref, optional_text,
    required_enum, required_number, required_number_array_2, MaterialView,
};
use crate::{LayerSetDirection, LogicalValue, MaterialError, MaterialResult};

borrowed_entity!(MaterialLayer, "IFCMATERIALLAYER");
borrowed_entity!(MaterialLayerWithOffsets, "IFCMATERIALLAYERWITHOFFSETS");

macro_rules! layer_accessors {
    ($type:ident, $ifc_name:literal) => {
        impl<'m> $type<'m> {
            pub fn material_id(self) -> MaterialResult<Option<EntityId>> {
                optional_ref($ifc_name, self.id(), self.entity(), 0, "Material")
            }

            pub fn thickness(self) -> MaterialResult<f64> {
                let value =
                    required_number($ifc_name, self.id(), self.entity(), 1, "LayerThickness")?;
                if value < 0.0 {
                    return Err(MaterialError::InvalidValue {
                        entity: $ifc_name,
                        id: self.id(),
                        attribute: "LayerThickness",
                        value: "expected a non-negative length".to_owned(),
                    });
                }
                Ok(value)
            }

            pub fn is_ventilated(self) -> MaterialResult<Option<LogicalValue>> {
                optional_logical($ifc_name, self.id(), self.entity(), 2, "IsVentilated")
            }

            pub fn name(self) -> MaterialResult<Option<&'m str>> {
                optional_text($ifc_name, self.id(), self.entity(), 3, "Name")
            }

            pub fn description(self) -> MaterialResult<Option<&'m str>> {
                optional_text($ifc_name, self.id(), self.entity(), 4, "Description")
            }

            pub fn category(self) -> MaterialResult<Option<&'m str>> {
                optional_text($ifc_name, self.id(), self.entity(), 5, "Category")
            }

            pub fn priority(self) -> MaterialResult<Option<i64>> {
                let value = optional_integer($ifc_name, self.id(), self.entity(), 6, "Priority")?;
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
        }
    };
}
layer_accessors!(MaterialLayer, "IFCMATERIALLAYER");
layer_accessors!(MaterialLayerWithOffsets, "IFCMATERIALLAYERWITHOFFSETS");

impl MaterialLayerWithOffsets<'_> {
    pub fn offset_direction(self) -> MaterialResult<LayerSetDirection> {
        let token = required_enum(
            "IFCMATERIALLAYERWITHOFFSETS",
            self.id(),
            self.entity(),
            7,
            "OffsetDirection",
        )?;
        LayerSetDirection::parse(token).ok_or_else(|| MaterialError::InvalidValue {
            entity: "IFCMATERIALLAYERWITHOFFSETS",
            id: self.id(),
            attribute: "OffsetDirection",
            value: token.to_owned(),
        })
    }

    pub fn offset_values(self) -> MaterialResult<[f64; 2]> {
        required_number_array_2(
            "IFCMATERIALLAYERWITHOFFSETS",
            self.id(),
            self.entity(),
            8,
            "OffsetValues",
        )
    }
}

impl<'m> MaterialView<'m> {
    pub fn layers(self) -> impl Iterator<Item = MaterialLayer<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIALLAYER")
            .map(|(id, entity)| MaterialLayer::from_known(id, entity))
    }

    pub fn layers_with_offsets(self) -> impl Iterator<Item = MaterialLayerWithOffsets<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIALLAYERWITHOFFSETS")
            .map(|(id, entity)| MaterialLayerWithOffsets::from_known(id, entity))
    }
}
