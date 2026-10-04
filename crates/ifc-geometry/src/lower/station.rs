//! IFC4X3 distance-along-curve geometry onto Axiolid stations (#307).
//!
//! `IfcPointByDistanceExpression` (IFC4.3 ADD2 8.9.3.48) names a point by a
//! distance along a basis curve plus three offsets; `IfcAxis2PlacementLinear`
//! (8.9.3.4) frames one; `IfcOffsetCurveByDistances` (8.9.3.42) runs through
//! several; `IfcSectionedSolidHorizontal` (8.8.3.35) and
//! `IfcSectionedSurface` (8.8.3.37) stand cross sections at them. Resolving
//! a distance to a point is curve evaluation, which this crate never does
//! (ADR 0004): each lowers to Axiolid's station relations (`axiolid-model`
//! 0.3.5, ADR 0082), stored exactly, which a kernel resolves.
//!
//! # Convention map
//!
//! Every row is the same in IFC4.3 ADD2 and in ADR 0082, so nothing is
//! converted. Each is pinned by a test that resolves the lowered station
//! through `axiolid-reference` (tests only).
//!
//! | IFC4.3 ADD2 | Axiolid |
//! | --- | --- |
//! | `DistanceAlong` as `IfcLengthMeasure`, from the basis curve's start | `Station::distance` |
//! | on an `IfcGradientCurve`, the `BaseCurve`'s parameter: plan arc length (8.9.3.34.1 "the value of the parameter equals the parameter value of BaseCurve") | plan distance on `Curve3::Elevated` |
//! | on any other curve, its arc length | arc length |
//! | `OffsetLateral`, "positive values indicate to the left of the basis curve as facing in the positive parametrization direction" | `StationOffsets::lateral`, positive to the left |
//! | `OffsetVertical`, "perpendicular to the tangent at DistanceAlong in the plane of the tangent perpendicular to the global XY plane" | `vertical` along the section up of `StationFrame::Section`: the reference-up frame, leaning back with the grade |
//! | `OffsetLongitudinal`, "parallel to the basis curve after applying DistanceAlong, OffsetLateral, and OffsetVertical" | `longitudinal` along the tangent |
//! | `IfcAxis2PlacementLinear` local `X`, `Y`, `Z` with `Axis`/`RefDirection` absent: the curve's tangent, left and up (8.9.3.4.1 "relative to the curve ... maintaining the relationship to the tangent") | the base frame `(tangent, lateral, up)` |
//! | `Axis`: "the exact direction of the local Z Axis"; `RefDirection` determines local `X`, adjusted to stay orthogonal to `Axis` | `StationOrientation { axis, ref_direction }`, components in the base frame, Gram-Schmidt with the axis primary |
//! | WR2: `Axis` and `RefDirection` not parallel | refused at push within `ORIENTATION_TOLERANCE`; checked here first and refused by name |
//!
//! `IfcParameterValue` distances are refused: IFC4.3 ADD2 states no
//! parameterisation for most bases, and a station is a distance.
//! `OffsetVertical` along the tangent-normal is contested
//! (buildingSMART/IFC4.x-development#1151 proposes global Z); this follows
//! the ADD2 text.
//!
//! # Where the conventions differ: tangent discontinuities
//!
//! 8.9.3.48.3: "If DistanceAlong coincides with a point of tangential
//! discontinuity (within precision limits), then the tangent of the previous
//! segment governs." The neutral evaluators read a seam with the piece that
//! STARTS there (`ElevationLaw::piece_at`, the polyline span), and ADR 0082
//! states no rule. A station on a seam where the tangent is not shown to be
//! continuous is therefore refused by name, as is a run of stations
//! (sections, an offset curve) that spans one: Axiolid interpolates in the
//! frame at each distance, which turns at a kink without the half-angle mitre
//! 8.8.3.35.1 asks for. The `seams` module states how seams are found from stored
//! data. "Within precision limits" is the model's declared `Precision`.
//!
//! # Frames
//!
//! The basis is lowered under the item's frame, like every curve here. Only
//! a frame that is rigid and keeps world `Z` is admitted: a scale changes
//! the measure along the curve, a mirror swaps left and right, and a tilt
//! moves "the global XY plane" the vertical offset is read against.

use axiolid_model::{CurveStation, GeometryNode, NodeId, OrientedCurveStation, Station};
use ifc_model::EntityId;

use crate::error::GeometryResult;
use crate::lower::curve::gradient::keeps_vertical;
use crate::lower::curve::lower_curve_node;
use crate::lower::session::LoweringSession;
use crate::transform::Transform;

pub(crate) mod axes;
pub(crate) mod offset;
pub(crate) mod read;
pub(crate) mod seams;
#[cfg(test)]
mod tests;

/// IFC type of the point.
pub(crate) const POINT: &str = "IFCPOINTBYDISTANCEEXPRESSION";
/// IFC type of the linear placement.
pub(crate) const PLACEMENT: &str = "IFCAXIS2PLACEMENTLINEAR";

/// Family label used for memoization.
const KIND: &str = "station";

/// Why a frame that is not rigid and vertical is refused.
const FRAME: &str = "a station is measured along its basis curve and offset left and up in \
                     its frame; only a frame that is rigid and keeps world Z keeps that \
                     measure, its left and the global XY plane the vertical offset is read \
                     against";

/// A lowered basis curve with what stations along it are checked against.
#[derive(Debug, Clone)]
pub(crate) struct Basis {
    /// The basis curve's node.
    pub node: NodeId,
    /// Where its tangent may be discontinuous, or why that is unknown.
    seams: Result<Vec<f64>, &'static str>,
    /// Its length, when stated by its data; `None` when unbounded or not
    /// stated.
    pub length: Option<f64>,
    /// The model's precision in metres; see [`Basis::tolerance`].
    precision: f64,
}

impl Basis {
    /// Lower `curve` under `frame` as the basis of `owner`.
    pub(crate) fn lower(
        session: &mut LoweringSession<'_>,
        owner: EntityId,
        owner_type: &str,
        curve: EntityId,
        frame: Transform,
    ) -> GeometryResult<Self> {
        if !keeps_vertical(&frame) {
            return Err(session.unsupported(owner, owner_type, FRAME));
        }
        let precision = seams::precision(session, owner, owner_type)?;
        let node = lower_curve_node(session, curve, frame)?;
        let (seams, length) = match session.atomic_curve(node) {
            Some(curve) => (seams::tangent_seams(curve), seams::stated_length(curve)),
            None => (Err(seams::RELATION), None),
        };
        if let Err(reason) = seams {
            if reason == seams::BANKED {
                return Err(session.unsupported(owner, owner_type, seams::BANKED));
            }
        }
        Ok(Self {
            node,
            seams,
            length,
            precision,
        })
    }

    /// How far apart two distances may be and still name the same place:
    /// the declared precision, or rounding at the distance's magnitude.
    pub(crate) fn tolerance(&self, distance: f64) -> f64 {
        self.precision.max(1e-9 * distance.abs().max(1.0))
    }

    /// Refuse a distance outside the curve, by name.
    pub(crate) fn check_on_curve(
        &self,
        session: &LoweringSession<'_>,
        owner: EntityId,
        owner_type: &str,
        distance: f64,
    ) -> GeometryResult<()> {
        if !distance.is_finite() || distance < 0.0 {
            return Err(session.degenerate(
                owner,
                owner_type,
                format!("DistanceAlong {distance} is before the basis curve's start"),
            ));
        }
        if let Some(length) = self.length {
            if distance > length + self.tolerance(length) {
                return Err(session.degenerate(
                    owner,
                    owner_type,
                    format!("DistanceAlong {distance} is beyond the basis curve's length {length}"),
                ));
            }
        }
        Ok(())
    }

    /// Refuse a seam where the tangent is not shown continuous within
    /// `[from, to]`, both ends included, by name.
    pub(crate) fn check_no_seam(
        &self,
        session: &LoweringSession<'_>,
        owner: EntityId,
        owner_type: &str,
        from: f64,
        to: f64,
        reason: &'static str,
    ) -> GeometryResult<()> {
        let seams = self
            .seams
            .as_ref()
            .map_err(|why| session.unsupported(owner, owner_type, why))?;
        let hit = seams
            .iter()
            .any(|seam| *seam >= from - self.tolerance(from) && *seam <= to + self.tolerance(to));
        if hit {
            Err(session.unsupported(owner, owner_type, reason))
        } else {
            Ok(())
        }
    }
}

/// Lower an `IfcPointByDistanceExpression` into a `CurveStation`.
pub fn lower_point_by_distance_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    memoized(session, id, frame, |session| {
        let (basis, station) = located(session, id, POINT, id, frame)?;
        session.node_for(
            id,
            GeometryNode::CurveStation(CurveStation::new(basis.node, station)),
        )
    })
}

/// Lower an `IfcAxis2PlacementLinear` into an `OrientedCurveStation`.
///
/// Its `Location` is the station (WR1); `Axis` and `RefDirection` become
/// the orientation, read in the station's base frame (see the module
/// documentation).
pub fn lower_axis2_placement_linear_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    memoized(session, id, frame, |session| {
        let placement = axes::read_placement(session, id)?;
        let (basis, station) = located(session, id, PLACEMENT, placement.location, frame)?;
        let orientation = axes::placement_orientation(session, id, &placement)?;
        session.node_for(
            id,
            GeometryNode::OrientedCurveStation(OrientedCurveStation::new(
                CurveStation::new(basis.node, station),
                orientation,
            )),
        )
    })
}

/// Why a single station on a seam is refused.
const ON_SEAM: &str = "DistanceAlong falls on a tangent discontinuity of the basis curve, where \
                       IFC4.3 ADD2 (8.9.3.48.3) lets the previous segment's tangent govern; \
                       the neutral station reads the next segment's and states no rule";

/// Read the point `point`, lower its basis and check the station on it.
fn located(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    owner_type: &str,
    point: EntityId,
    frame: Transform,
) -> GeometryResult<(Basis, Station)> {
    let expression = read::distance_expression(session, owner, owner_type, point)?;
    let basis = Basis::lower(session, owner, owner_type, expression.basis, frame)?;
    basis.check_on_curve(session, owner, owner_type, expression.distance)?;
    basis.check_no_seam(
        session,
        owner,
        owner_type,
        expression.distance,
        expression.distance,
        ON_SEAM,
    )?;
    Ok((basis, expression.station()))
}

fn memoized(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
    build: impl FnOnce(&mut LoweringSession<'_>) -> GeometryResult<NodeId>,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, KIND, frame) {
        return Ok(node);
    }
    session.enter(id, KIND)?;
    let result = build(session);
    session.exit(id);
    let node = result?;
    session.memoize(id, KIND, frame, node);
    Ok(node)
}
