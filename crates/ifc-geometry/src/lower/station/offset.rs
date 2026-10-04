//! `IfcOffsetCurveByDistances` (IFC4.3 ADD2 8.9.3.42) onto
//! `CurveRelation::OffsetByStations`.
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
//!   states its length ([`super::seams::stated_length`]). Without one (an
//!   unbounded line, a B-spline, a relation) the curve is refused by name:
//!   Axiolid's curve stops at its last station and does not extend.
//!
//! `OffsetLongitudinal` exists to reach a point past a tangent
//! discontinuity (8.9.3.48.3); IFC states nothing about carrying it along a
//! curve, so a non-zero one is refused. A run across a tangent discontinuity
//! is refused too: the offset curve has a gap or a loop there, and IFC
//! states no join. `Tag` names the curve for sections and changes no shape.

use axiolid_model::{CurveRelation, GeometryNode, NodeId, Station, StationFrame, StationOffsets};
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

/// Why a run over a seam is refused.
const OVER_SEAM: &str =
    "the offsets run over a tangent discontinuity of the basis curve, where an offset curve \
     has a gap or a loop and IFC4.3 ADD2 states no join";

/// Why offsets that stop short of an unbounded or unstated end are refused.
const NO_LENGTH: &str =
    "the offsets stop short of the basis curve's ends, where IFC4.3 ADD2 (8.9.3.42.3) \
     continues them unchanged, and the basis states no length to continue them to; the \
     neutral offset curve stops at its last station";

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

    let mut stations: Vec<Station> = Vec::with_capacity(expressions.len() + 2);
    for expression in &expressions {
        basis.check_on_curve(session, id, TYPE, expression.distance)?;
        if let Some(previous) = stations.last() {
            if expression.distance <= previous.distance {
                return Err(session.degenerate(
                    id,
                    TYPE,
                    "OffsetValues are sequential: their DistanceAlong must increase strictly",
                ));
            }
        }
        let mut station = expression.station();
        station.offsets.longitudinal = 0.0;
        stations.push(station);
    }

    // Constant continuation to the ends (8.9.3.42.3), where it is needed.
    let first = stations[0];
    let last = stations[stations.len() - 1];
    let head = first.distance > basis.tolerance(first.distance);
    let tail = match basis.length {
        Some(length) => last.distance < length - basis.tolerance(length),
        None => true,
    };
    if head || tail {
        let Some(length) = basis.length else {
            return Err(session.unsupported(id, TYPE, NO_LENGTH));
        };
        if head {
            stations.insert(0, Station::new(0.0, carried(first.offsets)));
        }
        if tail {
            stations.push(Station::new(length, carried(last.offsets)));
        }
    }
    let from = stations[0].distance;
    let to = stations[stations.len() - 1].distance;
    if stations.len() < 2 || to <= from {
        return Err(session.degenerate(
            id,
            TYPE,
            "the offsets and the basis curve leave no extent for the offset curve",
        ));
    }
    basis.check_no_seam(session, id, TYPE, from, to, OVER_SEAM)?;

    session.node_for(
        id,
        GeometryNode::CurveRelation(CurveRelation::OffsetByStations {
            basis: basis.node,
            stations,
            frame: StationFrame::Section,
        }),
    )
}

/// The lateral and vertical offsets carried on unchanged.
fn carried(offsets: StationOffsets) -> StationOffsets {
    StationOffsets::new(offsets.lateral, offsets.vertical, 0.0)
}
