//! `IfcConnectionGeometry`: where two elements actually meet.
//!
//! Used by `IfcRelConnectsWithRealizingElements` and the structural analysis
//! model to say *where* a connection happens: at a point, along a curve,
//! across a surface, or through a volume.
//!
//! # Why the geometry is optional per end
//!
//! Each connection names geometry "at the related element" and "at the
//! relating element". Both exist because the two elements may disagree about
//! where the joint is -- a beam's idealised centreline meets a column's
//! centreline at a point the physical parts do not touch. Modelling both ends
//! rather than one shared point is what makes eccentricity representable.

use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId};

/// Attribute slots shared by all `IfcConnectionGeometry` subtypes.
///
/// The supertype declares no explicit attributes, so each subtype's own
/// attributes start at 0. All four subtypes follow the same
/// `(AtRelatingElement, AtRelatedElement)` shape.
mod slot {
    /// Geometry in the relating element's coordinate system.
    pub const AT_RELATING: usize = 0;
    /// Geometry in the related element's coordinate system.
    pub const AT_RELATED: usize = 1;
}

/// Which kind of connection geometry an entity carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionKind {
    /// `IfcConnectionPointGeometry`: a vertex point.
    Point,
    /// `IfcConnectionPointEccentricity`: a point plus an offset vector.
    PointEccentricity,
    /// `IfcConnectionCurveGeometry`: a curve or edge curve.
    Curve,
    /// `IfcConnectionSurfaceGeometry`: a surface or face surface.
    Surface,
    /// `IfcConnectionVolumeGeometry`: a solid or shell.
    Volume,
}

impl ConnectionKind {
    /// Classify by IFC type name, or `None` if not a connection geometry.
    pub fn classify(type_name: &str) -> Option<Self> {
        match type_name.to_ascii_uppercase().as_str() {
            "IFCCONNECTIONPOINTGEOMETRY" => Some(Self::Point),
            "IFCCONNECTIONPOINTECCENTRICITY" => Some(Self::PointEccentricity),
            "IFCCONNECTIONCURVEGEOMETRY" => Some(Self::Curve),
            "IFCCONNECTIONSURFACEGEOMETRY" => Some(Self::Surface),
            "IFCCONNECTIONVOLUMEGEOMETRY" => Some(Self::Volume),
            _ => None,
        }
    }
}

/// `IfcConnectionPointEccentricity` eccentricity slots.
///
/// These follow the two inherited connection attributes.
mod eccentricity_slot {
    /// Offset along the connection X axis.
    pub const IN_X: usize = 2;
    /// Offset along the connection Y axis.
    pub const IN_Y: usize = 3;
    /// Offset along the connection Z axis.
    pub const IN_Z: usize = 4;
}

/// A borrowed view of any `IfcConnectionGeometry` subtype.
#[derive(Debug, Clone, Copy)]
pub struct ConnectionGeometry<'m> {
    slots: Slots<'m>,
    kind: ConnectionKind,
}

impl<'m> ConnectionGeometry<'m> {
    /// Wrap an entity, or `None` if it is not a connection geometry.
    pub fn new(id: EntityId, entity: &'m Entity) -> Option<Self> {
        let kind = ConnectionKind::classify(&entity.type_name)?;
        Some(Self {
            slots: Slots::new(id, entity),
            kind,
        })
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// Which kind of connection this is.
    pub fn kind(&self) -> ConnectionKind {
        self.kind
    }

    /// Geometry in the relating element's coordinate system.
    pub fn at_relating(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::AT_RELATING, "AtRelatingElement")
    }

    /// Geometry in the related element's coordinate system, if given.
    ///
    /// Optional because the two ends often coincide, in which case the file
    /// states the geometry once.
    pub fn at_related(&self) -> Option<EntityId> {
        self.slots.opt_ref(slot::AT_RELATED)
    }

    /// Eccentricity offsets, for `IfcConnectionPointEccentricity` only.
    ///
    /// Returns `None` for every other kind rather than silently reporting
    /// zeros, because "no eccentricity modelled" and "eccentricity of zero"
    /// are different statements about a structural joint.
    pub fn eccentricity(&self) -> Option<[f64; 3]> {
        if self.kind != ConnectionKind::PointEccentricity {
            return None;
        }
        Some([
            self.slots.opt_f64(eccentricity_slot::IN_X).unwrap_or(0.0),
            self.slots.opt_f64(eccentricity_slot::IN_Y).unwrap_or(0.0),
            self.slots.opt_f64(eccentricity_slot::IN_Z).unwrap_or(0.0),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::Value;

    #[test]
    fn classifies_every_connection_subtype() {
        for (name, expected) in [
            ("IFCCONNECTIONPOINTGEOMETRY", ConnectionKind::Point),
            (
                "IFCCONNECTIONPOINTECCENTRICITY",
                ConnectionKind::PointEccentricity,
            ),
            ("IFCCONNECTIONCURVEGEOMETRY", ConnectionKind::Curve),
            ("IFCCONNECTIONSURFACEGEOMETRY", ConnectionKind::Surface),
            ("IFCCONNECTIONVOLUMEGEOMETRY", ConnectionKind::Volume),
        ] {
            assert_eq!(ConnectionKind::classify(name), Some(expected));
        }
        assert_eq!(ConnectionKind::classify("IFCWALL"), None);
    }

    #[test]
    fn related_end_is_optional() {
        let e = Entity::new(
            "IFCCONNECTIONPOINTGEOMETRY",
            vec![Value::Ref(EntityId(5)), Value::Null],
        );
        let c = ConnectionGeometry::new(EntityId(1), &e).unwrap();
        assert_eq!(c.at_relating().unwrap(), EntityId(5));
        assert_eq!(c.at_related(), None);
    }

    /// "No eccentricity modelled" differs from "eccentricity is zero".
    #[test]
    fn eccentricity_is_absent_rather_than_zero_on_plain_point_connections() {
        let plain = Entity::new(
            "IFCCONNECTIONPOINTGEOMETRY",
            vec![Value::Ref(EntityId(5)), Value::Null],
        );
        assert_eq!(
            ConnectionGeometry::new(EntityId(1), &plain)
                .unwrap()
                .eccentricity(),
            None
        );

        let eccentric = Entity::new(
            "IFCCONNECTIONPOINTECCENTRICITY",
            vec![
                Value::Ref(EntityId(5)),
                Value::Null,
                Value::Real(0.1),
                Value::Real(0.2),
                Value::Null,
            ],
        );
        assert_eq!(
            ConnectionGeometry::new(EntityId(2), &eccentric)
                .unwrap()
                .eccentricity(),
            Some([0.1, 0.2, 0.0]),
            "an omitted axis offset is genuinely zero"
        );
    }

    #[test]
    fn non_connection_entities_are_rejected() {
        let e = Entity::new("IFCWALL", vec![]);
        assert!(ConnectionGeometry::new(EntityId(1), &e).is_none());
    }
}
