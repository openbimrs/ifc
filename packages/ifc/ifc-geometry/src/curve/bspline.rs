//! `IfcBSplineCurve` and its knotted / rational refinements.
//!
//! # What this module does and does not do
//!
//! It reads the NURBS *parameters*. It does not evaluate a basis function,
//! does not de-Boor, does not insert a knot. Those belong to a geometry
//! kernel. What this module does provide is the consistency checking a kernel
//! would otherwise have to repeat, because every one of these invariants shows
//! up in real files and every one of them produces a NaN or an out-of-bounds
//! read rather than an error.
//!
//! # The invariants worth enforcing at the boundary
//!
//! - **Knot vector length.** `sum(KnotMultiplicities)` must equal
//!   `ControlPoints + Degree + 1`. IFC stores knots in *compressed* form:
//!   distinct values plus their multiplicities, not the repeated vector a
//!   kernel wants. Getting this wrong is not detectable downstream, it just
//!   evaluates garbage.
//! - **Multiplicities and knots are parallel lists.** Different lengths mean
//!   the file is corrupt, not that the shorter one defaults.
//! - **Weights are positive and count-matched.** A zero weight divides by zero
//!   at the corresponding control point; a negative one produces a curve that
//!   flips through infinity. Both are legal IEEE arithmetic and neither is
//!   legal geometry.
//!
//! # Why `ClosedCurve` and `SelfIntersect` are only informational
//!
//! They are `IfcLogical`, may be `.U.`, and are derived facts a file asserts
//! rather than constraints it guarantees. They are exposed as `Option<bool>`
//! so a consumer can use them as a hint and never as a precondition.

use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId, Value};

/// `IfcBSplineCurve` family attribute slots.
///
/// From IFC4 ADD2 TC1. Slots 0-4 come from `IfcBSplineCurve`, 5-7 from
/// `IfcBSplineCurveWithKnots`, and 8 from
/// `IfcRationalBSplineCurveWithKnots`. Inherited attributes come first, so a
/// rational curve reads `Degree` at 0 and `WeightsData` at 8.
mod slot {
    /// `Degree`: `IfcInteger`, from `IfcBSplineCurve`.
    pub const DEGREE: usize = 0;
    /// `ControlPointsList`: `LIST [2:?] OF IfcCartesianPoint`.
    pub const CONTROL_POINTS: usize = 1;
    /// `CurveForm`: `IfcBSplineCurveForm`.
    pub const CURVE_FORM: usize = 2;
    /// `ClosedCurve`: `IfcLogical`.
    pub const CLOSED_CURVE: usize = 3;
    /// `SelfIntersect`: `IfcLogical`.
    pub const SELF_INTERSECT: usize = 4;
    /// `KnotMultiplicities`: from `IfcBSplineCurveWithKnots`.
    pub const KNOT_MULTIPLICITIES: usize = 5;
    /// `Knots`: distinct knot values, from `IfcBSplineCurveWithKnots`.
    pub const KNOTS: usize = 6;
    /// `KnotSpec`: `IfcKnotType`, from `IfcBSplineCurveWithKnots`.
    pub const KNOT_SPEC: usize = 7;
    /// `WeightsData`: from `IfcRationalBSplineCurveWithKnots`.
    pub const WEIGHTS_DATA: usize = 8;
}

/// `IfcBSplineCurveForm`: what shape the curve was originally.
///
/// Purely informational. A `CIRCULAR_ARC` form does not guarantee the control
/// points actually describe a circle, so it must never be used to substitute
/// an analytic curve for the spline. It is worth keeping because it lets a
/// consumer round-trip the hint and lets a UI say something useful.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BSplineCurveForm {
    /// The control points are a polyline.
    Polyline,
    /// Originally a circular arc.
    CircularArc,
    /// Originally an elliptic arc.
    EllipticArc,
    /// Originally a parabolic arc.
    ParabolicArc,
    /// Originally a hyperbolic arc.
    HyperbolicArc,
    /// No original form is claimed.
    Unspecified,
}

impl BSplineCurveForm {
    /// Parse the enumeration token, `None` if unrecognised.
    pub fn from_token(token: &str) -> Option<Self> {
        match token.to_ascii_uppercase().as_str() {
            "POLYLINE_FORM" => Some(Self::Polyline),
            "CIRCULAR_ARC" => Some(Self::CircularArc),
            "ELLIPTIC_ARC" => Some(Self::EllipticArc),
            "PARABOLIC_ARC" => Some(Self::ParabolicArc),
            "HYPERBOLIC_ARC" => Some(Self::HyperbolicArc),
            "UNSPECIFIED" => Some(Self::Unspecified),
            _ => None,
        }
    }
}

/// `IfcKnotType`: the shape of the knot vector.
///
/// Also informational: the knots themselves are stored explicitly, so this
/// only describes them. A kernel should trust the values, not the label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnotType {
    /// Evenly spaced knots including the ends.
    Uniform,
    /// Uniform interior knots with clamped ends.
    QuasiUniform,
    /// Interior multiplicity equal to the degree; Bezier segments.
    PiecewiseBezier,
    /// Nothing is claimed about the knot vector.
    Unspecified,
}

impl KnotType {
    /// Parse the enumeration token, `None` if unrecognised.
    pub fn from_token(token: &str) -> Option<Self> {
        match token.to_ascii_uppercase().as_str() {
            "UNIFORM_KNOTS" => Some(Self::Uniform),
            "QUASI_UNIFORM_KNOTS" => Some(Self::QuasiUniform),
            "PIECEWISE_BEZIER_KNOTS" => Some(Self::PiecewiseBezier),
            "UNSPECIFIED" => Some(Self::Unspecified),
            _ => None,
        }
    }
}

/// A borrowed view of any `IfcBSplineCurve` subtype.
///
/// The knot and weight accessors return `Ok(None)` when the entity is a
/// plainer subtype that does not carry them, so one view serves the whole
/// family without the caller re-dispatching on the type name.
#[derive(Debug, Clone, Copy)]
pub struct BSplineCurve<'m> {
    slots: Slots<'m>,
}

impl<'m> BSplineCurve<'m> {
    /// Wrap an entity known to be an `IfcBSplineCurve` subtype.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The polynomial degree, guaranteed at least 1.
    ///
    /// Degree 0 would make every "curve" a disconnected set of points.
    pub fn degree(&self) -> GeometryResult<usize> {
        let degree = self.slots.req_i64(slot::DEGREE, "Degree")?;
        if degree < 1 {
            return Err(self
                .slots
                .degenerate(format!("Degree must be at least 1, found {degree}")));
        }
        Ok(degree as usize)
    }

    /// The `IfcCartesianPoint` control point references, in order.
    ///
    /// Order defines the curve; sorting or deduplicating them destroys it.
    // TODO: `resource::point` will provide a typed point view to resolve these.
    pub fn control_point_refs(&self) -> GeometryResult<Vec<EntityId>> {
        let points = self
            .slots
            .req_ref_list(slot::CONTROL_POINTS, "ControlPointsList")?;
        if points.len() < 2 {
            return Err(self.slots.degenerate(format!(
                "ControlPointsList needs at least 2 points, found {}",
                points.len()
            )));
        }
        Ok(points)
    }

    /// The declared original form, defaulting to `Unspecified`.
    pub fn curve_form(&self) -> BSplineCurveForm {
        self.slots
            .opt_enum(slot::CURVE_FORM)
            .and_then(BSplineCurveForm::from_token)
            .unwrap_or(BSplineCurveForm::Unspecified)
    }

    /// The asserted `ClosedCurve` flag; `None` for `.U.` or absent.
    ///
    /// A hint only: nothing verifies that the first and last control points
    /// coincide.
    pub fn closed_curve(&self) -> Option<bool> {
        self.slots.opt_bool(slot::CLOSED_CURVE)
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
        self.slots.opt(slot::KNOTS).is_some()
    }

    /// Is this the rational subtype, i.e. does it carry weights?
    pub fn is_rational(&self) -> bool {
        self.slots.opt(slot::WEIGHTS_DATA).is_some()
    }

    /// The compressed knot vector: distinct values with their multiplicities.
    ///
    /// Returns `Ok(None)` for a curve without knots. When present, the two
    /// lists are validated to be the same length and the total multiplicity is
    /// checked against `ControlPoints + Degree + 1`, which is the invariant
    /// that silently produces garbage when violated.
    pub fn knots(&self) -> GeometryResult<Option<KnotVector>> {
        if !self.has_knots() {
            return Ok(None);
        }
        let values = self.slots.req_f64_list(slot::KNOTS, "Knots")?;
        let multiplicities = self.integer_list(slot::KNOT_MULTIPLICITIES, "KnotMultiplicities")?;

        if values.len() != multiplicities.len() {
            return Err(self.slots.degenerate(format!(
                "Knots has {} entries but KnotMultiplicities has {}; \
                 they are parallel lists",
                values.len(),
                multiplicities.len()
            )));
        }
        if values.is_empty() {
            return Err(self.slots.degenerate("Knots is empty"));
        }
        for (i, m) in multiplicities.iter().enumerate() {
            if *m < 1 {
                return Err(self.slots.degenerate(format!(
                    "knot multiplicity {m} at position {i} is not positive"
                )));
            }
        }
        // Knots must be strictly increasing: IFC stores each distinct value
        // once, so a repeat means the multiplicity was written twice instead
        // of being folded, which shifts the whole vector.
        for pair in values.windows(2) {
            if pair[1] <= pair[0] {
                return Err(self.slots.degenerate(format!(
                    "Knots must be strictly increasing; found {} after {}",
                    pair[1], pair[0]
                )));
            }
        }

        let total: usize = multiplicities.iter().map(|m| *m as usize).sum();
        let expected = self.control_point_refs()?.len() + self.degree()? + 1;
        if total != expected {
            return Err(self.slots.degenerate(format!(
                "knot multiplicities sum to {total} but must equal \
                 ControlPoints + Degree + 1 = {expected}"
            )));
        }

        Ok(Some(KnotVector {
            values,
            multiplicities: multiplicities.iter().map(|m| *m as usize).collect(),
        }))
    }

    /// The rational weights, one per control point, all positive.
    ///
    /// Returns `Ok(None)` for a non-rational curve. A zero weight is a
    /// division by zero at evaluation time and a negative one flips the curve
    /// through infinity; both are rejected here rather than in the kernel.
    pub fn weights(&self) -> GeometryResult<Option<Vec<f64>>> {
        if !self.is_rational() {
            return Ok(None);
        }
        let weights = self.slots.req_f64_list(slot::WEIGHTS_DATA, "WeightsData")?;
        let control_points = self.control_point_refs()?.len();
        if weights.len() != control_points {
            return Err(self.slots.degenerate(format!(
                "WeightsData has {} entries but there are {control_points} control points",
                weights.len()
            )));
        }
        for (i, w) in weights.iter().enumerate() {
            // NaN must be rejected too, hence the explicit `is_nan` rather
            // than relying on `<= 0.0`, which is false for NaN.
            if w.is_nan() || *w <= 0.0 {
                return Err(self
                    .slots
                    .degenerate(format!("weight {w} at control point {i} must be positive")));
            }
        }
        Ok(Some(weights))
    }

    /// Read a list of integers, rejecting reals.
    ///
    /// Multiplicities are counts. A real here means the file lied about a
    /// count and rounding it would hide the corruption.
    fn integer_list(&self, index: usize, name: &'static str) -> GeometryResult<Vec<i64>> {
        let value = self.slots.req(index, name)?;
        let items = value
            .as_list()
            .ok_or_else(|| self.slots.degenerate(format!("{name} must be a list")))?;
        items
            .iter()
            .map(|v| match v.unwrap_typed() {
                Value::Integer(i) => Ok(*i),
                other => Err(self
                    .slots
                    .degenerate(format!("{name} entry is not an integer: {other:?}"))),
            })
            .collect()
    }
}

/// A knot vector as IFC stores it: distinct values plus multiplicities.
///
/// Kept compressed because that is what the file says. [`Self::expanded`]
/// produces the repeated vector a kernel wants, in one place, so no consumer
/// re-implements the expansion.
#[derive(Debug, Clone, PartialEq)]
pub struct KnotVector {
    /// The distinct knot values, strictly increasing.
    pub values: Vec<f64>,
    /// How many times each value repeats; parallel to `values`.
    pub multiplicities: Vec<usize>,
}

impl KnotVector {
    /// The full, repeated knot vector.
    ///
    /// Length equals `ControlPoints + Degree + 1` for a valid curve, which
    /// [`BSplineCurve::knots`] has already checked.
    pub fn expanded(&self) -> Vec<f64> {
        let mut out = Vec::with_capacity(self.total_multiplicity());
        for (value, multiplicity) in self.values.iter().zip(&self.multiplicities) {
            out.extend(std::iter::repeat_n(*value, *multiplicity));
        }
        out
    }

    /// The length the expanded vector would have.
    pub fn total_multiplicity(&self) -> usize {
        self.multiplicities.iter().sum()
    }

    /// Is the curve clamped, i.e. does it touch its first and last control
    /// point?
    ///
    /// True when the end multiplicities equal `degree + 1`. Worth asking
    /// because an unclamped curve starts somewhere in the middle of its
    /// control polygon, which looks like a modelling error to anyone who has
    /// only seen clamped splines.
    pub fn is_clamped(&self, degree: usize) -> bool {
        let want = degree + 1;
        self.multiplicities.first() == Some(&want) && self.multiplicities.last() == Some(&want)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refs(n: usize) -> Value {
        Value::List((0..n).map(|i| Value::Ref(EntityId(i as u64 + 1))).collect())
    }

    fn integers(values: &[i64]) -> Value {
        Value::List(values.iter().map(|i| Value::Integer(*i)).collect())
    }

    fn reals(values: &[f64]) -> Value {
        Value::List(values.iter().map(|r| Value::Real(*r)).collect())
    }

    /// A degree-3 curve with 4 control points: knots 0 and 1, each x4.
    fn with_knots(control_points: usize, multiplicities: &[i64], knots: &[f64]) -> Entity {
        Entity::new(
            "IFCBSPLINECURVEWITHKNOTS",
            vec![
                Value::Integer(3),
                refs(control_points),
                Value::Enum("UNSPECIFIED".into()),
                Value::Bool(false),
                Value::Bool(false),
                integers(multiplicities),
                reals(knots),
                Value::Enum("UNSPECIFIED".into()),
            ],
        )
    }

    fn rational(control_points: usize, weights: &[f64]) -> Entity {
        let mut attributes = with_knots(control_points, &[4, 4], &[0.0, 1.0]).attributes;
        attributes.push(reals(weights));
        Entity::new("IFCRATIONALBSPLINECURVEWITHKNOTS", attributes)
    }

    #[test]
    fn inherited_bspline_slots_are_read_before_the_subtype_own_slots() {
        let e = with_knots(4, &[4, 4], &[0.0, 1.0]);
        let view = BSplineCurve::new(EntityId(1), &e);
        assert_eq!(view.degree().unwrap(), 3);
        assert_eq!(view.control_point_refs().unwrap().len(), 4);
        assert!(view.has_knots());
        assert!(!view.is_rational());
    }

    #[test]
    fn a_valid_knot_vector_expands_to_control_points_plus_degree_plus_one() {
        let e = with_knots(4, &[4, 4], &[0.0, 1.0]);
        let knots = BSplineCurve::new(EntityId(1), &e).knots().unwrap().unwrap();
        assert_eq!(
            knots.expanded(),
            vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0]
        );
        assert_eq!(knots.total_multiplicity(), 4 + 3 + 1);
        assert!(knots.is_clamped(3));
    }

    #[test]
    fn a_knot_multiplicity_sum_that_disagrees_with_the_degree_is_rejected() {
        let e = with_knots(4, &[4, 3], &[0.0, 1.0]);
        let err = BSplineCurve::new(EntityId(6), &e).knots().unwrap_err();
        assert!(err.to_string().contains("must equal"), "got: {err}");
        assert!(err.to_string().contains("#6"), "got: {err}");
    }

    #[test]
    fn knots_and_multiplicities_of_different_lengths_are_rejected() {
        let e = with_knots(4, &[4, 4], &[0.0, 0.5, 1.0]);
        let err = BSplineCurve::new(EntityId(1), &e).knots().unwrap_err();
        assert!(err.to_string().contains("parallel"), "got: {err}");
    }

    /// IFC stores each distinct knot once; a repeat means the file folded the
    /// multiplicity wrongly and every later knot is shifted.
    #[test]
    fn non_increasing_knot_values_are_rejected() {
        let e = with_knots(5, &[4, 1, 4], &[0.0, 1.0, 1.0]);
        let err = BSplineCurve::new(EntityId(1), &e).knots().unwrap_err();
        assert!(err.to_string().contains("increasing"), "got: {err}");
    }

    #[test]
    fn an_unclamped_knot_vector_is_recognised_as_such() {
        // Degree 3, 6 control points, all multiplicities 1: 10 knots.
        let e = Entity::new(
            "IFCBSPLINECURVEWITHKNOTS",
            vec![
                Value::Integer(3),
                refs(6),
                Value::Enum("UNSPECIFIED".into()),
                Value::Bool(false),
                Value::Bool(false),
                integers(&[1; 10]),
                reals(&[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]),
                Value::Enum("UNIFORM_KNOTS".into()),
            ],
        );
        let view = BSplineCurve::new(EntityId(1), &e);
        let knots = view.knots().unwrap().unwrap();
        assert!(!knots.is_clamped(3));
        assert_eq!(view.knot_spec(), KnotType::Uniform);
    }

    #[test]
    fn a_curve_without_knots_reports_none_rather_than_failing() {
        let e = Entity::new(
            "IFCBSPLINECURVE",
            vec![
                Value::Integer(3),
                refs(4),
                Value::Enum("UNSPECIFIED".into()),
                Value::Bool(false),
                Value::Bool(false),
            ],
        );
        let view = BSplineCurve::new(EntityId(1), &e);
        assert_eq!(view.knots().unwrap(), None);
        assert_eq!(view.weights().unwrap(), None);
    }

    #[test]
    fn rational_weights_are_read_from_the_slot_after_the_knot_attributes() {
        let e = rational(4, &[1.0, 0.5, 0.5, 1.0]);
        let view = BSplineCurve::new(EntityId(1), &e);
        assert!(view.is_rational());
        assert_eq!(view.weights().unwrap().unwrap(), vec![1.0, 0.5, 0.5, 1.0]);
    }

    /// A zero weight is a division by zero at the control point; IEEE will
    /// happily produce an infinity rather than an error.
    #[test]
    fn a_zero_weight_is_degenerate() {
        let e = rational(4, &[1.0, 0.0, 1.0, 1.0]);
        let err = BSplineCurve::new(EntityId(2), &e).weights().unwrap_err();
        assert!(err.to_string().contains("positive"), "got: {err}");
    }

    #[test]
    fn a_negative_weight_is_degenerate() {
        let e = rational(4, &[1.0, -1.0, 1.0, 1.0]);
        assert!(BSplineCurve::new(EntityId(1), &e).weights().is_err());
    }

    #[test]
    fn a_weight_count_that_differs_from_the_control_point_count_is_rejected() {
        let e = rational(4, &[1.0, 1.0, 1.0]);
        let err = BSplineCurve::new(EntityId(1), &e).weights().unwrap_err();
        assert!(err.to_string().contains("control points"), "got: {err}");
    }

    #[test]
    fn degree_zero_is_rejected_because_it_describes_no_curve() {
        let mut e = with_knots(4, &[4, 4], &[0.0, 1.0]);
        e.attributes[0] = Value::Integer(0);
        assert!(BSplineCurve::new(EntityId(1), &e).degree().is_err());
    }

    /// The form is a hint about provenance and must never license swapping in
    /// an analytic circle, so it is read but kept separate from the geometry.
    #[test]
    fn curve_form_tokens_parse_without_altering_the_control_points() {
        assert_eq!(
            BSplineCurveForm::from_token("CIRCULAR_ARC"),
            Some(BSplineCurveForm::CircularArc)
        );
        assert_eq!(BSplineCurveForm::from_token("SPIRAL"), None);
        assert_eq!(
            KnotType::from_token("PIECEWISE_BEZIER_KNOTS"),
            Some(KnotType::PiecewiseBezier)
        );
        assert_eq!(KnotType::from_token("WEIRD"), None);
    }
}
