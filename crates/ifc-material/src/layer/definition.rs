//! Material layer definitions, including authored offsets.
//!
//! IFC2X3 `IfcMaterialLayer` is `(Material, LayerThickness, IsVentilated)`
//! with a positive thickness; IFC4 and IFC4X3 append `Name`, `Description`,
//! `Category` and `Priority` and allow a zero thickness. An accessor for an
//! attribute the bound release lacks fails with `NotInSchema`.

use ifc_model::{Entity, EntityId};

use crate::release::Release;
use crate::view::{
    borrowed_entity, optional_integer, optional_logical, optional_ref, optional_text,
    required_enum, required_number, required_number_array_2, MaterialView,
};
use crate::{LayerSetDirection, LogicalValue, MaterialError, MaterialResult};

borrowed_entity!(MaterialLayer, "IFCMATERIALLAYER");
borrowed_entity!(MaterialLayerWithOffsets, "IFCMATERIALLAYERWITHOFFSETS");

/// `LayerThickness` of a layer record, checked against the bound release's
/// declared measure: IFC2X3 `IfcPositiveLengthMeasure` (> 0), IFC4 and
/// IFC4X3 `IfcNonNegativeLengthMeasure` (>= 0).
pub(crate) fn layer_thickness(
    entity_type: &'static str,
    id: EntityId,
    entity: &Entity,
    release: Release<'_>,
) -> MaterialResult<f64> {
    let (slot, declared) = release.attribute(entity_type, id, "LayerThickness")?;
    let value = required_number(entity_type, id, entity, slot, "LayerThickness")?;
    let positive = declared
        .type_name
        .eq_ignore_ascii_case("IfcPositiveLengthMeasure");
    if value < 0.0 || (positive && value == 0.0) {
        return Err(MaterialError::InvalidValue {
            entity: entity_type,
            id,
            attribute: "LayerThickness",
            value: if positive {
                "expected a positive length".to_owned()
            } else {
                "expected a non-negative length".to_owned()
            },
        });
    }
    Ok(value)
}

macro_rules! layer_accessors {
    ($type:ident, $ifc_name:literal) => {
        impl<'m> $type<'m> {
            /// `Material`, the associated `IfcMaterial`, if given.
            pub fn material_id(self) -> MaterialResult<Option<EntityId>> {
                optional_ref(
                    $ifc_name,
                    self.id(),
                    self.entity(),
                    self.slot("Material")?,
                    "Material",
                )
            }

            /// `LayerThickness`. Required; non-negative in IFC4 and IFC4X3,
            /// strictly positive in IFC2X3.
            pub fn thickness(self) -> MaterialResult<f64> {
                layer_thickness($ifc_name, self.id(), self.entity(), self.release())
            }

            /// `IsVentilated`, if given.
            pub fn is_ventilated(self) -> MaterialResult<Option<LogicalValue>> {
                optional_logical(
                    $ifc_name,
                    self.id(),
                    self.entity(),
                    self.slot("IsVentilated")?,
                    "IsVentilated",
                )
            }

            /// `Name`, if given. `NotInSchema` for IFC2X3.
            pub fn name(self) -> MaterialResult<Option<&'m str>> {
                optional_text(
                    $ifc_name,
                    self.id(),
                    self.entity(),
                    self.slot("Name")?,
                    "Name",
                )
            }

            /// `Description`, if given. `NotInSchema` for IFC2X3.
            pub fn description(self) -> MaterialResult<Option<&'m str>> {
                optional_text(
                    $ifc_name,
                    self.id(),
                    self.entity(),
                    self.slot("Description")?,
                    "Description",
                )
            }

            /// `Category`, if given. `NotInSchema` for IFC2X3.
            pub fn category(self) -> MaterialResult<Option<&'m str>> {
                optional_text(
                    $ifc_name,
                    self.id(),
                    self.entity(),
                    self.slot("Category")?,
                    "Category",
                )
            }

            /// `Priority`, if given. Must be in `0..=100`. `NotInSchema` for
            /// IFC2X3.
            pub fn priority(self) -> MaterialResult<Option<i64>> {
                let value = optional_integer(
                    $ifc_name,
                    self.id(),
                    self.entity(),
                    self.slot("Priority")?,
                    "Priority",
                )?;
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
    /// `IfcMaterialLayerWithOffsets.OffsetDirection`. Required.
    pub fn offset_direction(self) -> MaterialResult<LayerSetDirection> {
        let token = required_enum(
            "IFCMATERIALLAYERWITHOFFSETS",
            self.id(),
            self.entity(),
            self.slot("OffsetDirection")?,
            "OffsetDirection",
        )?;
        LayerSetDirection::parse(token).ok_or_else(|| MaterialError::InvalidValue {
            entity: "IFCMATERIALLAYERWITHOFFSETS",
            id: self.id(),
            attribute: "OffsetDirection",
            value: token.to_owned(),
        })
    }

    /// `IfcMaterialLayerWithOffsets.OffsetValues`. Required, a 2-element
    /// list.
    pub fn offset_values(self) -> MaterialResult<[f64; 2]> {
        required_number_array_2(
            "IFCMATERIALLAYERWITHOFFSETS",
            self.id(),
            self.entity(),
            self.slot("OffsetValues")?,
            "OffsetValues",
        )
    }
}

impl<'m> MaterialView<'m> {
    /// Iterates every `IfcMaterialLayer` instance in the model.
    pub fn layers(self) -> impl Iterator<Item = MaterialLayer<'m>> + 'm {
        let release = self.release();
        self.model()
            .of_type("IFCMATERIALLAYER")
            .map(move |(id, entity)| MaterialLayer::from_known(id, entity, release))
    }

    /// Iterates every `IfcMaterialLayerWithOffsets` instance in the model.
    pub fn layers_with_offsets(self) -> impl Iterator<Item = MaterialLayerWithOffsets<'m>> + 'm {
        let release = self.release();
        self.model()
            .of_type("IFCMATERIALLAYERWITHOFFSETS")
            .map(move |(id, entity)| MaterialLayerWithOffsets::from_known(id, entity, release))
    }
}
