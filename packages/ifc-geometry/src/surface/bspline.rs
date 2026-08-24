//! `IfcBSplineSurface` and its knotted / rational refinements.
//!
//! # The same job as [`crate::curve::bspline`], one dimension up
//!
//! Read the parameters, check the invariants, evaluate nothing. What changes
//! in 2D is that every invariant now has a u form and a v form, and the
//! control points are a *grid* rather than a list. The grid is where files go
//! wrong: `ControlPointsList` is `LIST OF LIST OF IfcCartesianPoint`, and a
//! ragged inner list means the surface is not a tensor product at all.
//!
//! # Which index is which
//!
//! `ControlPointsList[i][j]` has `i` running along **u** and `j` along **v**.
//! So the outer list length is the u control point count and the inner length
//! the v count. Transposing them yields a surface that is plausible, is not
//! the one in the file, and passes every count check because the two knot
//! vectors are usually the same length in test data. This module's accessors
//! are named `u_*` and `v_*` for exactly that reason, and the row/column
//! convention is asserted in the tests.
//!
//! # Invariants checked here
//!
//! - Control point grid is rectangular (no ragged rows).
//! - `sum(UMultiplicities) = u control points + UDegree + 1`, and the v form.
//! - `UKnots` / `UMultiplicities` are parallel lists, and the v form.
//! - Weights form a grid of the same shape as the control points, all positive.

use crate::curve::bspline::{KnotType, KnotVector};
use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId, Value};

/// `IfcBSplineSurface` family attribute slots.
///
/// From IFC4 ADD2 TC1. Slots 0-6 come from `IfcBSplineSurface`, 7-11 from
/// `IfcBSplineSurfaceWithKnots`, and 12 from
/// `IfcRationalBSplineSurfaceWithKnots`.
mod slot {
    /// `UDegree`: `IfcInteger`.
    pub const U_DEGREE: usize = 0;
    /// `VDegree`: `IfcInteger`.
    pub const V_DEGREE: usize = 1;
    /// `ControlPointsList`: `LIST OF LIST OF IfcCartesianPoint`, u outer.
    pub const CONTROL_POINTS: usize = 2;
    /// `SurfaceForm`: `IfcBSplineSurfaceForm`.
    pub const SURFACE_FORM: usize = 3;
    /// `UClosed`: `IfcLogical`.
    pub const U_CLOSED: usize = 4;
    /// `VClosed`: `IfcLogical`.
    pub const V_CLOSED: usize = 5;
    /// `SelfIntersect`: `IfcLogical`.
    pub const SELF_INTERSECT: usize = 6;
    /// `UMultiplicities`, from `IfcBSplineSurfaceWithKnots`.
    pub const U_MULTIPLICITIES: usize = 7;
    /// `VMultiplicities`, from `IfcBSplineSurfaceWithKnots`.
    pub const V_MULTIPLICITIES: usize = 8;
    /// `UKnots`, from `IfcBSplineSurfaceWithKnots`.
    pub const U_KNOTS: usize = 9;
    /// `VKnots`, from `IfcBSplineSurfaceWithKnots`.
    pub const V_KNOTS: usize = 10;
    /// `KnotSpec`: `IfcKnotType`, from `IfcBSplineSurfaceWithKnots`.
    pub const KNOT_SPEC: usize = 11;
    /// `WeightsData`, from `IfcRationalBSplineSurfaceWithKnots`.
    pub const WEIGHTS_DATA: usize = 12;
}

/// `IfcBSplineSurfaceForm`: what shape the surface originally was.
///
/// Informational only, exactly like [`crate::curve::BSplineCurveForm`]. A
/// `CYLINDRICAL_SURF` form is not a licence to substitute an
/// `IfcCylindricalSurface`: the control points are what the file actually
/// means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BSplineSurfaceForm {
    /// Originally a plane.
    PlaneSurf,
    /// Originally a cylinder.
    CylindricalSurf,
    /// Originally a cone.
    ConicalSurf,
    /// Originally a sphere.
    SphericalSurf,
    /// Originally a torus.
    ToroidalSurf,
    /// Originally a surface of revolution.
    SurfOfRevolution,
    /// Originally a ruled surface.
    RuledSurf,
    /// Originally a generalised cone.
    GeneralisedCone,
    /// Originally a quadric.
    QuadricSurf,
    /// Originally a linear extrusion.
    SurfOfLinearExtrusion,
    /// No original form is claimed.
    Unspecified,
}

impl BSplineSurfaceForm {
    /// Parse the enumeration token, `None` if unrecognised.
    pub fn from_token(token: &str) -> Option<Self> {
        match token.to_ascii_uppercase().as_str() {
            "PLANE_SURF" => Some(Self::PlaneSurf),
            "CYLINDRICAL_SURF" => Some(Self::CylindricalSurf),
            "CONICAL_SURF" => Some(Self::ConicalSurf),
            "SPHERICAL_SURF" => Some(Self::SphericalSurf),
            "TOROIDAL_SURF" => Some(Self::ToroidalSurf),
            "SURF_OF_REVOLUTION" => Some(Self::SurfOfRevolution),
            "RULED_SURF" => Some(Self::RuledSurf),
            "GENERALISED_CONE" => Some(Self::GeneralisedCone),
            "QUADRIC_SURF" => Some(Self::QuadricSurf),
            "SURF_OF_LINEAR_EXTRUSION" => Some(Self::SurfOfLinearExtrusion),
            "UNSPECIFIED" => Some(Self::Unspecified),
            _ => None,
        }
    }
}

/// The control point grid, `[u][v]`.
///
/// A distinct type rather than a bare `Vec<Vec<EntityId>>` so the u/v
/// convention is stated once and the rectangularity check cannot be skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlPointGrid {
    rows: Vec<Vec<EntityId>>,
}

impl ControlPointGrid {
    /// Number of control points along u; the outer list length.
    pub fn u_count(&self) -> usize {
        self.rows.len()
    }

    /// Number of control points along v; the inner list length.
    pub fn v_count(&self) -> usize {
        self.rows.first().map_or(0, Vec::len)
    }

    /// The control point at `(u_index, v_index)`.
    pub fn get(&self, u_index: usize, v_index: usize) -> Option<EntityId> {
        self.rows.get(u_index)?.get(v_index).copied()
    }

    /// The rows, each a constant-u run of control points.
    pub fn rows(&self) -> &[Vec<EntityId>] {
        &self.rows
    }
}

/// A borrowed view of any `IfcBSplineSurface` subtype.
#[derive(Debug, Clone, Copy)]
pub struct BSplineSurface<'m> {
    slots: Slots<'m>,
}

impl<'m> BSplineSurface<'m> {
    /// Wrap an entity known to be an `IfcBSplineSurface` subtype.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The u polynomial degree, guaranteed at least 1.
    pub fn u_degree(&self) -> GeometryResult<usize> {
        self.degree(slot::U_DEGREE, "UDegree")
    }

    /// The v polynomial degree, guaranteed at least 1.
    pub fn v_degree(&self) -> GeometryResult<usize> {
        self.degree(slot::V_DEGREE, "VDegree")
    }

    /// The control point grid, checked to be rectangular.
    ///
    /// A ragged grid is not a tensor-product surface, so it cannot be
    /// evaluated at all; catching it here beats an out-of-bounds read in a
    /// kernel's inner loop.
    // TODO: `resource::point` will provide a typed point view to resolve these.
    pub fn control_points(&self) -> GeometryResult<ControlPointGrid> {
        let value = self.slots.req(slot::CONTROL_POINTS, "ControlPointsList")?;
        let outer = value.as_list().ok_or_else(|| {
            self.slots
                .degenerate("ControlPointsList must be a list of lists")
        })?;
        if outer.len() < 2 {
            return Err(self.slots.degenerate(format!(
                "ControlPointsList needs at least 2 rows along u, found {}",
                outer.len()
            )));
        }

        let mut rows: Vec<Vec<EntityId>> = Vec::with_capacity(outer.len());
        for (u_index, row_value) in outer.iter().enumerate() {
            let inner = row_value.as_list().ok_or_else(|| {
                self.slots
                    .degenerate(format!("ControlPointsList row {u_index} is not a list"))
            })?;
            let mut row = Vec::with_capacity(inner.len());
            for (v_index, point) in inner.iter().enumerate() {
                let id = point.as_ref_id().ok_or_else(|| {
                    self.slots.degenerate(format!(
                        "ControlPointsList[{u_index}][{v_index}] is not an entity reference"
                    ))
                })?;
                row.push(id);
            }
            rows.push(row);
        }

        let v_count = rows[0].len();
        if v_count < 2 {
            return Err(self.slots.degenerate(format!(
                "ControlPointsList needs at least 2 columns along v, found {v_count}"
            )));
        }
        for (u_index, row) in rows.iter().enumerate() {
            if row.len() != v_count {
                return Err(self.slots.degenerate(format!(
                    "ControlPointsList row {u_index} has {} points but row 0 has {v_count}; \
                     the grid must be rectangular",
                    row.len()
                )));
            }
        }
        Ok(ControlPointGrid { rows })
    }

    /// The declared original form, defaulting to `Unspecified`.
    pub fn surface_form(&self) -> BSplineSurfaceForm {
        self.slots
            .opt_enum(slot::SURFACE_FORM)
            .and_then(BSplineSurfaceForm::from_token)
            .unwrap_or(BSplineSurfaceForm::Unspecified)
    }

    /// The asserted `UClosed` flag; `None` for `.U.` or absent.
    pub fn u_closed(&self) -> Option<bool> {
        self.slots.opt_bool(slot::U_CLOSED)
    }

    /// The asserted `VClosed` flag; `None` for `.U.` or absent.
    pub fn v_closed(&self) -> Option<bool> {
        self.slots.opt_bool(slot::V_CLOSED)
    }

    /// The asserted `SelfIntersect` flag; `None` for `.U.` or absent.
    pub fn self_intersect(&self) -> Option<bool> {
        self.slots.opt_bool(slot::SELF_INTERSECT)
    }

    /// The declared knot type, defaulting to `Unspecified`.
    pub fn knot_spec(&self) -> KnotType {
        self.slots
            .opt_enum(slot::KNOT_SPEC)
            .and_then(KnotType::from_token)
            .unwrap_or(KnotType::Unspecified)
    }

    /// Is this the knotted subtype?
    pub fn has_knots(&self) -> bool {
        self.slots.opt(slot::U_KNOTS).is_some()
    }

    /// Is this the rational subtype?
    pub fn is_rational(&self) -> bool {
        self.slots.opt(slot::WEIGHTS_DATA).is_some()
    }

    /// The u knot vector, checked against the u control point count.
    ///
    /// `Ok(None)` for a surface without knots.
    pub fn u_knots(&self) -> GeometryResult<Option<KnotVector>> {
        if !self.has_knots() {
            return Ok(None);
        }
        let expected = self.control_points()?.u_count() + self.u_degree()? + 1;
        self.knot_vector(
            slot::U_KNOTS,
            "UKnots",
            slot::U_MULTIPLICITIES,
            "UMultiplicities",
            expected,
        )
        .map(Some)
    }

    /// The v knot vector, checked against the v control point count.
    ///
    /// `Ok(None)` for a surface without knots.
    pub fn v_knots(&self) -> GeometryResult<Option<KnotVector>> {
        if !self.has_knots() {
            return Ok(None);
        }
        let expected = self.control_points()?.v_count() + self.v_degree()? + 1;
        self.knot_vector(
            slot::V_KNOTS,
            "VKnots",
            slot::V_MULTIPLICITIES,
            "VMultiplicities",
            expected,
        )
        .map(Some)
    }

    /// The weight grid, matching the control point grid exactly.
    ///
    /// `Ok(None)` for a non-rational surface. Every weight must be positive:
    /// a zero divides by zero at that control point and a negative one flips
    /// the patch through infinity.
    pub fn weights(&self) -> GeometryResult<Option<Vec<Vec<f64>>>> {
        if !self.is_rational() {
            return Ok(None);
        }
        let grid = self.control_points()?;
        let value = self.slots.req(slot::WEIGHTS_DATA, "WeightsData")?;
        let outer = value
            .as_list()
            .ok_or_else(|| self.slots.degenerate("WeightsData must be a list of lists"))?;

        if outer.len() != grid.u_count() {
            return Err(self.slots.degenerate(format!(
                "WeightsData has {} rows but the control point grid has {}",
                outer.len(),
                grid.u_count()
            )));
        }

        let mut weights = Vec::with_capacity(outer.len());
        for (u_index, row_value) in outer.iter().enumerate() {
            let inner = row_value.as_list().ok_or_else(|| {
                self.slots
                    .degenerate(format!("WeightsData row {u_index} is not a list"))
            })?;
            if inner.len() != grid.v_count() {
                return Err(self.slots.degenerate(format!(
                    "WeightsData row {u_index} has {} entries but the grid has {}",
                    inner.len(),
                    grid.v_count()
                )));
            }
            let mut row = Vec::with_capacity(inner.len());
            for (v_index, w) in inner.iter().enumerate() {
                let weight = w.unwrap_typed().as_f64().ok_or_else(|| {
                    self.slots
                        .degenerate(format!("WeightsData[{u_index}][{v_index}] is not a number"))
                })?;
                // NaN must be rejected too, hence the explicit `is_nan`
                // rather than relying on `<= 0.0`, false for NaN.
                if weight.is_nan() || weight <= 0.0 {
                    return Err(self.slots.degenerate(format!(
                        "weight {weight} at control point [{u_index}][{v_index}] must be positive"
                    )));
                }
                row.push(weight);
            }
            weights.push(row);
        }
        Ok(Some(weights))
    }

    fn degree(&self, index: usize, name: &'static str) -> GeometryResult<usize> {
        let degree = self.slots.req_i64(index, name)?;
        if degree < 1 {
            return Err(self
                .slots
                .degenerate(format!("{name} must be at least 1, found {degree}")));
        }
        Ok(degree as usize)
    }

    /// Read one knot vector and check it against `expected` total multiplicity.
    fn knot_vector(
        &self,
        knots_index: usize,
        knots_name: &'static str,
        mult_index: usize,
        mult_name: &'static str,
        expected: usize,
    ) -> GeometryResult<KnotVector> {
        let values = self.slots.req_f64_list(knots_index, knots_name)?;
        let raw = self.slots.req(mult_index, mult_name)?;
        let items = raw
            .as_list()
            .ok_or_else(|| self.slots.degenerate(format!("{mult_name} must be a list")))?;

        let mut multiplicities = Vec::with_capacity(items.len());
        for item in items {
            match item.unwrap_typed() {
                Value::Integer(i) if *i >= 1 => multiplicities.push(*i as usize),
                other => {
                    return Err(self.slots.degenerate(format!(
                        "{mult_name} entry must be a positive integer, found {other:?}"
                    )));
                }
            }
        }

        if values.len() != multiplicities.len() {
            return Err(self.slots.degenerate(format!(
                "{knots_name} has {} entries but {mult_name} has {}; they are parallel lists",
                values.len(),
                multiplicities.len()
            )));
        }
        for pair in values.windows(2) {
            if pair[1] <= pair[0] {
                return Err(self.slots.degenerate(format!(
                    "{knots_name} must be strictly increasing; found {} after {}",
                    pair[1], pair[0]
                )));
            }
        }
        let total: usize = multiplicities.iter().sum();
        if total != expected {
            return Err(self.slots.degenerate(format!(
                "{mult_name} sums to {total} but must equal control points + degree + 1 = {expected}"
            )));
        }
        Ok(KnotVector {
            values,
            multiplicities,
        })
    }
}

/// Marker kept so the curve module's knot types are visibly reused.
///
/// The knot representation is identical in one and two dimensions, so
/// duplicating [`KnotVector`] here would create two types that must be kept in
/// sync for no benefit.
pub type SurfaceKnotVector = KnotVector;

#[cfg(test)]
mod tests {
    use super::*;

    /// A `u_count` by `v_count` grid whose ids encode their position as
    /// `u * 10 + v`, so a transposition is visible in the assertion.
    fn grid(u_count: usize, v_count: usize) -> Value {
        Value::List(
            (0..u_count)
                .map(|u| {
                    Value::List(
                        (0..v_count)
                            .map(|v| Value::Ref(EntityId((u * 10 + v) as u64)))
                            .collect(),
                    )
                })
                .collect(),
        )
    }

    fn integers(values: &[i64]) -> Value {
        Value::List(values.iter().map(|i| Value::Integer(*i)).collect())
    }

    fn reals(values: &[f64]) -> Value {
        Value::List(values.iter().map(|r| Value::Real(*r)).collect())
    }

    /// Degree 1 in both directions, 2x2 control points, clamped knots.
    fn bilinear() -> Entity {
        Entity::new(
            "IFCBSPLINESURFACEWITHKNOTS",
            vec![
                Value::Integer(1),
                Value::Integer(1),
                grid(2, 2),
                Value::Enum("UNSPECIFIED".into()),
                Value::Bool(false),
                Value::Bool(false),
                Value::Bool(false),
                integers(&[2, 2]),
                integers(&[2, 2]),
                reals(&[0.0, 1.0]),
                reals(&[0.0, 1.0]),
                Value::Enum("UNSPECIFIED".into()),
            ],
        )
    }

    #[test]
    fn inherited_surface_slots_precede_the_knot_and_weight_slots() {
        let e = bilinear();
        let view = BSplineSurface::new(EntityId(1), &e);
        assert_eq!(view.u_degree().unwrap(), 1);
        assert_eq!(view.v_degree().unwrap(), 1);
        assert!(view.has_knots());
        assert!(!view.is_rational());
        assert_eq!(view.knot_spec(), KnotType::Unspecified);
    }

    /// The outer list runs along u and the inner along v. A transposed read
    /// still typechecks and still passes count checks on a square grid, so it
    /// is pinned by position-encoded ids on a non-square grid.
    #[test]
    fn the_outer_control_point_list_runs_along_u_and_the_inner_along_v() {
        let e = Entity::new(
            "IFCBSPLINESURFACE",
            vec![
                Value::Integer(1),
                Value::Integer(1),
                grid(3, 5),
                Value::Enum("UNSPECIFIED".into()),
                Value::Bool(false),
                Value::Bool(false),
                Value::Bool(false),
            ],
        );
        let points = BSplineSurface::new(EntityId(1), &e)
            .control_points()
            .unwrap();
        assert_eq!(points.u_count(), 3, "outer list length is the u count");
        assert_eq!(points.v_count(), 5, "inner list length is the v count");
        // Id u*10 + v: (2, 4) must be 24, not 42.
        assert_eq!(points.get(2, 4), Some(EntityId(24)));
        assert_eq!(points.rows().len(), 3);
    }

    /// A ragged grid is not a tensor-product surface and cannot be evaluated.
    #[test]
    fn a_ragged_control_point_grid_is_rejected() {
        let mut e = bilinear();
        e.attributes[slot::CONTROL_POINTS] = Value::List(vec![
            Value::List(vec![Value::Ref(EntityId(1)), Value::Ref(EntityId(2))]),
            Value::List(vec![Value::Ref(EntityId(3))]),
        ]);
        let err = BSplineSurface::new(EntityId(7), &e)
            .control_points()
            .unwrap_err();
        assert!(err.to_string().contains("rectangular"), "got: {err}");
        assert!(err.to_string().contains("#7"), "got: {err}");
    }

    #[test]
    fn both_knot_vectors_are_checked_against_their_own_control_point_count() {
        let e = bilinear();
        let view = BSplineSurface::new(EntityId(1), &e);
        let u = view.u_knots().unwrap().unwrap();
        let v = view.v_knots().unwrap().unwrap();
        assert_eq!(u.expanded(), vec![0.0, 0.0, 1.0, 1.0]);
        assert_eq!(v.expanded(), vec![0.0, 0.0, 1.0, 1.0]);
        assert!(u.is_clamped(1));
    }

    /// The u and v checks must be independent: a wrong v multiplicity must not
    /// be masked by a correct u one.
    #[test]
    fn a_wrong_v_multiplicity_sum_is_caught_even_when_u_is_right() {
        let mut e = bilinear();
        e.attributes[slot::V_MULTIPLICITIES] = integers(&[2, 3]);
        let view = BSplineSurface::new(EntityId(1), &e);
        assert!(view.u_knots().is_ok(), "u is untouched and must still pass");
        let err = view.v_knots().unwrap_err();
        assert!(err.to_string().contains("VMultiplicities"), "got: {err}");
    }

    #[test]
    fn parallel_knot_lists_of_different_lengths_are_rejected() {
        let mut e = bilinear();
        e.attributes[slot::U_KNOTS] = reals(&[0.0, 0.5, 1.0]);
        let err = BSplineSurface::new(EntityId(1), &e).u_knots().unwrap_err();
        assert!(err.to_string().contains("parallel"), "got: {err}");
    }

    #[test]
    fn non_increasing_knot_values_are_rejected() {
        let mut e = bilinear();
        e.attributes[slot::U_KNOTS] = reals(&[1.0, 0.0]);
        let err = BSplineSurface::new(EntityId(1), &e).u_knots().unwrap_err();
        assert!(err.to_string().contains("increasing"), "got: {err}");
    }

    #[test]
    fn a_surface_without_knots_reports_none_rather_than_failing() {
        let e = Entity::new(
            "IFCBSPLINESURFACE",
            vec![
                Value::Integer(1),
                Value::Integer(1),
                grid(2, 2),
                Value::Enum("UNSPECIFIED".into()),
                Value::Bool(false),
                Value::Bool(false),
                Value::Bool(false),
            ],
        );
        let view = BSplineSurface::new(EntityId(1), &e);
        assert_eq!(view.u_knots().unwrap(), None);
        assert_eq!(view.v_knots().unwrap(), None);
        assert_eq!(view.weights().unwrap(), None);
    }

    #[test]
    fn rational_weights_form_a_grid_of_the_same_shape_as_the_control_points() {
        let mut attributes = bilinear().attributes;
        attributes.push(Value::List(vec![reals(&[1.0, 0.5]), reals(&[0.5, 1.0])]));
        let e = Entity::new("IFCRATIONALBSPLINESURFACEWITHKNOTS", attributes);
        let view = BSplineSurface::new(EntityId(1), &e);
        assert!(view.is_rational());
        assert_eq!(
            view.weights().unwrap().unwrap(),
            vec![vec![1.0, 0.5], vec![0.5, 1.0]]
        );
    }

    #[test]
    fn a_weight_grid_of_the_wrong_shape_is_rejected() {
        let mut attributes = bilinear().attributes;
        attributes.push(Value::List(vec![reals(&[1.0, 1.0])]));
        let e = Entity::new("IFCRATIONALBSPLINESURFACEWITHKNOTS", attributes);
        let err = BSplineSurface::new(EntityId(1), &e).weights().unwrap_err();
        assert!(err.to_string().contains("rows"), "got: {err}");
    }

    #[test]
    fn a_non_positive_weight_anywhere_in_the_grid_is_degenerate() {
        for bad in [0.0, -1.0] {
            let mut attributes = bilinear().attributes;
            attributes.push(Value::List(vec![reals(&[1.0, 1.0]), reals(&[1.0, bad])]));
            let e = Entity::new("IFCRATIONALBSPLINESURFACEWITHKNOTS", attributes);
            let err = BSplineSurface::new(EntityId(1), &e).weights().unwrap_err();
            assert!(err.to_string().contains("positive"), "weight {bad}: {err}");
            assert!(err.to_string().contains("[1][1]"), "weight {bad}: {err}");
        }
    }

    #[test]
    fn degree_zero_in_either_direction_is_rejected() {
        let mut e = bilinear();
        e.attributes[slot::U_DEGREE] = Value::Integer(0);
        assert!(BSplineSurface::new(EntityId(1), &e).u_degree().is_err());

        let mut e = bilinear();
        e.attributes[slot::V_DEGREE] = Value::Integer(0);
        assert!(BSplineSurface::new(EntityId(1), &e).v_degree().is_err());
    }

    /// The form is provenance, never a licence to swap in an analytic surface.
    #[test]
    fn surface_form_tokens_parse_without_replacing_the_control_points() {
        assert_eq!(
            BSplineSurfaceForm::from_token("SURF_OF_LINEAR_EXTRUSION"),
            Some(BSplineSurfaceForm::SurfOfLinearExtrusion)
        );
        assert_eq!(
            BSplineSurfaceForm::from_token("CYLINDRICAL_SURF"),
            Some(BSplineSurfaceForm::CylindricalSurf)
        );
        assert_eq!(BSplineSurfaceForm::from_token("BLOB"), None);
    }

    #[test]
    fn closure_flags_are_read_independently_for_u_and_v() {
        let mut e = bilinear();
        e.attributes[slot::U_CLOSED] = Value::Bool(true);
        e.attributes[slot::V_CLOSED] = Value::LogicalUnknown;
        let view = BSplineSurface::new(EntityId(1), &e);
        assert_eq!(view.u_closed(), Some(true));
        assert_eq!(view.v_closed(), None, ".U. must not become false");
    }
}
