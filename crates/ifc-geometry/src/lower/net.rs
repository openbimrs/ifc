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
//!
//! # Reference View openings: taken as applied (#351)
//!
//! IFC4 Reference View exports author an `IfcOpeningElement` with only a
//! `Reference` representation, usually tessellated, against a host whose
//! `Body` already has the hole in it. The schema says so in the opening's own
//! definition (IFC4 ADD2 TC1, `IfcOpeningElement`, entity definition): with
//! a `'Body'` representation the voiding relationship is an implicit Boolean
//! difference, but "if the IfcShapeRepresentation.RepresentationIdentifier =
//! 'Reference', then the Reference shape representation of the opening is
//! not subtracted, it is provided in addition to the hole in the Body shape
//! representation of the voided element". Its Reference View concept
//! (*Reference Geometry*, same page) repeats it: "Since there are no Boolean
//! operations, either as IfcBooleanResult or implicitly by
//! IfcRelVoidsElement the geometry of the IfcOpeningElement shall not be used
//! to subtract the opening from the 'Body' shape representation of the voided
//! element." IFC4.3 keeps both readings (`IfcOpeningElement`, concepts
//! *Reference Geometry* and *Reference Tessellation Geometry*), and its
//! *Element Openings* template states that the element's representation
//! "does account for voids at export".
//!
//! Such an opening is therefore neither subtractable nor missing: the host's
//! gross Body already IS its net Body. [`lower_product_net`] still refuses
//! it, as it always has, because a caller asking for net geometry has not
//! said it accepts the exporter's word for the hole. ADR 0014 makes net
//! geometry an explicit request, so accepting it is explicit too:
//! [`lower_product_net_with`] with [`NetOptions`] set to
//! [`ReferenceOnlyOpenings::TakeAsApplied`]. Then the opening is listed in
//! [`NetLowering::taken_as_applied`] with its relation and the reason, and
//! nothing is subtracted for it. Nothing is guessed or dropped: the caller
//! sees every opening either subtracted or taken as applied.
//!
//! Only an `IfcOpeningElement` (or IFC4's `IfcOpeningStandardCase`) whose
//! representations are all, and only, `Reference` qualifies; the clause is
//! about that entity and that identifier. An opening with no representation
//! at all, one with any other representation beside `Reference`, a
//! `Reference`-only `IfcVoidingFeature`, and an opening whose Body cannot be
//! lowered are refused exactly as without the option.

use axiolid_core::{BooleanOperator, Transform3};
use axiolid_model::{GeometryNode, Instance, NodeId, SolidOperation};
use ifc_model::{EntityId, Model};

use crate::error::{GeometryError, GeometryResult};
use crate::input::openings::voidings_of;
use crate::input::product::Product;
use crate::input::representation::{ProductShape, Representation};
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

/// An opening whose void the host's Body already carries (#351).
///
/// Nothing is subtracted for it; see the module documentation for the IFC
/// clauses. Only reported under [`ReferenceOnlyOpenings::TakeAsApplied`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct TakenAsApplied {
    /// The opening element.
    pub opening: EntityId,
    /// The `IfcRelVoidsElement` that names it.
    pub relation: EntityId,
    /// Why the opening counts as already applied.
    pub reason: AppliedReason,
}

/// Why an opening is taken as already applied to its host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AppliedReason {
    /// Every representation of the opening is `Reference`, which IFC says is
    /// "not subtracted" but "provided in addition to the hole in the Body".
    ReferenceRepresentationOnly,
}

/// What [`lower_product_net_with`] does with an opening that has only a
/// `Reference` representation (#351).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReferenceOnlyOpenings {
    /// Refuse the host with [`GeometryError::OpeningNotSubtracted`], as
    /// [`lower_product_net`] does. The default.
    #[default]
    Refuse,
    /// Report the opening in [`NetLowering::taken_as_applied`] and subtract
    /// nothing for it: the host's Body is taken to carry the void already.
    TakeAsApplied,
}

/// Options for a net request; [`Default`] is [`lower_product_net`]'s
/// behaviour.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct NetOptions {
    /// What to do with a `Reference`-only opening.
    pub reference_only_openings: ReferenceOnlyOpenings,
}

impl NetOptions {
    /// Set what to do with a `Reference`-only opening.
    ///
    /// Builder-style, because the struct is `#[non_exhaustive]`:
    /// `NetOptions::default().with_reference_only_openings(policy)`.
    #[must_use]
    pub fn with_reference_only_openings(mut self, policy: ReferenceOnlyOpenings) -> Self {
        self.reference_only_openings = policy;
        self
    }
}

/// A host lowered as net geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NetLowering {
    /// The host's gross Body, before any subtraction.
    pub gross: NodeId,
    /// One entry per opening, in the order they are subtracted.
    pub subtractions: Vec<Subtraction>,
    /// Openings the host's Body already carries, in ascending opening id.
    ///
    /// Always empty unless the caller chose
    /// [`ReferenceOnlyOpenings::TakeAsApplied`]. Every voiding opening is in
    /// exactly one of this list and [`Self::subtractions`].
    pub taken_as_applied: Vec<TakenAsApplied>,
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
/// does; openings of a body-less host have nothing to cut. The same as
/// [`lower_product_net_with`] with [`NetOptions::default`].
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
    lower_product_net_with(session, product, NetOptions::default())
}

/// [`lower_product_net`] with explicit [`NetOptions`].
///
/// Under [`ReferenceOnlyOpenings::TakeAsApplied`], an `IfcOpeningElement`
/// whose representations are all `Reference` is listed in
/// [`NetLowering::taken_as_applied`] instead of refusing the host (#351; see
/// the module documentation). A host whose openings are all taken as applied
/// lowers to its gross Body, and `root == gross`.
///
/// # Errors
///
/// As [`lower_product_net`]. An opening with no representation, one with a
/// representation other than `Reference`, or one whose Body does not lower
/// is still [`GeometryError::OpeningNotSubtracted`] whatever the options.
pub fn lower_product_net_with(
    session: &mut LoweringSession<'_>,
    product: EntityId,
    options: NetOptions,
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
            taken_as_applied: Vec::new(),
            root: gross,
        }));
    }

    // Split lazily: a host whose openings are all taken as applied has
    // nothing cut from it, so it needs no solid parts.
    let mut heads: Option<Vec<NodeId>> = None;
    let mut subtractions = Vec::with_capacity(voidings.len());
    let mut taken_as_applied = Vec::new();
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
                if options.reference_only_openings == ReferenceOnlyOpenings::TakeAsApplied
                    && reference_only_opening(session.model(), voiding.opening).map_err(refuse)?
                {
                    taken_as_applied.push(TakenAsApplied {
                        opening: voiding.opening,
                        relation: voiding.relation,
                        reason: AppliedReason::ReferenceRepresentationOnly,
                    });
                    continue;
                }
                return Err(refuse(degenerate(
                    session,
                    voiding.opening,
                    "the opening has no Body representation, so there is nothing to subtract",
                )));
            }
            Err(error) => return Err(refuse(error)),
        };
        let tools = solid_parts(session, body, voiding.opening).map_err(refuse)?;
        let heads = match &mut heads {
            Some(heads) => heads,
            None => heads.insert(solid_parts(session, gross, product)?),
        };
        for head in heads.iter_mut() {
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
        taken_as_applied,
        root,
    }))
}

/// Is `opening` an `IfcOpeningElement` whose every representation is
/// `Reference` (#351)?
///
/// `false` for an opening with no representation at all: that one carries
/// no statement that the host is already voided, so it stays refused. A
/// dangling representation reference is a missing entity, not a `false`.
fn reference_only_opening(model: &Model, opening: EntityId) -> GeometryResult<bool> {
    let entity = model.get(opening).ok_or(GeometryError::MissingEntity {
        referrer: opening,
        missing: opening,
    })?;
    // The clause is IfcOpeningElement's own; IFC4's deprecated
    // IfcOpeningStandardCase is its subtype. Other feature subtractions
    // (IfcVoidingFeature) state no such thing.
    if !(entity.is_type("IFCOPENINGELEMENT") || entity.is_type("IFCOPENINGSTANDARDCASE")) {
        return Ok(false);
    }
    let Some(shape) = Product::new(opening, entity).representation() else {
        return Ok(false);
    };
    let shape_entity = model.get(shape).ok_or(GeometryError::MissingEntity {
        referrer: opening,
        missing: shape,
    })?;
    let representations = ProductShape::new(shape, shape_entity).representations()?;
    if representations.is_empty() {
        return Ok(false);
    }
    for representation in representations {
        let entity = model
            .get(representation)
            .ok_or(GeometryError::MissingEntity {
                referrer: shape,
                missing: representation,
            })?;
        if Representation::new(representation, entity)
            .identifier()
            .as_deref()
            != Some(REFERENCE)
        {
            return Ok(false);
        }
    }
    Ok(true)
}

/// The representation identifier IFC says is "not subtracted".
const REFERENCE: &str = "Reference";

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
