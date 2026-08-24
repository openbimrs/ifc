//! `IfcLine`: a point and a vector, unbounded in both directions.
//!
//! # The trap: `Dir` is an `IfcVector`, not an `IfcDirection`
//!
//! The parameterisation is `P(u) = Pnt + u * Dir`, where `Dir` carries a
//! *magnitude* as well as an orientation. A caller that normalises `Dir` to a
//! unit vector silently rescales the parameter space by that magnitude, which
//! matters the moment the line is used as an `IfcTrimmedCurve` basis curve
//! trimmed by parameter: the trim values are in units of `Dir`, not in length
//! units. This view therefore hands back the `IfcVector` reference untouched.

use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId};

/// `IfcLine` attribute slots.
///
/// From IFC4 ADD2 TC1: `IfcLine` inherits nothing explicit from
/// `IfcCurve` / `IfcGeometricRepresentationItem` / `IfcRepresentationItem`
/// (those declare only inverse and derived attributes), so its own two
/// attributes occupy slots 0 and 1.
mod slot {
    /// `Pnt`: `IfcCartesianPoint`, the point at parameter zero.
    pub const PNT: usize = 0;
    /// `Dir`: `IfcVector`, the direction *and* the parameter scale.
    pub const DIR: usize = 1;
}

/// A borrowed view of an `IfcLine`.
#[derive(Debug, Clone, Copy)]
pub struct Line<'m> {
    slots: Slots<'m>,
}

impl<'m> Line<'m> {
    /// Wrap an entity known to be an `IfcLine`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcCartesianPoint` at parameter zero.
    ///
    /// Returned as a raw reference rather than resolved coordinates.
    // TODO: `resource::point` will provide a typed point view to resolve this.
    pub fn point_ref(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::PNT, "Pnt")
    }

    /// The `IfcVector` giving direction and parameter scale.
    ///
    /// Deliberately *not* an `IfcDirection`: see the module docs on why
    /// normalising this reference changes the meaning of trim parameters.
    pub fn direction_vector_ref(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::DIR, "Dir")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::Value;

    fn line() -> Entity {
        Entity::new(
            "IFCLINE",
            vec![Value::Ref(EntityId(10)), Value::Ref(EntityId(11))],
        )
    }

    #[test]
    fn reads_origin_point_and_direction_vector_from_their_own_slots() {
        let e = line();
        let view = Line::new(EntityId(1), &e);
        assert_eq!(view.point_ref().unwrap(), EntityId(10));
        assert_eq!(view.direction_vector_ref().unwrap(), EntityId(11));
    }

    #[test]
    fn a_line_missing_its_direction_reports_the_attribute_by_name() {
        let e = Entity::new("IFCLINE", vec![Value::Ref(EntityId(10))]);
        let view = Line::new(EntityId(7), &e);
        let err = view.direction_vector_ref().unwrap_err();
        assert!(err.to_string().contains("Dir"), "got: {err}");
        assert!(err.to_string().contains("#7"), "got: {err}");
    }

    #[test]
    fn id_is_carried_through_for_error_attribution() {
        let e = line();
        assert_eq!(Line::new(EntityId(42), &e).id(), EntityId(42));
    }
}
