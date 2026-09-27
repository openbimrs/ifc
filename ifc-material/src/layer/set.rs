//! Ordered `IfcMaterialLayerSet` composition and total thickness.
//!
//! IFC2X3 declares `(MaterialLayers, LayerSetName)`; IFC4 and IFC4X3 append
//! `Description`.

use ifc_model::EntityId;

use crate::layer::definition::layer_thickness;
use crate::view::{borrowed_entity, optional_text, required_refs, MaterialView};
use crate::{MaterialError, MaterialResult};

borrowed_entity!(MaterialLayerSet, "IFCMATERIALLAYERSET");

impl<'m> MaterialLayerSet<'m> {
    /// `IfcMaterialLayerSet.MaterialLayers`, in set order. Required and
    /// must be non-empty.
    pub fn layer_ids(self) -> MaterialResult<Vec<EntityId>> {
        required_refs(
            "IFCMATERIALLAYERSET",
            self.id(),
            self.entity(),
            self.slot("MaterialLayers")?,
            "MaterialLayers",
            1,
        )
    }

    /// `IfcMaterialLayerSet.LayerSetName`, if given.
    pub fn name(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIALLAYERSET",
            self.id(),
            self.entity(),
            self.slot("LayerSetName")?,
            "LayerSetName",
        )
    }

    /// `IfcMaterialLayerSet.Description`, if given. `NotInSchema` for
    /// IFC2X3.
    pub fn description(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIALLAYERSET",
            self.id(),
            self.entity(),
            self.slot("Description")?,
            "Description",
        )
    }
}

impl<'m> MaterialView<'m> {
    /// Iterates every `IfcMaterialLayerSet` instance in the model.
    pub fn layer_sets(self) -> impl Iterator<Item = MaterialLayerSet<'m>> + 'm {
        let release = self.release();
        self.model()
            .of_type("IFCMATERIALLAYERSET")
            .map(move |(id, entity)| MaterialLayerSet::from_known(id, entity, release))
    }

    /// Evaluate the normative `IfcMlsTotalThickness` function.
    ///
    /// Each member is read against the set's release: an
    /// `IfcMaterialLayerWithOffsets` member of an IFC2X3 set is
    /// `EntityNotInSchema`, and a zero IFC2X3 thickness is invalid.
    pub fn total_thickness(self, set: MaterialLayerSet<'m>) -> MaterialResult<f64> {
        let mut total = 0.0;
        for layer_id in set.layer_ids()? {
            let layer = self.entity(set.id(), layer_id)?;
            let entity_type = if layer.is_type("IFCMATERIALLAYER") {
                "IFCMATERIALLAYER"
            } else if layer.is_type("IFCMATERIALLAYERWITHOFFSETS") {
                "IFCMATERIALLAYERWITHOFFSETS"
            } else {
                return Err(MaterialError::ReferenceType {
                    source_id: set.id(),
                    target: layer_id,
                    expected: "IFCMATERIALLAYER",
                    actual: layer.type_name.to_string(),
                });
            };
            total += layer_thickness(entity_type, layer_id, layer, set.release())?;
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
