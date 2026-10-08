//! Where each product sits and which representation draws it (#328).
//!
//! The language bindings carry geometry at two levels (ADR 0021). This is
//! the first, kernel-free one: for every product with a shape, its world
//! placement in metres and the Body representation a viewer would draw,
//! with the representation's identifier, type and context. It joins
//! [`products_world_transforms`](ifc_geometry::products_world_transforms)
//! with [`select_shape_representation`](ifc_geometry::select_shape_representation),
//! both public, so a change to either flows through unchanged.
//!
//! A failure is per product and per half: a placement chain that cycles
//! leaves every other product placed, and a product whose placement is
//! refused still reports its representation. Nothing is guessed: a refused
//! half is the [`GeometryError`] that refused it.

#![cfg(feature = "geometry-select")]

use ifc_geometry::{
    context_of, geometric_products, products_world_transforms, select_shape_representation, units,
    GeometryError, GeometryResult, Representation, RepresentationContext, TargetView, Transform,
};
use ifc_model::{EntityId, Model};

/// One product's world placement and selected Body representation.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ProductPlacement {
    /// The product.
    pub product: EntityId,
    /// Its `ObjectPlacement` resolved to world coordinates in metres; the
    /// identity for a product placed in model space.
    pub world: GeometryResult<Transform>,
    /// The representation a 3D viewer draws ([`select_shape_representation`]),
    /// or `None` when the product has no solid representation (an axis or
    /// footprint only).
    pub body: GeometryResult<Option<SelectedRepresentation>>,
}

/// An `IfcShapeRepresentation` chosen for a product.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SelectedRepresentation {
    /// The representation's entity id.
    pub id: EntityId,
    /// `RepresentationIdentifier`, e.g. `Body`.
    pub identifier: Option<String>,
    /// `RepresentationType`, e.g. `SweptSolid`, `Brep`, `MappedRepresentation`.
    pub representation_type: Option<String>,
    /// The context it is authored in (`ContextOfItems`), when it names one
    /// the file contains.
    pub context: Option<SelectedContext>,
}

/// The representation context of a [`SelectedRepresentation`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SelectedContext {
    /// The context's entity id.
    pub id: EntityId,
    /// `ContextType`, e.g. `Model`. A sub-context derives it from its
    /// parent, as the schema does (ADR 0009).
    pub context_type: Option<String>,
    /// `ContextIdentifier`, e.g. `Body`.
    pub identifier: Option<String>,
    /// A sub-context's `TargetView` as its enumeration literal, e.g.
    /// `MODEL_VIEW`; `None` on a root context.
    pub target_view: Option<String>,
}

/// World placement and Body selection for `products`, or, with `None`,
/// for every product that has a shape ([`geometric_products`]), in id
/// order.
///
/// Placements share one resolver, so a storey's chain is walked once. An
/// id the model lacks is reported with [`GeometryError::MissingEntity`] in
/// both halves.
pub fn product_placements(model: &Model, products: Option<&[EntityId]>) -> Vec<ProductPlacement> {
    let products = products.map_or_else(|| geometric_products(model), <[EntityId]>::to_vec);
    let scale = units::resolve(model);
    products_world_transforms(model, &scale, products)
        .into_iter()
        .map(|(product, world)| ProductPlacement {
            product,
            world,
            body: selected_body(model, product),
        })
        .collect()
}

fn selected_body(
    model: &Model,
    product: EntityId,
) -> GeometryResult<Option<SelectedRepresentation>> {
    let Some(id) = select_shape_representation(model, product)? else {
        return Ok(None);
    };
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer: product,
        missing: id,
    })?;
    let representation = Representation::new(id, entity);
    Ok(Some(SelectedRepresentation {
        id,
        identifier: representation.identifier(),
        representation_type: representation.representation_type(),
        context: context_of(model, id).map(|context| selected_context(model, &context)),
    }))
}

fn selected_context(model: &Model, context: &RepresentationContext<'_>) -> SelectedContext {
    // `ContextType` is DERIVED on a sub-context: the parent's value.
    let context_type = context.context_type().or_else(|| {
        let parent = context.parent()?;
        RepresentationContext::new(parent, model.get(parent)?).context_type()
    });
    SelectedContext {
        id: context.id(),
        context_type,
        identifier: context.identifier(),
        target_view: context.target_view().map(|view| target_view_literal(&view)),
    }
}

fn target_view_literal(view: &TargetView) -> String {
    match view {
        TargetView::PlanView => "PLAN_VIEW".into(),
        TargetView::ModelView => "MODEL_VIEW".into(),
        TargetView::ElevationView => "ELEVATION_VIEW".into(),
        TargetView::SectionView => "SECTION_VIEW".into(),
        TargetView::GraphView => "GRAPH_VIEW".into(),
        TargetView::SketchView => "SKETCH_VIEW".into(),
        TargetView::ReflectedPlanView => "REFLECTED_PLAN_VIEW".into(),
        TargetView::UserDefined(_) => "USERDEFINED".into(),
        TargetView::NotDefined => "NOTDEFINED".into(),
        TargetView::Other(literal) => literal.clone(),
    }
}

/// `transform` as a 4x4 column-major matrix, the layout WebGL, three.js
/// (`Matrix4.fromArray`) and most graphics APIs read: the three basis
/// columns, then the origin, each with its homogeneous coordinate.
#[must_use]
pub fn column_major(transform: &Transform) -> [f64; 16] {
    let [x, y, z] = transform.basis;
    let o = transform.origin;
    [
        x[0], x[1], x[2], 0.0, y[0], y[1], y[2], 0.0, z[0], z[1], z[2], 0.0, o[0], o[1], o[2], 1.0,
    ]
}
