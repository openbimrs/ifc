//! `IfcPlacement` and its three subtypes: where a local coordinate system is.
//!
//! # What the schema actually says
//!
//! `IfcPlacement` contributes `Location` (an `IfcCartesianPoint`) and nothing
//! else. Its subtypes add axes:
//!
//! | Entity | Adds | Meaning |
//! | --- | --- | --- |
//! | `IfcAxis1Placement` | `Axis` | one direction: a rotation/extrusion axis |
//! | `IfcAxis2Placement2D` | `RefDirection` | local X in the plane |
//! | `IfcAxis2Placement3D` | `Axis`, `RefDirection` | local Z and approximate local X |
//!
//! Because `Location` is inherited, it is **slot 0 in every subtype** and the
//! subtype's own attributes start at 1. Using local indices puts `Axis` where
//! `Location` is, which produces geometry that is placed plausibly but wrongly
//! -- the worst failure mode there is.
//!
//! # RefDirection is only approximate
//!
//! `IfcAxis2Placement3D` explicitly permits `Axis` and `RefDirection` to be
//! non-perpendicular; the derived `P` runs `IfcFirstProjAxis`, which projects
//! `RefDirection` onto the plane normal to `Axis`. That projection lives in
//! [`crate::transform::Transform::from_axes`] and is not repeated here.
//!
//! # Both axes are optional, and so is neither
//!
//! `AxisAndRefDirProvision` says `NOT (EXISTS(Axis) XOR EXISTS(RefDirection))`
//! -- give both or give neither. Files break this rule, so this module accepts
//! one alone and lets `from_axes` supply the schema default for the other
//! (global Z for `Axis`, projected global X for `RefDirection`) rather than
//! rejecting a file over a WHERE rule that costs nothing to tolerate.

use crate::error::{GeometryError, GeometryResult};
use crate::resource::direction::resolve_unit;
use crate::resource::point::cartesian_point_3d;
use crate::slots::Slots;
use crate::transform::Transform;
use ifc_model::{Entity, EntityId, Model};

/// Attribute slots as ABSOLUTE STEP positions, inherited attributes first.
mod slot {
    /// `Location : IfcCartesianPoint`, declared by `IfcPlacement`.
    ///
    /// Inherited, therefore slot 0 of `IfcAxis1Placement`,
    /// `IfcAxis2Placement2D` and `IfcAxis2Placement3D` alike.
    pub const LOCATION: usize = 0;

    /// `IfcAxis1Placement`.
    pub mod axis1 {
        /// `Axis : OPTIONAL IfcDirection` (after inherited `Location`).
        pub const AXIS: usize = 1;
    }

    /// `IfcAxis2Placement2D`.
    pub mod axis2_2d {
        /// `RefDirection : OPTIONAL IfcDirection` (after inherited `Location`).
        pub const REF_DIRECTION: usize = 1;
    }

    /// `IfcAxis2Placement3D`.
    pub mod axis2_3d {
        /// `Axis : OPTIONAL IfcDirection` (after inherited `Location`).
        pub const AXIS: usize = 1;
        /// `RefDirection : OPTIONAL IfcDirection`.
        pub const REF_DIRECTION: usize = 2;
    }
}

/// The part of a placement every subtype shares: its `Location`.
///
/// Exists so the three subtype views do not each re-derive location handling,
/// and so a caller holding an unknown `IfcPlacement` can still read the origin
/// without dispatching on the concrete type.
#[derive(Debug, Clone, Copy)]
pub struct Placement<'m> {
    slots: Slots<'m>,
}

impl<'m> Placement<'m> {
    /// Wrap an entity assumed to be an `IfcPlacement` subtype.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcCartesianPoint` reference giving the origin.
    pub fn location_ref(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::LOCATION, "Location")
    }

    /// The origin in the parent coordinate system, promoted to 3D.
    ///
    /// A 2D `Location` (legal under `IfcAxis2Placement2D`) becomes `z = 0`,
    /// which is the correct reading: the 2D system lies in the parent's
    /// z=0 plane.
    pub fn location(&self, model: &'m Model) -> GeometryResult<[f64; 3]> {
        cartesian_point_3d(model, self.id(), self.location_ref()?)
    }
}

/// A borrowed view of an `IfcAxis1Placement`: a point and one axis.
///
/// Used where only a single direction is meaningful -- a revolution axis, for
/// example. There is no local X, so this does **not** define a full coordinate
/// system and deliberately offers no `Transform`: manufacturing one would
/// invent an arbitrary rotation about the axis.
#[derive(Debug, Clone, Copy)]
pub struct Axis1Placement<'m> {
    placement: Placement<'m>,
}

impl<'m> Axis1Placement<'m> {
    /// Wrap an entity assumed to be an `IfcAxis1Placement`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            placement: Placement::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.placement.id()
    }

    /// The origin, promoted to 3D.
    pub fn location(&self, model: &'m Model) -> GeometryResult<[f64; 3]> {
        self.placement.location(model)
    }

    /// The axis direction, normalized; global Z when `Axis` is absent.
    ///
    /// The default matches the derived `Z` attribute:
    /// `NVL(IfcNormalise(Axis), IfcDirection([0,0,1]))`.
    pub fn axis(&self, model: &'m Model) -> GeometryResult<[f64; 3]> {
        match self.placement.slots.opt_ref(slot::axis1::AXIS) {
            Some(id) => resolve_unit(model, self.id(), id),
            None => Ok([0.0, 0.0, 1.0]),
        }
    }
}

/// A borrowed view of an `IfcAxis2Placement2D`: origin plus local X.
///
/// The local Y is not stored: it is the 90-degree counter-clockwise rotation
/// of X (`IfcOrthogonalComplement`), so it is derived here rather than read.
#[derive(Debug, Clone, Copy)]
pub struct Axis2Placement2D<'m> {
    placement: Placement<'m>,
}

impl<'m> Axis2Placement2D<'m> {
    /// Wrap an entity assumed to be an `IfcAxis2Placement2D`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            placement: Placement::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.placement.id()
    }

    /// The origin, promoted to 3D (`z = 0`).
    pub fn location(&self, model: &'m Model) -> GeometryResult<[f64; 3]> {
        self.placement.location(model)
    }

    /// The local X direction, normalized; global X when absent.
    pub fn ref_direction(&self, model: &'m Model) -> GeometryResult<[f64; 3]> {
        match self.placement.slots.opt_ref(slot::axis2_2d::REF_DIRECTION) {
            Some(id) => resolve_unit(model, self.id(), id),
            None => Ok([1.0, 0.0, 0.0]),
        }
    }

    /// The placement as a 3D transform in the z=0 plane.
    ///
    /// Local Z is forced to global Z so the derived Y matches
    /// `IfcOrthogonalComplement` (`[-x2, x1]`), i.e. X rotated a quarter turn
    /// counter-clockwise. Any other Z would silently mirror 2D profiles.
    pub fn transform(&self, model: &'m Model) -> GeometryResult<Transform> {
        let origin = self.location(model)?;
        let x = self.ref_direction(model)?;
        Transform::from_axes(origin, Some([0.0, 0.0, 1.0]), Some(x)).ok_or_else(|| {
            self.placement
                .slots
                .degenerate("RefDirection is parallel to the plane normal or zero-length")
        })
    }
}

/// A borrowed view of an `IfcAxis2Placement3D`: origin, local Z, local X.
#[derive(Debug, Clone, Copy)]
pub struct Axis2Placement3D<'m> {
    placement: Placement<'m>,
}

impl<'m> Axis2Placement3D<'m> {
    /// Wrap an entity assumed to be an `IfcAxis2Placement3D`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            placement: Placement::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.placement.id()
    }

    /// The origin, promoted to 3D.
    pub fn location(&self, model: &'m Model) -> GeometryResult<[f64; 3]> {
        self.placement.location(model)
    }

    /// The local Z direction if `Axis` is present, normalized.
    ///
    /// `None` means the file omitted it, which is legal and means global Z.
    /// The distinction is kept rather than defaulted here so
    /// [`Self::transform`] can hand `from_axes` the real "absent" case and get
    /// the spec's paired default for `RefDirection` too.
    pub fn axis(&self, model: &'m Model) -> GeometryResult<Option<[f64; 3]>> {
        self.optional_direction(model, slot::axis2_3d::AXIS)
    }

    /// The approximate local X direction if `RefDirection` is present.
    ///
    /// "Approximate" because the schema only requires it to be non-parallel to
    /// `Axis`; the true X is its projection onto the plane normal to `Axis`.
    pub fn ref_direction(&self, model: &'m Model) -> GeometryResult<Option<[f64; 3]>> {
        self.optional_direction(model, slot::axis2_3d::REF_DIRECTION)
    }

    /// The placement as an orthonormal transform.
    ///
    /// Delegates the Gram-Schmidt projection to
    /// [`Transform::from_axes`], which also supplies the schema defaults when
    /// `Axis` or `RefDirection` is absent. Parallel axes are
    /// [`crate::GeometryError::Degenerate`] -- they define no unique frame, and the
    /// schema's `AxisToRefDirPosition` rule forbids them.
    pub fn transform(&self, model: &'m Model) -> GeometryResult<Transform> {
        let origin = self.location(model)?;
        let axis = self.axis(model)?;
        let ref_direction = self.ref_direction(model)?;
        Transform::from_axes(origin, axis, ref_direction).ok_or_else(|| {
            self.placement
                .slots
                .degenerate("Axis and RefDirection are parallel, so they define no frame")
        })
    }

    /// Read an optional direction slot, normalizing when present.
    fn optional_direction(
        &self,
        model: &'m Model,
        index: usize,
    ) -> GeometryResult<Option<[f64; 3]>> {
        match self.placement.slots.opt_ref(index) {
            Some(id) => resolve_unit(model, self.id(), id).map(Some),
            None => Ok(None),
        }
    }
}

/// Resolve any `IfcAxis2Placement` reference to a transform.
///
/// `IfcAxis2Placement` is a SELECT over the 2D and 3D forms, so a slot typed
/// that way may hold either and every consumer has to dispatch. Doing it once
/// here keeps that dispatch from being re-invented -- and the 2D case quietly
/// mishandled -- at each call site.
///
/// `IfcAxis1Placement` is rejected rather than coerced: it carries one axis and
/// no local X, so any frame built from it would invent a rotation.
pub fn axis_placement_transform(
    model: &Model,
    id: EntityId,
    entity: &Entity,
) -> GeometryResult<Transform> {
    match entity.type_name.as_ref() {
        "IFCAXIS2PLACEMENT3D" => Axis2Placement3D::new(id, entity).transform(model),
        "IFCAXIS2PLACEMENT2D" => Axis2Placement2D::new(id, entity).transform(model),
        other => Err(GeometryError::WrongEntityType {
            entity: id,
            actual: other.to_string(),
            expected: "IfcAxis2Placement2D or IfcAxis2Placement3D",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::Value;

    fn coords(values: &[f64]) -> Value {
        Value::List(values.iter().copied().map(Value::Real).collect())
    }

    fn model_with_frame() -> Model {
        let mut model = Model::new();
        model.insert(
            EntityId(1),
            Entity::new("IFCCARTESIANPOINT", vec![coords(&[1.0, 2.0, 3.0])]),
        );
        model.insert(
            EntityId(2),
            Entity::new("IFCDIRECTION", vec![coords(&[0.0, 0.0, 1.0])]),
        );
        model.insert(
            EntityId(3),
            Entity::new("IFCDIRECTION", vec![coords(&[1.0, 0.0, 0.0])]),
        );
        model
    }

    fn close(a: [f64; 3], b: [f64; 3]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9)
    }

    /// `Location` is inherited from `IfcPlacement`, so it must be read at
    /// slot 0 even though the subtype declares `Axis` first in its own list.
    #[test]
    fn inherited_location_is_slot_zero_not_the_subtypes_first_attribute() {
        let model = model_with_frame();
        let e = Entity::new(
            "IFCAXIS2PLACEMENT3D",
            vec![
                Value::Ref(EntityId(1)),
                Value::Ref(EntityId(2)),
                Value::Ref(EntityId(3)),
            ],
        );
        let p = Axis2Placement3D::new(EntityId(10), &e);
        assert_eq!(p.location(&model).unwrap(), [1.0, 2.0, 3.0]);
        assert_eq!(p.axis(&model).unwrap(), Some([0.0, 0.0, 1.0]));
    }

    /// Both are optional; absent must mean global Z and X, not an error.
    #[test]
    fn absent_axis_and_ref_direction_default_to_global_z_and_x() {
        let model = model_with_frame();
        let e = Entity::new(
            "IFCAXIS2PLACEMENT3D",
            vec![Value::Ref(EntityId(1)), Value::Null, Value::Null],
        );
        let p = Axis2Placement3D::new(EntityId(10), &e);
        assert_eq!(p.axis(&model).unwrap(), None);
        assert_eq!(p.ref_direction(&model).unwrap(), None);

        let t = p.transform(&model).unwrap();
        assert!(close(t.basis[0], [1.0, 0.0, 0.0]));
        assert!(close(t.basis[1], [0.0, 1.0, 0.0]));
        assert!(close(t.basis[2], [0.0, 0.0, 1.0]));
        assert_eq!(t.origin, [1.0, 2.0, 3.0]);
    }

    /// A short record (trailing optionals simply omitted) is common and must
    /// behave exactly like explicit `$`.
    #[test]
    fn a_record_missing_its_trailing_optionals_still_places() {
        let model = model_with_frame();
        let e = Entity::new("IFCAXIS2PLACEMENT3D", vec![Value::Ref(EntityId(1))]);
        let t = Axis2Placement3D::new(EntityId(10), &e)
            .transform(&model)
            .unwrap();
        assert_eq!(t.origin, [1.0, 2.0, 3.0]);
        assert!(close(t.basis[2], [0.0, 0.0, 1.0]));
    }

    /// The spec derives X by projecting RefDirection onto the plane normal to
    /// Axis. Skipping that yields a sheared basis that looks almost right.
    #[test]
    fn non_perpendicular_ref_direction_is_projected_into_an_orthonormal_basis() {
        let mut model = model_with_frame();
        // RefDirection tilted 45 degrees out of the XY plane.
        model.insert(
            EntityId(4),
            Entity::new("IFCDIRECTION", vec![coords(&[1.0, 0.0, 1.0])]),
        );
        let e = Entity::new(
            "IFCAXIS2PLACEMENT3D",
            vec![
                Value::Ref(EntityId(1)),
                Value::Ref(EntityId(2)),
                Value::Ref(EntityId(4)),
            ],
        );
        let t = Axis2Placement3D::new(EntityId(10), &e)
            .transform(&model)
            .unwrap();
        assert!(close(t.basis[0], [1.0, 0.0, 0.0]), "got {:?}", t.basis[0]);
        assert!(
            t.basis[0]
                .iter()
                .zip(t.basis[2])
                .map(|(a, b)| a * b)
                .sum::<f64>()
                .abs()
                < 1e-12,
            "X must end up perpendicular to Z"
        );
    }

    /// Parallel axes define no unique frame; the result would be NaN.
    #[test]
    fn axis_parallel_to_ref_direction_is_degenerate() {
        let model = model_with_frame();
        let e = Entity::new(
            "IFCAXIS2PLACEMENT3D",
            vec![
                Value::Ref(EntityId(1)),
                Value::Ref(EntityId(2)),
                Value::Ref(EntityId(2)),
            ],
        );
        let err = Axis2Placement3D::new(EntityId(10), &e)
            .transform(&model)
            .unwrap_err();
        assert!(matches!(err, GeometryError::Degenerate { .. }), "{err}");
    }

    /// A zero-length Axis must fail before it can produce NaN components.
    #[test]
    fn zero_length_axis_is_degenerate_rather_than_nan() {
        let mut model = model_with_frame();
        model.insert(
            EntityId(5),
            Entity::new("IFCDIRECTION", vec![coords(&[0.0, 0.0, 0.0])]),
        );
        let e = Entity::new(
            "IFCAXIS2PLACEMENT3D",
            vec![
                Value::Ref(EntityId(1)),
                Value::Ref(EntityId(5)),
                Value::Null,
            ],
        );
        let err = Axis2Placement3D::new(EntityId(10), &e)
            .transform(&model)
            .unwrap_err();
        assert!(matches!(err, GeometryError::Degenerate { .. }), "{err}");
    }

    /// The schema pairs Axis and RefDirection, but files give one alone; the
    /// spec default for the other is well defined, so tolerate it.
    #[test]
    fn only_one_of_the_paired_axes_still_yields_a_frame() {
        let model = model_with_frame();
        let e = Entity::new(
            "IFCAXIS2PLACEMENT3D",
            vec![
                Value::Ref(EntityId(1)),
                Value::Null,
                Value::Ref(EntityId(3)),
            ],
        );
        let t = Axis2Placement3D::new(EntityId(10), &e)
            .transform(&model)
            .unwrap();
        assert!(close(t.basis[0], [1.0, 0.0, 0.0]));
        assert!(close(t.basis[2], [0.0, 0.0, 1.0]));
    }

    /// 2D placements keep `RefDirection` at slot 1, after inherited Location.
    #[test]
    fn two_d_placement_derives_y_as_a_quarter_turn_from_x() {
        let mut model = Model::new();
        model.insert(
            EntityId(1),
            Entity::new("IFCCARTESIANPOINT", vec![coords(&[4.0, 5.0])]),
        );
        model.insert(
            EntityId(2),
            Entity::new("IFCDIRECTION", vec![coords(&[0.0, 1.0])]),
        );
        let e = Entity::new(
            "IFCAXIS2PLACEMENT2D",
            vec![Value::Ref(EntityId(1)), Value::Ref(EntityId(2))],
        );
        let t = Axis2Placement2D::new(EntityId(10), &e)
            .transform(&model)
            .unwrap();
        assert_eq!(t.origin, [4.0, 5.0, 0.0], "2D location pads z with 0");
        assert!(close(t.basis[0], [0.0, 1.0, 0.0]));
        assert!(
            close(t.basis[1], [-1.0, 0.0, 0.0]),
            "Y must be X rotated a quarter turn counter-clockwise, got {:?}",
            t.basis[1]
        );
    }

    #[test]
    fn two_d_placement_without_ref_direction_defaults_to_global_x() {
        let mut model = Model::new();
        model.insert(
            EntityId(1),
            Entity::new("IFCCARTESIANPOINT", vec![coords(&[0.0, 0.0])]),
        );
        let e = Entity::new("IFCAXIS2PLACEMENT2D", vec![Value::Ref(EntityId(1))]);
        let p = Axis2Placement2D::new(EntityId(10), &e);
        assert_eq!(p.ref_direction(&model).unwrap(), [1.0, 0.0, 0.0]);
        assert!(p.transform(&model).unwrap().is_identity(1e-12));
    }

    /// `IfcAxis1Placement` has only one axis, so it defines no full frame and
    /// exposes no transform; the axis default is global Z.
    #[test]
    fn axis1_placement_defaults_its_axis_to_global_z() {
        let model = model_with_frame();
        let e = Entity::new("IFCAXIS1PLACEMENT", vec![Value::Ref(EntityId(1))]);
        let p = Axis1Placement::new(EntityId(10), &e);
        assert_eq!(p.axis(&model).unwrap(), [0.0, 0.0, 1.0]);
        assert_eq!(p.location(&model).unwrap(), [1.0, 2.0, 3.0]);
    }

    #[test]
    fn axis1_placement_reads_its_axis_after_the_inherited_location() {
        let model = model_with_frame();
        let e = Entity::new(
            "IFCAXIS1PLACEMENT",
            vec![Value::Ref(EntityId(1)), Value::Ref(EntityId(3))],
        );
        assert_eq!(
            Axis1Placement::new(EntityId(10), &e).axis(&model).unwrap(),
            [1.0, 0.0, 0.0]
        );
    }

    #[test]
    fn a_missing_location_names_the_entity_and_attribute() {
        let model = Model::new();
        let e = Entity::new("IFCAXIS2PLACEMENT3D", vec![]);
        let err = Axis2Placement3D::new(EntityId(77), &e)
            .location(&model)
            .unwrap_err();
        assert!(err.to_string().contains("#77"), "got: {err}");
        assert!(err.to_string().contains("Location"), "got: {err}");
    }

    #[test]
    fn a_dangling_location_reference_names_the_placement_as_referrer() {
        let model = Model::new();
        let e = Entity::new("IFCAXIS2PLACEMENT3D", vec![Value::Ref(EntityId(99))]);
        let err = Axis2Placement3D::new(EntityId(7), &e)
            .location(&model)
            .unwrap_err();
        assert_eq!(err.entity(), Some(EntityId(7)));
    }
}
