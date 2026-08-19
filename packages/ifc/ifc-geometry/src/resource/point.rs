//! `IfcPoint` subtypes and the IFC4 coordinate lists.
//!
//! # The dimension trap
//!
//! `IfcCartesianPoint.Coordinates` is `LIST [1:3]`, so a point in a real file
//! is 2D *or* 3D and nothing in the record says which except its length. A 2D
//! point silently read as `[x, y, 0.0]` is indistinguishable from a 3D point
//! that happens to sit on the z=0 plane, and the difference matters: a 2D
//! profile curve read as 3D geometry will be swept in the wrong space.
//!
//! So [`CartesianPoint::dimension`] is public and padding to 3D is an explicit
//! call ([`CartesianPoint::coordinates_3d`]), never something that happens
//! behind the caller's back.
//!
//! # Point lists
//!
//! IFC4 added `IfcCartesianPointList2D`/`3D` so tessellated geometry does not
//! need one entity per vertex; a 200k-triangle mesh would otherwise be 100k+
//! `IfcCartesianPoint` records. Everything that indexes into these lists
//! (`IfcIndexedPolyCurve`, `IfcTriangulatedFaceSet`) uses **1-based** indices,
//! which is why [`CartesianPointList2D::point`] takes one.

use crate::error::{GeometryError, GeometryResult};
use crate::slots::Slots;
use ifc_model::{Entity, EntityId, Model, Value};

/// Attribute slots, absolute STEP positions including inherited attributes.
mod slot {
    /// `IfcCartesianPoint` (no inherited explicit attributes).
    pub mod cartesian_point {
        /// `Coordinates : LIST [1:3] OF IfcLengthMeasure`.
        pub const COORDINATES: usize = 0;
    }

    /// `IfcPointOnCurve` (supertype `IfcPoint` declares nothing explicit).
    pub mod point_on_curve {
        /// `BasisCurve : IfcCurve`.
        pub const BASIS_CURVE: usize = 0;
        /// `PointParameter : IfcParameterValue`.
        pub const POINT_PARAMETER: usize = 1;
    }

    /// `IfcPointOnSurface`.
    pub mod point_on_surface {
        /// `BasisSurface : IfcSurface`.
        pub const BASIS_SURFACE: usize = 0;
        /// `PointParameterU : IfcParameterValue`.
        pub const POINT_PARAMETER_U: usize = 1;
        /// `PointParameterV : IfcParameterValue`.
        pub const POINT_PARAMETER_V: usize = 2;
    }

    /// `IfcCartesianPointList2D` / `3D`. The supertype `IfcCartesianPointList`
    /// declares only a DERIVE attribute, so `CoordList` is slot 0 in both.
    pub mod point_list {
        /// `CoordList : LIST [1:?] OF LIST [n:n] OF IfcLengthMeasure`.
        pub const COORD_LIST: usize = 0;
    }
}

/// A borrowed view of an `IfcCartesianPoint`.
#[derive(Debug, Clone, Copy)]
pub struct CartesianPoint<'m> {
    slots: Slots<'m>,
}

impl<'m> CartesianPoint<'m> {
    /// Wrap an entity assumed to be an `IfcCartesianPoint`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The raw `Coordinates` list, exactly as long as the file wrote it.
    pub fn coordinates(&self) -> GeometryResult<Vec<f64>> {
        self.slots
            .req_f64_list(slot::cartesian_point::COORDINATES, "Coordinates")
    }

    /// How many coordinates the point actually carries: 2 or 3.
    ///
    /// A length outside that range is [`crate::GeometryError::Degenerate`]: the
    /// schema's `CP2Dor3D` rule requires at least 2, and a 1- or 4-element
    /// list has no defined meaning.
    pub fn dimension(&self) -> GeometryResult<usize> {
        let n = self.coordinates()?.len();
        match n {
            2 | 3 => Ok(n),
            other => Err(self
                .slots
                .degenerate(format!("Coordinates has {other} entries, expected 2 or 3"))),
        }
    }

    /// The coordinates promoted to 3D, padding a 2D point with `z = 0`.
    ///
    /// Deliberately a separate call from [`Self::coordinates`]: padding is a
    /// decision about the point's meaning, so the caller makes it. Check
    /// [`Self::dimension`] first when the distinction matters.
    pub fn coordinates_3d(&self) -> GeometryResult<[f64; 3]> {
        let c = self.coordinates()?;
        match c.len() {
            2 => Ok([c[0], c[1], 0.0]),
            3 => Ok([c[0], c[1], c[2]]),
            other => Err(self
                .slots
                .degenerate(format!("Coordinates has {other} entries, expected 2 or 3"))),
        }
    }
}

/// A borrowed view of an `IfcPointOnCurve`.
///
/// The point is a parameter on a curve, not stored coordinates, so evaluating
/// it means evaluating the basis curve. This view resolves the reference and
/// the parameter; the evaluation belongs to the curve module.
#[derive(Debug, Clone, Copy)]
pub struct PointOnCurve<'m> {
    slots: Slots<'m>,
}

impl<'m> PointOnCurve<'m> {
    /// Wrap an entity assumed to be an `IfcPointOnCurve`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcCurve` this point is parameterized on.
    pub fn basis_curve(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(slot::point_on_curve::BASIS_CURVE, "BasisCurve")
    }

    /// The curve parameter.
    ///
    /// In the curve's own parameter space, which for a trimmed or reparameter-
    /// ized curve is not arc length and not normalized to `0..1`.
    pub fn point_parameter(&self) -> GeometryResult<f64> {
        self.slots
            .req_f64(slot::point_on_curve::POINT_PARAMETER, "PointParameter")
    }
}

/// A borrowed view of an `IfcPointOnSurface`.
#[derive(Debug, Clone, Copy)]
pub struct PointOnSurface<'m> {
    slots: Slots<'m>,
}

impl<'m> PointOnSurface<'m> {
    /// Wrap an entity assumed to be an `IfcPointOnSurface`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcSurface` this point lies on.
    pub fn basis_surface(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(slot::point_on_surface::BASIS_SURFACE, "BasisSurface")
    }

    /// The `(u, v)` parameters, in the surface's own parameter space.
    pub fn parameters(&self) -> GeometryResult<(f64, f64)> {
        let u = self
            .slots
            .req_f64(slot::point_on_surface::POINT_PARAMETER_U, "PointParameterU")?;
        let v = self
            .slots
            .req_f64(slot::point_on_surface::POINT_PARAMETER_V, "PointParameterV")?;
        Ok((u, v))
    }
}

/// A borrowed view of an `IfcCartesianPointList2D`.
#[derive(Debug, Clone, Copy)]
pub struct CartesianPointList2D<'m> {
    slots: Slots<'m>,
}

impl<'m> CartesianPointList2D<'m> {
    /// Wrap an entity assumed to be an `IfcCartesianPointList2D`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// Every coordinate pair, in file order.
    ///
    /// A row of the wrong width fails rather than being padded or truncated:
    /// the width is what distinguishes this entity from its 3D sibling.
    pub fn coordinates(&self) -> GeometryResult<Vec<[f64; 2]>> {
        rows::<2>(&self.slots, slot::point_list::COORD_LIST, "CoordList")
    }

    /// The point at a **1-based** index, as IFC index attributes write them.
    ///
    /// Returns `None` for 0 or for an index past the end, which is what a
    /// malformed `IfcIndexedPolyCurve` produces and must not panic.
    pub fn point(&self, one_based: usize) -> GeometryResult<Option<[f64; 2]>> {
        let coords = self.coordinates()?;
        Ok(one_based
            .checked_sub(1)
            .and_then(|i| coords.get(i).copied()))
    }
}

/// A borrowed view of an `IfcCartesianPointList3D`.
#[derive(Debug, Clone, Copy)]
pub struct CartesianPointList3D<'m> {
    slots: Slots<'m>,
}

impl<'m> CartesianPointList3D<'m> {
    /// Wrap an entity assumed to be an `IfcCartesianPointList3D`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// Every coordinate triple, in file order.
    pub fn coordinates(&self) -> GeometryResult<Vec<[f64; 3]>> {
        rows::<3>(&self.slots, slot::point_list::COORD_LIST, "CoordList")
    }

    /// The point at a **1-based** index, as IFC index attributes write them.
    pub fn point(&self, one_based: usize) -> GeometryResult<Option<[f64; 3]>> {
        let coords = self.coordinates()?;
        Ok(one_based
            .checked_sub(1)
            .and_then(|i| coords.get(i).copied()))
    }
}

/// Resolve a reference that must be an `IfcCartesianPoint`, promoted to 3D.
///
/// Placements, operators and curves all need exactly this, and each of them
/// getting the dangling-reference and wrong-type errors right independently is
/// how those errors end up inconsistent.
pub fn cartesian_point_3d(
    model: &Model,
    referrer: EntityId,
    id: EntityId,
) -> GeometryResult<[f64; 3]> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer,
        missing: id,
    })?;
    if !entity.is_type("IFCCARTESIANPOINT") {
        return Err(GeometryError::WrongEntityType {
            entity: id,
            actual: entity.type_name.to_string(),
            expected: "IfcCartesianPoint",
        });
    }
    CartesianPoint::new(id, entity).coordinates_3d()
}

/// Read a `LIST OF LIST OF REAL` where every row has exactly `N` entries.
fn rows<const N: usize>(
    slots: &Slots<'_>,
    index: usize,
    name: &'static str,
) -> GeometryResult<Vec<[f64; N]>> {
    let value = slots.req(index, name)?;
    let outer = value
        .as_list()
        .ok_or_else(|| wrong_kind(slots, name, "a list of coordinate rows", value))?;

    let mut out = Vec::with_capacity(outer.len());
    for row in outer {
        let items = row
            .as_list()
            .ok_or_else(|| wrong_kind(slots, name, "a list of coordinate rows", row))?;
        if items.len() != N {
            return Err(slots.degenerate(format!(
                "{name} row has {} entries, expected {N}",
                items.len()
            )));
        }
        let mut coords = [0.0; N];
        for (dst, src) in coords.iter_mut().zip(items) {
            *dst = src
                .unwrap_typed()
                .as_f64()
                .ok_or_else(|| wrong_kind(slots, name, "numeric coordinates", src))?;
        }
        out.push(coords);
    }
    Ok(out)
}

/// Build a `WrongValueKind` error for a nested aggregate.
///
/// `Slots` keeps its own equivalent private, and duplicating the message shape
/// here would let the two drift; this stays a one-liner over the public enum.
fn wrong_kind(
    slots: &Slots<'_>,
    attribute: &'static str,
    expected: &'static str,
    found: &Value,
) -> GeometryError {
    GeometryError::WrongValueKind {
        entity: slots.id(),
        type_name: slots.type_name().to_string(),
        attribute,
        expected,
        found: format!("{found:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reals(values: &[f64]) -> Value {
        Value::List(values.iter().copied().map(Value::Real).collect())
    }

    fn point_entity(values: &[f64]) -> Entity {
        Entity::new("IFCCARTESIANPOINT", vec![reals(values)])
    }

    #[test]
    fn two_dimensional_points_are_not_silently_promoted_to_3d() {
        let e = point_entity(&[1.0, 2.0]);
        let p = CartesianPoint::new(EntityId(1), &e);
        assert_eq!(p.dimension().unwrap(), 2, "the file wrote two coordinates");
        assert_eq!(p.coordinates().unwrap(), vec![1.0, 2.0]);
        // Padding happens only when the caller asks for it.
        assert_eq!(p.coordinates_3d().unwrap(), [1.0, 2.0, 0.0]);
    }

    #[test]
    fn three_dimensional_points_keep_their_z() {
        let e = point_entity(&[1.0, 2.0, 3.0]);
        let p = CartesianPoint::new(EntityId(1), &e);
        assert_eq!(p.dimension().unwrap(), 3);
        assert_eq!(p.coordinates_3d().unwrap(), [1.0, 2.0, 3.0]);
    }

    /// `LIST [1:3]` permits one entry syntactically; the `CP2Dor3D` rule does
    /// not, and a one-coordinate point has no geometric meaning.
    #[test]
    fn a_single_coordinate_is_degenerate_rather_than_zero_padded() {
        let e = point_entity(&[1.0]);
        let p = CartesianPoint::new(EntityId(9), &e);
        assert!(p.dimension().is_err());
        assert!(p.coordinates_3d().is_err());
    }

    #[test]
    fn a_missing_coordinate_list_names_the_entity() {
        let e = Entity::new("IFCCARTESIANPOINT", vec![]);
        let err = CartesianPoint::new(EntityId(7), &e)
            .coordinates()
            .unwrap_err();
        assert!(err.to_string().contains("#7"), "got: {err}");
        assert!(err.to_string().contains("Coordinates"), "got: {err}");
    }

    #[test]
    fn typed_length_measures_do_not_hide_the_number() {
        let e = Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![
                Value::Typed {
                    type_name: "IFCLENGTHMEASURE".into(),
                    value: Box::new(Value::Real(4.0)),
                },
                Value::Integer(0),
            ])],
        );
        let p = CartesianPoint::new(EntityId(1), &e);
        assert_eq!(p.coordinates_3d().unwrap(), [4.0, 0.0, 0.0]);
    }

    #[test]
    fn point_on_curve_exposes_its_basis_and_parameter() {
        let e = Entity::new(
            "IFCPOINTONCURVE",
            vec![
                Value::Ref(EntityId(5)),
                Value::Typed {
                    type_name: "IFCPARAMETERVALUE".into(),
                    value: Box::new(Value::Real(0.25)),
                },
            ],
        );
        let p = PointOnCurve::new(EntityId(1), &e);
        assert_eq!(p.basis_curve().unwrap(), EntityId(5));
        assert_eq!(p.point_parameter().unwrap(), 0.25);
    }

    #[test]
    fn point_on_surface_exposes_both_parameters_in_order() {
        let e = Entity::new(
            "IFCPOINTONSURFACE",
            vec![
                Value::Ref(EntityId(5)),
                Value::Real(0.25),
                Value::Real(0.75),
            ],
        );
        let p = PointOnSurface::new(EntityId(1), &e);
        assert_eq!(p.basis_surface().unwrap(), EntityId(5));
        assert_eq!(p.parameters().unwrap(), (0.25, 0.75));
    }

    #[test]
    fn point_lists_read_every_row_in_file_order() {
        let e = Entity::new(
            "IFCCARTESIANPOINTLIST3D",
            vec![Value::List(vec![
                reals(&[0.0, 0.0, 0.0]),
                reals(&[1.0, 0.0, 0.0]),
                reals(&[1.0, 1.0, 0.0]),
            ])],
        );
        let list = CartesianPointList3D::new(EntityId(1), &e);
        let coords = list.coordinates().unwrap();
        assert_eq!(coords.len(), 3);
        assert_eq!(coords[2], [1.0, 1.0, 0.0]);
    }

    /// IFC index attributes are 1-based; treating them as 0-based shifts every
    /// triangle by one vertex, which renders as plausible-looking garbage.
    #[test]
    fn point_list_indices_are_one_based_and_zero_is_out_of_range() {
        let e = Entity::new(
            "IFCCARTESIANPOINTLIST2D",
            vec![Value::List(vec![reals(&[7.0, 8.0]), reals(&[9.0, 10.0])])],
        );
        let list = CartesianPointList2D::new(EntityId(1), &e);
        assert_eq!(list.point(1).unwrap(), Some([7.0, 8.0]));
        assert_eq!(list.point(2).unwrap(), Some([9.0, 10.0]));
        assert_eq!(list.point(0).unwrap(), None, "there is no index 0 in IFC");
        assert_eq!(list.point(3).unwrap(), None);
    }

    /// A 3D row inside a 2D list is a real exporter bug; truncating it would
    /// silently drop the z of every vertex.
    #[test]
    fn a_row_of_the_wrong_width_fails_instead_of_being_truncated() {
        let e = Entity::new(
            "IFCCARTESIANPOINTLIST2D",
            vec![Value::List(vec![reals(&[1.0, 2.0, 3.0])])],
        );
        let err = CartesianPointList2D::new(EntityId(4), &e)
            .coordinates()
            .unwrap_err();
        assert!(err.to_string().contains("#4"), "got: {err}");
    }

    #[test]
    fn resolving_a_point_reference_rejects_the_wrong_entity_type() {
        let mut model = Model::new();
        model.insert(EntityId(1), Entity::new("IFCDIRECTION", vec![]));
        let err = cartesian_point_3d(&model, EntityId(2), EntityId(1)).unwrap_err();
        assert!(matches!(
            err,
            GeometryError::WrongEntityType {
                expected: "IfcCartesianPoint",
                ..
            }
        ));
    }

    #[test]
    fn resolving_a_dangling_point_reference_names_the_referrer() {
        let model = Model::new();
        let err = cartesian_point_3d(&model, EntityId(2), EntityId(99)).unwrap_err();
        assert_eq!(err.entity(), Some(EntityId(2)));
    }

    #[test]
    fn resolving_a_point_reference_yields_its_coordinates() {
        let mut model = Model::new();
        model.insert(EntityId(1), point_entity(&[1.0, 2.0, 3.0]));
        assert_eq!(
            cartesian_point_3d(&model, EntityId(2), EntityId(1)).unwrap(),
            [1.0, 2.0, 3.0]
        );
    }
}
