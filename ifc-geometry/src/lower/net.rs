//! Net product geometry: the Body minus every opening that voids it (#44).
//!
//! # Why this is a separate entry point
//!
//! A product's Body representation is its GROSS shape. `IfcRelVoidsElement`
//! says an opening's body is to be removed from it, but the file never authors
//! the result. Quantity takeoff wants gross; clearances, net areas, ratios and
//! clash tests through an opening want net. Both must stay reachable, so gross
//! keeps its existing entry point unchanged and net is asked for explicitly.
//!
//! # What the graph looks like
//!
//! The host body is lowered as usual, then each opening's Body is lowered
//! through ITS OWN placement chain (exporters place openings relative to the
//! host, and the chain resolves that). Exact `Boolean { Difference }` nodes
//! are appended; nothing is evaluated here.
//!
//! A boolean operand must be a solid, and a multi-item Body lowers to a
//! `Collection` (possibly under an `Instance`), which the graph rejects as an
//! operand. So both sides are first split into their solid PARTS, with any
//! enclosing instance transforms carried onto each part. Then, because
//! `(A u B) - O = (A - O) u (B - O)`, each host part is cut on its own:
//!
//! ```text
//! part_i -> (part_i - o1_a - o1_b) -> (... - o2_a) -> ...   for every host part i
//! step k  = Collection(every part's head after opening k)   (or the head itself)
//! root    = step of the last opening
//! ```
//!
//! Openings go in ascending id, so the graph is a function of the model's
//! content.
//!
//! # Refusal, not a quiet gross body
//!
//! Every per-opening failure becomes [`GeometryError::OpeningNotSubtracted`]
//! naming the opening. Skipping it would hand back geometry that claims to be
//! net and is not -- the silent failure this entry point exists to prevent.

use axiolid_core::{BooleanOperator, Transform3};
use axiolid_model::{GeometryNode, Instance, NodeId, SolidOperation};
use ifc_model::EntityId;

use crate::error::{GeometryError, GeometryResult};
use crate::input::openings::voidings_of;
use crate::lower::session::NodeShape;
use crate::lower::{lower_product_representation, LoweringSession, RepresentationPurpose};

/// One opening removed from a host, with the nodes that express it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Subtraction {
    /// The voiding element (`IfcOpeningElement`, `IfcVoidingFeature`, ...).
    pub opening: EntityId,
    /// The `IfcRelVoidsElement` that authored the subtraction.
    pub relation: EntityId,
    /// The opening's lowered Body, placed in world space.
    pub body: NodeId,
    /// The host with this opening and every earlier one removed.
    pub result: NodeId,
}

/// A host lowered as net geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NetLowering {
    /// The host's gross Body, before any subtraction.
    pub gross: NodeId,
    /// One entry per opening, in the order they are subtracted.
    pub subtractions: Vec<Subtraction>,
    /// The net result: the last subtraction, or `gross` when there are none.
    pub root: NodeId,
}

impl NetLowering {
    /// The subtracted openings, in subtraction order.
    pub fn openings(&self) -> Vec<EntityId> {
        self.subtractions.iter().map(|s| s.opening).collect()
    }
}

/// How deep `Collection`/`Instance` nesting may go while splitting a body.
///
/// Lowering has already bounded recursion, so this only guards against a
/// graph built by some future caller that nests without limit.
const MAX_PART_DEPTH: usize = 64;

/// Lower `product`'s Body with every voiding opening subtracted.
///
/// Returns `Ok(None)` when the product has no Body, exactly as the gross path
/// does; openings of a body-less host have nothing to cut.
///
/// # Errors
///
/// Any error lowering the host's own Body is returned as is, and so is a host
/// Body that contains no solid when it has openings to cut. A failure on an
/// opening -- its Body does not lower, it has no Body, it holds non-solid
/// geometry, or it names the host itself -- is
/// [`GeometryError::OpeningNotSubtracted`] naming the opening.
pub fn lower_product_net(
    session: &mut LoweringSession<'_>,
    product: EntityId,
) -> GeometryResult<Option<NetLowering>> {
    let Some(gross) = lower_product_representation(session, product, RepresentationPurpose::Body)?
    else {
        return Ok(None);
    };
    let voidings = voidings_of(session.model(), product);
    if voidings.is_empty() {
        return Ok(Some(NetLowering {
            gross,
            subtractions: Vec::new(),
            root: gross,
        }));
    }

    let mut heads = solid_parts(session, gross, product)?;
    let mut subtractions = Vec::with_capacity(voidings.len());
    for voiding in voidings {
        let refuse = |cause: GeometryError| GeometryError::OpeningNotSubtracted {
            host: product,
            opening: voiding.opening,
            cause: Box::new(cause),
        };
        if voiding.opening == product {
            // A host voiding itself would subtract to nothing. That is a broken
            // relation, not a request for an empty solid.
            return Err(refuse(degenerate(
                session,
                voiding.relation,
                "IfcRelVoidsElement names the same element as host and opening",
            )));
        }
        let body = match lower_product_representation(
            session,
            voiding.opening,
            RepresentationPurpose::Body,
        ) {
            Ok(Some(body)) => body,
            Ok(None) => {
                return Err(refuse(degenerate(
                    session,
                    voiding.opening,
                    "the opening has no Body representation, so there is nothing to subtract",
                )))
            }
            Err(error) => return Err(refuse(error)),
        };
        let tools = solid_parts(session, body, voiding.opening).map_err(refuse)?;
        for head in &mut heads {
            for &tool in &tools {
                // Attributed to the relation: it is the entity that authored the cut.
                *head = session
                    .node_for(
                        voiding.relation,
                        GeometryNode::SolidOperation(SolidOperation::Boolean {
                            left: *head,
                            right: tool,
                            operator: BooleanOperator::Difference,
                        }),
                    )
                    .map_err(refuse)?;
            }
        }
        let result = match heads.as_slice() {
            [single] => *single,
            _ => session
                .node_for(voiding.relation, GeometryNode::Collection(heads.clone()))
                .map_err(refuse)?,
        };
        subtractions.push(Subtraction {
            opening: voiding.opening,
            relation: voiding.relation,
            body,
            result,
        });
    }

    let root = subtractions.last().map_or(gross, |step| step.result);
    Ok(Some(NetLowering {
        gross,
        subtractions,
        root,
    }))
}

/// Split a lowered body into nodes the graph accepts as boolean operands.
///
/// A solid node is its own single part. A `Collection` contributes each
/// member's parts. An `Instance` whose chain ends in a solid is already a
/// valid operand and is kept as is; an `Instance` of a collection is pushed
/// down, so each part is re-instanced under the composed transform.
///
/// Anything else (a curve, a surface, an open shell lowered as a surface
/// model) bounds no volume, so it is refused naming `owner` rather than
/// dropped: dropping part of a body would under-cut or over-report.
fn solid_parts(
    session: &mut LoweringSession<'_>,
    root: NodeId,
    owner: EntityId,
) -> GeometryResult<Vec<NodeId>> {
    let mut parts = Vec::new();
    collect_parts(session, root, None, owner, 0, &mut parts)?;
    if parts.is_empty() {
        return Err(degenerate(
            session,
            owner,
            "the Body holds no solid, so there is nothing to subtract with or from",
        ));
    }
    Ok(parts)
}

fn collect_parts(
    session: &mut LoweringSession<'_>,
    node: NodeId,
    outer: Option<Transform3>,
    owner: EntityId,
    depth: usize,
    parts: &mut Vec<NodeId>,
) -> GeometryResult<()> {
    if depth > MAX_PART_DEPTH {
        return Err(GeometryError::ChainTooDeep {
            entity: owner,
            kind: "body part",
            limit: MAX_PART_DEPTH,
        });
    }
    let shape = session.shape(node).cloned();
    match shape {
        Some(NodeShape::Solid) => parts.push(place(session, node, outer, owner)?),
        Some(NodeShape::Instance { source, transform }) => {
            if ends_in_solid(session, source) {
                parts.push(place(session, node, outer, owner)?);
            } else {
                // `outer` is applied after this instance's own transform.
                let composed = outer.map_or(transform, |outer| outer * transform);
                collect_parts(session, source, Some(composed), owner, depth + 1, parts)?;
            }
        }
        Some(NodeShape::Collection(members)) => {
            for member in members {
                collect_parts(session, member, outer, owner, depth + 1, parts)?;
            }
        }
        Some(NodeShape::Other) | None => {
            return Err(degenerate(
                session,
                owner,
                "the Body holds geometry that bounds no volume, so it cannot be \
                 used in a boolean subtraction",
            ));
        }
    }
    Ok(())
}

/// Does this node's instance chain end in a solid?
fn ends_in_solid(session: &LoweringSession<'_>, mut node: NodeId) -> bool {
    for _ in 0..=MAX_PART_DEPTH {
        match session.shape(node) {
            Some(NodeShape::Solid) => return true,
            Some(NodeShape::Instance { source, .. }) => node = *source,
            _ => return false,
        }
    }
    false
}

/// `node` under `outer`, or `node` itself when there is no outer transform.
fn place(
    session: &mut LoweringSession<'_>,
    node: NodeId,
    outer: Option<Transform3>,
    owner: EntityId,
) -> GeometryResult<NodeId> {
    match outer {
        None => Ok(node),
        Some(transform) => session.node_for(
            owner,
            GeometryNode::Instance(Instance {
                source: node,
                transform,
            }),
        ),
    }
}

fn degenerate(session: &LoweringSession<'_>, entity: EntityId, detail: &str) -> GeometryError {
    let type_name = session
        .model()
        .get(entity)
        .map_or_else(String::new, |e| e.type_name.to_string());
    GeometryError::Degenerate {
        entity,
        type_name,
        detail: detail.to_string(),
    }
}
