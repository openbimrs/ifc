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
//! # Tangent discontinuities (#346)
//!
//! 8.9.3.48.3, on `OffsetLateral`: "If DistanceAlong coincides with a point
//! of tangential discontinuity (within precision limits), then the tangent
//! of the previous segment governs." Axiolid states the same choice since
//! `axiolid-model` 0.3.6 (ADR 0082 amendment, axiolid/kernel#263): a
//! station carries a `SeamSide`, and `SeamSide::Incoming` reads the piece
//! that ends at the seam. So a station on a seam is lowered at the seam's own distance with
//! the incoming side: an `IfcAxis2PlacementLinear` as an
//! `OrientedCurveStation` with `SeamSide::Incoming`, an
//! `IfcPointByDistanceExpression` as the unturned oriented station
//! `CurveStation::with_seam_side(SeamSide::Incoming)` gives (a plain
//! `CurveStation` always reads the outgoing piece). Off a seam nothing
//! changes. `OffsetLongitudinal`, which IFC offers "to reach locations for
//! the case of a tangentially discontinuous basis curve", then runs along
//! the incoming tangent.
//!
//! **Which stations are on a seam.** "Within precision limits" is the
//! model's declared `Precision`, capped at 1 mm (`Basis::tolerance`; the
//! same cap `ifc_alignment::SeamTolerance` applies), or rounding,
//! `1e-9 * max(1, |s|)`, where that is larger. Axiolid reads a station on a
//! seam within `ARC_LENGTH_TOLERANCE * max(1, s)` with `ARC_LENGTH_TOLERANCE
//! = 1e-12`, a window inside ours. The two are reconciled by snapping: a
//! station within our window is stored at the seam's distance, read from
//! the same stored data Axiolid reads it from (`seams`), so the kernel finds
//! it on the seam and reads the side stated. Without the snap a station
//! 0.5 mm past a corner in a model of millimetre precision would be on the
//! corner for IFC and on the outgoing leg for the kernel.
//!
//! **Runs.** Sections and offsets need no side. IFC 8.8.3.35.1: "If the
//! directrix is not tangent continuous, the resulting solid is created by a
//! miter at half angle between the two segments" (8.8.3.37.1 says the same
//! of the surface), and Axiolid mitres a run across a seam in the plane normal to the bisector of the two
//! tangents (`Mitre`, ADR 0082 amendment). A station of a run within
//! precision of a seam is snapped to it too, so the kernel stands that
//! section in the mitre plane instead of refusing a section the mitre would
//! cut. "Very sharp edges may result in nearly impossible miter": a seam
//! where the curve turns back on itself (the cosine of half the turn at most
//! Axiolid's `MITRE_TOLERANCE`, `seams::MITRE_TOLERANCE`) inside a run is
//! refused by name.
//!
//! **Relation bases.** A plain `IfcCompositeCurve`, an `IfcTrimmedCurve`
//! and segments placed at stations lower to a curve relation, along which
//! Axiolid measures a station end to end through its pieces since
//! `axiolid-model` 0.3.7 (axiolid/kernel#285, ADR 0082 amendment): each
//! piece in its own curve's convention, every joint a seam whose
//! `SeamSide::Incoming` reads the piece that ends there, IFC's previous
//! segment. The joints' distances are read from the stored relation
//! (`relation`), and a station within precision of one is snapped onto it
//! and reads that side, as on an atomic basis. A relation joining a
//! gradient curve with arc-length pieces is refused by name; a gap or an
//! undeclared reversed piece is refused by the kernel.
//!
//! **Offset bases** (#414). `IfcOffsetCurve2D`, `IfcOffsetCurve3D` and
//! `IfcOffsetCurveByDistances` lower to `CurveRelation::Offset` and
//! `OffsetByStations`, which Axiolid measures a station along since
//! `axiolid-mesh-compile` 0.3.19 (axiolid/kernel#289, ADR 0082 amendment
//! 2026-10-10): in the offset's OWN length from its start, not its basis's
//! (the station names a place along the curve it references), one offset
//! piece per span of the basis between its seams and, by distances, between
//! its stations, every piece boundary a joint read by the same seam rule.
//! IFC states an offset curve's parameterisation, "from the basis curve"
//! (IFC4.3 ADD2 8.9.3.40.1), but not a distance along it other than its
//! length. The joints' distances are the closed forms `relation::offset`
//! states, and where it states none (an offset whose length is a
//! quadrature) the kernel snaps the station (below); an offset of an
//! offset, a trim of an offset and a collapsed circle are refused by name,
//! and an offset across a corner of its basis,
//! whose sides do not meet (IFC: the basis "shall have a well-defined
//! tangent direction at every point"), is refused by the kernel.
//!
//! **Bases whose seams are quadratures** (#423). The seams are read from
//! stored data, never by evaluation. Where they lie at an arc-length
//! integral -- an offset beside a gradient curve, a spiral, a B-spline or
//! an ellipse, a relation with an ellipse or a B-spline piece, a B-spline's
//! corner knot -- this crate cannot place a station within precision of
//! one, and hands the snapping to the kernel instead: the station is
//! stored at its own distance as an `OrientedCurveStation` reading
//! `SeamSide::Incoming`, with the window this crate snaps within (the
//! declared precision, `Basis::tolerance`) as its seam-snapping window
//! (`OrientedCurveStation::with_seam_window`, `axiolid-model` 0.3.9,
//! axiolid/kernel#294, ADR 0082 amendment). Resolving it, the kernel reads
//! a station within that window of exactly one seam whose frame may jump
//! ON that seam, from the incoming side, at its own certified distance, and
//! refuses by name a window holding two seams (`SEAM_WINDOW_AMBIGUOUS`), a
//! seam whose certified position straddles its edge
//! (`SEAM_WINDOW_STRADDLED`) and a seam it cannot certify
//! (`UNCERTIFIED_LENGTH`); `crate::compile` names the station in a
//! [`GeometryError::StationSeamWindowRefused`](crate::GeometryError).
//! A station on a basis whose seams are stored data keeps the snapping
//! above, so its graph is unchanged (no window, wire format 1.0 content):
//! a run of sections or offsets needs that snapping anyway, since a run's
//! `Station`s carry no window, and the two agree on every seam the data
//! states. A run along a basis whose seams are quadratures is refused by
//! name (`RUN_UNLOCATED`).
//!
//! # Frames
//!
//! The basis is lowered under the item's frame, like every curve here. Only
//! a frame that is rigid and keeps world `Z` is admitted: a scale changes
//! the measure along the curve, a mirror swaps left and right, and a tilt
//! moves "the global XY plane" the vertical offset is read against.

use axiolid_model::{CurveStation, GeometryNode, NodeId, OrientedCurveStation, SeamSide, Station};
use ifc_model::EntityId;

use crate::error::GeometryResult;
use crate::lower::curve::gradient::keeps_vertical;
use crate::lower::curve::lower_curve_node;
use crate::lower::session::LoweringSession;
use crate::transform::Transform;

pub(crate) mod axes;
pub(crate) mod offset;
pub(crate) mod read;
pub(crate) mod relation;
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
    seams: Result<Vec<seams::Seam>, &'static str>,
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
            None => match relation::relation_seams(session, node) {
                Ok((seams, length)) => (Ok(seams), Some(length)),
                Err(reason) => (Err(reason), None),
            },
        };
        if let Err(reason) = seams {
            // Refused whatever the station: no station along such a basis
            // resolves, or the kernel reads its frame unlike IFC.
            if [
                seams::BANKED,
                relation::UNSUPPORTED,
                relation::MIXED,
                relation::TRIM_OF_OFFSET,
                relation::OFFSET_OF_OFFSET,
                relation::PLANAR_3D,
                relation::COLLAPSE,
            ]
            .contains(&reason)
            {
                return Err(session.unsupported(owner, owner_type, reason));
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
        seams::window(self.precision, distance)
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

    /// The seams, or a refusal naming why they are unknown.
    fn known_seams(
        &self,
        session: &LoweringSession<'_>,
        owner: EntityId,
        owner_type: &str,
    ) -> GeometryResult<&[seams::Seam]> {
        self.seams
            .as_deref()
            .map_err(|why| session.unsupported(owner, owner_type, why))
    }

    /// How a station at `distance` is read (module documentation): on a
    /// seam stated by the data, snapped to it; clear of every such seam,
    /// as it is; on a basis whose seams are quadratures, with the kernel's
    /// seam-snapping window.
    fn reading(
        &self,
        session: &LoweringSession<'_>,
        owner: EntityId,
        owner_type: &str,
        distance: f64,
    ) -> GeometryResult<Reading> {
        match &self.seams {
            Ok(seams) => Ok(match seams::nearest(seams, distance, self.precision) {
                Some(seam) => Reading::OnSeam(seam.distance),
                None => Reading::Clear,
            }),
            Err(reason) if WINDOWED.contains(reason) => {
                Ok(Reading::Window(self.tolerance(distance)))
            }
            Err(reason) => Err(session.unsupported(owner, owner_type, reason)),
        }
    }

    /// The seams a run needs, refusing a basis whose seams are quadratures
    /// by [`RUN_UNLOCATED`] rather than by the station's reason.
    fn run_seams(
        &self,
        session: &LoweringSession<'_>,
        owner: EntityId,
        owner_type: &str,
    ) -> GeometryResult<&[seams::Seam]> {
        match &self.seams {
            Err(reason) if WINDOWED.contains(reason) => {
                Err(session.unsupported(owner, owner_type, RUN_UNLOCATED))
            }
            _ => self.known_seams(session, owner, owner_type),
        }
    }

    /// The distance a station of a run is stored at: the seam's own
    /// distance when it lies on one, so the kernel stands its section in
    /// the mitre plane (module documentation).
    pub(crate) fn run_distance(
        &self,
        session: &LoweringSession<'_>,
        owner: EntityId,
        owner_type: &str,
        distance: f64,
    ) -> GeometryResult<f64> {
        Ok(seams::nearest(
            self.run_seams(session, owner, owner_type)?,
            distance,
            self.precision,
        )
        .map_or(distance, |seam| seam.distance))
    }

    /// Refuse a run over `[from, to]`, both ends included, across a seam
    /// where the curve turns back on itself: IFC's half-angle mitre
    /// (8.8.3.35.1) has no plane there.
    pub(crate) fn check_run(
        &self,
        session: &LoweringSession<'_>,
        owner: EntityId,
        owner_type: &str,
        from: f64,
        to: f64,
    ) -> GeometryResult<()> {
        let reversal = self
            .run_seams(session, owner, owner_type)?
            .iter()
            .any(|seam| {
                seam.reverses
                    && seam.distance >= from - self.tolerance(from)
                    && seam.distance <= to + self.tolerance(to)
            });
        if reversal {
            Err(session.unsupported(owner, owner_type, REVERSAL))
        } else {
            Ok(())
        }
    }
}

/// The reasons a basis's seams are unknown for which a station carries
/// the kernel's seam-snapping window instead (module documentation): its
/// seams lie at arc-length integrals.
const WINDOWED: [&str; 2] = [relation::INEXACT, seams::SPLINE];

/// How a station is read against its basis's seams.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Reading {
    /// Clear of every seam: stored as it is.
    Clear,
    /// Within precision of the seam at this distance: stored there,
    /// reading the incoming side.
    OnSeam(f64),
    /// On a basis whose seams are quadratures: stored as it is, reading
    /// the incoming side of any seam within this window, which the kernel
    /// snaps to.
    Window(f64),
}

/// Why a run along a basis whose seams are quadratures is refused.
pub(crate) const RUN_UNLOCATED: &str =
    "a run of sections or offsets along a basis whose seams lie at arc-length integrals (an \
     offset whose length is a quadrature, a spiral, a B-spline or an ellipse piece): a station \
     of a run within precision of a seam stands in IFC's half-angle mitre plane (8.8.3.35.1), \
     and a run's stations carry no seam-snapping window for the kernel to snap them with";

/// Why a run across a reversal is refused.
pub(crate) const REVERSAL: &str =
    "the run crosses a tangent discontinuity where the basis curve turns back on itself; \
     IFC4.3 ADD2 (8.8.3.35.1) mitres at half angle there and warns that \"very sharp edges \
     may result in nearly impossible miter\", and the half-angle plane would stretch a \
     section without bound";

/// Lower an `IfcPointByDistanceExpression` into a `CurveStation`, or, on a
/// tangent discontinuity of its basis, into the unturned
/// `OrientedCurveStation` reading the incoming side (module documentation).
pub fn lower_point_by_distance_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    memoized(session, id, frame, |session| {
        let (basis, station, reading) = located(session, id, POINT, id, frame)?;
        let station = CurveStation::new(basis.node, station);
        let node = match reading {
            Reading::Clear => GeometryNode::CurveStation(station),
            _ => GeometryNode::OrientedCurveStation(read_at(station.into(), reading)),
        };
        session.node_for(id, node)
    })
}

/// Lower an `IfcAxis2PlacementLinear` into an `OrientedCurveStation`.
///
/// Its `Location` is the station (WR1); `Axis` and `RefDirection` become
/// the orientation, read in the station's base frame. On a tangent
/// discontinuity it reads the incoming side (see the module documentation).
pub fn lower_axis2_placement_linear_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    memoized(session, id, frame, |session| {
        let oriented = linear_placement_station(session, id, frame)?;
        session.node_for(id, GeometryNode::OrientedCurveStation(oriented))
    })
}

/// The `OrientedCurveStation` an `IfcAxis2PlacementLinear` lowers to, as a
/// value: what places a curve at a station (`InstanceAtStation`, #311).
pub(crate) fn linear_placement_station(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<OrientedCurveStation> {
    let placement = axes::read_placement(session, id)?;
    let (basis, station, reading) = located(session, id, PLACEMENT, placement.location, frame)?;
    let orientation = axes::placement_orientation(session, id, &placement)?;
    let oriented = OrientedCurveStation::new(CurveStation::new(basis.node, station), orientation);
    Ok(read_at(oriented, reading))
}

/// `station` reading the incoming side on a seam, with the kernel's
/// seam-snapping window where the seams are quadratures.
fn read_at(station: OrientedCurveStation, reading: Reading) -> OrientedCurveStation {
    match reading {
        Reading::Clear => station,
        Reading::OnSeam(_) => station.with_seam_side(SeamSide::Incoming),
        Reading::Window(window) => station
            .with_seam_side(SeamSide::Incoming)
            .with_seam_window(window),
    }
}

/// Read the point `point`, lower its basis and place the station on it: at
/// a seam's own distance, reading the incoming side, when it lies on one
/// the data states; with the kernel's window where the seams are
/// quadratures (8.9.3.48.3, module documentation).
fn located(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    owner_type: &str,
    point: EntityId,
    frame: Transform,
) -> GeometryResult<(Basis, Station, Reading)> {
    let expression = read::distance_expression(session, owner, owner_type, point)?;
    let basis = Basis::lower(session, owner, owner_type, expression.basis, frame)?;
    basis.check_on_curve(session, owner, owner_type, expression.distance)?;
    let mut station = expression.station();
    let reading = basis.reading(session, owner, owner_type, expression.distance)?;
    match reading {
        Reading::OnSeam(seam) => station.distance = seam,
        Reading::Window(_) => session.record_seam_window(owner),
        Reading::Clear => {}
    }
    Ok((basis, station, reading))
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
