//! Context and representation-selection views.
//!
//! # Why selection is a policy, not a first entry
//!
//! A product commonly carries several representations: an Axis
//! centreline, a FootPrint outline and a Body solid, in file
//! order. Taking Representations[0] yields a 2D curve for any
//! wall authored by Revit, which renders as nothing. Callers
//! that want a solid must ask for one by identifier.

use ifc_model::{Entity, EntityId, Model};

use crate::error::{GeometryError, GeometryResult};
use crate::slots::Slots;

/// Absolute slots on IfcProductRepresentation.
pub mod product_shape_slot {
    /// LIST of IfcRepresentation.
    pub const REPRESENTATIONS: usize = 2;
}

/// Absolute slots on IfcRepresentation.
pub mod representation_slot {
    /// Body, Axis, FootPrint; OPTIONAL in the schema.
    pub const REPRESENTATION_IDENTIFIER: usize = 1;
    /// The representation items themselves.
    pub const ITEMS: usize = 3;
}

/// One IfcRepresentation: a named set of items in a context.
#[derive(Debug, Clone, Copy)]
pub struct Representation<'m> {
    slots: Slots<'m>,
}

impl<'m> Representation<'m> {
    /// Wrap an entity assumed to be an IfcRepresentation subtype.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// Body, Axis, FootPrint; absent when the author omitted it.
    pub fn identifier(&self) -> Option<String> {
        self.slots
            .opt_text(representation_slot::REPRESENTATION_IDENTIFIER)
    }

    /// The representation items to lower.
    pub fn items(&self) -> GeometryResult<Vec<EntityId>> {
        self.slots.req_ref_list(representation_slot::ITEMS, "Items")
    }
}

/// One IfcProductRepresentation: the ordered representations of a product.
#[derive(Debug, Clone, Copy)]
pub struct ProductShape<'m> {
    slots: Slots<'m>,
}

impl<'m> ProductShape<'m> {
    /// Wrap an entity assumed to be an IfcProductRepresentation subtype.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// Representations in authored order.
    pub fn representations(&self) -> GeometryResult<Vec<EntityId>> {
        self.slots
            .req_ref_list(product_shape_slot::REPRESENTATIONS, "Representations")
    }
}

/// Representation identifiers that carry solid geometry, best first.
///
/// Body is the shape a viewer draws. Facetation is the IFC2x3-era
/// fallback some exporters still emit. Axis and FootPrint are
/// deliberately absent: they are 2D annotations, and selecting one
/// silently replaces a solid with a line.
pub const SOLID_IDENTIFIERS: &[&str] = &["Body", "Facetation"];

/// Pick the representation a viewer should draw for this product.
///
/// Preference order is SOLID_IDENTIFIERS, then any representation whose
/// identifier is missing. A file whose only representation is an Axis
/// returns None rather than a curve masquerading as a body.
pub fn select_shape_representation(
    model: &Model,
    product: EntityId,
) -> GeometryResult<Option<EntityId>> {
    let entity = model.get(product).ok_or(GeometryError::MissingEntity {
        referrer: product,
        missing: product,
    })?;
    let Some(shape_id) = super::product::Product::new(product, entity).representation() else {
        return Ok(None);
    };

    let shape_entity = model.get(shape_id).ok_or(GeometryError::MissingEntity {
        referrer: product,
        missing: shape_id,
    })?;
    let candidates = ProductShape::new(shape_id, shape_entity).representations()?;

    for wanted in SOLID_IDENTIFIERS {
        for &candidate in &candidates {
            let Some(entity) = model.get(candidate) else {
                continue;
            };
            let identifier = Representation::new(candidate, entity).identifier();
            if identifier.as_deref() == Some(*wanted) {
                return Ok(Some(candidate));
            }
        }
    }

    // No named solid representation: accept an unnamed one, since some
    // authors omit the identifier entirely, but never an Axis/FootPrint.
    for &candidate in &candidates {
        let Some(entity) = model.get(candidate) else {
            continue;
        };
        if Representation::new(candidate, entity)
            .identifier()
            .is_none()
        {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}
