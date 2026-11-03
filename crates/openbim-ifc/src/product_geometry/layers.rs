//! The boundary planes of each material layer of a layered wall, slab or
//! plate (#455).
//!
//! `ifc-material` knows the layers (thickness, material, offset, sense) but
//! not what they are measured from; `ifc-geometry` knows the reference (a
//! wall's Axis polyline, a slab's profile plane) but nothing of materials.
//! Neither may depend on the other, so the join lives here: it reads the
//! usage from the product's material association (its own or its type's),
//! asks [`ifc_geometry::layer_set_reference`] where the layers start, and
//! stacks the thicknesses from there.
//!
//! The answer is exact boundary planes in the product's object frame, in
//! metres, not split bodies: this crate never tessellates or runs booleans.
//! Parts of the body outside the first and last boundary belong to no layer;
//! no check that the body fits the stack is made.

#![cfg(feature = "material")]

use std::fmt;

use ifc_geometry::{layer_set_reference, units, GeometryError};
use ifc_material::{
    MaterialDefinition, MaterialError, MaterialLayerSet, MaterialUsageDefinition, MaterialView,
    ResolvedMaterialSelect,
};
use ifc_model::{EntityId, Model};

/// The boundary planes of a layered product's material layers.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct LayerBounds {
    /// The product.
    pub product: EntityId,
    /// Its `IfcMaterialLayerSetUsage`.
    pub usage: EntityId,
    /// The `IfcMaterialLayerSet` the usage stacks.
    pub layer_set: EntityId,
    /// What the layers are measured from: the Axis `IfcPolyline` of a wall or
    /// the `IfcExtrudedAreaSolid` of a slab or plate.
    pub reference: EntityId,
    /// Unit normal of the boundary planes in the product's object frame.
    pub normal: [f64; 3],
    /// `n + 1` signed distances along `normal` from the object origin, metres,
    /// ascending (equal where a layer is zero-thick).
    pub boundaries: Vec<f64>,
    /// The layers, `layers[k]` between `boundaries[k]` and `boundaries[k + 1]`.
    pub layers: Vec<BoundedLayer>,
}

/// One layer between two boundary planes.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct BoundedLayer {
    /// The layer's index in `IfcMaterialLayerSet.MaterialLayers`.
    pub index: usize,
    /// The `IfcMaterialLayer`.
    pub layer: EntityId,
    /// Its `IfcMaterial`, or `None` for a layer without one (an air gap).
    pub material: Option<EntityId>,
    /// Its thickness, metres.
    pub thickness: f64,
}

/// Why a product's layer bounds could not be read.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum LayerBoundsError {
    /// The usage, set or a layer is malformed.
    Material(MaterialError),
    /// The product's geometry gives no unambiguous reference, or the units
    /// do not resolve.
    Geometry(GeometryError),
    /// A layer is an `IfcMaterialLayerWithOffsets`: its extent varies along
    /// another axis, so a plane per boundary would be wrong.
    LayerWithOffsets {
        /// The usage.
        usage: EntityId,
        /// The layer.
        layer: EntityId,
    },
}

impl fmt::Display for LayerBoundsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Material(error) => write!(f, "{error}"),
            Self::Geometry(error) => write!(f, "{error}"),
            Self::LayerWithOffsets { usage, layer } => write!(
                f,
                "layer {layer} of usage {usage} is an IfcMaterialLayerWithOffsets, whose extent \
                 varies along another axis; its bounds are not interpreted yet"
            ),
        }
    }
}

impl std::error::Error for LayerBoundsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Material(error) => Some(error),
            Self::Geometry(error) => Some(error),
            Self::LayerWithOffsets { .. } => None,
        }
    }
}

impl From<MaterialError> for LayerBoundsError {
    fn from(error: MaterialError) -> Self {
        Self::Material(error)
    }
}

impl From<GeometryError> for LayerBoundsError {
    fn from(error: GeometryError) -> Self {
        Self::Geometry(error)
    }
}

/// The boundary planes of each material layer of `product`.
///
/// `Ok(None)` when the product's material (its own, else its type's) is not
/// an `IfcMaterialLayerSetUsage`. Only walls along AXIS2 and slabs and plates
/// along AXIS3 are interpreted; every other product or direction is a
/// [`LayerBoundsError::Geometry`] naming what is unsupported.
pub fn material_layer_bounds(
    model: &Model,
    product: EntityId,
) -> Result<Option<LayerBounds>, LayerBoundsError> {
    model.reverse_index();
    let view = MaterialView::new(model);
    let Some(assigned) = view.assigned_material(product)? else {
        return Ok(None);
    };
    let ResolvedMaterialSelect::Usage(MaterialUsageDefinition::LayerSet(usage)) = assigned.material
    else {
        return Ok(None);
    };
    let layer_set = usage.layer_set_id()?;
    let set = MaterialLayerSet::try_from_view(view, layer_set)?;
    let mut members = Vec::new();
    let mut thicknesses = Vec::new();
    for id in set.layer_ids()? {
        match view.resolve_material_select(id)? {
            ResolvedMaterialSelect::Definition(MaterialDefinition::Layer(layer)) => {
                thicknesses.push(layer.thickness()?);
                members.push((id, layer.material_id()?));
            }
            ResolvedMaterialSelect::Definition(MaterialDefinition::LayerWithOffsets(_)) => {
                return Err(LayerBoundsError::LayerWithOffsets {
                    usage: usage.id(),
                    layer: id,
                });
            }
            _ => {
                return Err(MaterialError::WrongEntityType {
                    expected: "IfcMaterialLayer",
                    actual: model
                        .get(id)
                        .map(|entity| entity.type_name.to_string())
                        .unwrap_or_default(),
                }
                .into())
            }
        }
    }
    let direction = match usage.layer_set_direction()? {
        ifc_material::LayerSetDirection::Axis1 => ifc_geometry::LayerSetDirection::Axis1,
        ifc_material::LayerSetDirection::Axis2 => ifc_geometry::LayerSetDirection::Axis2,
        ifc_material::LayerSetDirection::Axis3 => ifc_geometry::LayerSetDirection::Axis3,
    };
    let sense = match usage.direction_sense()? {
        ifc_material::DirectionSense::Positive => ifc_geometry::DirectionSense::Positive,
        ifc_material::DirectionSense::Negative => ifc_geometry::DirectionSense::Negative,
    };
    let offset = usage.offset_from_reference_line()?;
    let units = units::resolve(model)?;
    let reference = layer_set_reference(model, &units, product, direction)?;
    let stack = reference.boundaries(&units, sense, offset, &thicknesses)?;
    let layers = stack
        .order
        .iter()
        .map(|&index| {
            let (layer, material) = members[index];
            BoundedLayer {
                index,
                layer,
                material,
                thickness: units.length(thicknesses[index]),
            }
        })
        .collect();
    Ok(Some(LayerBounds {
        product,
        usage: usage.id(),
        layer_set,
        reference: reference.source,
        normal: stack.normal,
        boundaries: stack.boundaries,
        layers,
    }))
}
