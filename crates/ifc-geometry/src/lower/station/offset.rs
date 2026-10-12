//! `IfcOffsetCurveByDistances` (IFC4.3 ADD2 8.9.3.42) onto
//! `CurveRelation::OffsetByStations`, or `OffsetsAtStations` where its
//! stations carry the kernel's seam-snapping window.
//!
//! Each `OffsetValues` member is a station along `BasisCurve`; the curve
//! runs through the offsets. Two readings need stating:
//!
//! - **Between stations.** IFC4.3 ADD2 gives no law between `OffsetValues`
//!   for this entity; it states linear interpolation for the sweeps built on
//!   the same stations (8.8.3.35.1 "linear interpolation between profile
//!   points", 8.8.3.37.1 "linear interpolation is assumed"), and
//!   `OffsetByStations` interpolates linearly in distance. That is the
//!   reading taken.
//! - **Beyond the first and last.** "If the offsets do not span the full
//!   extent of the basis curve (e.g. if the list contains only one item),
//!   then the lateral and vertical offsets implicitly continue with the same
//!   value towards the head and tail of the basis curve." That is stated
//!   data, not extrapolation: a station at the start and one at the end with
//!   the same offsets carry it exactly, and they are added when the basis
//!   states its length ([`super::seams::stated_length`], or a relation's
//!   pieces summed, [`super::relation`]). Without one (an unbounded line, a
//!   B-spline, a relation with an ellipse or a B-spline piece) the curve is
//!   refused by name:
//!   Axiolid's curve stops at its last station and does not extend.
//!
//! - **Across a tangent discontinuity** (#346). The offset curve would
//!   have a gap or a loop there, and 8.9.3.42 states no join. The sweeps
//!   built on the same stations do: "a miter at half angle between the two
//!   segments" (8.8.3.35.1, 8.8.3.37.1). Axiolid mitres an offset run as it
//!   does a run of sections, in the plane normal to the bisector of the two
//!   tangents, so the offset point at the seam is where the two legs'
//!   offsets meet, and a breakline offset this way stays on the surface
//!   swept along the same directrix. Stations within precision of the seam
//!   are snapped to it (`super`'s module documentation); a seam where the
//!   basis turns back on itself is refused by name.
//! - **Along a basis whose seams are quadratures** (#429). Its stations
//!   would carry the kernel's seam-snapping window, as `RunStation`s of
//!   `CurveRelation::OffsetsAtStations` (`axiolid-model` 0.3.11,
//!   axiolid/kernel#301), the general form of `OffsetByStations`, which
//!   stays the form of every window-free run, so those graphs are
//!   unchanged. Such a basis's length is a quadrature too, which this
//!   crate does not state, so the continuation to its tail cannot be
//!   placed and the curve is refused by name ([`NO_QUADRATURE_LENGTH`],
//!   #433) until the kernel can continue a run to the basis's end.
//!
//! `OffsetLongitudinal` exists to reach a point past a tangent
//! discontinuity (8.9.3.48.3); IFC states nothing about carrying it along a
//! curve, so a non-zero one is refused. `Tag` names the curve for sections
//! and changes no shape.

use axiolid_model::{
    CurveRelation, GeometryNode, NodeId, RunStation, SeamSide, Station, StationFrame,
    StationOffsets,
};
use ifc_model::EntityId;

use super::read::distance_expression;
use super::Basis;
use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;
use crate::transform::Transform;

/// IFC type of the offset curve.
pub(crate) const TYPE: &str = "IFCOFFSETCURVEBYDISTANCES";

/// `BasisCurve` (inherited from `IfcOffsetCurve`), `OffsetValues`, `Tag`.
mod slot {
    pub const BASIS_CURVE: usize = 0;
    pub const OFFSET_VALUES: usize = 1;
}

/// Why a non-zero longitudinal offset is refused.
const LONGITUDINAL: &str =
    "an OffsetValues member states a non-zero OffsetLongitudinal, which IFC4.3 ADD2 defines \
     for reaching a point past a tangent discontinuity (8.9.3.48.3), not as a law along a \
     curve";

/// Why an offset member on another basis is refused.
const OTHER_BASIS: &str =
    "an OffsetValues member is measured along another BasisCurve than the offset curve's own";

/// Why offsets that stop short of an unbounded or unstated end are refused.
const NO_LENGTH: &str =
    "the offsets stop short of the basis curve's ends, where IFC4.3 ADD2 (8.9.3.42.3) \
     continues them unchanged, and the basis states no length to continue them to; the \
     neutral offset curve stops at its last station";

/// Why offsets along a basis whose length is a quadrature are refused.
// TODO(#433): continue the run to the basis's end once the kernel can.
pub(crate) const NO_QUADRATURE_LENGTH: &str =
    "the offsets run along a basis whose seams and length are arc-length integrals (an offset \
     beside a gradient curve, a spiral, a B-spline or an ellipse, or a B-spline corner knot); \
     IFC4.3 ADD2 (8.9.3.42.3) continues them unchanged to the basis's end, whose distance this \
     crate does not compute, and the neutral offset curve stops at its last station (#433)";

/// Lower an `IfcOffsetCurveByDistances` to `OffsetByStations`.
pub(crate) fn offset_curve_by_distances(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let slots = session.slots(id)?;
    let basis_curve = slots.req_ref(slot::BASIS_CURVE, "BasisCurve")?;
    let members = slots.req_ref_list(slot::OFFSET_VALUES, "OffsetValues")?;
    if members.is_empty() {
        return Err(session.degenerate(id, TYPE, "OffsetValues is LIST [1:?] and is empty"));
    }
    let mut expressions = Vec::with_capacity(members.len());
    for member in members {
        let expression = distance_expression(session, id, TYPE, member)?;
        if expression.basis != basis_curve {
            return Err(session.unsupported(id, TYPE, OTHER_BASIS));
        }
        if expression.longitudinal.is_some_and(|g| g != 0.0) {
            return Err(session.unsupported(id, TYPE, LONGITUDINAL));
        }
        expressions.push(expression);
    }
    let basis = Basis::lower(session, id, TYPE, basis_curve, frame)?;

    let mut stations: Vec<RunStation> = Vec::with_capacity(expressions.len() + 2);
    for expression in &expressions {
        basis.check_on_curve(session, id, TYPE, expression.distance)?;
        let place = basis.run_place(session, id, TYPE, expression.distance)?;
        if let Some(previous) = stations.last() {
            if place.distance <= previous.station.distance {
                return Err(session.degenerate(
                    id,
                    TYPE,
                    "OffsetValues are sequential: their DistanceAlong must increase strictly, \
                     by more than the model's precision at a tangent discontinuity",
                ));
            }
        }
        let mut station = expression.station();
        station.distance = place.distance;
        station.offsets.longitudinal = 0.0;
        stations.push(windowed(station, place.window));
    }

    // Constant continuation to the ends (8.9.3.42.3), where it is needed.
    let first = stations[0].station;
    let last = stations[stations.len() - 1].station;
    let head = first.distance > basis.tolerance(first.distance);
    let tail = match basis.length {
        Some(length) => last.distance < length - basis.tolerance(length),
        None => true,
    };
    if head || tail {
        let Some(length) = basis.length else {
            let windowed = stations.iter().any(|station| station.seam_window > 0.0);
            let reason = if windowed {
                NO_QUADRATURE_LENGTH
            } else {
                NO_LENGTH
            };
            return Err(session.unsupported(id, TYPE, reason));
        };
        if head {
            stations.insert(0, Station::new(0.0, carried(first.offsets)).into());
        }
        if tail {
            stations.push(Station::new(length, carried(last.offsets)).into());
        }
    }
    let from = stations[0].station.distance;
    let to = stations[stations.len() - 1].station.distance;
    if stations.len() < 2 || to <= from {
        return Err(session.degenerate(
            id,
            TYPE,
            "the offsets and the basis curve leave no extent for the offset curve",
        ));
    }
    basis.check_run(session, id, TYPE, from, to)?;

    if stations.iter().any(|station| station.seam_window > 0.0) {
        session.record_seam_window_run(id);
    }
    session.node_for(
        id,
        GeometryNode::CurveRelation(offset_relation(basis.node, stations)),
    )
}

/// `station` with the kernel's seam-snapping `window`, reading the
/// incoming side, where its basis's seams are quadratures (#429).
fn windowed(station: Station, window: Option<f64>) -> RunStation {
    match window {
        Some(window) => RunStation::new(station)
            .with_seam_side(SeamSide::Incoming)
            .with_seam_window(window),
        None => station.into(),
    }
}

/// The offset curve through `stations`: `OffsetByStations` while none
/// carries a window or a side, so a window-free graph is the one it always
/// was (wire format 1.0); `OffsetsAtStations` otherwise (format 1.2).
fn offset_relation(basis: NodeId, stations: Vec<RunStation>) -> CurveRelation {
    let plain = stations
        .iter()
        .all(|station| *station == RunStation::from(station.station));
    if plain {
        CurveRelation::OffsetByStations {
            basis,
            stations: stations
                .into_iter()
                .map(|station| station.station)
                .collect(),
            frame: StationFrame::Section,
        }
    } else {
        CurveRelation::OffsetsAtStations {
            basis,
            stations,
            frame: StationFrame::Section,
        }
    }
}

/// The lateral and vertical offsets carried on unchanged.
fn carried(offsets: StationOffsets) -> StationOffsets {
    StationOffsets::new(offsets.lateral, offsets.vertical, 0.0)
}

#[cfg(test)]
mod tests {
    use axiolid_core::{Point3, Vec3};
    use axiolid_curve::{Curve3, Line3};
    use axiolid_model::GeometryGraphBuilder;

    use super::*;

    fn station(distance: f64) -> Station {
        Station::new(distance, StationOffsets::new(1.0, 0.0, 0.0))
    }

    /// A builder holding a line along +X, and the line.
    fn line() -> (GeometryGraphBuilder, NodeId) {
        let mut builder = GeometryGraphBuilder::new();
        let line = builder
            .push(GeometryNode::Curve3(Curve3::Line(Line3 {
                origin: Point3::ZERO,
                direction: Vec3::X,
            })))
            .expect("a line");
        (builder, line)
    }

    fn windowed_run(basis: NodeId) -> CurveRelation {
        offset_relation(
            basis,
            vec![
                windowed(station(0.0), Some(1e-5)),
                windowed(station(5.0), Some(1e-5)),
            ],
        )
    }

    /// Window-free stations keep the legacy `OffsetByStations`, so its
    /// graph and wire version are unchanged; windowed ones make the general
    /// form, carrying the window and the incoming side (#429).
    #[test]
    fn only_a_windowed_run_takes_the_general_form() {
        let (_, basis) = line();
        let plain = offset_relation(
            basis,
            vec![windowed(station(0.0), None), windowed(station(5.0), None)],
        );
        assert_eq!(
            plain,
            CurveRelation::OffsetByStations {
                basis,
                stations: vec![station(0.0), station(5.0)],
                frame: StationFrame::Section,
            }
        );
        let general = windowed_run(basis);
        let CurveRelation::OffsetsAtStations { stations, .. } = general else {
            panic!("the general form: {general:?}");
        };
        assert_eq!(stations.len(), 2);
        for run in stations {
            assert_eq!(run.seam, SeamSide::Incoming);
            assert_eq!(run.seam_window, 1e-5);
        }
    }

    /// A graph holding a windowed offset run writes wire format 1.2, and
    /// reads back to itself; the same run without windows writes 1.0.
    #[cfg(feature = "wire")]
    #[test]
    fn a_windowed_offset_run_writes_wire_format_1_2() {
        use axiolid_model::GeometryGraph;

        for (window, version) in [(Some(1e-5), "1.2"), (None, "1.0")] {
            let (mut builder, basis) = line();
            let run = offset_relation(
                basis,
                vec![
                    windowed(station(0.0), window),
                    windowed(station(5.0), window),
                ],
            );
            let node = builder
                .push(GeometryNode::CurveRelation(run))
                .expect("pushes");
            let graph = builder.finish(vec![node]).expect("a graph");
            let json = graph.to_json().expect("writes");
            assert!(
                json.contains(&format!("\"version\":\"{version}\"")),
                "{json}"
            );
            let read = GeometryGraph::from_json(&json).expect("reads back");
            assert_eq!(read.to_json().expect("writes"), json);
        }
    }
}
