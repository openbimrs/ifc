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
use crate::resource::point::CartesianPoint;
use crate::resource::resolve;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId, Model, Value};

/// `IfcBSplineSurface` family attribute slots.
///
/// From IFC4 ADD2 TC1. Slots 0-6 come from `IfcBSplineSurface`, 7-11 from
/// `IfcBSplineSurfaceWithKnots`, and 12 from
/// `IfcRationalBSplineSurfaceWithKnots`.
pub(crate) mod slot {
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

    /// The u polynomial degree, guaranteed to fit the u control count.
    pub fn u_degree(&self) -> GeometryResult<usize> {
        self.degree(slot::U_DEGREE, "UDegree", self.control_points()?.u_count())
    }

    /// The v polynomial degree, guaranteed to fit the v control count.
    pub fn v_degree(&self) -> GeometryResult<usize> {
        self.degree(slot::V_DEGREE, "VDegree", self.control_points()?.v_count())
    }

    /// The control point grid, checked to be rectangular.
    ///
    /// A ragged grid is not a tensor-product surface, so it cannot be
    /// evaluated at all; catching it here beats an out-of-bounds read in a
    /// kernel's inner loop. [`Self::control_point_views`] resolves the ids.
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

    /// The rectangular grid resolved to typed point views, `[u][v]`.
    ///
    /// Same shape and u/v convention as [`Self::control_points`]. One bad
    /// reference fails the whole grid: a hole in a tensor-product net has
    /// no meaning, so there is no partial result.
    pub fn control_point_views<'v>(
        &self,
        model: &'v Model,
    ) -> GeometryResult<Vec<Vec<CartesianPoint<'v>>>> {
        self.control_points()?
            .rows()
            .iter()
            .map(|row| resolve::cartesian_points(model, self.id(), row))
            .collect()
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

    /// Whether the entity declares the rational subtype.
    pub fn is_rational(&self) -> bool {
        self.slots
            .type_name()
            .eq_ignore_ascii_case("IFCRATIONALBSPLINESURFACEWITHKNOTS")
    }

    /// The u knot vector, checked against the u control point count.
    ///
    /// `Ok(None)` for a surface without knots.
    pub fn u_knots(&self) -> GeometryResult<Option<KnotVector>> {
        if !self.has_knots() {
            return Ok(None);
        }
        let expected = self
            .control_points()?
            .u_count()
            .checked_add(self.u_degree()?)
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| self.slots.degenerate("u knot count overflows usize"))?;
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
        let expected = self
            .control_points()?
            .v_count()
            .checked_add(self.v_degree()?)
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| self.slots.degenerate("v knot count overflows usize"))?;
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
    /// `Ok(None)` for a non-rational surface. Every weight must be finite and positive:
    /// a zero divides by zero at that control point and a negative one flips
    /// the patch through infinity.
    pub fn weights(&self) -> GeometryResult<Option<Vec<Vec<f64>>>> {
        let supplied = self.slots.opt(slot::WEIGHTS_DATA).is_some();
        match (self.is_rational(), supplied) {
            (false, false) => return Ok(None),
            (true, false) => {
                return Err(self
                    .slots
                    .degenerate("rational B-spline surface is missing WeightsData"));
            }
            (false, true) => {
                return Err(self
                    .slots
                    .degenerate("polynomial B-spline surface must not carry WeightsData"));
            }
            (true, true) => {}
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
                if !weight.is_finite() {
                    return Err(self.slots.degenerate(format!(
                        "weight {weight} at control point [{u_index}][{v_index}] must be finite"
                    )));
                }
                if weight <= 0.0 {
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

    fn degree(
        &self,
        index: usize,
        name: &'static str,
        control_count: usize,
    ) -> GeometryResult<usize> {
        let raw = self.slots.req_i64(index, name)?;
        if raw < 1 {
            return Err(self
                .slots
                .degenerate(format!("{name} must be at least 1, found {raw}")));
        }
        let degree = usize::try_from(raw).map_err(|_| {
            self.slots
                .degenerate(format!("{name} exceeds platform limits"))
        })?;
        if degree >= control_count {
            return Err(self.slots.degenerate(format!(
                "{name} {degree} must be smaller than the {control_count} control points"
            )));
        }
        Ok(degree)
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
                Value::Integer(i) if *i >= 1 => {
                    multiplicities.push(usize::try_from(*i).map_err(|_| {
                        self.slots
                            .degenerate(format!("{mult_name} exceeds platform limits"))
                    })?);
                }
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
        for (index, value) in values.iter().enumerate() {
            if !value.is_finite() {
                return Err(self.slots.degenerate(format!(
                    "{knots_name}[{index}] must be finite, found {value}"
                )));
            }
        }
        for pair in values.windows(2) {
            if pair[1] <= pair[0] {
                return Err(self.slots.degenerate(format!(
                    "{knots_name} must be strictly increasing; found {} after {}",
                    pair[1], pair[0]
                )));
            }
        }
        let total = multiplicities.iter().try_fold(0usize, |total, &value| {
            total.checked_add(value).ok_or_else(|| {
                self.slots
                    .degenerate(format!("{mult_name} total overflows usize"))
            })
        })?;
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
mod tests;
