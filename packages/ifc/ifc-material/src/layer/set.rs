//! Ordered `IfcMaterialLayerSet` composition and total thickness.

use ifc_model::EntityId;

use crate::view::{borrowed_entity, optional_text, required_number, required_refs, MaterialView};
use crate::{MaterialError, MaterialResult};

borrowed_entity!(MaterialLayerSet, "IFCMATERIALLAYERSET");

impl<'m> MaterialLayerSet<'m> {
    pub fn layer_ids(self) -> MaterialResult<Vec<EntityId>> {
        required_refs(
            "IFCMATERIALLAYERSET",
            self.id(),
            self.entity(),
            0,
            "MaterialLayers",
            1,
        )
    }

    pub fn name(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIALLAYERSET",
            self.id(),
            self.entity(),
            1,
            "LayerSetName",
        )
    }

    pub fn description(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIALLAYERSET",
            self.id(),
            self.entity(),
            2,
            "Description",
        )
    }
}

impl<'m> MaterialView<'m> {
    pub fn layer_sets(self) -> impl Iterator<Item = MaterialLayerSet<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIALLAYERSET")
            .map(|(id, entity)| MaterialLayerSet::from_known(id, entity))
    }

    /// Evaluate the normative `IfcMlsTotalThickness` function.
    pub fn total_thickness(self, set: MaterialLayerSet<'m>) -> MaterialResult<f64> {
        let mut total = 0.0;
        for layer_id in set.layer_ids()? {
            let layer = self.entity(set.id(), layer_id)?;
            if !layer.is_type("IFCMATERIALLAYER") && !layer.is_type("IFCMATERIALLAYERWITHOFFSETS") {
                return Err(MaterialError::ReferenceType {
                    source_id: set.id(),
                    target: layer_id,
                    expected: "IFCMATERIALLAYER",
                    actual: layer.type_name.to_string(),
                });
            }
            let thickness =
                required_number("IFCMATERIALLAYER", layer_id, layer, 1, "LayerThickness")?;
            if thickness < 0.0 {
                return Err(MaterialError::InvalidValue {
                    entity: "IFCMATERIALLAYER",
                    id: layer_id,
                    attribute: "LayerThickness",
                    value: "expected a non-negative length".to_owned(),
                });
            }
            total += thickness;
            if !total.is_finite() {
                return Err(MaterialError::InvalidValue {
                    entity: "IFCMATERIALLAYERSET",
                    id: set.id(),
                    attribute: "TotalThickness",
                    value: "finite layer thicknesses overflowed the aggregate".to_owned(),
                });
            }
        }
        Ok(total)
    }
}
