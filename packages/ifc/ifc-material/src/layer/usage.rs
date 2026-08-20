//! `IfcMaterialLayerSetUsage` authored fields.

use ifc_model::EntityId;

use crate::view::{
    borrowed_entity, optional_number, required_enum, required_number, required_ref, MaterialView,
};
use crate::{DirectionSense, LayerSetDirection, MaterialError, MaterialResult};

borrowed_entity!(MaterialLayerSetUsage, "IFCMATERIALLAYERSETUSAGE");

impl MaterialLayerSetUsage<'_> {
    pub fn layer_set_id(self) -> MaterialResult<EntityId> {
        required_ref(
            "IFCMATERIALLAYERSETUSAGE",
            self.id(),
            self.entity(),
            0,
            "ForLayerSet",
        )
    }

    pub fn layer_set_direction(self) -> MaterialResult<LayerSetDirection> {
        let token = required_enum(
            "IFCMATERIALLAYERSETUSAGE",
            self.id(),
            self.entity(),
            1,
            "LayerSetDirection",
        )?;
        LayerSetDirection::parse(token).ok_or_else(|| MaterialError::InvalidValue {
            entity: "IFCMATERIALLAYERSETUSAGE",
            id: self.id(),
            attribute: "LayerSetDirection",
            value: token.to_owned(),
        })
    }

    pub fn direction_sense(self) -> MaterialResult<DirectionSense> {
        let token = required_enum(
            "IFCMATERIALLAYERSETUSAGE",
            self.id(),
            self.entity(),
            2,
            "DirectionSense",
        )?;
        DirectionSense::parse(token).ok_or_else(|| MaterialError::InvalidValue {
            entity: "IFCMATERIALLAYERSETUSAGE",
            id: self.id(),
            attribute: "DirectionSense",
            value: token.to_owned(),
        })
    }

    pub fn offset_from_reference_line(self) -> MaterialResult<f64> {
        required_number(
            "IFCMATERIALLAYERSETUSAGE",
            self.id(),
            self.entity(),
            3,
            "OffsetFromReferenceLine",
        )
    }

    pub fn reference_extent(self) -> MaterialResult<Option<f64>> {
        let value = optional_number(
            "IFCMATERIALLAYERSETUSAGE",
            self.id(),
            self.entity(),
            4,
            "ReferenceExtent",
        )?;
        if value.is_some_and(|value| value <= 0.0) {
            return Err(MaterialError::InvalidValue {
                entity: "IFCMATERIALLAYERSETUSAGE",
                id: self.id(),
                attribute: "ReferenceExtent",
                value: "expected a positive length".to_owned(),
            });
        }
        Ok(value)
    }
}

impl<'m> MaterialView<'m> {
    pub fn layer_set_usages(self) -> impl Iterator<Item = MaterialLayerSetUsage<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIALLAYERSETUSAGE")
            .map(|(id, entity)| MaterialLayerSetUsage::from_known(id, entity))
    }
}
