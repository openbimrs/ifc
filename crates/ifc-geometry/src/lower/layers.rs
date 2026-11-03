//! A product's Body split by a stack of parallel planes: one exact part per
//! material layer (#455).
//!
//! # What this does and does not know
//!
//! The planes arrive as a neutral [`LayerCut`]: a unit normal and ascending
//! signed distances in the product's object frame, metres. Where they come
//! from (an `IfcMaterialLayerSetUsage`, its thicknesses, its offset) is the
//! facade's business; this crate does not depend on `ifc-material`.
//!
//! # The cut, in the host's frame
//!
//! The Body is lowered in the host's own frame (see the `net` module, #388),
//! where the planes are stated, split into its solid parts, and each part of
//! layer `k` gets one exact `Difference` per inner boundary of the layer:
//!
//! ```text
//! part_k = body - HalfSpace(n.x < b_k) - HalfSpace(n.x > b_(k+1))
//! ```
//!
//! One `Instance` with the host's world transform sits above each part, as
//! above the net body. Nothing is evaluated or tessellated here (ADR 0004).
//!
//! # Outer layers extend
//!
//! Only the inner boundaries cut. The first layer with thickness keeps
//! everything of the Body below its upper plane, the last everything above its
//! lower plane, so a Body that is thicker or thinner than its stack is never
//! left with a remainder that belongs to no layer, and the parts add up to the
//! Body by construction. A zero-thick layer owns no part (`None`) and no
//! boundary of its own: the next layer with thickness extends over it.
//!
//! # Polarity
//!
//! The kernel's `HalfSpace.agreement = true` selects the NORMAL side. The
//! tool removed below a boundary therefore has `agreement = false`, the one
//! removed above it `true`.

use axiolid_core::{BooleanOperator, Plane3, Point3, Vec3};
use axiolid_model::{GeometryNode, NodeId, SolidOperation};
use axiolid_primitive::HalfSpace;
use ifc_model::EntityId;

use crate::error::GeometryResult;
use crate::lower::net::{
    degenerate, lower_body_in_host_frame, place, solid_parts, NetLowering, NetOptions,
};
use crate::lower::LoweringSession;

/// How close to unit length a [`LayerCut`] normal must be.
const UNIT_TOLERANCE: f64 = 1e-9;

/// A stack of parallel planes in a product's object frame.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct LayerCut {
    /// Unit normal of the planes.
    pub normal: [f64; 3],
    /// `n + 1` signed distances along `normal` from the object origin,
    /// metres, non-decreasing (equal where a layer is zero-thick).
    pub boundaries: Vec<f64>,
}

impl LayerCut {
    /// A cut of `boundaries.len() - 1` layers.
    #[must_use]
    pub fn new(normal: [f64; 3], boundaries: Vec<f64>) -> Self {
        Self { normal, boundaries }
    }
}

impl From<&crate::LayerBoundaries> for LayerCut {
    fn from(bounds: &crate::LayerBoundaries) -> Self {
        Self::new(bounds.normal, bounds.boundaries.clone())
    }
}

/// Which Body is split.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum LayerBody {
    /// The gross Body. The default, as everywhere (ADR 0014).
    #[default]
    Gross,
    /// The Body with every voiding opening subtracted, on explicit request.
    Net(NetOptions),
}

/// A Body split by a [`LayerCut`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LayeredLowering {
    /// The Body that was split, in world space: the gross Body, or the net
    /// root for [`LayerBody::Net`].
    pub body: NodeId,
    /// The net lowering behind `body`, for [`LayerBody::Net`].
    pub net: Option<NetLowering>,
    /// One entry per layer, in world space; `None` for a zero-thick layer.
    pub parts: Vec<Option<NodeId>>,
}

/// Split `product`'s Body by `cut`: one exact part per layer.
///
/// `Ok(None)` when the product has no Body. The parts are exact nodes in the
/// session's graph; compile them with the `compile` feature's
/// `compile_product_layer_meshes_with`.
///
/// # Errors
///
/// [`GeometryError::Degenerate`](crate::error::GeometryError::Degenerate) naming `product` when the cut is invalid (a
/// normal that is not finite or not of unit length, fewer than two
/// boundaries, a boundary that is not finite or is smaller than the one
/// before it) or no layer has thickness. A net body that falls back to world
/// coordinates is [`GeometryError::Unsupported`](crate::error::GeometryError::Unsupported). Any error lowering the Body
/// is returned as is, including the net refusals of `lower_product_net`.
pub fn lower_product_layers(
    session: &mut LoweringSession<'_>,
    product: EntityId,
    cut: &LayerCut,
    body: LayerBody,
) -> GeometryResult<Option<LayeredLowering>> {
    let thick = validate(session, product, cut)?;
    let net = match body {
        LayerBody::Gross => None,
        LayerBody::Net(options) => Some(options),
    };
    let Some(local) = lower_body_in_host_frame(session, product, net)? else {
        return Ok(None);
    };
    let heads = solid_parts(session, local.local, product)?;

    let plane = |boundary: f64| Plane3 {
        origin: Point3::from_array(cut.normal.map(|n| n * boundary)),
        normal: Vec3::from_array(cut.normal),
    };
    // One node per boundary and side, shared by every part it cuts.
    let mut tools: Vec<[Option<NodeId>; 2]> = vec![[None; 2]; cut.boundaries.len()];
    let mut tool = |session: &mut LoweringSession<'_>, boundary: usize, above: bool| {
        let slot = &mut tools[boundary][usize::from(above)];
        if let Some(node) = *slot {
            return Ok(node);
        }
        let node = session.node_for(
            product,
            GeometryNode::HalfSpace(HalfSpace {
                boundary: plane(cut.boundaries[boundary]),
                agreement: above,
            }),
        )?;
        *slot = Some(node);
        GeometryResult::Ok(node)
    };

    let mut parts = vec![None; cut.boundaries.len() - 1];
    for (position, &layer) in thick.iter().enumerate() {
        let mut cut_heads = heads.clone();
        // Below the layer: everything under its lower plane is removed,
        // unless it is the first layer with thickness, which extends.
        // Above: likewise for the last.
        let mut sides = Vec::with_capacity(2);
        if position > 0 {
            sides.push((layer, false));
        }
        if position + 1 < thick.len() {
            sides.push((layer + 1, true));
        }
        for (boundary, above) in sides {
            let node = tool(session, boundary, above)?;
            for head in &mut cut_heads {
                *head = session.node_for(
                    product,
                    GeometryNode::SolidOperation(SolidOperation::Boolean {
                        left: *head,
                        right: node,
                        operator: BooleanOperator::Difference,
                    }),
                )?;
            }
        }
        let layer_node = match cut_heads.as_slice() {
            [single] => *single,
            _ => session.node_for(product, GeometryNode::Collection(cut_heads))?,
        };
        parts[layer] = Some(place(session, layer_node, local.to_world, product)?);
    }

    let body_node = local.net.as_ref().map_or(local.gross, |net| net.root);
    Ok(Some(LayeredLowering {
        body: body_node,
        net: local.net,
        parts,
    }))
}

/// Check the cut and return the layers that have thickness, ascending.
fn validate(
    session: &LoweringSession<'_>,
    product: EntityId,
    cut: &LayerCut,
) -> GeometryResult<Vec<usize>> {
    let invalid = |detail: &str| degenerate(session, product, detail);
    let length = cut.normal.iter().map(|n| n * n).sum::<f64>().sqrt();
    if !cut.normal.iter().all(|n| n.is_finite()) || (length - 1.0).abs() > UNIT_TOLERANCE {
        return Err(invalid(
            "the layer cut's normal is not a finite unit vector",
        ));
    }
    if cut.boundaries.len() < 2 {
        return Err(invalid("a layer cut needs at least two boundaries"));
    }
    if !cut.boundaries.iter().all(|b| b.is_finite()) {
        return Err(invalid("a layer cut boundary is not finite"));
    }
    if cut.boundaries.windows(2).any(|pair| pair[1] < pair[0]) {
        return Err(invalid("the layer cut's boundaries are not ascending"));
    }
    let thick: Vec<usize> = cut
        .boundaries
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| pair[1] > pair[0])
        .map(|(index, _)| index)
        .collect();
    if thick.is_empty() {
        return Err(invalid("no layer of the cut has thickness"));
    }
    Ok(thick)
}
