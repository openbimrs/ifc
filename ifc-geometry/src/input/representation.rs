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
    /// The IfcRepresentationContext this representation is authored into.
    pub const CONTEXT_OF_ITEMS: usize = 0;
    /// Body, Axis, FootPrint; OPTIONAL in the schema.
    pub const REPRESENTATION_IDENTIFIER: usize = 1;
    /// Plan, Curve2D, SweptSolid, Brep; OPTIONAL in the schema.
    pub const REPRESENTATION_TYPE: usize = 2;
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

    /// Plan, Curve2D, SweptSolid, Brep; absent when the author omitted it.
    pub fn representation_type(&self) -> Option<String> {
        self.slots
            .opt_text(representation_slot::REPRESENTATION_TYPE)
    }

    /// The `IfcRepresentationContext` this representation is authored into.
    pub fn context(&self) -> Option<EntityId> {
        match self.slots.opt(representation_slot::CONTEXT_OF_ITEMS)? {
            ifc_model::Value::Ref(id) => Some(*id),
            _ => None,
        }
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

/// Representation identifiers that carry 2D drawing geometry, best first.
///
/// The inverse of [`SOLID_IDENTIFIERS`]. `Plan` and `Annotation` are authored
/// for drawings directly; `FootPrint` is the product outline a plan is usually
/// built from; `Axis` is the centreline, useful but least specific.
///
/// `Body` is deliberately absent: a solid in a plan selector would have to be
/// sectioned before it means anything in 2D, and returning one would let a
/// caller draw a projected solid where a plan curve was expected.
pub const PLAN_IDENTIFIERS: &[&str] = &["Plan", "Annotation", "FootPrint", "Axis"];

/// Pick the representation a 2D drawing should use for this product.
///
/// Preference is, in order:
///
/// 1. a [`PLAN_IDENTIFIERS`] match **inside** a `PLAN_VIEW` sub-context --
///    drawable geometry the author explicitly targeted at a plan;
/// 2. otherwise the best [`PLAN_IDENTIFIERS`] match in any context.
///
/// The two rules are intersected, not ordered. Treating the context as
/// sufficient on its own looks reasonable -- an author who sets
/// `TargetView = .PLAN_VIEW.` has stated intent -- but ArchiCAD authors
/// `Box`/`BoundingBox` shape representations *inside* a `PLAN_VIEW`
/// sub-context. A context-first rule returns those boxes and never reaches
/// the identifier list: on `AC20-FZK-Haus.ifc` that was 107 of 253 shape
/// representations, and every plan lookup came back a box. Authorial intent
/// selects *between* drawable candidates; it does not make a bounding box
/// drawable.
///
/// Returns `None` when the product has only solid or bounding-box geometry.
/// That is a real answer, not a failure: deriving a plan from a solid needs
/// sectioning, which this crate does not do. A caller that wants an outline
/// anyway should say so explicitly rather than be handed a box that claims
/// to be a plan.
pub fn select_plan_representation(
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

    // 1. Drawable AND explicitly targeted at a plan.
    for wanted in PLAN_IDENTIFIERS {
        for &candidate in &candidates {
            let Some(entity) = model.get(candidate) else {
                continue;
            };
            if Representation::new(candidate, entity)
                .identifier()
                .as_deref()
                != Some(*wanted)
            {
                continue;
            }
            let Some(context) = super::context::context_of(model, candidate) else {
                continue;
            };
            if context.is_plan_view() {
                return Ok(Some(candidate));
            }
        }
    }

    // 2. Drawable in any context.

    for wanted in PLAN_IDENTIFIERS {
        for &candidate in &candidates {
            let Some(entity) = model.get(candidate) else {
                continue;
            };
            if Representation::new(candidate, entity)
                .identifier()
                .as_deref()
                == Some(*wanted)
            {
                return Ok(Some(candidate));
            }
        }
    }
    Ok(None)
}
