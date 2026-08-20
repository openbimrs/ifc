//! Product placement and representation selection.
//!
//! # Why placement is composed here and not per item
//!
//! A representation item is authored in its product local space.
//! The product hangs off an IfcLocalPlacement chain that walks up
//! the spatial tree to the site. Lowering an item without that
//! chain puts every product at the world origin: the geometry is
//! individually correct and the building is a heap.
//!
//! # Units are converted once
//!
//! Placement coordinates are raw file units, so the resolver
//! composes the chain unconverted and this module converts the
//! composed result. Converting per link would raise the scale to
//! the power of the chain depth.

use axiolid_model::NodeId;
use ifc_model::{EntityId, Model};

use crate::constraint::local::PlacementResolver;
use crate::error::{GeometryError, GeometryResult};
use crate::input::product::Product;
use crate::input::representation::Representation;
use crate::lower::dispatch::lower_representation_item;
use crate::lower::session::LoweringSession;
use crate::transform::Transform;
use crate::units::UnitScale;

pub use crate::input::representation::select_shape_representation;

/// The world transform for one product, in metres.
///
/// Resolves the IfcLocalPlacement chain and converts the composed result
/// once. A product with no ObjectPlacement is model-space, which the
/// schema allows, so it yields the identity rather than an error.
pub fn product_world_transform(
    model: &Model,
    units: &UnitScale,
    product: EntityId,
) -> GeometryResult<Transform> {
    let entity = model.get(product).ok_or(GeometryError::MissingEntity {
        referrer: product,
        missing: product,
    })?;
    let Some(placement) = Product::new(product, entity).object_placement() else {
        return Ok(Transform::identity());
    };
    let mut resolver = PlacementResolver::new();
    let file_units = resolver.world_transform(model, placement)?;
    Ok(file_units.to_metres(units))
}

/// Lower every item of a product selected representation into one graph.
///
/// All items share one session, so a product whose Body holds several
/// solids yields one graph with one Collection root rather than N
/// disconnected graphs the caller has to merge.
pub fn lower_product_items(
    session: &mut LoweringSession<'_>,
    product: EntityId,
) -> GeometryResult<Option<NodeId>> {
    let world = product_world_transform(session.model(), session.units(), product)?;
    let Some(representation) = select_shape_representation(session.model(), product)? else {
        return Ok(None);
    };

    let entity = session
        .model()
        .get(representation)
        .ok_or(GeometryError::MissingEntity {
            referrer: product,
            missing: representation,
        })?;
    let items = Representation::new(representation, entity).items()?;
    let mut roots = Vec::with_capacity(items.len());
    for item in items {
        roots.push(lower_representation_item(session, item, world)?);
    }
    match roots.len() {
        0 => Ok(None),
        1 => Ok(Some(roots[0])),
        _ => Ok(Some(session.node_for(
            product,
            axiolid_model::GeometryNode::Collection(roots),
        )?)),
    }
}

/// Products in the model that carry geometry, in stable id order.
///
/// IfcProduct is abstract with hundreds of subtypes, so enumerating names
/// would rot. A product is recognised structurally instead: it has a
/// Representation in slot 6 pointing at an IfcProductRepresentation.
pub fn geometric_products(model: &Model) -> Vec<EntityId> {
    let mut found: Vec<EntityId> = model
        .iter()
        .filter(|(id, entity)| {
            let product = Product::new(*id, entity);
            product.representation().is_some_and(|shape| {
                model.get(shape).is_some_and(|e| {
                    e.type_name.contains("PRODUCTREPRESENTATION")
                        || e.type_name.contains("PRODUCTDEFINITIONSHAPE")
                })
            })
        })
        .map(|(id, _)| id)
        .collect();
    found.sort();
    found
}
