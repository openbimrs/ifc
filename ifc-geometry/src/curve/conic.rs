//! `IfcConic` subtypes: `IfcCircle` and `IfcEllipse`.
//!
//! # Shared shape
//!
//! `IfcConic` contributes exactly one attribute, `Position`, which is an
//! `IfcAxis2Placement` *select*: an `IfcAxis2Placement2D` for a planar conic
//! or an `IfcAxis2Placement3D` for one in space. Both subtypes therefore read
//! `Position` from slot 0 and their own radii from slot 1 onwards.
//!
//! # Why radii are validated here
//!
//! `IfcPositiveLengthMeasure` is a *constrained type*, not a Rust type: STEP
//! parsers do not enforce it and real exporters emit zero-radius circles for
//! degenerate sweeps. A zero radius reaches a kernel as a division by zero or
//! a NaN vertex thousands of entities later. Rejecting it at the view boundary
//! turns an unfindable numerical bug into a located [`crate::GeometryError`].
//!
//! Note that IFC constrains the two ellipse semi-axes only to be positive. It
//! does *not* require `SemiAxis1 >= SemiAxis2`, so a "prolate" ellipse whose
//! major axis is the local Y direction is legal and must not be normalised
//! away: the placement's `RefDirection` fixes which axis is which.

use crate::error::GeometryResult;
use crate::resource::placement::Axis2Placement;
use crate::resource::resolve;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId, Model};

/// `IfcCircle` attribute slots.
///
/// From IFC4 ADD2 TC1: slot 0 `Position` is inherited from `IfcConic`; slot 1
/// `Radius` is the subtype's own.
pub(crate) mod circle_slot {
    /// `Position`: `IfcAxis2Placement` (2D or 3D), from `IfcConic`.
    pub const POSITION: usize = 0;
    /// `Radius`: `IfcPositiveLengthMeasure`.
    pub const RADIUS: usize = 1;
}

/// `IfcEllipse` attribute slots.
///
/// From IFC4 ADD2 TC1: slot 0 `Position` is inherited from `IfcConic`.
pub(crate) mod ellipse_slot {
    /// `Position`: `IfcAxis2Placement` (2D or 3D), from `IfcConic`.
    pub const POSITION: usize = 0;
    /// `SemiAxis1`: extent along the placement's local X direction.
    pub const SEMI_AXIS_1: usize = 1;
    /// `SemiAxis2`: extent along the placement's local Y direction.
    pub const SEMI_AXIS_2: usize = 2;
}

/// A borrowed view of an `IfcCircle`.
#[derive(Debug, Clone, Copy)]
pub struct Circle<'m> {
    slots: Slots<'m>,
}

impl<'m> Circle<'m> {
    /// Wrap an entity known to be an `IfcCircle`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcAxis2Placement` reference locating the circle.
    ///
    /// May resolve to either an `IfcAxis2Placement2D` or an
    /// `IfcAxis2Placement3D`; the select is not narrowed by the schema and
    /// both occur in practice for the same geometry depending on whether the
    /// circle is a profile outline or a swept directrix.
    pub fn position_ref(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(circle_slot::POSITION, "Position")
    }

    /// The placement as a typed view, resolved from the model.
    ///
    /// The select is not narrowed, so the result keeps whichever of the 2D
    /// or 3D forms the file wrote; [`Axis2Placement::transform`] handles both.
    pub fn position<'v>(&self, model: &'v Model) -> GeometryResult<Axis2Placement<'v>> {
        resolve::axis2_placement(model, self.id(), self.position_ref()?)
    }

    /// The radius, guaranteed positive.
    ///
    /// A zero or negative radius is [`crate::GeometryError::Degenerate`]: the
    /// file parses but the circle does not exist.
    pub fn radius(&self) -> GeometryResult<f64> {
        let r = self.slots.req_f64(circle_slot::RADIUS, "Radius")?;
        if r > 0.0 {
            Ok(r)
        } else {
            Err(self
                .slots
                .degenerate(format!("Radius must be positive, found {r}")))
        }
    }
}

/// A borrowed view of an `IfcEllipse`.
#[derive(Debug, Clone, Copy)]
pub struct Ellipse<'m> {
    slots: Slots<'m>,
}

impl<'m> Ellipse<'m> {
    /// Wrap an entity known to be an `IfcEllipse`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcAxis2Placement` reference locating the ellipse.
    pub fn position_ref(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(ellipse_slot::POSITION, "Position")
    }

    /// The placement as a typed view, resolved from the model.
    ///
    /// Its local X is the `SemiAxis1` direction, so the ellipse's orientation
    /// is only known once this is resolved.
    pub fn position<'v>(&self, model: &'v Model) -> GeometryResult<Axis2Placement<'v>> {
        resolve::axis2_placement(model, self.id(), self.position_ref()?)
    }

    /// Semi-axis along the placement's local X direction, guaranteed positive.
    pub fn semi_axis_1(&self) -> GeometryResult<f64> {
        self.positive(ellipse_slot::SEMI_AXIS_1, "SemiAxis1")
    }

    /// Semi-axis along the placement's local Y direction, guaranteed positive.
    ///
    /// May legitimately exceed [`Self::semi_axis_1`]; IFC does not order them.
    pub fn semi_axis_2(&self) -> GeometryResult<f64> {
        self.positive(ellipse_slot::SEMI_AXIS_2, "SemiAxis2")
    }

    /// Both semi-axes in declaration order.
    ///
    /// Offered because a kernel almost always needs the pair together, and
    /// fetching them separately doubles the chance of a caller swapping them.
    pub fn semi_axes(&self) -> GeometryResult<(f64, f64)> {
        Ok((self.semi_axis_1()?, self.semi_axis_2()?))
    }

    fn positive(&self, index: usize, name: &'static str) -> GeometryResult<f64> {
        let v = self.slots.req_f64(index, name)?;
        if v > 0.0 {
            Ok(v)
        } else {
            Err(self
                .slots
                .degenerate(format!("{name} must be positive, found {v}")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::Value;

    fn circle(radius: Value) -> Entity {
        Entity::new("IFCCIRCLE", vec![Value::Ref(EntityId(5)), radius])
    }

    fn ellipse(a: f64, b: f64) -> Entity {
        Entity::new(
            "IFCELLIPSE",
            vec![Value::Ref(EntityId(5)), Value::Real(a), Value::Real(b)],
        )
    }

    fn placement_model(type_name: &str) -> Model {
        let mut model = Model::new();
        let coords = Value::List(vec![Value::Real(1.0), Value::Real(2.0)]);
        model.insert(EntityId(4), Entity::new("IFCCARTESIANPOINT", vec![coords]));
        let placement = Entity::new(type_name, vec![Value::Ref(EntityId(4))]);
        model.insert(EntityId(5), placement);
        model
    }

    /// Both members of the select resolve, each to its own variant.
    #[test]
    fn conic_positions_resolve_to_the_select_member_the_file_wrote() {
        let e = circle(Value::Real(1.0));
        let model = placement_model("IFCAXIS2PLACEMENT2D");
        let position = Circle::new(EntityId(1), &e).position(&model).unwrap();
        assert!(matches!(position, Axis2Placement::TwoD(_)));
        assert_eq!(position.location(&model).unwrap(), [1.0, 2.0, 0.0]);

        let e = ellipse(2.0, 1.0);
        let model = placement_model("IFCAXIS2PLACEMENT3D");
        let position = Ellipse::new(EntityId(1), &e).position(&model).unwrap();
        assert!(matches!(position, Axis2Placement::ThreeD(_)));
        let transform = position.transform(&model).unwrap();
        assert_eq!(transform.apply([0.0; 3]), [1.0, 2.0, 0.0]);
    }

    /// An `IfcAxis1Placement` is not in the select: it has no local X.
    #[test]
    fn a_conic_placed_by_an_axis1_placement_is_a_typed_error() {
        let model = placement_model("IFCAXIS1PLACEMENT");
        let e = circle(Value::Real(1.0));
        let err = Circle::new(EntityId(1), &e).position(&model).unwrap_err();
        assert!(matches!(
            err,
            crate::GeometryError::WrongEntityType {
                entity: EntityId(5),
                ..
            }
        ));
        let e = ellipse(2.0, 1.0);
        let err = Ellipse::new(EntityId(1), &e)
            .position(&Model::new())
            .unwrap_err();
        assert_eq!(err.entity(), Some(EntityId(1)));
    }

    #[test]
    fn circle_reads_position_and_radius_from_conic_inherited_slots() {
        let e = circle(Value::Real(2.5));
        let view = Circle::new(EntityId(1), &e);
        assert_eq!(view.position_ref().unwrap(), EntityId(5));
        assert_eq!(view.radius().unwrap(), 2.5);
    }

    /// A measure wrapper is how conforming files write this; it must not hide
    /// the number.
    #[test]
    fn circle_radius_reads_through_a_positive_length_measure_wrapper() {
        let e = circle(Value::Typed {
            type_name: "IFCPOSITIVELENGTHMEASURE".into(),
            value: Box::new(Value::Real(3.0)),
        });
        assert_eq!(Circle::new(EntityId(1), &e).radius().unwrap(), 3.0);
    }

    #[test]
    fn zero_radius_circle_is_degenerate_not_a_zero_length_curve() {
        let e = circle(Value::Real(0.0));
        let err = Circle::new(EntityId(9), &e).radius().unwrap_err();
        assert!(err.to_string().contains("#9"), "got: {err}");
        assert!(err.to_string().contains("positive"), "got: {err}");
        assert!(
            !err.is_unsupported(),
            "a bad radius is corruption, not a gap"
        );
    }

    #[test]
    fn negative_radius_circle_is_degenerate() {
        let e = circle(Value::Real(-1.0));
        assert!(Circle::new(EntityId(1), &e).radius().is_err());
    }

    #[test]
    fn ellipse_returns_both_semi_axes_in_declaration_order() {
        let e = ellipse(4.0, 2.0);
        assert_eq!(
            Ellipse::new(EntityId(1), &e).semi_axes().unwrap(),
            (4.0, 2.0)
        );
    }

    /// IFC does not require SemiAxis1 >= SemiAxis2; normalising would rotate
    /// the ellipse ninety degrees.
    #[test]
    fn ellipse_accepts_a_second_semi_axis_larger_than_the_first() {
        let e = ellipse(1.0, 7.0);
        assert_eq!(
            Ellipse::new(EntityId(1), &e).semi_axes().unwrap(),
            (1.0, 7.0)
        );
    }

    #[test]
    fn zero_semi_axis_is_degenerate_and_names_which_one() {
        let e = ellipse(4.0, 0.0);
        let err = Ellipse::new(EntityId(1), &e).semi_axis_2().unwrap_err();
        assert!(err.to_string().contains("SemiAxis2"), "got: {err}");
    }
}
