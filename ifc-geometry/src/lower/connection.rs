//! Connection-surface lowering: the surface where two elements meet.
//!
//! # Why a helper and not a representation item
//!
//! `IfcConnectionSurfaceGeometry` is not a representation item. It is what
//! `IfcRelSpaceBoundary.ConnectionGeometry` and the element-connection
//! relationships point at, and its payload is an `IfcSurfaceOrFaceSurface`:
//! a bare `IfcSurface`, an `IfcFaceSurface` (or its `IfcAdvancedFace`
//! subtype), or an `IfcFaceBasedSurfaceModel`. The select is resolved here so
//! a consumer measuring space-boundary coverage does not re-derive it, and
//! each member goes through the lowerer that already owns its family.
//!
//! # Two ends, two coordinate systems
//!
//! `SurfaceOnRelatingElement` is given in the relating element's local
//! coordinate system, the optional `SurfaceOnRelatedElement` in the related
//! element's (IFC4 ADD2 TC1, `IfcConnectionSurfaceGeometry` attribute
//! definitions). One frame cannot place both, so each end has its own entry
//! point and the caller passes the placement of the element that end belongs
//! to. For a space boundary that is the space's placement.
//!
//! # Only surfaces
//!
//! The point, curve, volume and (IFC2X3) port connection kinds are legal
//! connection geometry but carry no surface. They are refused with a typed
//! [`crate::GeometryError::Unsupported`] naming the connection, never
//! substituted by a degenerate or bounding surface. Anything that is not a
//! connection geometry at all is a wrong-type error.
//!
//! Slots are identical in IFC2X3 TC1, IFC4 ADD2 TC1 and IFC4X3 ADD2:
//! `IfcConnectionGeometry` declares no attributes, so
//! `SurfaceOnRelatingElement` is 0 and `SurfaceOnRelatedElement` is 1
//! (`data/absolute-slots.txt`).

use axiolid_model::NodeId;
use ifc_model::EntityId;

use crate::constraint::connection::slot;
use crate::error::{GeometryError, GeometryResult};
use crate::lower::brep::lower_face_surface_node;
use crate::lower::collection::lower_collection_node;
use crate::lower::session::LoweringSession;
use crate::lower::surface::lower_surface_node;
use crate::select::SurfaceOrFaceSurface;
use crate::transform::Transform;

/// The accepted connection type; every other one is classified below.
const SURFACE_CONNECTION: &str = "IFCCONNECTIONSURFACEGEOMETRY";

/// Connection kinds that are valid IFC but carry no surface, with the reason.
///
/// `IFCCONNECTIONPORTGEOMETRY` exists only in IFC2X3 TC1, and
/// `IFCCONNECTIONVOLUMEGEOMETRY` only from IFC4 on; both are listed so a file
/// of either release gets a typed refusal rather than a wrong-type error.
const REFUSED: &[(&str, &str)] = &[
    (
        "IFCCONNECTIONPOINTGEOMETRY",
        "a point connection carries a point or vertex point, not a surface",
    ),
    (
        "IFCCONNECTIONPOINTECCENTRICITY",
        "an eccentric point connection carries a point or vertex point, not a surface",
    ),
    (
        "IFCCONNECTIONCURVEGEOMETRY",
        "a curve connection carries a curve or edge curve, not a surface",
    ),
    (
        "IFCCONNECTIONVOLUMEGEOMETRY",
        "a volume connection carries a solid or shell, not a surface",
    ),
    (
        "IFCCONNECTIONPORTGEOMETRY",
        "an IFC2X3 port connection carries a placement and a profile, not a surface",
    ),
];

/// Lower `SurfaceOnRelatingElement` of an `IfcConnectionSurfaceGeometry`.
///
/// `frame` is the RELATING element's placement: the surface is authored in
/// that element's local coordinate system. Returns the node of the lowered
/// surface, face surface (a single-face open `BRep`), or face-based surface
/// model (a `Collection` of open shells).
///
/// # Errors
///
/// - [`GeometryError::Unsupported`] naming `id` for a point, curve, volume
///   or port connection.
/// - [`GeometryError::WrongEntityType`] when `id` is not a connection
///   geometry, or the surface is not an `IfcSurfaceOrFaceSurface` member.
/// - [`GeometryError::MissingEntity`] for a dangling `id` or surface
///   reference, and [`GeometryError::MissingAttribute`] when the required
///   surface is `$`.
/// - Any error of the member's own lowerer.
pub fn lower_connection_surface(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    check_connection(session, id)?;
    let target = session
        .slots(id)?
        .req_ref(slot::AT_RELATING, "SurfaceOnRelatingElement")?;
    lower_surface_or_face_surface(session, id, target, frame)
}

/// Lower the optional `SurfaceOnRelatedElement` of an
/// `IfcConnectionSurfaceGeometry`.
///
/// `frame` is the RELATED element's placement. Returns `Ok(None)` when the
/// file omits the attribute: the schema then means "the relating surface is
/// the whole statement", and inventing a copy in another frame would place it
/// wrongly.
///
/// # Errors
///
/// As [`lower_connection_surface`]; a present value that is not an entity
/// reference is [`GeometryError::WrongValueKind`].
pub fn lower_related_connection_surface(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<Option<NodeId>> {
    check_connection(session, id)?;
    let slots = session.slots(id)?;
    if slots.opt(slot::AT_RELATED).is_none() {
        return Ok(None);
    }
    let target = slots.req_ref(slot::AT_RELATED, "SurfaceOnRelatedElement")?;
    lower_surface_or_face_surface(session, id, target, frame).map(Some)
}

/// Accept a surface connection; refuse every other kind with its reason.
fn check_connection(session: &LoweringSession<'_>, id: EntityId) -> GeometryResult<()> {
    let type_name = session.type_name(id)?;
    if type_name == SURFACE_CONNECTION {
        return Ok(());
    }
    if let Some((_, reason)) = REFUSED.iter().find(|(name, _)| *name == type_name) {
        return Err(session.unsupported(id, &type_name, reason));
    }
    Err(GeometryError::WrongEntityType {
        entity: id,
        actual: type_name,
        expected: "IfcConnectionSurfaceGeometry",
    })
}

/// Route one `IfcSurfaceOrFaceSurface` member to the lowerer owning it.
///
/// Resolution checks the face surface before the plain surface: the select
/// helper classifies most-derived first, so an `IfcAdvancedFace` reaches the
/// face path with its bounds rather than only its carrier surface.
fn lower_surface_or_face_surface(
    session: &mut LoweringSession<'_>,
    referrer: EntityId,
    target: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    match SurfaceOrFaceSurface::resolve(session.model(), referrer, target)? {
        SurfaceOrFaceSurface::Surface(surface) => lower_surface_node(session, surface, frame),
        SurfaceOrFaceSurface::FaceSurface(face) => lower_face_surface_node(session, face, frame),
        SurfaceOrFaceSurface::FaceBasedSurfaceModel(model) => {
            lower_collection_node(session, model, frame)
        }
    }
}
