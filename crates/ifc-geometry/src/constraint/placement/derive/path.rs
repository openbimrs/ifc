//! A basis curve that lowers to a curve relation, as Axiolid's neutral
//! [`CurvePath`] (#418).
//!
//! A plain `IfcCompositeCurve`, an `IfcTrimmedCurve`, a composite of
//! `IfcCurveSegment`s placed by `IfcAxis2PlacementLinear`s and an offset
//! curve (`IfcOffsetCurve2D`, `IfcOffsetCurve3D`,
//! `IfcOffsetCurveByDistances`, #414) lower to curve relations (#346), which the evaluator's single-curve queries cannot
//! read. Since `axiolid-curve-evaluate-contract` 0.3.4 (axiolid/kernel#290)
//! an evaluator reads such a basis as a `CurvePath`: spans of atomic curves
//! laid end to end in their station measure, each forwards or backwards and
//! carried by a rigid placement, through its `path_*` queries.
//!
//! The path is built here from the stored relation the station lowering
//! already reads (`lower::station::relation`), never from an execution
//! provider (ADR 0004): the pieces, their spans and their senses are data.
//! The one step that is evaluation, framing the station a segment is placed
//! at, is asked of the caller's evaluator, and composed as Axiolid composes
//! a node placed at a station (`axiolid-mesh-compile`'s
//! `station::placement`): the station's section frame `(tangent, left, up)`,
//! in plan when the station says so, turned by its orientation, at the point
//! its offsets locate. A segment on a seam of the curve it is placed along
//! is framed from the side its station names. Each placement is exact only
//! where the frame it is read in is (a line, or a path the evaluator calls
//! exact there).
//!
//! A piece's span is stated in its own measure, an offset piece's in its
//! own length. Where that is a quadrature (an offset beside a gradient
//! curve, a spiral, an ellipse or a B-spline, a variable offset of a
//! circle, an ellipse or a B-spline piece whole) only an execution provider
//! computes it, so the piece is left open (#427): `PathPiece::whole`, which
//! runs to its curve's end, resolved by the evaluator to its own length of
//! the curve (`axiolid-curve-evaluate-contract` 0.3.6, axiolid/kernel#298),
//! so the distance runs through the path as through the stated path the
//! kernel flattens the same relation into. A span that starts inside such
//! a curve (a trim at an interior parameter of an ellipse or a B-spline, a
//! span between stations along a relation with an open piece) has no open
//! form and is refused by [`GeometryError::PathPieceLengthUnstated`].
//!
//! # Joints of an open path (#427)
//!
//! Where a joint follows an open piece, its distance is the evaluator's,
//! not stored data, so it is not snapped to here as a stated path's joints
//! are. As the kernel does for the same basis's station (#423,
//! axiolid/kernel#294), the joints are certified instead: each joint's
//! [`CurveEvaluator::path_joint_distances`] interval, to a thousandth
//! (`open::WINDOW_CERTIFICATION`) of the window this crate snaps within (the
//! model's precision, capped at 1 mm, or rounding: `seams::window`), and a
//! stated piece's own seams after it, offset by the same interval. A
//! `DistanceAlong` whose window holds exactly one of them wholly is read ON
//! it, from the incoming side (IFC4.3 ADD2 8.9.3.48.3; [`SeamSide::Incoming`]):
//! the end of the path cut there, read backwards from its end, so the point
//! is the joint's own wherever the evaluator measures it. Two seams in the
//! window, a seam whose interval straddles its edge, and a joint the
//! evaluator cannot certify are refused by
//! [`GeometryError::PlacementSeamWindowRefused`], as the kernel refuses the
//! station. Every joint is certified, so a path holding a piece the
//! evaluator cannot certify is refused wherever the placement lies. An
//! evaluator that reads stated paths but not open ends, or does not
//! certify joints, is refused by
//! [`GeometryError::CurvePathOpenEndUnsupported`] or
//! [`GeometryError::PathJointDistancesUnsupported`], never read on an
//! estimate.
//!
//! The tests build the same relation's path with `axiolid-mesh-compile`'s
//! `station::curve_path` and compare the two.

use axiolid_contracts::GeomError;
use axiolid_core::{Scalar, Transform3, Vec3};
use axiolid_curve::{Curve2, Curve3, CurvePath, PathCurve, PathOffset, PathPiece};
use axiolid_curve_evaluate_contract::{
    CurveEvaluator, CurveMeasure, CurveMeasure as KernelMeasure, DistanceConvention, SeamSide,
    CURVE_PATH_UNSUPPORTED, OPEN_PATH_END_UNSUPPORTED, PATH_JOINT_DISTANCES_UNSUPPORTED,
    SEAM_SIDE_UNSUPPORTED,
};
use axiolid_model::{NodeId, OrientedCurveStation, StationFrame};
use ifc_alignment::{CurveMeasure as IfcMeasure, PointByDistance};
use ifc_model::{EntityId, Model};

use super::{basis_curve, model_precision, offset_frame, refusal_detail, snap, Basis};

use crate::error::{GeometryError, GeometryResult};
use crate::lower::curve::lower_curve_node;
use crate::lower::session::{AtomicCurve, LoweringSession};
use crate::lower::station::relation::{relation_pieces, relation_run, Piece, INEXACT};
use crate::lower::station::seams::{window, Seam};

mod open;

#[cfg(test)]
pub(crate) use open::WINDOW_CERTIFICATION;
use open::{read_open, OpenRefusal};

use crate::transform::Transform;
use crate::units::UnitScale;

/// Most stations a basis may be placed through, nested, before it is
/// refused (Axiolid's own limit on nested relations and placements).
const MAX_DEPTH: usize = 64;

/// A basis curve read as a curve path.
#[derive(Debug, Clone)]
pub(super) struct PathBasis {
    /// The path, its placements resolved.
    pub path: CurvePath,
    /// Where its tangent may jump.
    pub seams: PathSeams,
    /// The distance IFC states along it: plan distance when every piece is
    /// a gradient curve, arc length when none is.
    pub convention: DistanceConvention,
}

/// Where a path's tangent may jump.
#[derive(Debug, Clone)]
pub(super) enum PathSeams {
    /// Every length stated: every interior joint and every seam of a piece
    /// inside it, ascending, in the path's distance.
    Stated(Vec<Seam>),
    /// A piece left open (#427): each stated piece's own seams, at their
    /// distance into it, with its index. The joints are the evaluator's.
    Open(Vec<(usize, Seam)>),
}

/// Derive the placement along a basis read as a curve path (#418, module
/// documentation).
pub(super) fn derive_on_path(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
    expression: &PointByDistance,
    basis: &PathBasis,
    evaluator: &dyn CurveEvaluator,
) -> GeometryResult<Transform> {
    let unsupported = |detail: &'static str| GeometryError::Unsupported {
        entity: placement,
        type_name: "IFCLINEARPLACEMENT".into(),
        detail,
    };
    let basis_id = expression.basis_curve;
    let typed = |error: &GeomError| path_refusal(error, placement, basis_id);
    // A parameter is refused with the basis (`basis_curve`).
    let IfcMeasure::Length(value) = expression.distance_along else {
        return Err(unsupported(RELATION_PARAMETER));
    };
    let distance = units.length(value);
    let path = &basis.path;
    let convention = evaluator.path_distance_convention(path);
    if convention != basis.convention {
        if convention != DistanceConvention::Unsupported {
            return Err(unsupported(DIFFERENT_DISTANCE));
        }
        // An evaluator that cannot measure the path: one without curve
        // paths at all, or one that refuses this path (pieces that do not
        // meet, a piece it does not know). Its answer tells which.
        return Err(
            match evaluator.path_frame_at(path, KernelMeasure::Distance(distance)) {
                Err(error) => typed(&error).unwrap_or(unsupported(UNMEASURED_PATH)),
                Ok(_) => unsupported(UNMEASURED_PATH),
            },
        );
    }
    let seams = match &basis.seams {
        PathSeams::Stated(seams) => seams,
        PathSeams::Open(inner) => {
            let precision = model_precision(model, units, placement)?;
            let frame = read_open(
                evaluator,
                path,
                inner,
                distance,
                window(precision, distance),
                SeamSide::Incoming,
            )
            .map_err(|refusal| match refusal {
                OpenRefusal::Window(refusal) => GeometryError::PlacementSeamWindowRefused {
                    placement,
                    basis: basis_id,
                    distance,
                    refusal,
                },
                OpenRefusal::Evaluator(error) => {
                    typed(&error).unwrap_or_else(|| unsupported(refusal_detail(&error)))
                }
            })?;
            return offset_frame(&frame, expression, units).ok_or(unsupported(VERTICAL_BASIS));
        }
    };
    let frame = match snap(model, units, placement, seams, distance)? {
        // On a joint or a seam the previous segment governs (8.9.3.48.3).
        Some(seam) => evaluator
            .path_frame_at_on(path, KernelMeasure::Distance(seam), SeamSide::Incoming)
            .map_err(|error| match error {
                GeomError::UnsupportedInput { input, .. } if input == SEAM_SIDE_UNSUPPORTED => {
                    GeometryError::SeamSideUnsupported {
                        placement,
                        basis: expression.basis_curve,
                        distance: seam,
                    }
                }
                other => typed(&other).unwrap_or_else(|| unsupported(refusal_detail(&other))),
            })?,
        None => evaluator
            .path_frame_at(path, KernelMeasure::Distance(distance))
            .map_err(|error| {
                typed(&error).unwrap_or_else(|| unsupported(refusal_detail(&error)))
            })?,
    };
    offset_frame(&frame, expression, units).ok_or(unsupported(VERTICAL_BASIS))
}

/// The basis curve of `placement` read as the neutral [`CurvePath`] its
/// derivation evaluates (#418): a plain `IfcCompositeCurve`, an
/// `IfcTrimmedCurve` or a composite of segments placed at stations, as the
/// station lowering flattens it, each segment's placement framed through
/// `evaluator` (module documentation).
///
/// This is the path [`derive_placement_transform`](super::derive_placement_transform) hands to
/// [`CurveEvaluator::path_frame_at_on`]. It is public so a caller can check
/// it, or read the same basis through the contract's other `path_*`
/// queries.
///
/// # Errors
///
/// A basis that is not a curve relation this module reads, a relation the
/// station lowering refuses (an offset of an offset, say), a relation whose
/// pieces' lengths
/// its data does not state, and a placement `evaluator` refuses to frame,
/// by name; [`GeometryError::CurvePathUnsupported`] when a segment is placed
/// along a curve relation and `evaluator` reads single curves only.
pub fn basis_curve_path(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
    basis: EntityId,
    evaluator: &dyn CurveEvaluator,
) -> GeometryResult<CurvePath> {
    match basis_curve(model, units, placement, basis, false, evaluator)? {
        Basis::Path(basis) => Ok(basis.path),
        Basis::Curve(..) => Err(GeometryError::Unsupported {
            entity: placement,
            type_name: model
                .get(basis)
                .map_or_else(String::new, |entity| entity.type_name.to_string()),
            detail: "the basis curve is a single curve, not a curve relation",
        }),
    }
}

/// Why a relation basis the evaluator measures differently is refused.
const DIFFERENT_DISTANCE: &str =
    "the evaluator measures a different distance along this basis curve than IFC states (plan \
     distance on an alignment, arc length on a polyline or a composite curve)";

/// Why a relation basis the evaluator cannot measure is refused.
const UNMEASURED_PATH: &str =
    "the evaluator cannot measure a distance along this basis curve relation: its pieces do not \
     meet, are measured differently, or include a curve family it does not read";

/// Why a placement on a vertical tangent is refused.
pub(super) const VERTICAL_BASIS: &str =
    "the basis curve is vertical there, so OffsetLateral has no \
                              horizontal direction and the placement no roll";

/// Why an `IfcParameterValue` along a curve relation is refused.
pub(super) const RELATION_PARAMETER: &str =
    "an IfcParameterValue DistanceAlong on a basis curve that is a curve relation (a composite \
     or trimmed curve): its segments' parameters do not run through it, and IFC4.3 ADD2 measures \
     IfcCurveSegments by length; state an IfcLengthMeasure";

/// Why a segment whose placement the evaluator refused is refused.
const PLACED_SEGMENT: &str =
    "a segment of the basis curve is placed at a station of another curve, and the curve \
     evaluator refused to frame that station";

/// Why a segment placed on a seam the evaluator reads only one side of is
/// refused.
const PLACED_ON_SEAM: &str =
    "a segment of the basis curve is placed at a station on a tangent discontinuity of another \
     curve, where IFC4.3 ADD2 (8.9.3.48.3) takes the previous segment's tangent, and the curve \
     evaluator reads only the segment that starts there";

/// Why a placement whose station frame this bridge does not know is
/// refused.
const STATION_FRAME: &str =
    "a segment of the basis curve is placed at a station in a frame other than the section or \
     the plan frame";

/// Why a segment placed at a station whose orientation is degenerate is
/// refused.
const ORIENTATION: &str =
    "a segment of the basis curve is placed at a station whose Axis or RefDirection is zero, \
     not finite, or whose Axis and RefDirection are parallel";

/// Why a segment placed where the curve is vertical is refused.
const VERTICAL: &str =
    "a segment of the basis curve is placed at a station where the curve it is placed along is \
     vertical, so its left and its plan direction are undefined";

/// `basis`, the basis curve of `placement`, lowered and read as a
/// [`PathBasis`]: a curve relation's pieces, or an atomic curve as one
/// piece, whole.
///
/// # Errors
///
/// A basis that does not lower, or whose relation the station lowering
/// refuses (an offset of an offset, a mix of plan-measured and arc-length
/// pieces, a piece whose length its data does not state), by name; a placement
/// the evaluator refuses to frame, and an evaluator without curve paths,
/// as [`resolve_placement`].
pub(super) fn path_basis(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
    basis: EntityId,
    type_name: &str,
    evaluator: &dyn CurveEvaluator,
) -> GeometryResult<PathBasis> {
    let mut session = LoweringSession::new(model, units);
    let node = lower_curve_node(&mut session, basis, Transform::identity())?;
    let reader = Reader {
        session: &session,
        evaluator,
        placement,
        basis,
    };
    let refused = |reason: &'static str| {
        if reason == INEXACT {
            GeometryError::PathPieceLengthUnstated { placement, basis }
        } else {
            GeometryError::Unsupported {
                entity: placement,
                type_name: type_name.to_owned(),
                detail: reason,
            }
        }
    };
    let convention = |plan: bool| {
        if plan {
            DistanceConvention::PlanDistance
        } else {
            DistanceConvention::ArcLength3d
        }
    };
    match relation_run(&session, node) {
        Ok(run) => Ok(PathBasis {
            path: reader.path(&run.pieces, 0)?,
            seams: PathSeams::Stated(run.seams),
            convention: convention(run.plan),
        }),
        // A piece whose length is a quadrature: read open (#427).
        Err(reason) if reason == INEXACT => {
            let (pieces, plan) = relation_pieces(&session, node).map_err(refused)?;
            Ok(PathBasis {
                path: reader.path(&pieces, 0)?,
                seams: PathSeams::Open(inner_seams(&pieces)),
                convention: convention(plan),
            })
        }
        Err(reason) => Err(refused(reason)),
    }
}

/// Each piece's own seams strictly inside it, with its index.
fn inner_seams(pieces: &[Piece<'_>]) -> Vec<(usize, Seam)> {
    pieces
        .iter()
        .enumerate()
        .flat_map(|(index, piece)| piece.inner_seams().map(move |seam| (index, seam)))
        .collect()
}

/// The typed refusal an evaluator's answer names, if it is one of a
/// capability it lacks: curve paths, open ends, certified joints.
fn path_refusal(error: &GeomError, placement: EntityId, basis: EntityId) -> Option<GeometryError> {
    let GeomError::UnsupportedInput { input, .. } = error else {
        return None;
    };
    if *input == CURVE_PATH_UNSUPPORTED {
        Some(GeometryError::CurvePathUnsupported { placement, basis })
    } else if *input == OPEN_PATH_END_UNSUPPORTED {
        Some(GeometryError::CurvePathOpenEndUnsupported { placement, basis })
    } else if *input == PATH_JOINT_DISTANCES_UNSUPPORTED {
        Some(GeometryError::PathJointDistancesUnsupported { placement, basis })
    } else {
        None
    }
}

/// The lowered basis and what is needed to read it.
struct Reader<'r, 'a> {
    session: &'r LoweringSession<'a>,
    evaluator: &'r dyn CurveEvaluator,
    /// The `IfcLinearPlacement` being derived.
    placement: EntityId,
    /// Its basis curve.
    basis: EntityId,
}

impl Reader<'_, '_> {
    /// The pieces as path pieces, their placements resolved.
    fn path(&self, pieces: &[Piece<'_>], depth: usize) -> GeometryResult<CurvePath> {
        pieces
            .iter()
            .map(|piece| self.piece(piece, depth))
            .collect()
    }

    /// One piece as a path piece, its placements resolved: an offset piece
    /// (#414) as `PathCurve::Offset` of its base span and law, in its own
    /// measure `[0, L]`, as `axiolid-mesh-compile` hands it out.
    fn piece(&self, piece: &Piece<'_>, depth: usize) -> GeometryResult<PathPiece> {
        let curve = match &piece.offset {
            Some(offset) => PathCurve::Offset(Box::new(PathOffset::new(
                self.piece(&offset.base, depth)?,
                offset.law,
            ))),
            None => path_curve(piece.curve),
        };
        // A piece whose length is a quadrature runs to its curve's end,
        // which the evaluator resolves (#427); it starts at the curve's
        // start.
        let mut out = if piece.open {
            PathPiece::whole(curve)
        } else {
            PathPiece::new(curve, piece.start, piece.end)
        };
        if piece.reversed {
            out = out.reversed();
        }
        for station in &piece.placements {
            let (rigid, exact) = self.placement(station, depth + 1)?;
            out = out.placed(rigid, exact);
        }
        Ok(out)
    }

    /// The rigid motion of a node placed at `station`, and whether it is
    /// exact (module documentation).
    fn placement(
        &self,
        station: &OrientedCurveStation,
        depth: usize,
    ) -> GeometryResult<(Transform3, bool)> {
        if depth > MAX_DEPTH {
            return Err(self.unsupported(crate::lower::station::relation::UNSUPPORTED));
        }
        let at = station.station.station.distance;
        let measure = CurveMeasure::Distance(at);
        let side = station.seam;
        let basis = station.station.basis;
        let (frame, exact) = match self.session.atomic_curve(basis) {
            Some(AtomicCurve::Three(curve)) => (
                self.evaluator.frame_at_on(curve, measure, side),
                matches!(curve, Curve3::Line(_)),
            ),
            Some(AtomicCurve::Two(curve)) => {
                // The contract reads a 2D curve as a path piece: the whole
                // curve, or for a line, which a station reads unbounded,
                // the span from its origin through the station.
                let (piece, distance) = match curve {
                    Curve2::Line(line) => {
                        let length = line.direction.length();
                        let start = at.min(0.0);
                        (
                            PathPiece::new(path_curve2(curve), start, length.max(at)),
                            at - start,
                        )
                    }
                    _ => {
                        let run = self.run(basis)?;
                        let Some(piece) = run.first() else {
                            return Err(self.unsupported(PLACED_SEGMENT));
                        };
                        let whole = if piece.open {
                            PathPiece::whole(path_curve2(curve))
                        } else {
                            PathPiece::new(path_curve2(curve), piece.start, piece.end)
                        };
                        (whole, at)
                    }
                };
                let path = CurvePath::from(piece);
                let distance = CurveMeasure::Distance(distance);
                (
                    self.evaluator.path_frame_at_on(&path, distance, side),
                    matches!(curve, Curve2::Line(_)),
                )
            }
            None => {
                let pieces = self.run(basis)?;
                let path = self.path(&pieces, depth)?;
                if pieces.iter().any(|piece| piece.open) {
                    // Along a path with an open piece the station is read
                    // within its own seam-snapping window, as the kernel
                    // reads it (#427, module documentation); never exact.
                    let frame = read_open(
                        self.evaluator,
                        &path,
                        &inner_seams(&pieces),
                        at,
                        station.seam_window,
                        side,
                    )
                    .map_err(|refusal| match refusal {
                        OpenRefusal::Window(refusal) => GeometryError::PlacementSeamWindowRefused {
                            placement: self.placement,
                            basis: self.basis,
                            distance: at,
                            refusal,
                        },
                        OpenRefusal::Evaluator(error) => self.placement_refused(&error),
                    })?;
                    (Ok(frame), false)
                } else {
                    (
                        self.evaluator.path_frame_at_on(&path, measure, side),
                        self.evaluator.path_frame_is_exact_at(&path, measure, side),
                    )
                }
            }
        };
        let frame = frame.map_err(|error| self.placement_refused(&error))?;
        let section = Section::from_tangent(frame.origin, frame.x)
            .ok_or_else(|| self.unsupported(VERTICAL))?;
        let section = match station.station.frame {
            StationFrame::Section => section,
            StationFrame::Plan => section.plan().ok_or_else(|| self.unsupported(VERTICAL))?,
            _ => return Err(self.unsupported(STATION_FRAME)),
        };
        let offsets = station.station.station.offsets;
        let point = section.place(offsets.lateral, offsets.vertical, offsets.longitudinal);
        let turned = if station.orientation.is_base() {
            section
        } else {
            let (axis, reference) = station
                .orientation
                .unit_axes()
                .map_err(|_| self.unsupported(ORIENTATION))?;
            section
                .oriented(axis, reference)
                .ok_or_else(|| self.unsupported(ORIENTATION))?
        };
        Ok((
            Transform3::from_cols(turned.tangent, turned.lateral, turned.up, point),
            exact,
        ))
    }

    /// The pieces of the curve a segment is placed along, any whose length
    /// is a quadrature left open (#427).
    fn run(&self, node: NodeId) -> GeometryResult<Vec<Piece<'_>>> {
        relation_pieces(self.session, node)
            .map(|(pieces, _)| pieces)
            .map_err(|reason| {
                if reason == INEXACT {
                    GeometryError::PathPieceLengthUnstated {
                        placement: self.placement,
                        basis: self.basis,
                    }
                } else {
                    self.unsupported(reason)
                }
            })
    }

    fn unsupported(&self, detail: &'static str) -> GeometryError {
        GeometryError::Unsupported {
            entity: self.placement,
            type_name: "IFCLINEARPLACEMENT".into(),
            detail,
        }
    }

    /// The evaluator's refusal to frame a placement's station, by name.
    fn placement_refused(&self, error: &GeomError) -> GeometryError {
        if let Some(typed) = path_refusal(error, self.placement, self.basis) {
            return typed;
        }
        match error {
            GeomError::UnsupportedInput { input, .. } if *input == SEAM_SIDE_UNSUPPORTED => {
                self.unsupported(PLACED_ON_SEAM)
            }
            _ => self.unsupported(PLACED_SEGMENT),
        }
    }
}

/// A stored curve as a path piece's curve.
fn path_curve(curve: &AtomicCurve) -> PathCurve {
    match curve {
        AtomicCurve::Two(curve) => path_curve2(curve),
        AtomicCurve::Three(curve) => PathCurve::Three(curve.clone()),
    }
}

fn path_curve2(curve: &Curve2) -> PathCurve {
    PathCurve::Two(curve.clone())
}

/// A section frame: point, unit tangent, left lateral and up, as Axiolid's
/// `SectionFrame` (repeated here because that crate is not linked).
#[derive(Debug, Clone, Copy)]
struct Section {
    point: Vec3,
    tangent: Vec3,
    lateral: Vec3,
    up: Vec3,
}

impl Section {
    /// The reference-up section at `point` with unit `tangent`, built from
    /// the tangent alone as the derived placement frame is (the module
    /// documentation of `derive`): `lateral = normalise(Z x tangent)`,
    /// `up = tangent x lateral`. `None` where the tangent is vertical.
    fn from_tangent(point: Vec3, tangent: Vec3) -> Option<Self> {
        let horizontal = (tangent.x * tangent.x + tangent.y * tangent.y).sqrt();
        if !horizontal.is_finite() || horizontal <= 1e-12 {
            return None;
        }
        let lateral = Vec3::new(-tangent.y / horizontal, tangent.x / horizontal, 0.0);
        Some(Self {
            point,
            tangent,
            lateral,
            up: tangent.cross(lateral),
        })
    }

    /// The same point framed vertically: the tangent's horizontal
    /// projection, the horizontal left normal and `+Z`.
    fn plan(&self) -> Option<Self> {
        let horizontal = Vec3::new(self.tangent.x, self.tangent.y, 0.0);
        let length = horizontal.length();
        if !length.is_finite() || length <= 1e-12 {
            return None;
        }
        let tangent = horizontal / length;
        Some(Self {
            point: self.point,
            tangent,
            lateral: Vec3::new(-tangent.y, tangent.x, 0.0),
            up: Vec3::Z,
        })
    }

    /// The point `lateral` to the left, `vertical` up and `longitudinal`
    /// along the tangent.
    fn place(&self, lateral: Scalar, vertical: Scalar, longitudinal: Scalar) -> Vec3 {
        self.point + lateral * self.lateral + vertical * self.up + longitudinal * self.tangent
    }

    /// The axes turned by a unit `axis` and unit `reference`, components in
    /// this frame: Gram-Schmidt with the axis primary, as the station
    /// lowering states an `IfcAxis2PlacementLinear`'s `Axis` and
    /// `RefDirection` (`lower::station::axes`). `None` when they are
    /// parallel.
    fn oriented(&self, axis: Vec3, reference: Vec3) -> Option<Self> {
        let unit = |vector: Vec3| {
            let length = vector.length();
            (vector.is_finite() && length.is_finite() && length > 1e-12).then(|| vector / length)
        };
        let axis = unit(axis)?;
        let reference = unit(reference)?;
        // Axiolid's `ORIENTATION_TOLERANCE`.
        if axis.cross(reference).length() <= 1e-9 {
            return None;
        }
        let x = (reference - reference.dot(axis) * axis).normalize();
        let y = axis.cross(x);
        let world = |v: Vec3| v.x * self.tangent + v.y * self.lateral + v.z * self.up;
        Some(Self {
            point: self.point,
            tangent: world(x),
            lateral: world(y),
            up: world(axis),
        })
    }
}
