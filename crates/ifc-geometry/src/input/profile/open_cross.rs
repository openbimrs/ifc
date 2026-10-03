//! `IfcOpenCrossProfileDef` (IFC4X3): read, check and resolve to vertices.
//!
//! The profile is an open chain of straight segments. Each segment is a
//! `Widths[i]` paired with a `Slopes[i]`; the chain starts at `OffsetPoint`
//! (the profile origin when omitted) and each segment starts where the
//! previous one ends.
//!
//! # The slope convention
//!
//! IFC4.3 ADD2 (figure for `IfcOpenCrossProfileDef`): "Positive sense of
//! PlaneAngleMeasure values in Slopes is clockwise and starting from positive
//! x-axis according to the underlying position coordinate system". That
//! figure draws the profile X axis pointing LEFT (the profile X axis is "to
//! the left of the Directrix ... as facing forward") and Y up, so a turn that
//! reads clockwise on the page carries +X towards +Y: the ordinary positive
//! sense in the profile's own coordinates. A segment with slope `s`
//! therefore runs along `(cos s, sin s)`.
//!
//! `HorizontalWidths` says what the width measures:
//!
//! - `.F.`: the length along the slope, so the step is `w (cos s, sin s)`;
//! - `.T.`: the horizontal run, so the step is the multiple of
//!   `(cos s, sin s)` whose X component has magnitude `w`:
//!   `w (sgn cos s, |tan s| sgn sin s)`, i.e. `w / |cos s| (cos s, sin s)`.
//!   The schema forbids a vertical slope here, and so does this reader.
//!
//! The vertices are this closed-form arithmetic on authored values; nothing is
//! sampled or approximated.

use ifc_model::{Model, Value};

use super::ProfileParameters;
use crate::error::GeometryResult;
use crate::slots::{section_slot as slot, Slots};
use crate::units::UnitScale;

/// Read `IfcOpenCrossProfileDef`'s attributes into SI units, checking the
/// WHERE rules that make the vertex construction well defined.
///
/// Refused as [`crate::GeometryError::Degenerate`]: an empty or
/// mismatched `Widths`/`Slopes` pair (`CorrespondingSlopeWidths`), a `Tags`
/// list that is not one longer than `Widths` (`CorrespondingTags`), a
/// negative or non-finite width, a non-finite slope, a vertical slope with
/// horizontal widths, and an `OffsetPoint` that is not at least 2D.
pub(super) fn read(
    model: &Model,
    units: &UnitScale,
    slots: &Slots<'_>,
) -> GeometryResult<ProfileParameters> {
    let horizontal_widths = slots.req_bool(slot::OC_HORIZONTAL_WIDTHS, "HorizontalWidths")?;
    let widths = slots.req_f64_list(slot::OC_WIDTHS, "Widths")?;
    let slopes = slots.req_f64_list(slot::OC_SLOPES, "Slopes")?;
    if widths.is_empty() {
        return Err(slots.degenerate("Widths is empty; the schema requires LIST [1:?]"));
    }
    if widths.len() != slopes.len() {
        return Err(slots.degenerate(format!(
            "CorrespondingSlopeWidths: {} widths but {} slopes",
            widths.len(),
            slopes.len()
        )));
    }
    if let Some(index) = widths.iter().position(|w| !w.is_finite() || *w < 0.0) {
        return Err(slots.degenerate(format!(
            "width {index} is negative or not finite (IfcNonNegativeLengthMeasure)"
        )));
    }
    if let Some(index) = slopes.iter().position(|s| !s.is_finite()) {
        return Err(slots.degenerate(format!("slope {index} is not finite")));
    }
    let widths: Vec<f64> = widths.into_iter().map(|w| units.length(w)).collect();
    let slopes: Vec<f64> = slopes.into_iter().map(|s| units.angle(s)).collect();
    if horizontal_widths {
        if let Some(index) = slopes.iter().position(|s| is_vertical(*s)) {
            return Err(slots.degenerate(format!(
                "slope {index} is vertical, which has no horizontal width \
                 (HorizontalWidths=.T. forbids +/-90 degrees)"
            )));
        }
    }

    let tags = match slots.opt(slot::OC_TAGS) {
        None | Some(Value::Null) => None,
        Some(value) => {
            let items = value
                .as_list()
                .ok_or_else(|| slots.kind_error("Tags", "a list", value))?;
            let mut tags = Vec::with_capacity(items.len());
            for item in items {
                match item.unwrap_typed() {
                    Value::Text(text) => tags.push(text.to_string()),
                    other => return Err(slots.kind_error("Tags", "a list of labels", other)),
                }
            }
            if tags.len() != widths.len() + 1 {
                return Err(slots.degenerate(format!(
                    "CorrespondingTags: {} tags for {} segments; expected {}",
                    tags.len(),
                    widths.len(),
                    widths.len() + 1
                )));
            }
            Some(tags)
        }
    };

    let offset_point = match slots.opt_ref(slot::OC_OFFSET_POINT) {
        None => None,
        Some(point) => {
            let entity = slots.resolve(model, point)?;
            let coordinates = Slots::new(point, entity).req_f64_list(0, "Coordinates")?;
            if coordinates.len() < 2 {
                return Err(slots.degenerate("OffsetPoint is not at least 2D"));
            }
            Some([units.length(coordinates[0]), units.length(coordinates[1])])
        }
    };

    Ok(ProfileParameters::OpenCross {
        horizontal_widths,
        widths,
        slopes,
        tags,
        offset_point,
    })
}

/// Is this slope, in radians, vertical: is its cosine zero in `f64`?
///
/// `cos(pi / 2)` evaluates to about `6e-17`, not zero, so the test is against
/// the machine epsilon rather than equality. This bounds representability, not
/// geometry: a horizontal run of `w` would otherwise rise by `w * 1.6e16`.
fn is_vertical(slope: f64) -> bool {
    slope.cos().abs() < f64::EPSILON
}

/// The chain's vertices, in metres, from already-checked SI parameters.
///
/// One more vertex than there are segments, in authored order, so vertex `i`
/// carries `Tags[i]`. A zero width yields a repeated vertex; it is kept,
/// because dropping it would shift every later tag onto the wrong point.
#[cfg_attr(not(feature = "lowering"), allow(dead_code))]
pub(crate) fn vertices(
    horizontal_widths: bool,
    widths: &[f64],
    slopes: &[f64],
    offset_point: Option<[f64; 2]>,
) -> Vec<[f64; 2]> {
    let mut point = offset_point.unwrap_or([0.0, 0.0]);
    let mut out = Vec::with_capacity(widths.len() + 1);
    out.push(point);
    for (width, slope) in widths.iter().zip(slopes) {
        let (sin, cos) = slope.sin_cos();
        // Along the slope the width is the segment length. Horizontally it is
        // the X extent, so the length is w / |cos s|.
        let length = if horizontal_widths {
            width / cos.abs()
        } else {
            *width
        };
        point = [point[0] + length * cos, point[1] + length * sin];
        out.push(point);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::vertices;
    use std::f64::consts::FRAC_PI_4;

    fn close(a: [f64; 2], b: [f64; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-12 && (a[1] - b[1]).abs() < 1e-12
    }

    /// Along-slope widths are segment lengths.
    #[test]
    fn along_slope_widths_are_lengths() {
        // 45 degrees, length sqrt(2): one unit across and one up.
        let v = vertices(false, &[2f64.sqrt()], &[FRAC_PI_4], None);
        assert!(close(v[1], [1.0, 1.0]), "{v:?}");
    }

    /// Horizontal widths are X extents, whichever way the slope points.
    #[test]
    fn horizontal_widths_are_x_extents() {
        // 45 degrees with a 2 m run rises 2 m.
        let v = vertices(true, &[2.0], &[FRAC_PI_4], None);
        assert!(close(v[1], [2.0, 2.0]), "{v:?}");
        // 135 degrees (towards -X, rising) with a 2 m run.
        let v = vertices(true, &[2.0], &[3.0 * FRAC_PI_4], None);
        assert!(close(v[1], [-2.0, 2.0]), "{v:?}");
    }

    /// The chain starts at the offset point and accumulates.
    #[test]
    fn the_chain_starts_at_the_offset_point() {
        let v = vertices(
            false,
            &[1.0, 0.0, 1.0],
            &[0.0, 0.0, -FRAC_PI_4],
            Some([5.0, 1.0]),
        );
        assert_eq!(v.len(), 4);
        assert!(close(v[0], [5.0, 1.0]));
        assert!(close(v[1], [6.0, 1.0]));
        assert!(close(v[2], [6.0, 1.0]), "a zero width repeats its vertex");
        let h = 0.5f64.sqrt();
        assert!(close(v[3], [6.0 + h, 1.0 - h]), "{v:?}");
    }
}
