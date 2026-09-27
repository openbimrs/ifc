//! Surfaces: the analytic and bounded forms.
//!
//! Two things here are worth more than slot filling.
//!
//! # A B-spline surface has two knot vectors
//!
//! The identity from the curve writers applies once per direction:
//!
//! ```text
//! sum(UMultiplicities) = UDegree + rows + 1
//! sum(VMultiplicities) = VDegree + columns + 1
//! ```
//!
//! and the control grid must be rectangular, because
//! `LIST OF LIST OF IfcCartesianPoint` does not itself say the inner
//! lists agree in length. A ragged grid parses and denotes nothing.
//!
//! # A rectangular trim states its own direction twice
//!
//! `IfcRectangularTrimmedSurface` carries `Usense`/`Vsense` *and* the
//! parameters they describe, and the schema requires they agree:
//! `Vsense = (V2 > V1)`. Rather than take a sense from the caller and
//! hope, this writer derives both from the parameters -- the one value
//! that cannot then contradict the other.

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::error::GeometryError;
use crate::surface::bounded::{plane_slot, surface_slot, trimmed_slot};
use crate::surface::bspline::slot as bspline_slot;
use crate::surface::elementary::slot as elementary_slot;

use super::std_profile::positive;
use super::{invalid, refs, require_finite};

/// Stage an `IfcSphericalSurface`.
///
/// # Errors
///
/// Refuses a non-positive or non-finite radius.
pub fn spherical_surface(
    tx: &mut Transaction,
    position: EntityId,
    radius: f64,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCSPHERICALSURFACE";
    positive(T, "Radius", radius)?;
    let mut attrs = vec![Value::Null; 2];
    attrs[elementary_slot::POSITION] = Value::Ref(position);
    attrs[elementary_slot::RADIUS] = Value::Real(radius);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcToroidalSurface`.
///
/// `major_radius` runs from the axis to the centre of the tube;
/// `minor_radius` is the tube itself. The schema types both as positive
/// lengths and does not relate them: a minor radius exceeding the major
/// one gives a self-intersecting spindle torus, which is unusual but
/// legal, so it is not refused here.
///
/// # Errors
///
/// Refuses a non-positive or non-finite radius.
pub fn toroidal_surface(
    tx: &mut Transaction,
    position: EntityId,
    major_radius: f64,
    minor_radius: f64,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCTOROIDALSURFACE";
    positive(T, "MajorRadius", major_radius)?;
    positive(T, "MinorRadius", minor_radius)?;
    let mut attrs = vec![Value::Null; 3];
    attrs[elementary_slot::POSITION] = Value::Ref(position);
    attrs[elementary_slot::MAJOR_RADIUS] = Value::Real(major_radius);
    attrs[elementary_slot::MINOR_RADIUS] = Value::Real(minor_radius);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcCurveBoundedPlane`: a plane cut to an outline.
///
/// `inner_boundaries` is `SET [0:?]`, so a plane with no holes is
/// written with an empty set rather than `$`.
pub fn curve_bounded_plane(
    tx: &mut Transaction,
    basis: EntityId,
    outer_boundary: EntityId,
    inner_boundaries: &[EntityId],
) -> EntityId {
    let mut attrs = vec![Value::Null; 3];
    attrs[plane_slot::BASIS_SURFACE] = Value::Ref(basis);
    attrs[plane_slot::OUTER_BOUNDARY] = Value::Ref(outer_boundary);
    // SET [0:?]: empty is a legal value and means "no holes", which is
    // not the same claim as `$` ("not stated").
    attrs[plane_slot::INNER_BOUNDARIES] = refs(inner_boundaries);
    tx.create(Entity::new("IFCCURVEBOUNDEDPLANE", attrs))
}

/// Stage an `IfcCurveBoundedSurface`.
///
/// `implicit_outer` says the surface's own extent bounds it, in which
/// case `boundaries` carries only the holes.
///
/// # Errors
///
/// Refuses an empty boundary set: `SET [1:?]`.
pub fn curve_bounded_surface(
    tx: &mut Transaction,
    basis: EntityId,
    boundaries: &[EntityId],
    implicit_outer: bool,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCCURVEBOUNDEDSURFACE";
    if boundaries.is_empty() {
        return Err(invalid(T, "Boundaries", "expected at least one boundary"));
    }
    let mut attrs = vec![Value::Null; 3];
    attrs[surface_slot::BASIS_SURFACE] = Value::Ref(basis);
    attrs[surface_slot::BOUNDARIES] = refs(boundaries);
    attrs[surface_slot::IMPLICIT_OUTER] = Value::Bool(implicit_outer);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcRectangularTrimmedSurface`.
///
/// The senses are **derived from the parameters**, not taken from the
/// caller: the schema requires `Vsense = (V2 > V1)` (and the same for u
/// on most basis surfaces), so accepting a sense that could disagree
/// would only create a way to write an invalid file.
///
/// # Errors
///
/// Refuses `u1 == u2` or `v1 == v2` -- a trim of zero extent in either
/// direction -- or a non-finite parameter.
pub fn rectangular_trimmed_surface(
    tx: &mut Transaction,
    basis: EntityId,
    u: (f64, f64),
    v: (f64, f64),
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCRECTANGULARTRIMMEDSURFACE";
    let (u1, u2) = u;
    let (v1, v2) = v;
    require_finite(T, "U1", &[u1, u2])?;
    require_finite(T, "V1", &[v1, v2])?;
    if u1 == u2 {
        return Err(invalid(T, "U1", "U1 and U2 must differ"));
    }
    if v1 == v2 {
        return Err(invalid(T, "V1", "V1 and V2 must differ"));
    }

    let mut attrs = vec![Value::Null; 7];
    attrs[trimmed_slot::BASIS_SURFACE] = Value::Ref(basis);
    attrs[trimmed_slot::U1] = parameter(u1);
    attrs[trimmed_slot::V1] = parameter(v1);
    attrs[trimmed_slot::U2] = parameter(u2);
    attrs[trimmed_slot::V2] = parameter(v2);
    // UsenseCompatible / VsenseCompatible: the sense is a restatement of
    // the parameter order, so derive it and it cannot contradict.
    attrs[trimmed_slot::USENSE] = Value::Bool(u2 > u1);
    attrs[trimmed_slot::VSENSE] = Value::Bool(v2 > v1);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// An `IfcParameterValue`, which must carry its measure type.
fn parameter(value: f64) -> Value {
    Value::Typed {
        type_name: "IFCPARAMETERVALUE".into(),
        value: Box::new(Value::Real(value)),
    }
}

/// A knot vector along one surface direction.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceKnots<'a> {
    /// How many times each distinct knot repeats.
    pub multiplicities: &'a [i64],
    /// The distinct knot values, strictly increasing.
    pub knots: &'a [f64],
}

/// Everything a B-spline surface states apart from its control grid.
///
/// Grouped because the u and v data must be kept straight: four bare
/// arguments of two types invite a transposition that still compiles.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceBasis<'a> {
    /// `UDegree` and `VDegree`, in that order.
    pub degree: (i64, i64),
    /// The u knot vector, checked against the grid's row count.
    pub u: SurfaceKnots<'a>,
    /// The v knot vector, checked against the grid's column count.
    pub v: SurfaceKnots<'a>,
    /// `SurfaceForm`, informational.
    pub form: &'a str,
    /// `KnotSpec`, e.g. `UNSPECIFIED`.
    pub knot_spec: &'a str,
}

/// Stage an `IfcBSplineSurfaceWithKnots`.
///
/// `control_points` is indexed `[u][v]`: the outer list runs along u,
/// matching `ControlPointsList` in the schema.
///
/// # Errors
///
/// Refuses a degree below one in either direction, a control grid with
/// fewer than two rows or columns, a **ragged** grid, non-increasing or
/// non-positive knot data, or either knot identity broken:
/// `sum(UMultiplicities) = UDegree + rows + 1`, and likewise for v.
pub fn bspline_surface_with_knots(
    tx: &mut Transaction,
    control_points: &[&[EntityId]],
    basis: SurfaceBasis<'_>,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCBSPLINESURFACEWITHKNOTS";
    let attrs = surface_attrs(T, control_points, basis)?;
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcRationalBSplineSurfaceWithKnots`.
///
/// # Errors
///
/// Everything [`bspline_surface_with_knots`] refuses, plus a weight
/// grid whose shape does not match the control grid exactly.
pub fn rational_bspline_surface_with_knots(
    tx: &mut Transaction,
    control_points: &[&[EntityId]],
    basis: SurfaceBasis<'_>,
    weights: &[&[f64]],
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCRATIONALBSPLINESURFACEWITHKNOTS";
    if weights.len() != control_points.len() {
        return Err(invalid(
            T,
            "WeightsData",
            format!(
                "{} weight rows for {} control point rows",
                weights.len(),
                control_points.len()
            ),
        ));
    }
    for (index, (row, points)) in weights.iter().zip(control_points).enumerate() {
        if row.len() != points.len() {
            return Err(invalid(
                T,
                "WeightsData",
                format!(
                    "weight row {index} has {} entries, its control row has {}",
                    row.len(),
                    points.len()
                ),
            ));
        }
        require_finite(T, "WeightsData", row)?;
    }
    let mut attrs = surface_attrs(T, control_points, basis)?;
    attrs.push(Value::List(
        weights
            .iter()
            .map(|row| Value::List(row.iter().copied().map(Value::Real).collect()))
            .collect(),
    ));
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Build the twelve shared slots, enforcing both knot identities.
fn surface_attrs(
    type_name: &'static str,
    control_points: &[&[EntityId]],
    basis: SurfaceBasis<'_>,
) -> Result<Vec<Value>, GeometryError> {
    let SurfaceBasis {
        degree: (u_degree, v_degree),
        u,
        v,
        form,
        knot_spec,
    } = basis;
    let rows = control_points.len();
    if rows < 2 {
        return Err(invalid(
            type_name,
            "ControlPointsList",
            format!("expected at least 2 rows, got {rows}"),
        ));
    }
    let columns = control_points[0].len();
    if columns < 2 {
        return Err(invalid(
            type_name,
            "ControlPointsList",
            format!("expected at least 2 columns, got {columns}"),
        ));
    }
    // LIST OF LIST does not constrain the inner lengths, so a ragged
    // grid is expressible and meaningless. Catch it here.
    if let Some(bad) = control_points.iter().position(|row| row.len() != columns) {
        return Err(invalid(
            type_name,
            "ControlPointsList",
            format!(
                "row {bad} has {} control points, row 0 has {columns}; the grid must be rectangular",
                control_points[bad].len()
            ),
        ));
    }

    check_knots(type_name, "U", u_degree, rows, u)?;
    check_knots(type_name, "V", v_degree, columns, v)?;

    let mut attrs = vec![Value::Null; 12];
    attrs[bspline_slot::U_DEGREE] = Value::Integer(u_degree);
    attrs[bspline_slot::V_DEGREE] = Value::Integer(v_degree);
    attrs[bspline_slot::CONTROL_POINTS] =
        Value::List(control_points.iter().map(|row| refs(row)).collect());
    attrs[bspline_slot::SURFACE_FORM] = Value::Enum(form.into());
    // Closure and self-intersection need an evaluator to decide.
    attrs[bspline_slot::U_CLOSED] = Value::LogicalUnknown;
    attrs[bspline_slot::V_CLOSED] = Value::LogicalUnknown;
    attrs[bspline_slot::SELF_INTERSECT] = Value::LogicalUnknown;
    attrs[bspline_slot::U_MULTIPLICITIES] = integers(u.multiplicities);
    attrs[bspline_slot::V_MULTIPLICITIES] = integers(v.multiplicities);
    attrs[bspline_slot::U_KNOTS] = reals(u.knots);
    attrs[bspline_slot::V_KNOTS] = reals(v.knots);
    attrs[bspline_slot::KNOT_SPEC] = Value::Enum(knot_spec.into());
    Ok(attrs)
}

/// One direction's knot vector, against its own degree and extent.
fn check_knots(
    type_name: &'static str,
    axis: &'static str,
    degree: i64,
    extent: usize,
    knots: SurfaceKnots<'_>,
) -> Result<(), GeometryError> {
    if degree < 1 {
        return Err(invalid(
            type_name,
            "Degree",
            format!("{axis}Degree must be at least 1, got {degree}"),
        ));
    }
    if knots.multiplicities.len() != knots.knots.len() {
        return Err(invalid(
            type_name,
            "Multiplicities",
            format!(
                "{axis}: {} multiplicities for {} knots",
                knots.multiplicities.len(),
                knots.knots.len()
            ),
        ));
    }
    if knots.knots.len() < 2 {
        return Err(invalid(
            type_name,
            "Knots",
            format!("{axis}: expected at least 2 distinct knots"),
        ));
    }
    require_finite(type_name, "Knots", knots.knots)?;
    if knots.multiplicities.iter().any(|m| *m < 1) {
        return Err(invalid(
            type_name,
            "Multiplicities",
            format!("{axis}: a multiplicity is not positive"),
        ));
    }
    if knots.knots.windows(2).any(|w| w[1] <= w[0]) {
        return Err(invalid(
            type_name,
            "Knots",
            format!("{axis}: knots must strictly increase"),
        ));
    }
    let total: i64 = knots.multiplicities.iter().sum();
    let expected = degree + extent as i64 + 1;
    if total != expected {
        return Err(invalid(
            type_name,
            "Multiplicities",
            format!(
                "{axis}: multiplicities sum to {total}, but degree {degree} over {extent} control points requires {expected}"
            ),
        ));
    }
    Ok(())
}

/// A list of integers.
fn integers(values: &[i64]) -> Value {
    Value::List(values.iter().copied().map(Value::Integer).collect())
}

/// A list of reals.
fn reals(values: &[f64]) -> Value {
    Value::List(values.iter().copied().map(Value::Real).collect())
}
