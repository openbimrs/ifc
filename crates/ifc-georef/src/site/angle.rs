//! `IfcCompoundPlaneAngleMeasure = LIST [3:4] OF INTEGER`, validated against
//! the declared release's own WHERE rules and converted to decimal degrees.
//!
//! The rules differ between releases (`IFC2X3_TC1.exp` versus `IFC4.exp` and
//! `IFC4X3_ADD2.exp`, which agree):
//!
//! | IFC2X3 | IFC4, IFC4X3 |
//! | --- | --- |
//! | `WR1 : -360 <= SELF[1] < 360` | none on degrees |
//! | `WR2 : -60 <= SELF[2] < 60` | `MinutesInRange : ABS(SELF[2]) < 60` |
//! | `WR3 : -60 <= SELF[3] < 60` | `SecondsInRange : ABS(SELF[3]) < 60` |
//! | none on millionths | `MicrosecondsInRange : ABS(SELF[4]) < 1000000` |
//! | `WR4`: first three components share a sign | `ConsistentSign`: all components share a sign |
//!
//! After the type's rules, the WGS84 range the `IfcSite` attribute
//! definitions state is checked: latitude within `[-90, 90]`, longitude
//! within `[-180, 180]` degrees.

use ifc_model::value::Value;
use ifc_model::EntityId;
use ifc_schema::SchemaVersion;

use crate::error::{GeorefError, GeorefResult};

/// An `IfcCompoundPlaneAngleMeasure` as authored: signed integer degrees,
/// minutes, seconds and optional millionths of a second.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompoundPlaneAngle {
    /// Whole degrees (`SELF[1]`).
    pub degrees: i64,
    /// Minutes (`SELF[2]`).
    pub minutes: i64,
    /// Seconds (`SELF[3]`).
    pub seconds: i64,
    /// Millionths of a second (`SELF[4]`), when the list has four members.
    pub millionths: Option<i64>,
}

impl CompoundPlaneAngle {
    /// The angle in decimal degrees: the signed sum
    /// `degrees + minutes/60 + seconds/3600 + millionths/3.6e9`.
    ///
    /// The components share one sign under every release's sign rule
    /// (IFC2X3 `WR4` leaves the fourth free; it is summed with its own sign).
    #[must_use]
    pub fn decimal_degrees(&self) -> f64 {
        self.degrees as f64
            + self.minutes as f64 / 60.0
            + self.seconds as f64 / 3600.0
            + self.millionths.unwrap_or(0) as f64 / 3_600_000_000.0
    }
}

/// Which geographic coordinate an angle is, fixing its WGS84 range.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Axis {
    Latitude,
    Longitude,
}

impl Axis {
    const fn name(self) -> &'static str {
        match self {
            Self::Latitude => "RefLatitude",
            Self::Longitude => "RefLongitude",
        }
    }
}

/// Read an optional compound angle at `index` of `entity`.
pub(crate) fn read_angle(
    value: Option<&Value>,
    entity: EntityId,
    index: usize,
    axis: Axis,
    version: SchemaVersion,
) -> GeorefResult<Option<CompoundPlaneAngle>> {
    let name = axis.name();
    let invalid = GeorefError::InvalidAttribute {
        entity,
        index,
        name,
    };
    let items = match value.map(Value::unwrap_typed) {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::List(items)) => items,
        Some(_) => return Err(invalid),
    };
    // `LIST [3:4] OF INTEGER`: a real would invent or drop precision.
    if !(3..=4).contains(&items.len()) {
        return Err(invalid);
    }
    let mut parts = [0_i64; 4];
    for (slot, item) in parts.iter_mut().zip(items) {
        *slot = item.unwrap_typed().as_i64().ok_or(invalid.clone())?;
    }
    let angle = CompoundPlaneAngle {
        degrees: parts[0],
        minutes: parts[1],
        seconds: parts[2],
        millionths: (items.len() == 4).then_some(parts[3]),
    };
    let violation = |rule| GeorefError::InvalidCompoundAngle {
        entity,
        index,
        name,
        rule,
    };
    if let Some(rule) = broken_rule(&angle, version) {
        return Err(violation(rule));
    }
    let degrees = angle.decimal_degrees();
    let (limit, rule) = match axis {
        Axis::Latitude => (90.0, "WGS84 latitude range [-90, 90]"),
        Axis::Longitude => (180.0, "WGS84 longitude range [-180, 180]"),
    };
    if degrees.abs() > limit {
        return Err(violation(rule));
    }
    Ok(Some(angle))
}

/// The first WHERE rule of the declared release the angle breaks.
fn broken_rule(angle: &CompoundPlaneAngle, version: SchemaVersion) -> Option<&'static str> {
    let CompoundPlaneAngle {
        degrees: d,
        minutes: m,
        seconds: s,
        millionths: u,
    } = *angle;
    if version == SchemaVersion::Ifc2x3 {
        let same_sign = (d >= 0 && m >= 0 && s >= 0) || (d <= 0 && m <= 0 && s <= 0);
        return if !(-360..360).contains(&d) {
            Some("WR1")
        } else if !(-60..60).contains(&m) {
            Some("WR2")
        } else if !(-60..60).contains(&s) {
            Some("WR3")
        } else if !same_sign {
            Some("WR4")
        } else {
            None
        };
    }
    let u_all = u.unwrap_or(0);
    let same_sign =
        (d >= 0 && m >= 0 && s >= 0 && u_all >= 0) || (d <= 0 && m <= 0 && s <= 0 && u_all <= 0);
    if m.unsigned_abs() >= 60 {
        Some("MinutesInRange")
    } else if s.unsigned_abs() >= 60 {
        Some("SecondsInRange")
    } else if u.is_some_and(|u| u.unsigned_abs() >= 1_000_000) {
        Some("MicrosecondsInRange")
    } else if !same_sign {
        Some("ConsistentSign")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(parts: &[i64]) -> Value {
        Value::List(parts.iter().map(|p| Value::Integer(*p)).collect())
    }

    fn read(parts: &[i64], axis: Axis, version: SchemaVersion) -> GeorefResult<f64> {
        read_angle(Some(&list(parts)), EntityId(1), 9, axis, version)
            .map(|a| a.expect("present").decimal_degrees())
    }

    fn rule_of(result: GeorefResult<f64>) -> &'static str {
        match result {
            Err(GeorefError::InvalidCompoundAngle { rule, .. }) => rule,
            other => panic!("expected a compound-angle violation, got {other:?}"),
        }
    }

    #[test]
    fn converts_degrees_minutes_seconds_and_millionths() {
        let v = read(&[41, 53, 30, 500_000], Axis::Latitude, SchemaVersion::Ifc4).unwrap();
        assert!((v - (41.0 + 53.0 / 60.0 + 30.5 / 3600.0)).abs() < 1e-12);
        let w = read(&[-87, -35, -40], Axis::Longitude, SchemaVersion::Ifc4x3).unwrap();
        assert!((w + (87.0 + 35.0 / 60.0 + 40.0 / 3600.0)).abs() < 1e-12);
    }

    #[test]
    fn ifc4_and_ifc4x3_rules_are_named() {
        for version in [SchemaVersion::Ifc4, SchemaVersion::Ifc4x3] {
            let lat = Axis::Latitude;
            assert_eq!(rule_of(read(&[1, 60, 0], lat, version)), "MinutesInRange");
            assert_eq!(rule_of(read(&[1, 0, -60], lat, version)), "SecondsInRange");
            assert_eq!(
                rule_of(read(&[1, 0, 0, 1_000_000], lat, version)),
                "MicrosecondsInRange"
            );
            assert_eq!(rule_of(read(&[1, -2, 0], lat, version)), "ConsistentSign");
            assert_eq!(
                rule_of(read(&[-1, 0, 0, 5], lat, version)),
                "ConsistentSign"
            );
        }
    }

    #[test]
    fn ifc2x3_rules_are_its_own() {
        let v = SchemaVersion::Ifc2x3;
        let lon = Axis::Longitude;
        assert_eq!(rule_of(read(&[360, 0, 0], lon, v)), "WR1");
        assert_eq!(rule_of(read(&[1, 60, 0], lon, v)), "WR2");
        assert_eq!(rule_of(read(&[1, 0, 60], lon, v)), "WR3");
        assert_eq!(rule_of(read(&[1, -1, 0], lon, v)), "WR4");
        // WR4 constrains only the first three members; IFC2X3 has no
        // millionths range rule, so these are valid there but not in IFC4.
        assert!(read(&[-1, 0, 0, 5], lon, v).is_ok());
        assert!(read(&[1, 0, 0, 2_000_000], lon, v).is_ok());
        assert_eq!(
            rule_of(read(&[1, 0, 0, 2_000_000], lon, SchemaVersion::Ifc4)),
            "MicrosecondsInRange"
        );
    }

    #[test]
    fn wgs84_ranges_are_enforced_per_axis() {
        let v = SchemaVersion::Ifc4;
        assert!(read(&[90, 0, 0], Axis::Latitude, v).is_ok());
        assert_eq!(
            rule_of(read(&[90, 0, 1], Axis::Latitude, v)),
            "WGS84 latitude range [-90, 90]"
        );
        assert!(read(&[-180, 0, 0], Axis::Longitude, v).is_ok());
        assert_eq!(
            rule_of(read(&[181, 0, 0], Axis::Longitude, v)),
            "WGS84 longitude range [-180, 180]"
        );
    }

    #[test]
    fn malformed_lists_are_invalid_attributes() {
        for value in [
            list(&[1, 2]),
            list(&[1, 2, 3, 4, 5]),
            Value::List(vec![Value::Integer(1), Value::Real(2.5), Value::Integer(0)]),
            Value::Real(41.5),
        ] {
            assert!(matches!(
                read_angle(
                    Some(&value),
                    EntityId(1),
                    9,
                    Axis::Latitude,
                    SchemaVersion::Ifc4
                ),
                Err(GeorefError::InvalidAttribute {
                    index: 9,
                    name: "RefLatitude",
                    ..
                })
            ));
        }
        assert_eq!(
            read_angle(
                Some(&Value::Null),
                EntityId(1),
                9,
                Axis::Latitude,
                SchemaVersion::Ifc4
            ),
            Ok(None)
        );
    }
}
