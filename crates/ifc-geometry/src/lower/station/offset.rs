//! `IfcOffsetCurveByDistances` (IFC4.3 ADD2 8.9.3.42) onto
//! `CurveRelation::OffsetByStations`, or `OffsetsAtStations` where its
//! stations carry the kernel's seam-snapping window or stand at a basis end
//! the kernel measures.
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
//!   pieces summed, [`super::relation`]). Where the basis is bounded but its
//!   length is a quadrature (an offset beside a gradient curve, a spiral, a
//!   B-spline or an ellipse, a relation with an ellipse or a B-spline piece,
//!   a B-spline or an ellipse itself), this crate cannot state that number
//!   (ADR 0004), and the kernel measures it instead (#433, `axiolid-model`
//!   0.3.12, axiolid/kernel#303): the tail is `RunStation::at_basis_end`
//!   with the last offsets, which the kernel resolves to the basis's length
//!   by its own measure, and the head, where the first offset stops short
//!   of the start, `RunStation::at_basis_start` with the first. Such a run
//!   is wire format 1.3; a basis whose length is stated keeps its stated
//!   stations and its graph. A stated station at or past the end the
//!   kernel measures leaves the run no strictly increasing distances, which
//!   the kernel refuses by name (`SeamWindowRefusal::PastBasisEnd`). An
//!   unbounded line has no end to continue to, so a run along it that stops
//!   short is refused by name ([`NO_LENGTH`]): Axiolid's curve stops at its
//!   last station and does not extend.
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
//!   carry the kernel's seam-snapping window, as `RunStation`s of
//!   `CurveRelation::OffsetsAtStations` (`axiolid-model` 0.3.11,
//!   axiolid/kernel#301), the general form of `OffsetByStations`, which
//!   stays the form of every run whose stations carry neither a window nor
//!   a basis end, so those graphs are unchanged. The kernel snaps a station
//!   onto the one seam within its window after it has resolved the
//!   basis's ends. The end station carries no window: it is the basis's
//!   end, not a station near a seam.
//!
//! `OffsetLongitudinal` exists to reach a point past a tangent
//! discontinuity (8.9.3.48.3); IFC states nothing about carrying it along a
//! curve, so a non-zero one is refused. `Tag` names the curve for sections
//! and changes no shape.

use axiolid_model::{
    BasisEnd, CurveRelation, GeometryNode, NodeId, RunStation, SeamSide, Station, StationFrame,
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

/// Why offsets that stop short of an unbounded end are refused.
const NO_LENGTH: &str =
    "the offsets stop short of the basis curve's ends, where IFC4.3 ADD2 (8.9.3.42.3) \
     continues them unchanged, and the basis is unbounded, so it has no end to continue them \
     to; the neutral offset curve stops at its last station";

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
    match basis.length {
        Some(length) => {
            if head {
                stations.insert(0, Station::new(0.0, carried(first.offsets)).into());
            }
            if last.distance < length - basis.tolerance(length) {
                stations.push(Station::new(length, carried(last.offsets)).into());
            }
        }
        // The length is a quadrature: the kernel measures the end (#433).
        // Whether the last station stops short of it is not known here, so
        // the tail is always added; one at or past it is refused by the
        // kernel by name.
        None if basis.measured_end => {
            if head {
                stations.insert(0, RunStation::at_basis_start(carried(first.offsets)));
            }
            stations.push(RunStation::at_basis_end(carried(last.offsets)));
        }
        None => return Err(session.unsupported(id, TYPE, NO_LENGTH)),
    }
    let from = stations[0].station.distance;
    // The kernel's end lies past every stated station, or the kernel
    // refuses the run's order.
    let to = match stations[stations.len() - 1].basis_end {
        Some(BasisEnd::End) => f64::INFINITY,
        _ => stations[stations.len() - 1].station.distance,
    };
    if stations.len() < 2 || to <= from {
        return Err(session.degenerate(
            id,
            TYPE,
            "the offsets and the basis curve leave no extent for the offset curve",
        ));
    }
    basis.check_run(session, id, TYPE, from, to)?;

    if stations
        .iter()
        .any(|station| station.seam_window > 0.0 || station.basis_end.is_some())
    {
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
/// carries a window, a side or a basis end, so such a graph is the one it
/// always was (wire format 1.0); `OffsetsAtStations` otherwise (format 1.2,
/// or 1.3 with a station at the basis's end, #433).
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

    /// A run continued to the basis's ends the kernel measures takes the
    /// general form and writes wire format 1.3, with or without windows
    /// (#433); it reads back to itself.
    #[cfg(feature = "wire")]
    #[test]
    fn a_run_to_the_basis_end_writes_wire_format_1_3() {
        use axiolid_model::GeometryGraph;

        let offsets = StationOffsets::new(1.0, 0.0, 0.0);
        for window in [Some(1e-5), None] {
            let (mut builder, basis) = line();
            let run = offset_relation(
                basis,
                vec![
                    RunStation::at_basis_start(offsets),
                    windowed(station(5.0), window),
                    RunStation::at_basis_end(offsets),
                ],
            );
            assert!(
                matches!(run, CurveRelation::OffsetsAtStations { .. }),
                "{run:?}"
            );
            let node = builder
                .push(GeometryNode::CurveRelation(run))
                .expect("pushes");
            let graph = builder.finish(vec![node]).expect("a graph");
            let json = graph.to_json().expect("writes");
            assert!(json.contains("\"version\":\"1.3\""), "{json}");
            let read = GeometryGraph::from_json(&json).expect("reads back");
            assert_eq!(read.to_json().expect("writes"), json);
        }
    }
}
