//! Reading along a curve path with an open piece (#427): the joints after
//! it located by the evaluator's certified joint distances, a distance
//! within the model's precision of one read on it from the side the reading
//! names, as the kernel reads the same basis's station (#423); see the
//! parent module's documentation.

use axiolid_contracts::{GeomError, GeomResult};
use axiolid_core::{Frame3, Scalar};
use axiolid_curve::{CurvePath, PathPiece};
use axiolid_curve_evaluate_contract::{
    CurveEvaluator, CurveMeasure as KernelMeasure, DistanceInterval, SeamSide,
};

use crate::compile::SeamWindowRefusal;
use crate::lower::station::seams::Seam;

/// How finely a joint's distance is certified for a window: at most this
/// fraction of the window wide, as the kernel certifies a station's seams
/// (`axiolid_evaluate::station::window::WINDOW_CERTIFICATION`, repeated
/// because that crate is not linked; a test pins the two equal).
pub(crate) const WINDOW_CERTIFICATION: Scalar = 1e-3;

/// Axiolid's arc-length tolerance, within which two distances are one.
const ARC_LENGTH_TOLERANCE: Scalar = 1e-12;

/// Why reading along an open path failed.
pub(super) enum OpenRefusal {
    /// The evaluator refused a query.
    Evaluator(GeomError),
    /// The window did not name one place (or a joint is uncertified).
    Window(SeamWindowRefusal),
}

impl From<GeomError> for OpenRefusal {
    fn from(error: GeomError) -> Self {
        match SeamWindowRefusal::of(&error) {
            Some(refusal) => Self::Window(refusal),
            None => Self::Evaluator(error),
        }
    }
}

/// Where along a path a seam lies.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Place {
    /// The joint where piece `k` starts.
    Joint(usize),
    /// `u` into piece `k`, a stated piece.
    Inside(usize, Scalar),
}

/// The frame at `distance` along `path`, a path with an open piece, read
/// on the seam its `window` holds from `side` (module documentation).
pub(super) fn read_open(
    evaluator: &dyn CurveEvaluator,
    path: &CurvePath,
    inner: &[(usize, Seam)],
    distance: Scalar,
    window: Scalar,
    side: SeamSide,
) -> Result<Frame3, OpenRefusal> {
    let at = KernelMeasure::Distance(distance);
    if !(window > 0.0 && window.is_finite() && distance.is_finite()) {
        return Ok(evaluator.path_frame_at_on(path, at, side)?);
    }
    let pieces = path.pieces();
    let joints = evaluator.path_joint_distances(path, window * WINDOW_CERTIFICATION)?;
    if joints.len() + 1 != pieces.len() {
        return Err(OpenRefusal::Evaluator(GeomError::InvalidInput(format!(
            "the evaluator certified {} joints of a path of {} pieces",
            joints.len(),
            pieces.len()
        ))));
    }
    let starts = |k: usize| {
        k.checked_sub(1)
            .map_or(DistanceInterval::point(0.0), |j| joints[j])
    };
    let seams = joints
        .iter()
        .enumerate()
        .map(|(j, interval)| (*interval, Place::Joint(j + 1)))
        .chain(inner.iter().map(|(k, seam)| {
            let start = starts(*k);
            (
                DistanceInterval::new(start.lower + seam.distance, start.upper + seam.distance),
                Place::Inside(*k, seam.distance),
            )
        }));
    match snapped(seams, distance, window)? {
        Some(place) => Ok(frame_on(evaluator, path, place, side)?),
        None => Ok(evaluator.path_frame_at(path, at)?),
    }
}

/// The seam of `seams` a station at `distance` with `window` stands on, by
/// the kernel's rule (`axiolid_evaluate::station::window`): one whose
/// interval lies within `[d - w, d + w]`, none if every interval lies
/// outside it; two within it, or one straddling its edge, refused.
fn snapped(
    seams: impl Iterator<Item = (DistanceInterval, Place)>,
    distance: Scalar,
    window: Scalar,
) -> Result<Option<Place>, OpenRefusal> {
    let (lo, hi) = (distance - window, distance + window);
    let mut inside: Option<(Scalar, Place)> = None;
    let (mut ambiguous, mut straddled) = (false, false);
    for (interval, place) in seams {
        if interval.upper < lo || interval.lower > hi {
            continue;
        }
        if interval.lower >= lo && interval.upper <= hi {
            let middle = interval.midpoint();
            match inside {
                Some((first, _))
                    if (first - middle).abs() > ARC_LENGTH_TOLERANCE * first.abs().max(1.0) =>
                {
                    ambiguous = true;
                }
                _ => inside = Some((middle, place)),
            }
        } else {
            straddled = true;
        }
    }
    if ambiguous {
        return Err(OpenRefusal::Window(SeamWindowRefusal::Ambiguous));
    }
    if straddled {
        return Err(OpenRefusal::Window(SeamWindowRefusal::Straddled));
    }
    Ok(inside.map(|(_, place)| place))
}

/// The frame ON the seam at `place`, read from `side`: the path cut there,
/// its part before the seam read backwards from its end (the incoming
/// piece's end, tangent and left turned back) or its part after it read
/// from its start. Both are the evaluator's own point of the seam,
/// wherever it measures the open pieces before it.
fn frame_on(
    evaluator: &dyn CurveEvaluator,
    path: &CurvePath,
    place: Place,
    side: SeamSide,
) -> GeomResult<Frame3> {
    let pieces = path.pieces();
    let (k, cut) = match place {
        Place::Joint(k) => (k, None),
        Place::Inside(k, u) => (k, Some(u)),
    };
    let clipped = |from: Scalar, to: Scalar| {
        let mut piece = pieces[k].clone();
        let (start, end) = (piece.start, piece.end);
        if piece.reversed {
            piece.start = end - to;
            piece.end = end - from;
        } else {
            piece.start = start + from;
            piece.end = start + to;
        }
        piece
    };
    let zero = KernelMeasure::Distance(0.0);
    if side == SeamSide::Incoming {
        let mut before: Vec<PathPiece> = pieces[..k].to_vec();
        before.extend(cut.map(|u| clipped(0.0, u)));
        let frame = evaluator.path_frame_at(&CurvePath::new(before).reversed(), zero)?;
        // Read backwards, the tangent and the right (z) turn, up stays.
        Ok(Frame3 {
            origin: frame.origin,
            x: -frame.x,
            y: frame.y,
            z: -frame.z,
        })
    } else {
        let mut after: Vec<PathPiece> = cut
            .map(|u| clipped(u, pieces[k].end - pieces[k].start))
            .into_iter()
            .collect();
        after.extend(pieces[k..].iter().skip(usize::from(cut.is_some())).cloned());
        evaluator.path_frame_at(&CurvePath::new(after), zero)
    }
}
