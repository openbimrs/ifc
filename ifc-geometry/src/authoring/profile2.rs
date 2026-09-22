//! The profile forms beyond the parameterized catalogue sections.
//!
//! Three carry something a slot-filling writer would miss.
//!
//! # `IfcMirroredProfileDef` states nothing of its own
//!
//! It subtypes `IfcDerivedProfileDef` and redeclares `Operator` as
//! **DERIVE**: the mirror transform is fixed by the schema, not chosen
//! by the file. So the slot is written `*`, not `$` and not a
//! hand-built operator -- the same distinction as `IfcOrientedEdge`.
//!
//! # `ProfileType` is not free on every subtype
//!
//! `IfcArbitraryProfileDefWithVoids` requires `AREA`: a profile with
//! voids that claims to be a curve is contradictory. The writer does
//! not take the type as an argument where the schema fixes it.
//!
//! # A rounded rectangle's radius is bounded by the rectangle
//!
//! `ValidRadius` caps `RoundingRadius` at half of each dimension. A
//! larger radius describes no shape, and the check is arithmetic.

use ifc_model::{Entity, EntityId, Transaction, Value};

use super::profile::ProfileType;
use crate::error::GeometryError;
use crate::slots::profile_slot as slot;

use super::std_profile::positive;
use super::{invalid, reals, refs, require_finite};

/// Write `ProfileType` and the optional `ProfileName`.
fn header(attrs: &mut [Value], profile_type: ProfileType, name: Option<&str>) {
    attrs[slot::PROFILE_TYPE] = Value::Enum(profile_type.token().into());
    attrs[slot::PROFILE_NAME] = name.map_or(Value::Null, |n| Value::Text(n.into()));
}

/// Stage an `IfcArbitraryOpenProfileDef`.
///
/// An open profile is a `CURVE`, not an `AREA`: it has no enclosed
/// region to extrude, and is swept as a surface or used as a
/// centreline. The type is fixed here rather than taken from the
/// caller, because `IfcArbitraryOpenProfileDef` has a WHERE rule
/// forbidding `AREA` on everything except its centre-line subtype.
pub fn arbitrary_open_profile(
    tx: &mut Transaction,
    name: Option<&str>,
    curve: EntityId,
) -> EntityId {
    let mut attrs = vec![Value::Null; 3];
    header(&mut attrs, ProfileType::Curve, name);
    attrs[slot::OUTER_CURVE] = Value::Ref(curve);
    tx.create(Entity::new("IFCARBITRARYOPENPROFILEDEF", attrs))
}

/// Stage an `IfcCenterLineProfileDef`.
///
/// The one open profile that *is* an area: the curve is a centreline
/// and `thickness` gives it width, so the profile encloses a region
/// after all. That is why the schema's `AREA` prohibition on open
/// profiles exempts this subtype.
///
/// # Errors
///
/// Refuses a non-positive or non-finite thickness.
pub fn center_line_profile(
    tx: &mut Transaction,
    name: Option<&str>,
    curve: EntityId,
    thickness: f64,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCCENTERLINEPROFILEDEF";
    positive(T, "Thickness", thickness)?;
    let mut attrs = vec![Value::Null; 4];
    header(&mut attrs, ProfileType::Area, name);
    attrs[slot::OUTER_CURVE] = Value::Ref(curve);
    attrs[3] = Value::Real(thickness);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcArbitraryProfileDefWithVoids`.
///
/// # Errors
///
/// Refuses an empty void set: `SET [1:?]`, and a profile with no voids
/// is an `IfcArbitraryClosedProfileDef` instead. The schema also
/// forbids an `IfcLine` as an inner curve -- a straight line encloses
/// nothing -- but that needs the referenced entity's type, which this
/// writer does not resolve; the conformance validator catches it.
pub fn arbitrary_profile_with_voids(
    tx: &mut Transaction,
    name: Option<&str>,
    outer_curve: EntityId,
    inner_curves: &[EntityId],
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCARBITRARYPROFILEDEFWITHVOIDS";
    if inner_curves.is_empty() {
        return Err(invalid(
            T,
            "InnerCurves",
            "expected at least one void; without one this is an IfcArbitraryClosedProfileDef",
        ));
    }
    let mut attrs = vec![Value::Null; 4];
    // WR1 fixes the profile type to AREA, so it is not an argument.
    header(&mut attrs, ProfileType::Area, name);
    attrs[slot::OUTER_CURVE] = Value::Ref(outer_curve);
    attrs[slot::INNER_CURVES] = refs(inner_curves);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcCompositeProfileDef`.
///
/// # Errors
///
/// Refuses fewer than two component profiles: `SET [2:?]`, because a
/// composite of one is just that one.
pub fn composite_profile(
    tx: &mut Transaction,
    profile_type: ProfileType,
    name: Option<&str>,
    profiles: &[EntityId],
    label: Option<&str>,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCCOMPOSITEPROFILEDEF";
    if profiles.len() < 2 {
        return Err(invalid(
            T,
            "Profiles",
            format!("expected at least 2 profiles, got {}", profiles.len()),
        ));
    }
    let mut attrs = vec![Value::Null; 4];
    header(&mut attrs, profile_type, name);
    attrs[2] = refs(profiles);
    attrs[3] = label.map_or(Value::Null, |l| Value::Text(l.into()));
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcDerivedProfileDef`: a parent profile plus a transform.
///
/// `operator` is an `IfcCartesianTransformationOperator2D`. For a plain
/// mirror use [`mirrored_profile`], which the schema derives instead.
pub fn derived_profile(
    tx: &mut Transaction,
    profile_type: ProfileType,
    name: Option<&str>,
    parent: EntityId,
    operator: EntityId,
    label: Option<&str>,
) -> EntityId {
    let mut attrs = vec![Value::Null; 5];
    header(&mut attrs, profile_type, name);
    attrs[2] = Value::Ref(parent);
    attrs[3] = Value::Ref(operator);
    attrs[4] = label.map_or(Value::Null, |l| Value::Text(l.into()));
    tx.create(Entity::new("IFCDERIVEDPROFILEDEF", attrs))
}

/// Stage an `IfcMirroredProfileDef`.
///
/// The mirror is about the y axis and is **fixed by the schema**, which
/// derives `Operator` rather than reading it. So slot 3 is written `*`:
/// writing an operator there would restate a derived value, and writing
/// `$` would claim the transform is absent. Neither is conforming.
///
/// This is why there is no operator argument: there is nothing to
/// choose.
pub fn mirrored_profile(
    tx: &mut Transaction,
    profile_type: ProfileType,
    name: Option<&str>,
    parent: EntityId,
    label: Option<&str>,
) -> EntityId {
    let mut attrs = vec![Value::Null; 5];
    header(&mut attrs, profile_type, name);
    attrs[2] = Value::Ref(parent);
    attrs[3] = Value::Derived;
    attrs[4] = label.map_or(Value::Null, |l| Value::Text(l.into()));
    tx.create(Entity::new("IFCMIRROREDPROFILEDEF", attrs))
}

/// Stage an `IfcRoundedRectangleProfileDef`.
///
/// # Errors
///
/// Refuses a non-positive dimension or radius, and a radius exceeding
/// half of either dimension: `ValidRadius`. At exactly half the smaller
/// dimension the profile becomes a stadium, which is legal.
pub fn rounded_rectangle_profile(
    tx: &mut Transaction,
    name: Option<&str>,
    position: Option<EntityId>,
    x_dim: f64,
    y_dim: f64,
    rounding_radius: f64,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCROUNDEDRECTANGLEPROFILEDEF";
    positive(T, "XDim", x_dim)?;
    positive(T, "YDim", y_dim)?;
    positive(T, "RoundingRadius", rounding_radius)?;
    if rounding_radius > x_dim / 2.0 || rounding_radius > y_dim / 2.0 {
        return Err(invalid(
            T,
            "RoundingRadius",
            format!("{rounding_radius} exceeds half of the {x_dim} by {y_dim} rectangle"),
        ));
    }
    let mut attrs = vec![Value::Null; 6];
    header(&mut attrs, ProfileType::Area, name);
    attrs[slot::POSITION] = position.map_or(Value::Null, Value::Ref);
    attrs[slot::X_DIM] = Value::Real(x_dim);
    attrs[slot::Y_DIM] = Value::Real(y_dim);
    attrs[5] = Value::Real(rounding_radius);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage a bare `IfcProfileDef`.
///
/// The supertype is concrete: it names a section and says whether it
/// bounds an area, without describing its shape. A file may carry one
/// where the geometry is supplied elsewhere, so this exists to
/// round-trip such a model rather than to author new sections --
/// prefer a subtype that states the shape.
pub fn profile_def(
    tx: &mut Transaction,
    profile_type: ProfileType,
    name: Option<&str>,
) -> EntityId {
    let attrs = vec![
        Value::Enum(profile_type.token().into()),
        name.map_or(Value::Null, |n| Value::Text(n.into())),
    ];
    tx.create(Entity::new("IFCPROFILEDEF", attrs))
}

/// Stage an `IfcOpenCrossProfileDef`.
///
/// A road or rail cross-section given as a chain of segments: each
/// width is paired with the slope of that segment, so the two lists
/// run in parallel and must be the same length
/// (`CorrespondingSlopeWidths`). `Tags` names the *points* between
/// segments, so it carries exactly one more entry than there are
/// segments (`CorrespondingTags`).
///
/// `ProfileType` is fixed to CURVE by `CorrectProfileType`: an open
/// chain bounds no area, so it is not a parameter.
///
/// # Errors
///
/// Refuses empty or mismatched width and slope lists, a `tags` list
/// that is not `widths.len() + 1`, a negative width
/// (`IfcNonNegativeLengthMeasure`), and non-finite values.
pub fn open_cross_profile(
    tx: &mut Transaction,
    name: Option<&str>,
    horizontal_widths: bool,
    widths: &[f64],
    slopes: &[f64],
    tags: Option<&[&str]>,
    offset_point: Option<EntityId>,
) -> Result<EntityId, GeometryError> {
    const ENTITY: &str = "IFCOPENCROSSPROFILEDEF";
    if widths.is_empty() {
        return Err(invalid(ENTITY, "Widths", "expected LIST [1:?]"));
    }
    if widths.len() != slopes.len() {
        return Err(invalid(
            ENTITY,
            "Slopes",
            format!(
                "expected one slope per width: {} widths, {} slopes",
                widths.len(),
                slopes.len()
            ),
        ));
    }
    require_finite(ENTITY, "Widths", widths)?;
    require_finite(ENTITY, "Slopes", slopes)?;
    if let Some(bad) = widths.iter().position(|w| *w < 0.0) {
        return Err(invalid(
            ENTITY,
            "Widths",
            format!("width at index {bad} is negative"),
        ));
    }
    if let Some(tags) = tags {
        // Tags name the points between segments, so a chain of n
        // segments has n+1 of them.
        if tags.len() != widths.len() + 1 {
            return Err(invalid(
                ENTITY,
                "Tags",
                format!("expected {} tags, got {}", widths.len() + 1, tags.len()),
            ));
        }
    }
    let attrs = vec![
        // CorrectProfileType fixes this to CURVE.
        Value::Enum(ProfileType::Curve.token().into()),
        name.map_or(Value::Null, |n| Value::Text(n.into())),
        Value::Bool(horizontal_widths),
        reals(widths),
        reals(slopes),
        tags.map_or(Value::Null, |t| {
            Value::List(t.iter().map(|v| Value::Text((*v).into())).collect())
        }),
        offset_point.map_or(Value::Null, Value::Ref),
    ];
    Ok(tx.create(Entity::new(ENTITY, attrs)))
}
