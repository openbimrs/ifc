//! `IfcDirection` and `IfcVector`: orientation, with and without magnitude.
//!
//! # DirectionRatios are ratios, not a unit vector
//!
//! The attribute is named `DirectionRatios` because only the *ratios* between
//! the components are meaningful: `(3, 4, 0)` and `(0.6, 0.8, 0)` denote the
//! same direction, and exporters write both. Nothing in the schema requires
//! normalization, so any consumer that treats the raw values as a unit vector
//! is wrong on real files: a 5x-long "unit" axis fed into a placement basis
//! scales the geometry it positions.
//!
//! Hence [`Direction::unit`] normalizes and [`Direction::ratios`] does not,
//! and the names say which is which.
//!
//! # Zero length is degenerate, not zero
//!
//! The schema's `MagnitudeGreaterZero` rule forbids an all-zero direction, but
//! files contain them. Normalizing one produces `NaN`, which then propagates
//! silently through every transform it touches until geometry disappears far
//! from the cause. [`Direction::unit`] returns
//! [`crate::GeometryError::Degenerate`] instead.
//!
//! # 2D or 3D
//!
//! `DirectionRatios` is `LIST [2:3]`, so a direction in a 2D context has two
//! components. As with points, promotion to 3D is explicit.

use crate::error::{GeometryError, GeometryResult};
use crate::slots::Slots;
use ifc_model::{Entity, EntityId, Model};

/// Below this length a direction carries no orientation information.
///
/// Matches the tolerance `crate::transform` normalizes with, so a direction
/// this module accepts is one `Transform::from_axes` can also use. Two
/// different thresholds would mean a direction that reads fine here still
/// fails there, with a less informative error.
const MIN_LENGTH: f64 = 1e-12;

/// Attribute slots, absolute STEP positions including inherited attributes.
mod slot {
    /// `IfcDirection` (supertype `IfcGeometricRepresentationItem` declares no
    /// explicit attributes, so this index is its own).
    pub mod direction {
        /// `DirectionRatios : LIST [2:3] OF IfcReal`.
        pub const DIRECTION_RATIOS: usize = 0;
    }

    /// `IfcVector`.
    pub mod vector {
        /// `Orientation : IfcDirection`.
        pub const ORIENTATION: usize = 0;
        /// `Magnitude : IfcLengthMeasure`.
        pub const MAGNITUDE: usize = 1;
    }
}

/// A borrowed view of an `IfcDirection`.
#[derive(Debug, Clone, Copy)]
pub struct Direction<'m> {
    slots: Slots<'m>,
}

impl<'m> Direction<'m> {
    /// Wrap an entity assumed to be an `IfcDirection`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The raw `DirectionRatios`, unnormalized and of the file's own length.
    pub fn ratios(&self) -> GeometryResult<Vec<f64>> {
        self.slots
            .req_f64_list(slot::direction::DIRECTION_RATIOS, "DirectionRatios")
    }

    /// How many ratios the direction carries: 2 or 3.
    pub fn dimension(&self) -> GeometryResult<usize> {
        let n = self.ratios()?.len();
        match n {
            2 | 3 => Ok(n),
            other => Err(self.slots.degenerate(format!(
                "DirectionRatios has {other} entries, expected 2 or 3"
            ))),
        }
    }

    /// The ratios promoted to 3D with `z = 0`, still unnormalized.
    ///
    /// A 2D direction's third component is genuinely zero (it lies in the
    /// plane), unlike a 2D point's z, so padding here is not a guess. It is
    /// still a separate call from [`Self::ratios`] so a caller that needs the
    /// dimension can ask for it.
    pub fn ratios_3d(&self) -> GeometryResult<[f64; 3]> {
        let r = self.ratios()?;
        match r.len() {
            2 => Ok([r[0], r[1], 0.0]),
            3 => Ok([r[0], r[1], r[2]]),
            other => Err(self.slots.degenerate(format!(
                "DirectionRatios has {other} entries, expected 2 or 3"
            ))),
        }
    }

    /// The direction as a normalized 3D vector.
    ///
    /// Fails with [`crate::GeometryError::Degenerate`] on a zero-length direction
    /// rather than returning `NaN` components, because a `NaN` here is found
    /// three subsystems later as missing geometry.
    pub fn unit(&self) -> GeometryResult<[f64; 3]> {
        let v = self.ratios_3d()?;
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if len < MIN_LENGTH {
            return Err(self
                .slots
                .degenerate("DirectionRatios are all zero, so there is no direction"));
        }
        Ok([v[0] / len, v[1] / len, v[2] / len])
    }
}

/// A borrowed view of an `IfcVector`: a direction plus a length.
///
/// IFC separates the two because `IfcDirection` deliberately has no magnitude.
/// A vector's `Magnitude` may legitimately be `0.0` (the schema only requires
/// `>= 0`), giving a zero vector with a well-defined orientation -- so a zero
/// magnitude is **not** an error here, unlike a zero direction.
#[derive(Debug, Clone, Copy)]
pub struct Vector<'m> {
    slots: Slots<'m>,
}

impl<'m> Vector<'m> {
    /// Wrap an entity assumed to be an `IfcVector`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcDirection` giving this vector's orientation.
    pub fn orientation_ref(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::vector::ORIENTATION, "Orientation")
    }

    /// The vector's length, in project length units.
    pub fn magnitude(&self) -> GeometryResult<f64> {
        self.slots.req_f64(slot::vector::MAGNITUDE, "Magnitude")
    }

    /// The vector resolved to components: unit orientation times magnitude.
    pub fn components(&self, model: &'m Model) -> GeometryResult<[f64; 3]> {
        let magnitude = self.magnitude()?;
        let unit = resolve_unit(model, self.id(), self.orientation_ref()?)?;
        Ok([
            unit[0] * magnitude,
            unit[1] * magnitude,
            unit[2] * magnitude,
        ])
    }
}

/// Resolve a reference that must be an `IfcDirection`, normalized to 3D.
///
/// Placements and transformation operators both take optional direction
/// references and both need the same dangling/wrong-type/degenerate handling.
pub fn resolve_unit(model: &Model, referrer: EntityId, id: EntityId) -> GeometryResult<[f64; 3]> {
    direction_view(model, referrer, id)?.unit()
}

/// Resolve a reference that must be an `IfcDirection`, keeping raw ratios.
///
/// Use when the caller needs the unnormalized values, e.g. to inspect
/// dimension before deciding what the direction means.
pub fn resolve_ratios_3d(
    model: &Model,
    referrer: EntityId,
    id: EntityId,
) -> GeometryResult<[f64; 3]> {
    direction_view(model, referrer, id)?.ratios_3d()
}

/// Resolve and type-check a direction reference.
fn direction_view<'m>(
    model: &'m Model,
    referrer: EntityId,
    id: EntityId,
) -> GeometryResult<Direction<'m>> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer,
        missing: id,
    })?;
    if !entity.is_type("IFCDIRECTION") {
        return Err(GeometryError::WrongEntityType {
            entity: id,
            actual: entity.type_name.to_string(),
            expected: "IfcDirection",
        });
    }
    Ok(Direction::new(id, entity))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::Value;

    fn direction_entity(values: &[f64]) -> Entity {
        Entity::new(
            "IFCDIRECTION",
            vec![Value::List(
                values.iter().copied().map(Value::Real).collect(),
            )],
        )
    }

    fn close(a: [f64; 3], b: [f64; 3]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-12)
    }

    /// The attribute is `DirectionRatios`, and exporters take that literally.
    #[test]
    fn direction_ratios_are_returned_unnormalized() {
        let e = direction_entity(&[3.0, 4.0, 0.0]);
        let d = Direction::new(EntityId(1), &e);
        assert_eq!(d.ratios().unwrap(), vec![3.0, 4.0, 0.0]);
        assert!(close(d.unit().unwrap(), [0.6, 0.8, 0.0]));
    }

    #[test]
    fn two_dimensional_directions_report_their_dimension() {
        let e = direction_entity(&[1.0, 0.0]);
        let d = Direction::new(EntityId(1), &e);
        assert_eq!(d.dimension().unwrap(), 2);
        assert_eq!(d.ratios_3d().unwrap(), [1.0, 0.0, 0.0]);
    }

    /// Normalizing `(0,0,0)` yields `NaN`, which then poisons every transform
    /// downstream and surfaces as geometry missing for no visible reason.
    #[test]
    fn zero_length_direction_is_degenerate_rather_than_nan() {
        let e = direction_entity(&[0.0, 0.0, 0.0]);
        let d = Direction::new(EntityId(3), &e);
        let err = d.unit().unwrap_err();
        assert!(matches!(err, GeometryError::Degenerate { .. }));
        assert!(err.to_string().contains("#3"), "got: {err}");
    }

    /// Not exactly zero, but below any meaningful length: same failure mode.
    #[test]
    fn near_zero_direction_is_degenerate_too() {
        let e = direction_entity(&[1e-20, 0.0, 0.0]);
        assert!(Direction::new(EntityId(1), &e).unit().is_err());
    }

    #[test]
    fn a_single_ratio_is_degenerate() {
        let e = direction_entity(&[1.0]);
        let d = Direction::new(EntityId(1), &e);
        assert!(d.dimension().is_err());
        assert!(d.ratios_3d().is_err());
    }

    #[test]
    fn missing_direction_ratios_names_the_entity_and_attribute() {
        let e = Entity::new("IFCDIRECTION", vec![]);
        let err = Direction::new(EntityId(8), &e).ratios().unwrap_err();
        assert!(err.to_string().contains("#8"), "got: {err}");
        assert!(err.to_string().contains("DirectionRatios"), "got: {err}");
    }

    #[test]
    fn vector_components_are_orientation_times_magnitude() {
        let mut model = Model::new();
        model.insert(EntityId(1), direction_entity(&[3.0, 4.0, 0.0]));
        let v = Entity::new(
            "IFCVECTOR",
            vec![
                Value::Ref(EntityId(1)),
                Value::Typed {
                    type_name: "IFCLENGTHMEASURE".into(),
                    value: Box::new(Value::Real(10.0)),
                },
            ],
        );
        let view = Vector::new(EntityId(2), &v);
        assert_eq!(view.magnitude().unwrap(), 10.0);
        assert!(close(view.components(&model).unwrap(), [6.0, 8.0, 0.0]));
    }

    /// `MagGreaterOrEqualZero` permits zero, so a zero-length vector with a
    /// valid orientation is legal IFC and must not be rejected.
    #[test]
    fn zero_magnitude_vector_is_legal_and_yields_the_zero_vector() {
        let mut model = Model::new();
        model.insert(EntityId(1), direction_entity(&[0.0, 0.0, 1.0]));
        let v = Entity::new("IFCVECTOR", vec![Value::Ref(EntityId(1)), Value::Real(0.0)]);
        assert_eq!(
            Vector::new(EntityId(2), &v).components(&model).unwrap(),
            [0.0, 0.0, 0.0]
        );
    }

    #[test]
    fn a_vector_pointing_at_a_non_direction_reports_the_wrong_type() {
        let mut model = Model::new();
        model.insert(EntityId(1), Entity::new("IFCCARTESIANPOINT", vec![]));
        let v = Entity::new("IFCVECTOR", vec![Value::Ref(EntityId(1)), Value::Real(1.0)]);
        let err = Vector::new(EntityId(2), &v).components(&model).unwrap_err();
        assert!(matches!(
            err,
            GeometryError::WrongEntityType {
                expected: "IfcDirection",
                ..
            }
        ));
    }

    #[test]
    fn a_dangling_direction_reference_names_the_referrer() {
        let model = Model::new();
        let err = resolve_unit(&model, EntityId(5), EntityId(99)).unwrap_err();
        assert_eq!(err.entity(), Some(EntityId(5)));
    }

    #[test]
    fn resolving_ratios_keeps_them_unnormalized() {
        let mut model = Model::new();
        model.insert(EntityId(1), direction_entity(&[0.0, 2.0]));
        assert_eq!(
            resolve_ratios_3d(&model, EntityId(2), EntityId(1)).unwrap(),
            [0.0, 2.0, 0.0]
        );
    }
}
