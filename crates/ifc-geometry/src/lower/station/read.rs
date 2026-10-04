//! Reading an `IfcPointByDistanceExpression` (IFC4.3 ADD2 8.9.3.48).

use axiolid_model::{Station, StationOffsets};
use ifc_model::{EntityId, Value};

use super::POINT;
use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;

/// `DistanceAlong`, `OffsetLateral`, `OffsetVertical`, `OffsetLongitudinal`,
/// `BasisCurve`: the entity's own slots, nothing inherited ahead of them.
mod slot {
    pub const DISTANCE_ALONG: usize = 0;
    pub const OFFSET_LATERAL: usize = 1;
    pub const OFFSET_VERTICAL: usize = 2;
    pub const OFFSET_LONGITUDINAL: usize = 3;
    pub const BASIS_CURVE: usize = 4;
}

/// Why an `IfcParameterValue` distance is refused.
pub(crate) const PARAMETER: &str =
    "DistanceAlong is an IfcParameterValue; IFC4.3 ADD2 gives most basis curves no \
     parameterisation a distance can be read from, and a neutral station is a distance";

/// One `IfcPointByDistanceExpression`, in metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DistanceExpression {
    /// The `IfcPointByDistanceExpression` entity.
    pub entity: EntityId,
    /// `BasisCurve`.
    pub basis: EntityId,
    /// `DistanceAlong`, an `IfcLengthMeasure`.
    pub distance: f64,
    /// `OffsetLateral`, when stated.
    pub lateral: Option<f64>,
    /// `OffsetVertical`, when stated.
    pub vertical: Option<f64>,
    /// `OffsetLongitudinal`, when stated.
    pub longitudinal: Option<f64>,
}

impl DistanceExpression {
    /// The neutral station: an absent offset is zero.
    pub(crate) fn station(&self) -> Station {
        Station::new(
            self.distance,
            StationOffsets::new(
                self.lateral.unwrap_or(0.0),
                self.vertical.unwrap_or(0.0),
                self.longitudinal.unwrap_or(0.0),
            ),
        )
    }

    /// Whether any offset is stated, zero included (an `EXISTS` test).
    pub(crate) fn states_an_offset(&self) -> bool {
        self.lateral.is_some() || self.vertical.is_some() || self.longitudinal.is_some()
    }
}

/// Read `point`, which `owner` uses, as an `IfcPointByDistanceExpression`.
///
/// Refusals are attributed to `owner`: a wrong type, a distance that is not
/// a typed `IfcLengthMeasure` (an `IfcParameterValue` is unsupported), and a
/// non-finite length.
pub(crate) fn distance_expression(
    session: &LoweringSession<'_>,
    owner: EntityId,
    owner_type: &str,
    point: EntityId,
) -> GeometryResult<DistanceExpression> {
    let kind = session.type_name(point)?;
    if kind != POINT {
        return Err(session.degenerate(
            owner,
            owner_type,
            format!(
                "a linear position must be an IfcPointByDistanceExpression (WR1), found {kind}"
            ),
        ));
    }
    let slots = session.slots(point)?;
    let units = session.units();
    let distance = match slots.req(slot::DISTANCE_ALONG, "DistanceAlong")? {
        Value::Typed { type_name, value } if type_name.eq_ignore_ascii_case("IFCLENGTHMEASURE") => {
            value.as_f64().map(|raw| units.length(raw))
        }
        Value::Typed { type_name, .. } if type_name.eq_ignore_ascii_case("IFCPARAMETERVALUE") => {
            return Err(session.unsupported(owner, owner_type, PARAMETER))
        }
        _ => None,
    };
    let Some(distance) = distance.filter(|d| d.is_finite()) else {
        return Err(session.degenerate(
            owner,
            owner_type,
            "DistanceAlong must be a finite, typed IfcLengthMeasure (IfcCurveMeasureSelect)",
        ));
    };
    let offset = |index: usize, name: &str| -> GeometryResult<Option<f64>> {
        match slots.opt(index) {
            None => Ok(None),
            Some(_) => match slots.opt_f64(index).map(|raw| units.length(raw)) {
                Some(value) if value.is_finite() => Ok(Some(value)),
                _ => Err(session.degenerate(
                    owner,
                    owner_type,
                    format!("{name} must be a finite IfcLengthMeasure"),
                )),
            },
        }
    };
    Ok(DistanceExpression {
        entity: point,
        basis: slots.req_ref(slot::BASIS_CURVE, "BasisCurve")?,
        distance,
        lateral: offset(slot::OFFSET_LATERAL, "OffsetLateral")?,
        vertical: offset(slot::OFFSET_VERTICAL, "OffsetVertical")?,
        longitudinal: offset(slot::OFFSET_LONGITUDINAL, "OffsetLongitudinal")?,
    })
}
