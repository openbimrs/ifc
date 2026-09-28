//! `IfcRelSpaceBoundary`: which element bounds a space, and how.
//!
//! A space knows its volume; it does not know which wall encloses it. That
//! adjacency lives entirely in `IfcRelSpaceBoundary` entities, and it is what
//! thermal analysis, area take-off and daylight models consume. Without it a
//! room is a shape with no relationship to the fabric around it.
//!
//! # The subtype trap
//!
//! ```text
//! IfcRelSpaceBoundary            4 = RelatingSpace  5 = RelatedBuildingElement
//!                                6 = ConnectionGeometry (OPTIONAL)
//!   IfcRelSpaceBoundary1stLevel    + 9 = ParentBoundary
//!     IfcRelSpaceBoundary2ndLevel    + 10 = CorrespondingBoundary
//! ```
//!
//! `Model::ids_of_type` matches the EXACT type name. Every real second-level
//! BEM export writes `IfcRelSpaceBoundary2ndLevel`, so a lookup of the
//! supertype alone finds nothing and the crate reports a building with no
//! boundaries -- an empty answer, not an error. All three concrete types are
//! therefore queried explicitly.
//!
//! # What is reported and what is not
//!
//! `PhysicalOrVirtualBoundary` and `InternalOrExternalBoundary` are read as
//! stated. The schema's `CorrectPhysOrVirt` rule ties the first to whether
//! the related element is an `IfcVirtualElement`, but enforcing it belongs to
//! `ifc-validate`: this crate reports what the file says and never rejects it
//! as a domain view. A disagreement is surfaced through
//! [`SpaceBoundary::physical_matches_element`] so a caller can decide.
//!
//! The same holds for `ConnectionGeometry`: a reference that names nothing,
//! or names something that is not a connection geometry, is returned by
//! [`SpaceBoundary::connection_geometry`] as a
//! [`ConnectionGeometryAnomaly`] carrying the offending target, never
//! dropped and never passed off as a shape.

use ifc_model::{EntityId, Model, Value};

use super::link::refs_in_slot;
use super::slots::SPACE_BOUNDARY_TYPES;

/// Attribute positions beyond the two ends.
mod slot {
    /// `IfcRelSpaceBoundary.ConnectionGeometry`, `OPTIONAL` in IFC2x3 TC1,
    /// IFC4 ADD2 TC1 and IFC4X3 ADD2 alike.
    pub const CONNECTION_GEOMETRY: usize = 6;
    /// `IfcRelSpaceBoundary.PhysicalOrVirtualBoundary`.
    pub const PHYSICAL_OR_VIRTUAL: usize = 7;
    /// `IfcRelSpaceBoundary.InternalOrExternalBoundary`.
    pub const INTERNAL_OR_EXTERNAL: usize = 8;
    /// `IfcRelSpaceBoundary1stLevel.ParentBoundary`.
    pub const PARENT_BOUNDARY: usize = 9;
    /// `IfcRelSpaceBoundary2ndLevel.CorrespondingBoundary`.
    pub const CORRESPONDING_BOUNDARY: usize = 10;
}

/// Whether a boundary is real fabric or an analytical divider.
///
/// `NotDefined` is a distinct member rather than an `Option` because the
/// schema states it explicitly: a file saying "not defined" is making a
/// claim, and collapsing it into "absent" loses that.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BoundaryPhysicality {
    /// Real fabric: a wall, slab or roof.
    Physical,
    /// An analytical divider with no element, e.g. across an opening.
    Virtual,
    /// Stated as undetermined by the file.
    NotDefined,
    /// The slot held something that is not one of the three enum members.
    Unrecognized,
}

/// Whether the boundary faces conditioned space or outside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BoundaryExposure {
    /// Faces another internal space.
    Internal,
    /// Faces outside.
    External,
    /// Faces earth, water, or another external variant IFC4 names separately.
    ExternalVariant,
    /// Stated as undetermined by the file.
    NotDefined,
    /// The slot held something that is not a recognised member.
    Unrecognized,
}

/// Every concrete `IfcConnectionGeometry` subtype across the three releases.
///
/// `IfcConnectionGeometry` is `ABSTRACT`, so a well-formed slot names one of
/// these. The list is the union over IFC2x3 TC1 (which alone has
/// `IfcConnectionPortGeometry`) and IFC4 / IFC4X3 (which add
/// `IfcConnectionVolumeGeometry`); this crate does not know which release a
/// file declares, and rejecting a subtype for the wrong release is
/// `ifc-validate`'s job. Asserted against all three bundled schemas in
/// `tests/slot_layout.rs`.
const CONNECTION_GEOMETRY_TYPES: [&str; 6] = [
    "IFCCONNECTIONCURVEGEOMETRY",
    "IFCCONNECTIONPOINTECCENTRICITY",
    "IFCCONNECTIONPOINTGEOMETRY",
    "IFCCONNECTIONPORTGEOMETRY",
    "IFCCONNECTIONSURFACEGEOMETRY",
    "IFCCONNECTIONVOLUMEGEOMETRY",
];

/// A `ConnectionGeometry` slot the file fills with something unusable.
///
/// Returned by [`SpaceBoundary::connection_geometry`] instead of the shape.
/// Each variant names the boundary and, where there is one, the offending
/// target, so a caller can point at the record rather than silently losing a
/// surface from its coverage.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConnectionGeometryAnomaly {
    /// The slot names an entity the model does not contain.
    Dangling {
        /// The space boundary holding the reference.
        boundary: EntityId,
        /// The missing target.
        target: EntityId,
    },
    /// The slot names an entity that is not a concrete
    /// `IfcConnectionGeometry` subtype.
    WrongKind {
        /// The space boundary holding the reference.
        boundary: EntityId,
        /// The target that was found.
        target: EntityId,
        /// The target's STEP type, upper-cased.
        type_name: String,
    },
    /// The slot holds a value that is not a single entity reference, such as
    /// a list or a literal.
    NotAReference {
        /// The space boundary holding the value.
        boundary: EntityId,
    },
    /// The boundary itself is not in the model it was asked about, so its
    /// slot cannot be read. Only reachable with a model other than the one
    /// the boundary was read from.
    MissingBoundary {
        /// The boundary that was looked up.
        boundary: EntityId,
    },
}

/// One space boundary as the file states it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SpaceBoundary {
    /// The relationship entity itself.
    pub id: EntityId,
    /// Concrete STEP type, e.g. `IFCRELSPACEBOUNDARY2NDLEVEL`.
    pub type_name: String,
    /// `RelatingSpace`: the space being bounded.
    pub space: Option<EntityId>,
    /// `RelatedBuildingElement`: the element forming the boundary.
    pub element: Option<EntityId>,
    /// `PhysicalOrVirtualBoundary`.
    pub physicality: BoundaryPhysicality,
    /// `InternalOrExternalBoundary`.
    pub exposure: BoundaryExposure,
    /// `ParentBoundary`, on 1st-level boundaries and above.
    ///
    /// An inner boundary -- a window within a wall boundary -- names the
    /// wall's boundary here. `None` on a plain `IfcRelSpaceBoundary`, which
    /// has no such slot at all.
    pub parent: Option<EntityId>,
    /// `CorrespondingBoundary`, on 2nd-level boundaries only.
    ///
    /// The boundary on the OTHER side of the same fabric. This pairing is
    /// what makes second-level boundaries usable for heat transfer: each
    /// side's area is known and the two are linked.
    pub corresponding: Option<EntityId>,
}

impl SpaceBoundary {
    /// Does `physicality` agree with the related element's type?
    ///
    /// The schema's `CorrectPhysOrVirt` rule: a `Physical` boundary must not
    /// name an `IfcVirtualElement`, and a `Virtual` one must name either an
    /// `IfcVirtualElement` or an `IfcOpeningElement`. `NotDefined` is always
    /// admissible.
    ///
    /// Returns `None` when the element is absent or unresolvable, since
    /// agreement is then unknowable rather than false.
    #[must_use]
    pub fn physical_matches_element(&self, model: &Model) -> Option<bool> {
        let element = self.element?;
        let entity = model.get(element)?;
        let name = entity.type_name.to_ascii_uppercase();
        let is_virtual = name == "IFCVIRTUALELEMENT";
        let is_opening = name == "IFCOPENINGELEMENT";
        Some(match self.physicality {
            BoundaryPhysicality::Physical => !is_virtual,
            BoundaryPhysicality::Virtual => is_virtual || is_opening,
            BoundaryPhysicality::NotDefined => true,
            BoundaryPhysicality::Unrecognized => false,
        })
    }

    /// `ConnectionGeometry`: the boundary's shape, if the file gives one.
    ///
    /// The coordinates are in the object placement of the relating space
    /// ([`space`](Self::space)): IFC4 ADD2 TC1 states the connection
    /// geometry is "given within the local placement of each space", and
    /// that `SurfaceOnRelatingElement` / `CurveOnRelatingElement` are in the
    /// space's local coordinate system while the `...OnRelatedElement`
    /// counterparts, rarely exported, are in the element's. Placing the
    /// shape in the model therefore needs the space's placement, not the
    /// element's.
    ///
    /// The slot is `OPTIONAL` in IFC2x3, IFC4 and IFC4X3 and sits at the same
    /// position on `IfcRelSpaceBoundary1stLevel` and `2ndLevel`. The target
    /// is normally an `IfcConnectionSurfaceGeometry` (3D) or
    /// `IfcConnectionCurveGeometry` (2D); any concrete
    /// `IfcConnectionGeometry` subtype is returned, since which ones a
    /// boundary may use is a validation question.
    ///
    /// `model` must be the model the boundary was read from: the slot is
    /// read from it on each call rather than stored on the struct.
    ///
    /// # Errors
    ///
    /// Returns a [`ConnectionGeometryAnomaly`] when the slot names a missing
    /// entity or a non-connection-geometry entity, holds something other
    /// than one reference, or when the boundary is not in `model`. An absent
    /// (`$`) or missing slot is `Ok(None)`: the boundary is then stated
    /// logically, without a shape.
    pub fn connection_geometry(
        &self,
        model: &Model,
    ) -> Result<Option<EntityId>, ConnectionGeometryAnomaly> {
        let boundary = self.id;
        let entity = model
            .get(boundary)
            .ok_or(ConnectionGeometryAnomaly::MissingBoundary { boundary })?;
        let target = match entity.attribute(slot::CONNECTION_GEOMETRY) {
            None | Some(Value::Null) => return Ok(None),
            Some(Value::Ref(target)) => *target,
            Some(_) => return Err(ConnectionGeometryAnomaly::NotAReference { boundary }),
        };
        let resolved = model
            .get(target)
            .ok_or(ConnectionGeometryAnomaly::Dangling { boundary, target })?;
        let type_name = resolved.type_name.to_ascii_uppercase();
        if CONNECTION_GEOMETRY_TYPES.contains(&type_name.as_str()) {
            Ok(Some(target))
        } else {
            Err(ConnectionGeometryAnomaly::WrongKind {
                boundary,
                target,
                type_name,
            })
        }
    }
}

/// Read the enumeration in a slot, or `NotDefined` when absent.
fn physicality(model: &Model, id: EntityId, slot: usize) -> BoundaryPhysicality {
    match enum_text(model, id, slot).as_deref() {
        None => BoundaryPhysicality::NotDefined,
        Some("PHYSICAL") => BoundaryPhysicality::Physical,
        Some("VIRTUAL") => BoundaryPhysicality::Virtual,
        Some("NOTDEFINED") => BoundaryPhysicality::NotDefined,
        Some(_) => BoundaryPhysicality::Unrecognized,
    }
}

/// Read the exposure enumeration in a slot.
///
/// IFC4 adds `EXTERNAL_EARTH`, `EXTERNAL_WATER` and `EXTERNAL_FIRE` to the
/// original pair. They are grouped as `ExternalVariant` rather than folded
/// into `External`: a caller computing ground-contact losses needs them
/// apart, and silently merging would make that impossible.
fn exposure(model: &Model, id: EntityId, slot: usize) -> BoundaryExposure {
    match enum_text(model, id, slot).as_deref() {
        None => BoundaryExposure::NotDefined,
        Some("INTERNAL") => BoundaryExposure::Internal,
        Some("EXTERNAL") => BoundaryExposure::External,
        Some("EXTERNAL_EARTH" | "EXTERNAL_WATER" | "EXTERNAL_FIRE") => {
            BoundaryExposure::ExternalVariant
        }
        Some("NOTDEFINED") => BoundaryExposure::NotDefined,
        Some(_) => BoundaryExposure::Unrecognized,
    }
}

/// The enumeration text in a slot, upper-cased, or `None` if absent/null.
fn enum_text(model: &Model, id: EntityId, slot: usize) -> Option<String> {
    let entity = model.get(id)?;
    match entity.attribute(slot)? {
        Value::Enum(text) => Some(text.to_ascii_uppercase()),
        _ => None,
    }
}

/// Every space boundary in the model, across all three concrete types.
#[must_use]
pub fn all(model: &Model) -> Vec<SpaceBoundary> {
    let mut out = Vec::new();
    for slots in SPACE_BOUNDARY_TYPES {
        for id in model.ids_of_type(slots.type_name) {
            let Some(entity) = model.get(*id) else {
                continue;
            };
            out.push(SpaceBoundary {
                id: *id,
                type_name: entity.type_name.to_ascii_uppercase(),
                space: refs_in_slot(model, *id, slots.relating).into_iter().next(),
                element: refs_in_slot(model, *id, slots.related).into_iter().next(),
                physicality: physicality(model, *id, slot::PHYSICAL_OR_VIRTUAL),
                exposure: exposure(model, *id, slot::INTERNAL_OR_EXTERNAL),
                parent: refs_in_slot(model, *id, slot::PARENT_BOUNDARY)
                    .into_iter()
                    .next(),
                corresponding: refs_in_slot(model, *id, slot::CORRESPONDING_BOUNDARY)
                    .into_iter()
                    .next(),
            });
        }
    }
    out
}

/// The boundaries of one space.
#[must_use]
pub fn of_space(model: &Model, space: EntityId) -> Vec<SpaceBoundary> {
    all(model)
        .into_iter()
        .filter(|boundary| boundary.space == Some(space))
        .collect()
}

/// The boundaries naming one element.
///
/// A wall between two rooms is named by two boundaries, one per space.
#[must_use]
pub fn of_element(model: &Model, element: EntityId) -> Vec<SpaceBoundary> {
    all(model)
        .into_iter()
        .filter(|boundary| boundary.element == Some(element))
        .collect()
}
