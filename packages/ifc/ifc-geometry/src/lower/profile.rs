//! Profiles become closed polygons.
//!
//! # Why this is here and not in a profile crate
//!
//! `IfcProfileDef` belongs to `IfcProfileResource`, a fourth schema outside
//! the three this crate implements. It is pulled in anyway because no swept
//! solid can be lowered without it: `IfcExtrudedAreaSolid.SweptArea` IS an
//! `IfcProfileDef`, so a lowering layer that stopped at the schema boundary
//! could not emit a single extrusion. Only the profile subtypes actually
//! reachable from a swept solid are handled here.
//!
//! # Slot layout
//!
//! `IfcProfileDef` contributes `ProfileType` and `ProfileName`, so every
//! subtype's own attributes begin at slot 2. `IfcParameterizedProfileDef`
//! adds `Position` at slot 2, putting `IfcRectangleProfileDef.XDim` at 3.
//! Verified against a real record:
//!
//! ```text
//! #338077= IFCRECTANGLEPROFILEDEF(.AREA.,'0AR_FIN...',#338076,0.02,0.7099)
//!                                  0     1            2       3    4
//! ```

use crate::error::{GeometryError, GeometryResult};
use crate::kernel::{Contour, Profile};
use crate::lower::Tolerance;
use crate::slots::Slots;
use crate::units::UnitScale;
use ifc_model::{EntityId, Model};

/// Slot indices, absolute, inherited attributes first.
mod slot {
    /// `IfcParameterizedProfileDef.Position`.
    ///
    /// Not yet applied: the profile-local 2D placement is a further transform
    /// on the contour. Declared here so the slot layout stays documented and
    /// the gap is visible rather than forgotten.
    #[allow(dead_code)]
    pub const POSITION: usize = 2;
    /// `IfcRectangleProfileDef.XDim`.
    pub const X_DIM: usize = 3;
    /// `IfcRectangleProfileDef.YDim`.
    pub const Y_DIM: usize = 4;
    /// `IfcCircleProfileDef.Radius`.
    pub const RADIUS: usize = 3;
    /// `IfcArbitraryClosedProfileDef.OuterCurve`. Not parameterized, so it
    /// follows ProfileType and ProfileName directly.
    pub const OUTER_CURVE: usize = 2;
    /// `IfcArbitraryProfileDefWithVoids.InnerCurves`.
    pub const INNER_CURVES: usize = 3;
    /// `IfcCircleHollowProfileDef.WallThickness`.
    ///
    /// Slot 4 because `IfcCircleProfileDef` contributes only `Radius` at 3.
    pub const CIRCLE_WALL_THICKNESS: usize = 4;
    /// `IfcRectangleHollowProfileDef.WallThickness`.
    ///
    /// Slot 5, NOT 4: `IfcRectangleProfileDef` contributes both `XDim` (3)
    /// and `YDim` (4) before it. Sharing one constant with the circle case
    /// made this read `YDim` as the wall thickness, which rejected a valid
    /// 250x250x7 hollow section as degenerate. Caught by lowering the real
    /// corpus, not by any hand-built model.
    pub const RECT_WALL_THICKNESS: usize = 5;
}

/// Lower an `IfcProfileDef` to a closed polygon in profile coordinates.
///
/// The returned contour is in the profile's own 2D space; the swept solid's
/// `Position` and the product placement are applied later, by the caller.
/// Keeping those separate is what lets one profile be reused by many solids.
pub fn lower_profile(
    model: &Model,
    id: EntityId,
    units: &UnitScale,
    tol: &Tolerance,
) -> GeometryResult<Profile> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    let slots = Slots::new(id, entity);
    let type_name = entity.type_name.to_ascii_uppercase();

    match type_name.as_str() {
        "IFCRECTANGLEPROFILEDEF" | "IFCROUNDEDRECTANGLEPROFILEDEF" => rectangle(&slots, units),
        "IFCRECTANGLEHOLLOWPROFILEDEF" => rectangle_hollow(&slots, units),
        "IFCCIRCLEPROFILEDEF" => circle(&slots, units, tol),
        "IFCCIRCLEHOLLOWPROFILEDEF" => circle_hollow(&slots, units, tol),
        "IFCARBITRARYCLOSEDPROFILEDEF" => arbitrary(model, &slots, units, tol, false),
        "IFCARBITRARYPROFILEDEFWITHVOIDS" => arbitrary(model, &slots, units, tol, true),
        other => Err(GeometryError::Unsupported {
            entity: id,
            type_name: other.to_string(),
            detail: "profile subtype is not lowered yet",
        }),
    }
}

/// `IfcRectangleProfileDef`: centred on the profile origin.
///
/// The centring is the trap. XDim and YDim are the FULL width and height, and
/// the rectangle straddles the origin, so the corners are at +/- half. Placing
/// a corner at the origin instead shifts every wall by half its thickness --
/// a bug that looks plausible in a viewer and is wrong everywhere.
fn rectangle(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<Profile> {
    let x = units.length(slots.req_f64(slot::X_DIM, "XDim")?);
    let y = units.length(slots.req_f64(slot::Y_DIM, "YDim")?);
    Ok(Profile {
        outer: rect_contour(x, y),
        inner: Vec::new(),
    })
}

/// A centred rectangle as a closed contour, counter-clockwise.
fn rect_contour(x: f64, y: f64) -> Contour {
    let (hx, hy) = (x / 2.0, y / 2.0);
    Contour {
        points: vec![[-hx, -hy], [hx, -hy], [hx, hy], [-hx, hy]],
    }
}

/// `IfcRectangleHollowProfileDef`: a rectangle with a rectangular void.
///
/// Fillet radii are ignored deliberately; they change the section modulus but
/// not the gross shape, and approximating them silently would be worse than
/// omitting them visibly. Recorded here so the omission is discoverable.
fn rectangle_hollow(slots: &Slots<'_>, units: &UnitScale) -> GeometryResult<Profile> {
    let x = units.length(slots.req_f64(slot::X_DIM, "XDim")?);
    let y = units.length(slots.req_f64(slot::Y_DIM, "YDim")?);
    let t = units.length(slots.req_f64(slot::RECT_WALL_THICKNESS, "WallThickness")?);
    if t <= 0.0 || 2.0 * t >= x || 2.0 * t >= y {
        return Err(slots.degenerate("wall thickness consumes the whole section"));
    }
    Ok(Profile {
        outer: rect_contour(x, y),
        inner: vec![rect_contour(x - 2.0 * t, y - 2.0 * t)],
    })
}

/// `IfcCircleProfileDef`: approximated at the caller's tolerance.
fn circle(slots: &Slots<'_>, units: &UnitScale, tol: &Tolerance) -> GeometryResult<Profile> {
    let r = units.length(slots.req_f64(slot::RADIUS, "Radius")?);
    if r <= 0.0 {
        return Err(slots.degenerate("circle profile has non-positive radius"));
    }
    Ok(Profile {
        outer: circle_contour(r, tol),
        inner: Vec::new(),
    })
}

/// `IfcCircleHollowProfileDef`: a tube.
fn circle_hollow(slots: &Slots<'_>, units: &UnitScale, tol: &Tolerance) -> GeometryResult<Profile> {
    let r = units.length(slots.req_f64(slot::RADIUS, "Radius")?);
    let t = units.length(slots.req_f64(slot::CIRCLE_WALL_THICKNESS, "WallThickness")?);
    if r <= 0.0 || t <= 0.0 || t >= r {
        return Err(slots.degenerate("hollow circle has a non-physical wall thickness"));
    }
    Ok(Profile {
        outer: circle_contour(r, tol),
        inner: vec![circle_contour(r - t, tol)],
    })
}

/// A circle as a closed polygon, counter-clockwise, first point on +X.
///
/// The last point is NOT a repeat of the first: `Contour` closure is implicit.
/// Emitting a duplicate would give the kernel a zero-length edge.
fn circle_contour(radius: f64, tol: &Tolerance) -> Contour {
    let n = tol.segments_for_arc(radius, std::f64::consts::TAU).max(3);
    let step = std::f64::consts::TAU / f64::from(n);
    let points = (0..n)
        .map(|i| {
            let a = step * f64::from(i);
            [radius * a.cos(), radius * a.sin()]
        })
        .collect();
    Contour { points }
}

/// `IfcArbitraryClosedProfileDef` and its with-voids subtype.
///
/// The outer curve is usually an `IfcPolyline`, occasionally an
/// `IfcIndexedPolyCurve` or `IfcCompositeCurve`. Only polyline-shaped curves
/// are flattened here; a composite curve with arc segments reports
/// Unsupported rather than dropping the arcs, because silently straightening
/// a curved wall is exactly the class of lie this crate refuses to tell.
fn arbitrary(
    model: &Model,
    slots: &Slots<'_>,
    units: &UnitScale,
    tol: &Tolerance,
    with_voids: bool,
) -> GeometryResult<Profile> {
    let outer_id = slots.req_ref(slot::OUTER_CURVE, "OuterCurve")?;
    let outer = curve_to_contour(model, outer_id, units, tol)?;

    let mut inner = Vec::new();
    if with_voids {
        for void_id in slots.req_ref_list(slot::INNER_CURVES, "InnerCurves")? {
            inner.push(curve_to_contour(model, void_id, units, tol)?);
        }
    }
    Ok(Profile { outer, inner })
}

/// Flatten a bounded curve into a closed 2D contour.
///
/// Trailing duplicate points are dropped: exporters commonly repeat the first
/// point to close an `IfcPolyline`, but `Contour` closes implicitly and a
/// duplicated vertex is a zero-length edge that breaks meshing.
fn curve_to_contour(
    model: &Model,
    id: EntityId,
    units: &UnitScale,
    _tol: &Tolerance,
) -> GeometryResult<Contour> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    let type_name = entity.type_name.to_ascii_uppercase();
    if type_name != "IFCPOLYLINE" {
        return Err(GeometryError::Unsupported {
            entity: id,
            type_name,
            detail: "only polyline profile boundaries are lowered so far",
        });
    }
    let slots = Slots::new(id, entity);
    let mut points = Vec::new();
    for point_id in slots.req_ref_list(0, "Points")? {
        let p = model.get(point_id).ok_or(GeometryError::MissingEntity {
            referrer: id,
            missing: point_id,
        })?;
        let coords = Slots::new(point_id, p).req_f64_list(0, "Coordinates")?;
        if coords.len() < 2 {
            return Err(GeometryError::Degenerate {
                entity: point_id,
                type_name: p.type_name.to_string(),
                detail: "profile boundary point is not at least 2D".to_string(),
            });
        }
        points.push([units.length(coords[0]), units.length(coords[1])]);
    }
    drop_closing_duplicate(&mut points);
    if points.len() < 3 {
        return Err(GeometryError::Degenerate {
            entity: id,
            type_name: entity.type_name.to_string(),
            detail: "profile boundary has fewer than 3 distinct points".to_string(),
        });
    }
    Ok(Contour { points })
}

/// Remove a trailing point equal to the first.
fn drop_closing_duplicate(points: &mut Vec<[f64; 2]>) {
    if points.len() >= 2 {
        let first = points[0];
        let last = points[points.len() - 1];
        if (first[0] - last[0]).abs() < 1e-12 && (first[1] - last[1]).abs() < 1e-12 {
            points.pop();
        }
    }
}
