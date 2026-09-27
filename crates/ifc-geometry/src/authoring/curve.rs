//! Curves: the parametric geometry profiles and sweeps are built from.
//!
//! Most of these are a placement plus a radius. Three are not, and they
//! are where a writer earns its keep:
//!
//! - **`IfcLine`** carries direction *and parameter scale* in one
//!   `IfcVector`. The vector's `Magnitude` is not cosmetic: it decides
//!   what parameter 1 means, so a unit direction with magnitude 5 is a
//!   different curve from the same direction with magnitude 1.
//! - **`IfcTrimmedCurve`** takes trims as a SET of one or two values,
//!   mixing a parameter and a point. `MasterRepresentation` says which
//!   to believe when both are given, and writing the wrong one silently
//!   changes where the curve starts.
//! - **B-splines** carry a knot vector whose multiplicities must sum to
//!   `Degree + |ControlPoints| + 1`. That is an arithmetic invariant,
//!   checkable without evaluating anything, so this module checks it.
//!
//! Everything here stays kernel-free: no curve is evaluated, sampled or
//! tested for self-intersection. `SelfIntersect` is written as the
//! caller states it, because deciding it needs an evaluator.

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::curve::bspline::slot as bspline_slot;
use crate::curve::composite::{curve_slot, segment_slot};
use crate::curve::conic::{circle_slot, ellipse_slot};
use crate::curve::line::slot as line_slot;
use crate::curve::polyline::indexed_slot;
use crate::curve::trimmed::{slot as trimmed_slot, Trim};
use crate::curve::{TransitionCode, TrimmingPreference};
use crate::error::GeometryError;

use super::std_profile::positive;
use super::{invalid, reals, refs, require_finite};

/// Stage an `IfcVector`: a direction with a magnitude.
///
/// # Errors
///
/// Refuses a non-finite or non-positive magnitude. A zero-magnitude
/// vector gives the line it parameterizes no scale at all.
pub fn vector(
    tx: &mut Transaction,
    orientation: EntityId,
    magnitude: f64,
) -> Result<EntityId, GeometryError> {
    positive("IFCVECTOR", "Magnitude", magnitude)?;
    let attrs = vec![Value::Ref(orientation), Value::Real(magnitude)];
    Ok(tx.create(Entity::new("IFCVECTOR", attrs)))
}

/// Stage an `IfcLine` through a point along a vector.
///
/// The vector sets both direction and parameterization; see the module
/// note. Pass a vector authored by [`vector`].
pub fn line(tx: &mut Transaction, point: EntityId, dir: EntityId) -> EntityId {
    let mut attrs = vec![Value::Null; 2];
    attrs[line_slot::PNT] = Value::Ref(point);
    attrs[line_slot::DIR] = Value::Ref(dir);
    tx.create(Entity::new("IFCLINE", attrs))
}

/// Stage an `IfcCircle`.
///
/// `position` may be a 2D or 3D placement: `IfcConic.Position` is an
/// `IfcAxis2Placement` select, and the dimensionality of the curve
/// follows the placement rather than being stated separately.
///
/// # Errors
///
/// Refuses a non-positive or non-finite radius.
pub fn circle(
    tx: &mut Transaction,
    position: EntityId,
    radius: f64,
) -> Result<EntityId, GeometryError> {
    positive("IFCCIRCLE", "Radius", radius)?;
    let mut attrs = vec![Value::Null; 2];
    attrs[circle_slot::POSITION] = Value::Ref(position);
    attrs[circle_slot::RADIUS] = Value::Real(radius);
    Ok(tx.create(Entity::new("IFCCIRCLE", attrs)))
}

/// Stage an `IfcEllipse`.
///
/// # Errors
///
/// Refuses a non-positive or non-finite semi-axis.
pub fn ellipse(
    tx: &mut Transaction,
    position: EntityId,
    semi_axis_1: f64,
    semi_axis_2: f64,
) -> Result<EntityId, GeometryError> {
    positive("IFCELLIPSE", "SemiAxis1", semi_axis_1)?;
    positive("IFCELLIPSE", "SemiAxis2", semi_axis_2)?;
    let mut attrs = vec![Value::Null; 3];
    attrs[ellipse_slot::POSITION] = Value::Ref(position);
    attrs[ellipse_slot::SEMI_AXIS_1] = Value::Real(semi_axis_1);
    attrs[ellipse_slot::SEMI_AXIS_2] = Value::Real(semi_axis_2);
    Ok(tx.create(Entity::new("IFCELLIPSE", attrs)))
}

/// Encode one end of a trim as a `SET [1:2] OF IfcTrimmingSelect`.
///
/// Reuses [`crate::curve::trimmed::Trim`] -- the type the reader already
/// returns -- rather than declaring a second shape for the same thing.
/// A trim written here reads back as an equal value.
fn trim_members(trim: Trim, which: &'static str) -> Result<Value, GeometryError> {
    let mut set = Vec::with_capacity(2);
    if let Some(point) = trim.cartesian {
        set.push(Value::Ref(point));
    }
    if let Some(parameter) = trim.parameter {
        require_finite("IFCTRIMMEDCURVE", which, &[parameter])?;
        // The select member must carry its measure type. A bare real is
        // tolerated by lenient readers but is not conforming, and dropping
        // the wrapper loses the only marker distinguishing a parameter
        // from anything else numeric in the set.
        set.push(Value::Typed {
            type_name: "IFCPARAMETERVALUE".into(),
            value: Box::new(Value::Real(parameter)),
        });
    }
    if set.is_empty() {
        return Err(invalid(
            "IFCTRIMMEDCURVE",
            which,
            "a trim needs at least a Cartesian point or a parameter",
        ));
    }
    Ok(Value::List(set))
}

/// Stage an `IfcTrimmedCurve`.
///
/// `sense_agreement` states whether the trimmed curve runs in the basis
/// curve's own direction. `master` says which trim form is
/// authoritative where both a point and a parameter are given.
///
/// # Errors
///
/// Refuses a trim that states neither a point nor a parameter: the
/// schema's `SET [1:2]` has a lower bound of one, so an empty trim is
/// not expressible.
pub fn trimmed_curve(
    tx: &mut Transaction,
    basis: EntityId,
    trim_1: Trim,
    trim_2: Trim,
    sense_agreement: bool,
    master: TrimmingPreference,
) -> Result<EntityId, GeometryError> {
    let mut attrs = vec![Value::Null; 5];
    attrs[trimmed_slot::BASIS_CURVE] = Value::Ref(basis);
    attrs[trimmed_slot::TRIM_1] = trim_members(trim_1, "Trim1")?;
    attrs[trimmed_slot::TRIM_2] = trim_members(trim_2, "Trim2")?;
    attrs[trimmed_slot::SENSE_AGREEMENT] = Value::Bool(sense_agreement);
    attrs[trimmed_slot::MASTER_REPRESENTATION] = Value::Enum(master.token().into());
    Ok(tx.create(Entity::new("IFCTRIMMEDCURVE", attrs)))
}

/// Stage an `IfcCompositeCurveSegment`.
///
/// `transition` describes what holds where this segment meets the
/// *next* one, so the last segment of an open curve is `Discontinuous`.
pub fn composite_curve_segment(
    tx: &mut Transaction,
    transition: TransitionCode,
    same_sense: bool,
    parent_curve: EntityId,
) -> EntityId {
    let mut attrs = vec![Value::Null; 3];
    attrs[segment_slot::TRANSITION] = Value::Enum(transition.token().into());
    attrs[segment_slot::SAME_SENSE] = Value::Bool(same_sense);
    attrs[segment_slot::PARENT_CURVE] = Value::Ref(parent_curve);
    tx.create(Entity::new("IFCCOMPOSITECURVESEGMENT", attrs))
}

/// Stage an `IfcCompositeCurve` over existing segments.
///
/// # Errors
///
/// Refuses an empty segment list: `LIST [1:?]`. Whether the segments
/// actually join is not checked -- that needs an evaluator, and the
/// `Transition` codes are the file's own claim about it.
pub fn composite_curve(
    tx: &mut Transaction,
    segments: &[EntityId],
    self_intersect: Option<bool>,
) -> Result<EntityId, GeometryError> {
    if segments.is_empty() {
        return Err(invalid(
            "IFCCOMPOSITECURVE",
            "Segments",
            "expected at least one segment",
        ));
    }
    let mut attrs = vec![Value::Null; 2];
    attrs[curve_slot::SEGMENTS] = refs(segments);
    attrs[curve_slot::SELF_INTERSECT] = match self_intersect {
        Some(value) => Value::Bool(value),
        // IfcLogical has a third state, and it is the honest answer
        // when nobody evaluated the curve.
        None => Value::LogicalUnknown,
    };
    Ok(tx.create(Entity::new("IFCCOMPOSITECURVE", attrs)))
}

/// Stage an `IfcOffsetCurve2D`.
///
/// A negative distance offsets the other way and is legal:
/// `IfcLengthMeasure`, not `IfcPositiveLengthMeasure`.
///
/// # Errors
///
/// Refuses a non-finite distance.
pub fn offset_curve_2d(
    tx: &mut Transaction,
    basis: EntityId,
    distance: f64,
    self_intersect: Option<bool>,
) -> Result<EntityId, GeometryError> {
    require_finite("IFCOFFSETCURVE2D", "Distance", &[distance])?;
    let attrs = vec![
        Value::Ref(basis),
        Value::Real(distance),
        logical(self_intersect),
    ];
    Ok(tx.create(Entity::new("IFCOFFSETCURVE2D", attrs)))
}

/// Stage an `IfcOffsetCurve3D`.
///
/// `ref_direction` fixes the offset plane; in 3D the offset is
/// otherwise ambiguous.
///
/// # Errors
///
/// Refuses a non-finite distance.
pub fn offset_curve_3d(
    tx: &mut Transaction,
    basis: EntityId,
    distance: f64,
    self_intersect: Option<bool>,
    ref_direction: EntityId,
) -> Result<EntityId, GeometryError> {
    require_finite("IFCOFFSETCURVE3D", "Distance", &[distance])?;
    let attrs = vec![
        Value::Ref(basis),
        Value::Real(distance),
        logical(self_intersect),
        Value::Ref(ref_direction),
    ];
    Ok(tx.create(Entity::new("IFCOFFSETCURVE3D", attrs)))
}

/// An `IfcLogical`, where absence means `UNKNOWN` rather than false.
fn logical(value: Option<bool>) -> Value {
    match value {
        Some(value) => Value::Bool(value),
        None => Value::LogicalUnknown,
    }
}

/// The knot vector of a B-spline curve.
#[derive(Debug, Clone, Copy)]
pub struct KnotVector<'a> {
    /// How many times each distinct knot repeats.
    pub multiplicities: &'a [i64],
    /// The distinct knot values, strictly increasing.
    pub knots: &'a [f64],
    /// `IfcKnotType`, e.g. `UNSPECIFIED` or `QUASI_UNIFORM_KNOTS`.
    pub spec: &'a str,
}

/// Stage an `IfcBSplineCurveWithKnots`.
///
/// # The invariant this checks
///
/// A B-spline is only well formed when
///
/// ```text
/// sum(KnotMultiplicities) = Degree + |ControlPointsList| + 1
/// ```
///
/// That is arithmetic, not geometry: it needs no evaluator, and a file
/// violating it describes no curve at all. Checking it here turns a
/// silent downstream failure into a refusal at the point of authoring.
///
/// # Errors
///
/// Refuses a degree below one, fewer than two control points, a
/// multiplicity/knot length mismatch, non-increasing knots, a
/// non-positive multiplicity, or a knot sum that breaks the identity
/// above.
pub fn bspline_curve_with_knots(
    tx: &mut Transaction,
    degree: i64,
    control_points: &[EntityId],
    curve_form: &str,
    knots: KnotVector<'_>,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCBSPLINECURVEWITHKNOTS";
    let attrs = bspline_attrs(T, degree, control_points, curve_form, knots)?;
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcRationalBSplineCurveWithKnots`.
///
/// # Errors
///
/// Everything [`bspline_curve_with_knots`] refuses, plus a weight count
/// that does not match the control points: the schema requires one
/// weight per control point.
pub fn rational_bspline_curve_with_knots(
    tx: &mut Transaction,
    degree: i64,
    control_points: &[EntityId],
    curve_form: &str,
    knots: KnotVector<'_>,
    weights: &[f64],
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCRATIONALBSPLINECURVEWITHKNOTS";
    if weights.len() != control_points.len() {
        return Err(invalid(
            T,
            "WeightsData",
            format!(
                "{} weights for {} control points",
                weights.len(),
                control_points.len()
            ),
        ));
    }
    require_finite(T, "WeightsData", weights)?;
    let mut attrs = bspline_attrs(T, degree, control_points, curve_form, knots)?;
    attrs.push(Value::List(
        weights.iter().copied().map(Value::Real).collect(),
    ));
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Build the eight shared B-spline slots, enforcing the knot identity.
fn bspline_attrs(
    type_name: &'static str,
    degree: i64,
    control_points: &[EntityId],
    curve_form: &str,
    knots: KnotVector<'_>,
) -> Result<Vec<Value>, GeometryError> {
    if degree < 1 {
        return Err(invalid(
            type_name,
            "Degree",
            format!("expected a degree of at least 1, got {degree}"),
        ));
    }
    if control_points.len() < 2 {
        return Err(invalid(
            type_name,
            "ControlPointsList",
            format!(
                "expected at least 2 control points, got {}",
                control_points.len()
            ),
        ));
    }
    if knots.multiplicities.len() != knots.knots.len() {
        return Err(invalid(
            type_name,
            "KnotMultiplicities",
            format!(
                "{} multiplicities for {} knots",
                knots.multiplicities.len(),
                knots.knots.len()
            ),
        ));
    }
    if knots.knots.len() < 2 {
        return Err(invalid(
            type_name,
            "Knots",
            "expected at least 2 distinct knots",
        ));
    }
    require_finite(type_name, "Knots", knots.knots)?;
    if let Some(bad) = knots.multiplicities.iter().position(|m| *m < 1) {
        return Err(invalid(
            type_name,
            "KnotMultiplicities",
            format!("multiplicity at index {bad} is not positive"),
        ));
    }
    if let Some(bad) = knots.knots.windows(2).position(|w| w[1] <= w[0]) {
        return Err(invalid(
            type_name,
            "Knots",
            format!("knots must strictly increase; index {bad} does not"),
        ));
    }

    // The identity that makes the knot vector describe this curve and
    // not some other one. See the doc comment on the public writers.
    let total: i64 = knots.multiplicities.iter().sum();
    let expected = degree + control_points.len() as i64 + 1;
    if total != expected {
        return Err(invalid(
            type_name,
            "KnotMultiplicities",
            format!(
                "multiplicities sum to {total}, but degree {degree} with {} control points requires {expected}",
                control_points.len()
            ),
        ));
    }

    let mut attrs = vec![Value::Null; 8];
    attrs[bspline_slot::DEGREE] = Value::Integer(degree);
    attrs[bspline_slot::CONTROL_POINTS] = refs(control_points);
    attrs[bspline_slot::CURVE_FORM] = Value::Enum(curve_form.into());
    // ClosedCurve and SelfIntersect both need an evaluator to decide, so
    // the honest value is IfcLogical UNKNOWN rather than a guess.
    attrs[bspline_slot::CLOSED_CURVE] = Value::LogicalUnknown;
    attrs[bspline_slot::SELF_INTERSECT] = Value::LogicalUnknown;
    attrs[bspline_slot::KNOT_MULTIPLICITIES] = Value::List(
        knots
            .multiplicities
            .iter()
            .copied()
            .map(Value::Integer)
            .collect(),
    );
    attrs[bspline_slot::KNOTS] =
        Value::List(knots.knots.iter().copied().map(Value::Real).collect());
    attrs[bspline_slot::KNOT_SPEC] = Value::Enum(knots.spec.into());
    Ok(attrs)
}

/// Stage an `IfcIndexedPolyCurve` over a point list.
///
/// Segments are optional: without them the curve is the polyline
/// through every point in order. With them, each segment indexes into
/// the list -- and those indices are 1-based, like everything else
/// index-shaped in IFC.
///
/// # Errors
///
/// Refuses an index outside the point list.
pub fn indexed_poly_curve(
    tx: &mut Transaction,
    points: EntityId,
    segments: Option<&[PolyCurveSegment<'_>]>,
    point_count: usize,
    self_intersect: Option<bool>,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCINDEXEDPOLYCURVE";
    let mut attrs = vec![Value::Null; 3];
    attrs[indexed_slot::POINTS] = Value::Ref(points);
    if let Some(segments) = segments {
        let mut list = Vec::with_capacity(segments.len());
        for segment in segments {
            list.push(segment.to_value(T, point_count)?);
        }
        attrs[indexed_slot::SEGMENTS] = Value::List(list);
    }
    attrs[indexed_slot::SELF_INTERSECT] = logical(self_intersect);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// One member of an `IfcSegmentIndexSelect`.
///
/// Indices are given 0-based here and written 1-based, matching the
/// tessellation writers: the schema counts from one, the API does not,
/// and the conversion happens in exactly one place.
#[derive(Debug, Clone, Copy)]
pub enum PolyCurveSegment<'a> {
    /// `IfcLineIndex`: two or more points joined by straight segments.
    Line(&'a [usize]),
    /// `IfcArcIndex`: exactly three points -- start, on-arc, end.
    Arc([usize; 3]),
}

impl PolyCurveSegment<'_> {
    /// Encode as a typed list of 1-based indices.
    fn to_value(self, type_name: &'static str, point_count: usize) -> Result<Value, GeometryError> {
        let (label, indices): (&str, &[usize]) = match self {
            Self::Line(indices) => ("IFCLINEINDEX", indices),
            Self::Arc(ref indices) => ("IFCARCINDEX", indices),
        };
        if let Self::Line(indices) = self {
            if indices.len() < 2 {
                return Err(invalid(
                    type_name,
                    "Segments",
                    format!(
                        "a line index needs at least 2 points, got {}",
                        indices.len()
                    ),
                ));
            }
        }
        for index in indices {
            if *index >= point_count {
                return Err(invalid(
                    type_name,
                    "Segments",
                    format!("index {index} is past the {point_count} point list"),
                ));
            }
        }
        let encoded = indices
            .iter()
            .map(|index| Value::Integer(*index as i64 + 1))
            .collect();
        Ok(Value::Typed {
            type_name: label.into(),
            value: Box::new(Value::List(encoded)),
        })
    }
}

/// The per-axis coefficient lists of an [`polynomial_curve`].
///
/// Each is `LIST [2:?] OF IfcReal`, lowest degree first. At least two
/// axes must be present; see `ValidCoefficients`.
#[derive(Debug, Clone, Copy, Default)]
pub struct PolynomialCoefficients<'a> {
    /// `CoefficientsX`.
    pub x: Option<&'a [f64]>,
    /// `CoefficientsY`.
    pub y: Option<&'a [f64]>,
    /// `CoefficientsZ`. Requires a 3D position.
    pub z: Option<&'a [f64]>,
}

/// Stage an `IfcPolynomialCurve`.
///
/// Each coefficient list is `LIST [2:?] OF IfcReal`, lowest degree
/// first, and at least two of the three axes must be given
/// (`ValidCoefficients`): a curve defined along one axis alone is a
/// line segment expressed as a curve, which the schema declines to
/// call a polynomial curve.
///
/// # Errors
///
/// Refuses fewer than two coefficient lists, a list shorter than two
/// entries, a non-finite coefficient, and `CoefficientsZ` against a 2D
/// placement (`CorrectPositionDim`) when `position_is_3d` is false.
pub fn polynomial_curve(
    tx: &mut Transaction,
    position: EntityId,
    coefficients: PolynomialCoefficients<'_>,
    position_is_3d: bool,
) -> Result<EntityId, GeometryError> {
    const ENTITY: &str = "IFCPOLYNOMIALCURVE";
    let PolynomialCoefficients { x, y, z } = coefficients;
    if z.is_some() && !position_is_3d {
        return Err(invalid(
            ENTITY,
            "CoefficientsZ",
            "a 2D position cannot carry Z coefficients",
        ));
    }
    let given = [x, y, z].iter().filter(|c| c.is_some()).count();
    if given < 2 {
        return Err(invalid(
            ENTITY,
            "Coefficients",
            "expected at least two of X, Y, Z, per ValidCoefficients",
        ));
    }
    for (values, attribute) in [
        (x, "CoefficientsX"),
        (y, "CoefficientsY"),
        (z, "CoefficientsZ"),
    ] {
        let Some(values) = values else { continue };
        if values.len() < 2 {
            return Err(invalid(ENTITY, attribute, "expected LIST [2:?]"));
        }
        require_finite(ENTITY, attribute, values)?;
    }
    let attrs = vec![
        Value::Ref(position),
        x.map_or(Value::Null, reals),
        y.map_or(Value::Null, reals),
        z.map_or(Value::Null, reals),
    ];
    Ok(tx.create(Entity::new(ENTITY, attrs)))
}

/// Stage an `IfcOffsetCurveByDistances`.
///
/// Unlike [`offset_curve_2d`] and [`offset_curve_3d`], which offset by
/// one constant, this varies the offset along the basis curve: each
/// `IfcPointByDistanceExpression` fixes a distance at a station, and
/// the offset interpolates between them. That is what alignment
/// widenings need -- a lay-by is not a constant offset.
///
/// # Errors
///
/// Refuses an empty `offset_values` list, which is `LIST [1:?]`.
pub fn offset_curve_by_distances(
    tx: &mut Transaction,
    basis: EntityId,
    offset_values: &[EntityId],
    tag: Option<&str>,
) -> Result<EntityId, GeometryError> {
    const ENTITY: &str = "IFCOFFSETCURVEBYDISTANCES";
    if offset_values.is_empty() {
        return Err(invalid(
            ENTITY,
            "OffsetValues",
            "expected at least one offset, per LIST [1:?]",
        ));
    }
    let attrs = vec![
        Value::Ref(basis),
        refs(offset_values),
        tag.map_or(Value::Null, |t| Value::Text(t.into())),
    ];
    Ok(tx.create(Entity::new(ENTITY, attrs)))
}

/// Stage an `IfcSegmentedReferenceCurve`.
///
/// A cant curve: the segments describe how a rail pair tilts along the
/// base curve. `SelfIntersect` is an `IfcLogical`, so an unstated value
/// is UNKNOWN rather than false -- claiming a curve does not self
/// intersect is a different assertion from not having checked.
///
/// # Errors
///
/// Refuses an empty `segments` list, which is `LIST [1:?]`.
pub fn segmented_reference_curve(
    tx: &mut Transaction,
    segments: &[EntityId],
    self_intersect: Option<bool>,
    base_curve: EntityId,
    end_point: Option<EntityId>,
) -> Result<EntityId, GeometryError> {
    const ENTITY: &str = "IFCSEGMENTEDREFERENCECURVE";
    if segments.is_empty() {
        return Err(invalid(
            ENTITY,
            "Segments",
            "expected at least one segment, per LIST [1:?]",
        ));
    }
    let attrs = vec![
        refs(segments),
        logical(self_intersect),
        Value::Ref(base_curve),
        end_point.map_or(Value::Null, Value::Ref),
    ];
    Ok(tx.create(Entity::new(ENTITY, attrs)))
}
