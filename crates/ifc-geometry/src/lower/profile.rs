//! Exact lowering of swept-area profile definitions.
//!
//! IFC units and profile-local placements are resolved here, but curves remain
//! exact. Tessellation is a geometry-kernel decision and never occurs in the
//! format adapter.
//!
//! # Who reads the slots
//!
//! Not this module. The kernel-free [`describe_profile`] reads every family's
//! attributes into SI parameters, and [`lower_profile`] maps that description
//! onto the neutral profile model. [`crate::body_description`] reports the same
//! description, so a rule check and the kernel see identical numbers. Only the
//! boundary curves of arbitrary and centre-line profiles are read here,
//! because turning a curve into a contour is lowering.

use axiolid_core::{Interval, Transform2, Vec2};
use axiolid_curve::{Curve2, Line2};
use axiolid_model::{GeometryNode, NodeId};
use axiolid_profile::{CenterLineProfile, Contour, ContourProfile, Profile, ProfileSegment};
use ifc_model::{EntityId, Model};

use crate::error::{GeometryError, GeometryResult};
use crate::input::profile::{
    describe_profile, ProfileDescription, ProfileOperator, ProfileParameters, ProfilePosition,
};
use crate::lower::session::LoweringSession;
use crate::slots::Slots;
use crate::units::UnitScale;

mod composite;
mod open;
mod sections;
pub use open::lower_open_profile_node;

/// Concrete `IfcProfileDef` families represented exactly by the neutral profile model.
pub const IMPLEMENTED_PROFILES: &[&str] = &[
    "IFCARBITRARYCLOSEDPROFILEDEF",
    "IFCARBITRARYOPENPROFILEDEF",
    "IFCARBITRARYPROFILEDEFWITHVOIDS",
    "IFCASYMMETRICISHAPEPROFILEDEF",
    "IFCCENTERLINEPROFILEDEF",
    "IFCCIRCLEHOLLOWPROFILEDEF",
    "IFCCIRCLEPROFILEDEF",
    "IFCCOMPOSITEPROFILEDEF",
    "IFCCSHAPEPROFILEDEF",
    "IFCDERIVEDPROFILEDEF",
    "IFCELLIPSEPROFILEDEF",
    "IFCISHAPEPROFILEDEF",
    "IFCLSHAPEPROFILEDEF",
    "IFCMIRROREDPROFILEDEF",
    "IFCRECTANGLEHOLLOWPROFILEDEF",
    "IFCRECTANGLEPROFILEDEF",
    "IFCROUNDEDRECTANGLEPROFILEDEF",
    "IFCTSHAPEPROFILEDEF",
    "IFCTRAPEZIUMPROFILEDEF",
    "IFCUSHAPEPROFILEDEF",
    "IFCZSHAPEPROFILEDEF",
];

/// Profile families that are recognized but carry no lowerable geometry.
///
/// `IfcProfileDef` is instantiable in IFC4 (it is not declared ABSTRACT), but
/// it is the bare supertype: it declares only `ProfileType`, `ProfileName` and
/// the curve slots its subtypes add. A file authoring one has supplied a
/// profile *label*, not a section, so this is a permanent typed refusal rather
/// than work awaiting a neutral contract. Every concrete IFC4 subtype lowers;
/// IFC4X3 additions that do not yet are listed in [`UNLOWERED`].
///
/// The refusal itself is raised by [`describe_profile`]; a unit test keeps its
/// reason identical to the one stated here.
pub const PLANNED_PROFILES: &[(&str, &str)] = &[(
    "IFCPROFILEDEF",
    "generic profile declaration carries no concrete geometry to lower",
)];

pub(crate) use crate::slots::profile_slot as slot;

/// Concrete profile families this lowerer does not yet build, with reasons.
///
/// Paired with `tests/schema_coverage.rs`, which fails if a concrete profile
/// is neither read by `input/profile/mod.rs` nor listed here. That is what
/// makes the gap visible: the committed corpus contains no steel sections, so
/// a corpus-shaped census reported full coverage while 13 families were absent.
///
/// A reason starting with `kernel:` needs a change in `axiolid-profile`; the
/// rest are IFC-side wiring. Every IFC4 ADD2 TC1 profile family is lowered;
/// the entries are IFC4X3 ADD2 families.
pub const UNLOWERED: &[(&str, &str)] = &[("IFCOPENCROSSPROFILEDEF", "in progress (#243)")];

/// Family label used for profile memoization.
const PROFILE: &str = "profile";
const OPEN_PROFILE: &str = "open-profile";

/// Append one `IfcProfileDef` to a shared session and return its node.
///
/// Profiles are the most-shared geometry in a real model: one section
/// definition backs every beam of a type. Memoizing here is what keeps a
/// shared profile a single node instead of one copy per referencing solid.
/// The frame is the identity because a profile is defined in its own 2D space;
/// placement is applied by the referencing solid, not baked into the section.
pub fn lower_profile_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
) -> GeometryResult<NodeId> {
    let frame = crate::transform::Transform::identity();
    if let Some(node) = session.memoized(id, PROFILE, frame) {
        return Ok(node);
    }
    let profile = lower_profile(session.model(), id, session.units())?;
    let node = session.node_for(id, GeometryNode::Profile(profile))?;
    session.memoize(id, PROFILE, frame, node);
    Ok(node)
}

/// Lower one `IfcProfileDef` to an exact, format-neutral profile.
///
/// The family's slots are read once, by the kernel-free [`describe_profile`];
/// this function only maps the SI description onto the neutral model.
pub fn lower_profile(model: &Model, id: EntityId, units: &UnitScale) -> GeometryResult<Profile> {
    let description = describe_profile(model, units, id)?;
    build(model, units, &description)
}

/// Map one description, and its nested members, onto the neutral model.
///
/// Recursion is bounded: the description tree was built under the reader's
/// nesting budget and cycle check.
fn build(
    model: &Model,
    units: &UnitScale,
    description: &ProfileDescription,
) -> GeometryResult<Profile> {
    let profile = match &description.parameters {
        ProfileParameters::ArbitraryClosed { outer_curve } => Profile::Contour(ContourProfile {
            outer: curve_to_contour(model, *outer_curve, units)?,
            holes: Vec::new(),
        }),
        ProfileParameters::ArbitraryWithVoids {
            outer_curve,
            inner_curves,
        } => {
            let outer = curve_to_contour(model, *outer_curve, units)?;
            let mut holes = Vec::with_capacity(inner_curves.len());
            for curve in inner_curves {
                holes.push(curve_to_contour(model, *curve, units)?);
            }
            Profile::Contour(ContourProfile { outer, holes })
        }
        // An open profile is a curve, not an area. The neutral profile model
        // is built on closed contours, so there is nothing to map it onto:
        // closing the curve would fabricate a face the file never described,
        // and silently sweeping it would produce a solid from a shape that
        // bounds no area. State that rather than emitting a generic gap.
        ProfileParameters::ArbitraryOpen { .. } | ProfileParameters::OpenCross { .. } => {
            return Err(GeometryError::Unsupported {
                entity: description.entity,
                type_name: description.type_name.clone(),
                detail: "open profiles have no area; use lower_open_profile_node",
            });
        }
        // `Thickness` is the FULL width across the path, which the kernel
        // stores halved so both offset sides are symmetric by construction.
        ProfileParameters::CenterLine { curve, thickness } => {
            let path = open::open_polyline_path(model, *curve, units)?;
            Profile::CenterLine(CenterLineProfile::from_width(path, *thickness))
        }
        // Order is preserved because it is the only identity a composite
        // member has: nothing else distinguishes two same-shaped members.
        ProfileParameters::Composite { profiles, .. } => {
            let mut members = Vec::with_capacity(profiles.len());
            for member in profiles {
                members.push(build(model, units, member)?);
            }
            Profile::Composite(members)
        }
        ProfileParameters::Derived {
            parent, operator, ..
        } => Profile::Derived {
            basis: Box::new(build(model, units, parent)?),
            transform: operator_2d(operator),
        },
        // The mirror about the local y axis is implied by the TYPE: the
        // schema derives the operator, so there is none to read.
        ProfileParameters::Mirrored { parent, .. } => Profile::Derived {
            basis: Box::new(build(model, units, parent)?),
            transform: Transform2::from_scale(Vec2::new(-1.0, 1.0)),
        },
        parameterized => sections::parameterized(description, parameterized)?,
    };
    // `IfcParameterizedProfileDef.Position` applies to every parameterised
    // family, steel sections included.
    Ok(match &description.position {
        Some(position) => Profile::Derived {
            basis: Box::new(profile),
            transform: position_2d(position),
        },
        None => profile,
    })
}

/// `IfcParameterizedProfileDef.Position` as a neutral 2D transform.
fn position_2d(position: &ProfilePosition) -> Transform2 {
    Transform2::from_cols(
        Vec2::from_array(position.x_axis),
        Vec2::from_array(position.y_axis()),
        Vec2::from_array(position.origin),
    )
}

/// `IfcDerivedProfileDef.Operator` as a neutral 2D transform.
fn operator_2d(operator: &ProfileOperator) -> Transform2 {
    Transform2::from_cols(
        Vec2::from_array(operator.x_axis),
        Vec2::from_array(operator.y_axis),
        Vec2::from_array(operator.origin),
    )
}

/// Lower one closed profile boundary curve into an exact contour.
///
/// `IfcPolyline` is one ring of straight edges. `IfcCompositeCurve` chains
/// polylines, trimmed circles and lines, and nested composites (#43); it is
/// lowered in `composite`, which refuses gaps rather than bridging them.
fn curve_to_contour(model: &Model, id: EntityId, units: &UnitScale) -> GeometryResult<Contour> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    let type_name = entity.type_name.to_ascii_uppercase();
    match type_name.as_str() {
        "IFCPOLYLINE" => {}
        "IFCCOMPOSITECURVE" => return composite::composite_contour(model, id, units),
        _ => {
            return Err(GeometryError::Unsupported {
                entity: id,
                type_name,
                detail: "profile boundaries lower IfcPolyline and IfcCompositeCurve only",
            })
        }
    }

    let slots = Slots::new(id, entity);
    let mut points = polyline_points(model, id, units)?;
    drop_closing_duplicate(&mut points);
    if points.len() < 3 {
        return Err(slots.degenerate("profile boundary has fewer than 3 distinct points"));
    }

    let segments = (0..points.len())
        .map(|index| {
            let origin = points[index];
            let next = points[(index + 1) % points.len()];
            ProfileSegment {
                curve: Curve2::Line(Line2 {
                    origin,
                    direction: next - origin,
                }),
                domain: Interval::UNIT,
                same_sense: true,
            }
        })
        .collect();
    Ok(Contour::new(segments))
}

/// An `IfcPolyline`'s points as 2D metres, in authored order.
///
/// Shared by the closed-ring and composite-segment readers so both apply the
/// same unit conversion and the same 2D check.
fn polyline_points(model: &Model, id: EntityId, units: &UnitScale) -> GeometryResult<Vec<Vec2>> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    let slots = Slots::new(id, entity);
    let mut points = Vec::new();
    for point_id in slots.req_ref_list(0, "Points")? {
        let point = model.get(point_id).ok_or(GeometryError::MissingEntity {
            referrer: id,
            missing: point_id,
        })?;
        let coordinates = Slots::new(point_id, point).req_f64_list(0, "Coordinates")?;
        if coordinates.len() < 2 {
            return Err(GeometryError::Degenerate {
                entity: point_id,
                type_name: point.type_name.to_string(),
                detail: "profile boundary point is not at least 2D".to_string(),
            });
        }
        points.push(Vec2::new(
            units.length(coordinates[0]),
            units.length(coordinates[1]),
        ));
    }
    Ok(points)
}

fn drop_closing_duplicate(points: &mut Vec<Vec2>) {
    if points.len() >= 2 && points[0].distance(*points.last().expect("length checked")) < 1e-12 {
        points.pop();
    }
}

#[cfg(test)]
mod tests {
    /// The refusal the reader raises is the reason this table documents.
    #[test]
    fn the_generic_profile_refusal_matches_the_declared_reason() {
        assert_eq!(
            super::PLANNED_PROFILES[0].1,
            crate::input::profile::GENERIC_PROFILE
        );
    }
}
