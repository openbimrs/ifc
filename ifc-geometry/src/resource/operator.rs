//! `IfcCartesianTransformationOperator`: the mapped-item transform.
//!
//! An `IfcMappedItem` reuses one representation many times (a door type placed
//! 400 times) and each instance carries an operator saying how to move, rotate
//! and scale the shared geometry. That makes this the one place in IFC where a
//! transform may be **scaled**, including non-uniformly, so it cannot be
//! folded into the rigid placement path.
//!
//! # Slots are shared across the whole family
//!
//! `Axis1`, `Axis2`, `LocalOrigin` and `Scale` are declared by the abstract
//! supertype, so they occupy slots 0..=3 in *every* concrete subtype. The
//! subtypes append: `Axis3` at 4 (3D), `Scale2` at 4 (2D non-uniform), and
//! `Axis3`, `Scale2`, `Scale3` at 4..=6 (3D non-uniform). Note that the 2D and
//! 3D non-uniform variants put `Scale2` at *different* indices.
//!
//! # Scale defaults chain
//!
//! `Scl := NVL(Scale, 1.0)`, then `Scl2 := NVL(Scale2, Scl)` and
//! `Scl3 := NVL(Scale3, Scl)`. So a non-uniform operator with only `Scale` set
//! is uniform, and `Scale2` defaults to `Scale`, **not** to 1.0. Defaulting the
//! secondary axes to 1.0 silently squashes every instance of the mapped item.
//!
//! # Axis1 is X, Axis2 is Y (unlike a placement)
//!
//! A placement gives Z first (`Axis`) and X second (`RefDirection`). An
//! operator gives X first (`Axis1`), Y second (`Axis2`), Z last (`Axis3`).
//! Reading them in placement order transposes the frame.

use crate::error::{GeometryError, GeometryResult};
use crate::resource::axes::{base_axes_2d, base_axes_3d};
use crate::resource::direction::resolve_unit;
use crate::resource::point::cartesian_point_3d;
use crate::slots::Slots;
use crate::transform::Transform;
use ifc_model::{Entity, EntityId, Model};

/// Attribute slots as ABSOLUTE STEP positions, inherited attributes first.
mod slot {
    /// `Axis1 : OPTIONAL IfcDirection` (supertype; local X).
    pub const AXIS1: usize = 0;
    /// `Axis2 : OPTIONAL IfcDirection` (supertype; local Y).
    pub const AXIS2: usize = 1;
    /// `LocalOrigin : IfcCartesianPoint` (supertype).
    pub const LOCAL_ORIGIN: usize = 2;
    /// `Scale : OPTIONAL IfcReal` (supertype).
    pub const SCALE: usize = 3;

    /// `Axis3 : OPTIONAL IfcDirection`, declared by
    /// `IfcCartesianTransformationOperator3D` and inherited by its non-uniform
    /// subtype, so index 4 in both.
    pub const AXIS3: usize = 4;

    /// `Scale2` on `IfcCartesianTransformationOperator2DnonUniform`.
    ///
    /// Index 4 here because the 2D branch has no `Axis3`; the 3D non-uniform
    /// variant puts the same-named attribute at 5.
    pub const SCALE2_2D: usize = 4;

    /// `Scale2` on `IfcCartesianTransformationOperator3DnonUniform`.
    pub const SCALE2_3D: usize = 5;
    /// `Scale3` on `IfcCartesianTransformationOperator3DnonUniform`.
    pub const SCALE3_3D: usize = 6;
}

/// The attributes every operator shares, from the abstract supertype.
///
/// A view in its own right so a caller holding an operator of unknown subtype
/// can still read the origin and scale without dispatching, and so the four
/// concrete views do not each re-derive the default chain.
#[derive(Debug, Clone, Copy)]
pub struct CartesianTransformationOperator<'m> {
    slots: Slots<'m>,
}

impl<'m> CartesianTransformationOperator<'m> {
    /// Wrap an entity assumed to be an `IfcCartesianTransformationOperator`
    /// subtype.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `LocalOrigin` reference. Required by the schema.
    pub fn local_origin_ref(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::LOCAL_ORIGIN, "LocalOrigin")
    }

    /// The translation part, promoted to 3D.
    pub fn local_origin(&self, model: &'m Model) -> GeometryResult<[f64; 3]> {
        cartesian_point_3d(model, self.id(), self.local_origin_ref()?)
    }

    /// The raw `Scale`, `None` when the file omitted it.
    ///
    /// Prefer [`Self::scale`] unless you specifically need to know whether the
    /// value was written, e.g. to decide what `Scale2` defaults to.
    pub fn scale_attribute(&self) -> Option<f64> {
        self.slots.opt_f64(slot::SCALE)
    }

    /// The effective uniform scale: the derived `Scl := NVL(Scale, 1.0)`.
    ///
    /// A non-positive scale is [`crate::GeometryError::Degenerate`]: the schema's
    /// `ScaleGreaterZero` rule forbids it, zero collapses the mapped geometry
    /// to a point, and a negative value mirrors it while leaving the winding
    /// order inverted, which shows up much later as inside-out normals.
    pub fn scale(&self) -> GeometryResult<f64> {
        let value = self.scale_attribute().unwrap_or(1.0);
        self.checked_scale(value, "Scale")
    }

    /// The local X direction (`Axis1`), normalized; `None` when absent.
    pub fn axis1(&self, model: &'m Model) -> GeometryResult<Option<[f64; 3]>> {
        self.optional_direction(model, slot::AXIS1)
    }

    /// The local Y direction (`Axis2`), normalized; `None` when absent.
    ///
    /// Only its *sign* relative to `Axis3 x Axis1` survives: the derived `U`
    /// re-orthogonalizes it, so a slightly-off `Axis2` is corrected while an
    /// opposing one flips the handedness of the frame.
    pub fn axis2(&self, model: &'m Model) -> GeometryResult<Option<[f64; 3]>> {
        self.optional_direction(model, slot::AXIS2)
    }

    /// Read an optional direction slot, normalizing when present.
    fn optional_direction(
        &self,
        model: &'m Model,
        index: usize,
    ) -> GeometryResult<Option<[f64; 3]>> {
        match self.slots.opt_ref(index) {
            Some(id) => resolve_unit(model, self.id(), id).map(Some),
            None => Ok(None),
        }
    }

    /// Reject a scale the schema forbids, naming which attribute it came from.
    fn checked_scale(&self, value: f64, attribute: &str) -> GeometryResult<f64> {
        if value > 0.0 {
            Ok(value)
        } else {
            Err(self.slots.degenerate(format!(
                "{attribute} is {value}, but the schema requires a scale greater than zero"
            )))
        }
    }
}

/// A borrowed view of an `IfcCartesianTransformationOperator2D`.
#[derive(Debug, Clone, Copy)]
pub struct CartesianTransformationOperator2D<'m> {
    base: CartesianTransformationOperator<'m>,
}

impl<'m> CartesianTransformationOperator2D<'m> {
    /// Wrap an entity assumed to be an `IfcCartesianTransformationOperator2D`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            base: CartesianTransformationOperator::new(id, entity),
        }
    }

    /// The shared supertype attributes.
    pub fn base(&self) -> CartesianTransformationOperator<'m> {
        self.base
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.base.id()
    }

    /// The operator as a 3D transform with uniform scale applied.
    pub fn transform(&self, model: &'m Model) -> GeometryResult<Transform> {
        let scale = self.base.scale()?;
        self.scaled_transform(model, [scale, scale])
    }

    /// Build the frame and apply per-axis factors.
    ///
    /// Z is left unscaled on purpose: a 2D operator has no third axis, so
    /// scaling Z would be inventing behaviour the file never asked for.
    fn scaled_transform(&self, model: &'m Model, factors: [f64; 2]) -> GeometryResult<Transform> {
        let origin = self.base.local_origin(model)?;
        let axis1 = self.base.axis1(model)?;
        let axis2 = self.base.axis2(model)?;
        let frame = base_axes_2d(origin, axis1, axis2).ok_or_else(|| {
            self.base
                .slots
                .degenerate("Axis1 and Axis2 do not define a 2D frame")
        })?;
        Ok(frame.scaled_nonuniform([factors[0], factors[1], 1.0]))
    }
}

/// A borrowed view of an `IfcCartesianTransformationOperator2DnonUniform`.
#[derive(Debug, Clone, Copy)]
pub struct CartesianTransformationOperator2DnonUniform<'m> {
    inner: CartesianTransformationOperator2D<'m>,
}

impl<'m> CartesianTransformationOperator2DnonUniform<'m> {
    /// Wrap an entity assumed to be the 2D non-uniform operator.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            inner: CartesianTransformationOperator2D::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.inner.id()
    }

    /// The shared supertype attributes.
    pub fn base(&self) -> CartesianTransformationOperator<'m> {
        self.inner.base()
    }

    /// The Y-axis scale: `Scl2 := NVL(Scale2, Scl)`.
    ///
    /// Defaults to `Scale`, not to 1.0.
    pub fn scale2(&self) -> GeometryResult<f64> {
        let base = self.base();
        let value = base.slots.opt_f64(slot::SCALE2_2D).unwrap_or(base.scale()?);
        base.checked_scale(value, "Scale2")
    }

    /// The operator as a 3D transform with per-axis scale applied.
    pub fn transform(&self, model: &'m Model) -> GeometryResult<Transform> {
        let factors = [self.base().scale()?, self.scale2()?];
        self.inner.scaled_transform(model, factors)
    }
}

/// A borrowed view of an `IfcCartesianTransformationOperator3D`.
#[derive(Debug, Clone, Copy)]
pub struct CartesianTransformationOperator3D<'m> {
    base: CartesianTransformationOperator<'m>,
}

impl<'m> CartesianTransformationOperator3D<'m> {
    /// Wrap an entity assumed to be an `IfcCartesianTransformationOperator3D`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            base: CartesianTransformationOperator::new(id, entity),
        }
    }

    /// The shared supertype attributes.
    pub fn base(&self) -> CartesianTransformationOperator<'m> {
        self.base
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.base.id()
    }

    /// The local Z direction (`Axis3`), normalized; `None` when absent.
    pub fn axis3(&self, model: &'m Model) -> GeometryResult<Option<[f64; 3]>> {
        self.base.optional_direction(model, slot::AXIS3)
    }

    /// The operator as a transform with uniform scale applied.
    pub fn transform(&self, model: &'m Model) -> GeometryResult<Transform> {
        let scale = self.base.scale()?;
        self.scaled_transform(model, [scale; 3])
    }

    /// Build the frame and apply per-axis factors.
    fn scaled_transform(&self, model: &'m Model, factors: [f64; 3]) -> GeometryResult<Transform> {
        let origin = self.base.local_origin(model)?;
        let axis1 = self.base.axis1(model)?;
        let axis2 = self.base.axis2(model)?;
        let axis3 = self.axis3(model)?;
        let frame = base_axes_3d(origin, axis1, axis2, axis3).ok_or_else(|| {
            self.base
                .slots
                .degenerate("Axis1 and Axis3 are parallel, so they define no frame")
        })?;
        Ok(frame.scaled_nonuniform(factors))
    }
}

/// A borrowed view of an `IfcCartesianTransformationOperator3DnonUniform`.
#[derive(Debug, Clone, Copy)]
pub struct CartesianTransformationOperator3DnonUniform<'m> {
    inner: CartesianTransformationOperator3D<'m>,
}

impl<'m> CartesianTransformationOperator3DnonUniform<'m> {
    /// Wrap an entity assumed to be the 3D non-uniform operator.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            inner: CartesianTransformationOperator3D::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.inner.id()
    }

    /// The shared supertype attributes.
    pub fn base(&self) -> CartesianTransformationOperator<'m> {
        self.inner.base()
    }

    /// The Y-axis scale: `Scl2 := NVL(Scale2, Scl)`.
    pub fn scale2(&self) -> GeometryResult<f64> {
        self.derived_scale(slot::SCALE2_3D, "Scale2")
    }

    /// The Z-axis scale: `Scl3 := NVL(Scale3, Scl)`.
    pub fn scale3(&self) -> GeometryResult<f64> {
        self.derived_scale(slot::SCALE3_3D, "Scale3")
    }

    /// The operator as a transform with per-axis scale applied.
    pub fn transform(&self, model: &'m Model) -> GeometryResult<Transform> {
        let factors = [self.base().scale()?, self.scale2()?, self.scale3()?];
        self.inner.scaled_transform(model, factors)
    }

    /// A secondary scale, falling back to `Scl` rather than to 1.0.
    fn derived_scale(&self, index: usize, attribute: &str) -> GeometryResult<f64> {
        let base = self.base();
        let value = base.slots.opt_f64(index).unwrap_or(base.scale()?);
        base.checked_scale(value, attribute)
    }
}

/// Resolve any `IfcCartesianTransformationOperator` subtype to a transform.
///
/// The attribute is typed as the supertype, so a slot holding one may contain
/// any of the four concrete forms and every consumer would otherwise have to
/// dispatch. Doing it once here keeps the 2D and non-uniform cases from being
/// quietly mishandled at each call site, mirroring
/// [`crate::resource::placement::axis_placement_transform`].
pub fn operator_transform(
    model: &Model,
    id: EntityId,
    entity: &Entity,
) -> GeometryResult<Transform> {
    match entity.type_name.to_ascii_uppercase().as_str() {
        "IFCCARTESIANTRANSFORMATIONOPERATOR3D" => {
            CartesianTransformationOperator3D::new(id, entity).transform(model)
        }
        "IFCCARTESIANTRANSFORMATIONOPERATOR3DNONUNIFORM" => {
            CartesianTransformationOperator3DnonUniform::new(id, entity).transform(model)
        }
        "IFCCARTESIANTRANSFORMATIONOPERATOR2D" => {
            CartesianTransformationOperator2D::new(id, entity).transform(model)
        }
        "IFCCARTESIANTRANSFORMATIONOPERATOR2DNONUNIFORM" => {
            CartesianTransformationOperator2DnonUniform::new(id, entity).transform(model)
        }
        other => Err(GeometryError::WrongEntityType {
            entity: id,
            actual: other.to_string(),
            expected: "IfcCartesianTransformationOperator",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::GeometryError;
    use ifc_model::Value;

    fn coords(values: &[f64]) -> Value {
        Value::List(values.iter().copied().map(Value::Real).collect())
    }

    /// #1 origin at (1,2,3), #2 = global X, #3 = global Y, #4 = global Z.
    fn model() -> Model {
        let mut model = Model::new();
        model.insert(
            EntityId(1),
            Entity::new("IFCCARTESIANPOINT", vec![coords(&[1.0, 2.0, 3.0])]),
        );
        model.insert(
            EntityId(2),
            Entity::new("IFCDIRECTION", vec![coords(&[1.0, 0.0, 0.0])]),
        );
        model.insert(
            EntityId(3),
            Entity::new("IFCDIRECTION", vec![coords(&[0.0, 1.0, 0.0])]),
        );
        model.insert(
            EntityId(4),
            Entity::new("IFCDIRECTION", vec![coords(&[0.0, 0.0, 1.0])]),
        );
        model
    }

    /// `IFCCARTESIANTRANSFORMATIONOPERATOR3D(Axis1, Axis2, LocalOrigin, Scale,
    /// Axis3)` with every axis omitted unless given.
    fn operator_3d(scale: Value) -> Entity {
        Entity::new(
            "IFCCARTESIANTRANSFORMATIONOPERATOR3D",
            vec![
                Value::Null,
                Value::Null,
                Value::Ref(EntityId(1)),
                scale,
                Value::Null,
            ],
        )
    }

    fn close(a: [f64; 3], b: [f64; 3]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9)
    }

    /// `Scl := NVL(Scale, 1.0)` -- an omitted scale is 1, never 0.
    #[test]
    fn absent_scale_defaults_to_one_not_zero() {
        let e = operator_3d(Value::Null);
        let op = CartesianTransformationOperator::new(EntityId(9), &e);
        assert_eq!(op.scale_attribute(), None);
        assert_eq!(op.scale().unwrap(), 1.0);
    }

    /// `LocalOrigin` sits at slot 2, after the two optional axes; reading it
    /// from slot 0 would silently place every mapped item at a direction.
    #[test]
    fn local_origin_is_read_after_the_two_optional_axes() {
        let model = model();
        let e = operator_3d(Value::Null);
        let op = CartesianTransformationOperator::new(EntityId(9), &e);
        assert_eq!(op.local_origin(&model).unwrap(), [1.0, 2.0, 3.0]);
    }

    #[test]
    fn a_uniform_operator_scales_every_axis_by_scale() {
        let model = model();
        let e = operator_3d(Value::Real(2.0));
        let t = CartesianTransformationOperator3D::new(EntityId(9), &e)
            .transform(&model)
            .unwrap();
        assert!(close(t.basis[0], [2.0, 0.0, 0.0]));
        assert!(close(t.basis[1], [0.0, 2.0, 0.0]));
        assert!(close(t.basis[2], [0.0, 0.0, 2.0]));
        assert_eq!(t.origin, [1.0, 2.0, 3.0]);
    }

    /// Axis1 is the operator's X (unlike a placement's Axis, which is Z).
    #[test]
    fn axis1_is_the_local_x_and_axis3_the_local_z() {
        let model = model();
        let e = Entity::new(
            "IFCCARTESIANTRANSFORMATIONOPERATOR3D",
            vec![
                Value::Ref(EntityId(3)), // Axis1 = global Y
                Value::Null,
                Value::Ref(EntityId(1)),
                Value::Null,
                Value::Ref(EntityId(4)), // Axis3 = global Z
            ],
        );
        let t = CartesianTransformationOperator3D::new(EntityId(9), &e)
            .transform(&model)
            .unwrap();
        assert!(close(t.basis[0], [0.0, 1.0, 0.0]), "got {:?}", t.basis[0]);
        assert!(close(t.basis[2], [0.0, 0.0, 1.0]));
    }

    /// `Scl2 := NVL(Scale2, Scl)`. Defaulting to 1.0 instead would squash
    /// every instance of the mapped item along Y and Z.
    #[test]
    fn nonuniform_secondary_scales_default_to_scale_not_to_one() {
        let model = model();
        let e = Entity::new(
            "IFCCARTESIANTRANSFORMATIONOPERATOR3DNONUNIFORM",
            vec![
                Value::Null,
                Value::Null,
                Value::Ref(EntityId(1)),
                Value::Real(3.0),
                Value::Null,
                Value::Null, // Scale2 omitted
                Value::Null, // Scale3 omitted
            ],
        );
        let op = CartesianTransformationOperator3DnonUniform::new(EntityId(9), &e);
        assert_eq!(op.scale2().unwrap(), 3.0);
        assert_eq!(op.scale3().unwrap(), 3.0);
        let t = op.transform(&model).unwrap();
        assert!(close(t.basis[1], [0.0, 3.0, 0.0]));
        assert!(close(t.basis[2], [0.0, 0.0, 3.0]));
    }

    #[test]
    fn nonuniform_scales_are_applied_per_axis_when_given() {
        let model = model();
        let e = Entity::new(
            "IFCCARTESIANTRANSFORMATIONOPERATOR3DNONUNIFORM",
            vec![
                Value::Null,
                Value::Null,
                Value::Ref(EntityId(1)),
                Value::Real(2.0),
                Value::Null,
                Value::Real(5.0),
                Value::Real(7.0),
            ],
        );
        let t = CartesianTransformationOperator3DnonUniform::new(EntityId(9), &e)
            .transform(&model)
            .unwrap();
        assert!(close(t.basis[0], [2.0, 0.0, 0.0]));
        assert!(close(t.basis[1], [0.0, 5.0, 0.0]));
        assert!(close(t.basis[2], [0.0, 0.0, 7.0]));
    }

    /// The 2D and 3D non-uniform variants put `Scale2` at different absolute
    /// slots (4 and 5), because only the 3D branch inherits `Axis3`.
    #[test]
    fn two_d_nonuniform_reads_scale2_one_slot_earlier_than_the_three_d_one() {
        let model = model();
        let e = Entity::new(
            "IFCCARTESIANTRANSFORMATIONOPERATOR2DNONUNIFORM",
            vec![
                Value::Null,
                Value::Null,
                Value::Ref(EntityId(1)),
                Value::Real(2.0),
                Value::Real(6.0), // Scale2 at slot 4, no Axis3 in this branch
            ],
        );
        let op = CartesianTransformationOperator2DnonUniform::new(EntityId(9), &e);
        assert_eq!(op.scale2().unwrap(), 6.0);
        let t = op.transform(&model).unwrap();
        assert!(close(t.basis[0], [2.0, 0.0, 0.0]));
        assert!(close(t.basis[1], [0.0, 6.0, 0.0]));
    }

    /// A 2D operator has no third axis, so Z must not be scaled.
    #[test]
    fn two_d_operator_leaves_the_z_axis_unscaled() {
        let model = model();
        let e = Entity::new(
            "IFCCARTESIANTRANSFORMATIONOPERATOR2D",
            vec![
                Value::Null,
                Value::Null,
                Value::Ref(EntityId(1)),
                Value::Real(4.0),
            ],
        );
        let t = CartesianTransformationOperator2D::new(EntityId(9), &e)
            .transform(&model)
            .unwrap();
        assert!(close(t.basis[0], [4.0, 0.0, 0.0]));
        assert!(close(t.basis[1], [0.0, 4.0, 0.0]));
        assert!(close(t.basis[2], [0.0, 0.0, 1.0]), "got {:?}", t.basis[2]);
    }

    /// `IfcOrthogonalComplement`: Y is X turned a quarter turn counter-
    /// clockwise when the file does not say otherwise.
    #[test]
    fn two_d_y_axis_is_the_orthogonal_complement_of_axis1() {
        let mut model = model();
        model.insert(
            EntityId(5),
            Entity::new("IFCDIRECTION", vec![coords(&[0.0, 1.0])]),
        );
        let e = Entity::new(
            "IFCCARTESIANTRANSFORMATIONOPERATOR2D",
            vec![
                Value::Ref(EntityId(5)),
                Value::Null,
                Value::Ref(EntityId(1)),
                Value::Null,
            ],
        );
        let t = CartesianTransformationOperator2D::new(EntityId(9), &e)
            .transform(&model)
            .unwrap();
        assert!(close(t.basis[0], [0.0, 1.0, 0.0]));
        assert!(close(t.basis[1], [-1.0, 0.0, 0.0]), "got {:?}", t.basis[1]);
    }

    /// An Axis2 opposing the derived Y flips it -- that is how a mirrored
    /// mapped item is written, so dropping the check loses the mirroring.
    #[test]
    fn an_opposing_axis2_flips_the_derived_y_axis() {
        let mut model = model();
        model.insert(
            EntityId(6),
            Entity::new("IFCDIRECTION", vec![coords(&[0.0, -1.0])]),
        );
        model.insert(
            EntityId(7),
            Entity::new("IFCDIRECTION", vec![coords(&[1.0, 0.0])]),
        );
        let e = Entity::new(
            "IFCCARTESIANTRANSFORMATIONOPERATOR2D",
            vec![
                Value::Ref(EntityId(7)), // Axis1 = X
                Value::Ref(EntityId(6)), // Axis2 = -Y
                Value::Ref(EntityId(1)),
                Value::Null,
            ],
        );
        let t = CartesianTransformationOperator2D::new(EntityId(9), &e)
            .transform(&model)
            .unwrap();
        assert!(close(t.basis[1], [0.0, -1.0, 0.0]), "got {:?}", t.basis[1]);
    }

    /// Same in 3D: Axis2 only decides handedness, since the derived Y is
    /// re-orthogonalized regardless.
    #[test]
    fn an_opposing_axis2_flips_handedness_in_three_d_too() {
        let mut model = model();
        model.insert(
            EntityId(8),
            Entity::new("IFCDIRECTION", vec![coords(&[0.0, -1.0, 0.0])]),
        );
        let e = Entity::new(
            "IFCCARTESIANTRANSFORMATIONOPERATOR3D",
            vec![
                Value::Ref(EntityId(2)),
                Value::Ref(EntityId(8)),
                Value::Ref(EntityId(1)),
                Value::Null,
                Value::Ref(EntityId(4)),
            ],
        );
        let t = CartesianTransformationOperator3D::new(EntityId(9), &e)
            .transform(&model)
            .unwrap();
        assert!(close(t.basis[1], [0.0, -1.0, 0.0]), "got {:?}", t.basis[1]);
    }

    /// `ScaleGreaterZero`: zero collapses the geometry, so it is a hard error
    /// rather than a transform that quietly erases every mapped item.
    #[test]
    fn zero_scale_is_degenerate() {
        let e = operator_3d(Value::Real(0.0));
        let err = CartesianTransformationOperator::new(EntityId(9), &e)
            .scale()
            .unwrap_err();
        assert!(matches!(err, GeometryError::Degenerate { .. }), "{err}");
    }

    #[test]
    fn negative_scale_is_degenerate() {
        let e = operator_3d(Value::Real(-1.0));
        assert!(CartesianTransformationOperator::new(EntityId(9), &e)
            .scale()
            .is_err());
    }

    #[test]
    fn a_negative_secondary_scale_names_that_attribute() {
        let e = Entity::new(
            "IFCCARTESIANTRANSFORMATIONOPERATOR3DNONUNIFORM",
            vec![
                Value::Null,
                Value::Null,
                Value::Ref(EntityId(1)),
                Value::Real(1.0),
                Value::Null,
                Value::Real(-2.0),
                Value::Null,
            ],
        );
        let err = CartesianTransformationOperator3DnonUniform::new(EntityId(9), &e)
            .scale2()
            .unwrap_err();
        assert!(err.to_string().contains("Scale2"), "got: {err}");
    }

    /// Zero-length axes would otherwise normalize to NaN and propagate.
    #[test]
    fn a_zero_length_axis_is_degenerate_rather_than_nan() {
        let mut model = model();
        model.insert(
            EntityId(9),
            Entity::new("IFCDIRECTION", vec![coords(&[0.0, 0.0, 0.0])]),
        );
        let e = Entity::new(
            "IFCCARTESIANTRANSFORMATIONOPERATOR3D",
            vec![
                Value::Null,
                Value::Null,
                Value::Ref(EntityId(1)),
                Value::Null,
                Value::Ref(EntityId(9)),
            ],
        );
        let err = CartesianTransformationOperator3D::new(EntityId(20), &e)
            .transform(&model)
            .unwrap_err();
        assert!(matches!(err, GeometryError::Degenerate { .. }), "{err}");
    }

    /// Axis1 parallel to Axis3 leaves no plane to project X into.
    #[test]
    fn axis1_parallel_to_axis3_is_degenerate() {
        let model = model();
        let e = Entity::new(
            "IFCCARTESIANTRANSFORMATIONOPERATOR3D",
            vec![
                Value::Ref(EntityId(4)),
                Value::Null,
                Value::Ref(EntityId(1)),
                Value::Null,
                Value::Ref(EntityId(4)),
            ],
        );
        assert!(CartesianTransformationOperator3D::new(EntityId(20), &e)
            .transform(&model)
            .is_err());
    }

    #[test]
    fn a_missing_local_origin_names_the_entity_and_attribute() {
        let e = Entity::new("IFCCARTESIANTRANSFORMATIONOPERATOR3D", vec![]);
        let err = CartesianTransformationOperator::new(EntityId(42), &e)
            .local_origin_ref()
            .unwrap_err();
        assert!(err.to_string().contains("#42"), "got: {err}");
        assert!(err.to_string().contains("LocalOrigin"), "got: {err}");
    }

    /// With no axes at all the frame is the identity translated to LocalOrigin.
    #[test]
    fn an_operator_without_axes_is_a_pure_translation_and_scale() {
        let model = model();
        let e = operator_3d(Value::Null);
        let t = CartesianTransformationOperator3D::new(EntityId(9), &e)
            .transform(&model)
            .unwrap();
        assert_eq!(t, Transform::translation([1.0, 2.0, 3.0]));
    }

    /// Axis2 alone must still produce a right-handed 2D frame.
    #[test]
    fn a_two_d_operator_with_only_axis2_derives_x_from_it() {
        let mut model = model();
        model.insert(
            EntityId(5),
            Entity::new("IFCDIRECTION", vec![coords(&[0.0, 1.0])]),
        );
        let e = Entity::new(
            "IFCCARTESIANTRANSFORMATIONOPERATOR2D",
            vec![
                Value::Null,
                Value::Ref(EntityId(5)),
                Value::Ref(EntityId(1)),
                Value::Null,
            ],
        );
        let t = CartesianTransformationOperator2D::new(EntityId(9), &e)
            .transform(&model)
            .unwrap();
        assert!(close(t.basis[0], [1.0, 0.0, 0.0]), "got {:?}", t.basis[0]);
        assert!(close(t.basis[1], [0.0, 1.0, 0.0]), "got {:?}", t.basis[1]);
    }
}
