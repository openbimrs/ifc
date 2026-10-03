//! `IfcProfileDef` families read into SI parameters, without a kernel.
//!
//! # One reader, two consumers
//!
//! [`describe_profile`] is the only place a profile family's slots are read.
//! `lower::profile` builds its exact neutral profile from the returned
//! [`ProfileDescription`], and [`crate::body_description`] hands the same
//! value to rule checks. A slot index, a unit conversion or an IFC2X3 layout
//! decision is therefore made once, and the parameters a checker reads are the
//! parameters the kernel receives.
//!
//! It lives in `input` rather than `lower` because a rule check asking "is
//! this beam an HEA300" must not link a geometry kernel to get the answer;
//! `tests/kernel_free_build.rs` holds that line.
//!
//! # Units
//!
//! Lengths are converted to metres and plane angles to radians here, exactly
//! once. That includes the translation of an `IfcDerivedProfileDef` operator,
//! which is a length like any other coordinate.
//!
//! # Refusals
//!
//! A missing required attribute, a dangling reference, a non-physical
//! dimension, a nesting cycle and an unknown family are all typed errors
//! naming the entity. Nothing is defaulted to make a description succeed.

mod open_cross;
mod outline;
mod section;
mod types;

#[cfg(feature = "lowering")]
pub(crate) use open_cross::vertices as open_cross_vertices;

pub use outline::{profile_outline, ProfileOutline};
pub use types::{ProfileDescription, ProfileOperator, ProfileParameters, ProfilePosition};

use ifc_model::{EntityId, Model};

use crate::error::{GeometryError, GeometryResult};
use crate::resource::operator::operator_transform;
use crate::slots::{profile_slot as slot, section_slot, Slots};
use crate::units::UnitScale;

/// Why a bare `IfcProfileDef` is refused.
///
/// `IfcProfileDef` is instantiable in IFC4 (it is not declared ABSTRACT), but
/// it is the bare supertype: it declares only `ProfileType` and
/// `ProfileName`. A file authoring one has supplied a profile *label*, not a
/// section. `lower::profile::PLANNED_PROFILES` states the same reason.
pub(crate) const GENERIC_PROFILE: &str =
    "generic profile declaration carries no concrete geometry to lower";

/// Maximum profile nesting depth.
///
/// `IfcCompositeProfileDef` and `IfcDerivedProfileDef` both reference other
/// profiles, so a broken file can nest them without end. A composite of
/// derived sections is realistic; sixteen levels is not.
const MAX_PROFILE_DEPTH: usize = 16;

/// Chain kind reported for a profile nesting cycle or budget refusal.
const KIND: &str = "profile";

/// Read one `IfcProfileDef` into its family parameters, in metres and radians.
///
/// Every concrete IFC4 profile family is described, and the IFC4X3
/// `IfcOpenCrossProfileDef`. A bare `IfcProfileDef`
/// and any unknown type are refused as [`GeometryError::Unsupported`]; a
/// profile that references itself through a composite or derived chain is
/// refused as [`GeometryError::CyclicChain`].
pub fn describe_profile(
    model: &Model,
    units: &UnitScale,
    profile: EntityId,
) -> GeometryResult<ProfileDescription> {
    let mut chain = Vec::new();
    describe(model, units, profile, profile, &mut chain)
}

/// Describe one profile, tracking the chain of nesting profiles above it.
fn describe(
    model: &Model,
    units: &UnitScale,
    referrer: EntityId,
    id: EntityId,
    chain: &mut Vec<EntityId>,
) -> GeometryResult<ProfileDescription> {
    if chain.contains(&id) {
        return Err(GeometryError::CyclicChain {
            entity: id,
            kind: KIND,
        });
    }
    if chain.len() >= MAX_PROFILE_DEPTH {
        return Err(GeometryError::ChainTooDeep {
            entity: id,
            kind: KIND,
            limit: MAX_PROFILE_DEPTH,
        });
    }
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer,
        missing: id,
    })?;
    let slots = Slots::new(id, entity);
    let type_name = entity.type_name.to_ascii_uppercase();

    chain.push(id);
    let parameters = parameters(model, units, &slots, &type_name, chain);
    chain.pop();
    let parameters = parameters?;

    let position = if is_parameterized(&parameters) {
        position(model, units, &slots)?
    } else {
        None
    };
    Ok(ProfileDescription {
        entity: id,
        type_name,
        name: slots.opt_text(slot::PROFILE_NAME),
        position,
        parameters,
    })
}

/// Dispatch on the concrete family.
fn parameters(
    model: &Model,
    units: &UnitScale,
    slots: &Slots<'_>,
    type_name: &str,
    chain: &mut Vec<EntityId>,
) -> GeometryResult<ProfileParameters> {
    let parameters = match type_name {
        "IFCRECTANGLEPROFILEDEF" => section::rectangle(slots, units, false)?,
        "IFCROUNDEDRECTANGLEPROFILEDEF" => section::rectangle(slots, units, true)?,
        "IFCRECTANGLEHOLLOWPROFILEDEF" => section::rectangle_hollow(slots, units)?,
        "IFCCIRCLEPROFILEDEF" => section::circle(slots, units, false)?,
        "IFCCIRCLEHOLLOWPROFILEDEF" => section::circle(slots, units, true)?,
        "IFCELLIPSEPROFILEDEF" => section::ellipse(slots, units)?,
        "IFCISHAPEPROFILEDEF" => section::i_shape(slots, units)?,
        "IFCASYMMETRICISHAPEPROFILEDEF" => section::asymmetric_i(slots, units, is_ifc2x3(model))?,
        "IFCLSHAPEPROFILEDEF" => section::l_shape(slots, units)?,
        "IFCTSHAPEPROFILEDEF" => section::t_shape(slots, units)?,
        "IFCUSHAPEPROFILEDEF" => section::u_shape(slots, units)?,
        "IFCCSHAPEPROFILEDEF" => section::c_shape(slots, units)?,
        "IFCZSHAPEPROFILEDEF" => section::z_shape(slots, units)?,
        "IFCTRAPEZIUMPROFILEDEF" => section::trapezium(slots, units)?,
        "IFCARBITRARYCLOSEDPROFILEDEF" => ProfileParameters::ArbitraryClosed {
            outer_curve: curve_ref(model, slots, slot::OUTER_CURVE, "OuterCurve")?,
        },
        "IFCARBITRARYPROFILEDEFWITHVOIDS" => {
            let outer_curve = curve_ref(model, slots, slot::OUTER_CURVE, "OuterCurve")?;
            let inner_curves = slots.req_ref_list(slot::INNER_CURVES, "InnerCurves")?;
            for curve in &inner_curves {
                slots.resolve(model, *curve)?;
            }
            ProfileParameters::ArbitraryWithVoids {
                outer_curve,
                inner_curves,
            }
        }
        // An open profile is described, not refused: it is a valid profile,
        // it simply bounds no area. `ProfileDescription::bounds_area` says so,
        // and the area consumers (sweeps, lowering) refuse it there.
        "IFCARBITRARYOPENPROFILEDEF" => ProfileParameters::ArbitraryOpen {
            curve: curve_ref(model, slots, slot::OUTER_CURVE, "Curve")?,
        },
        // IFC4X3. Open like the arbitrary open profile: described, and
        // refused by the area consumers through `bounds_area`.
        "IFCOPENCROSSPROFILEDEF" => open_cross::read(model, units, slots)?,
        "IFCCENTERLINEPROFILEDEF" => ProfileParameters::CenterLine {
            curve: curve_ref(model, slots, section_slot::CL_CURVE, "Curve")?,
            thickness: units.length(slots.req_f64(section_slot::CL_THICKNESS, "Thickness")?),
        },
        "IFCCOMPOSITEPROFILEDEF" => {
            let refs = slots.req_ref_list(section_slot::COMPOSITE_PROFILES, "Profiles")?;
            let mut profiles = Vec::with_capacity(refs.len());
            for member in refs {
                profiles.push(describe(model, units, slots.id(), member, chain)?);
            }
            ProfileParameters::Composite {
                profiles,
                label: slots.opt_text(section_slot::COMPOSITE_LABEL),
            }
        }
        "IFCDERIVEDPROFILEDEF" => {
            let parent = slots.req_ref(section_slot::DERIVED_PARENT, "ParentProfile")?;
            let parent = Box::new(describe(model, units, slots.id(), parent, chain)?);
            ProfileParameters::Derived {
                parent,
                operator: operator(model, units, slots)?,
                label: slots.opt_text(section_slot::DERIVED_LABEL),
            }
        }
        // IfcMirroredProfileDef derives its Operator in the schema, so no file
        // carries one: the mirror about the local y axis is implied by the
        // TYPE alone. Reading Operator here would find `*` and either fail or
        // yield an unmirrored profile, so this subtype cannot share the
        // parent's operator read.
        "IFCMIRROREDPROFILEDEF" => {
            let parent = slots.req_ref(section_slot::DERIVED_PARENT, "ParentProfile")?;
            ProfileParameters::Mirrored {
                parent: Box::new(describe(model, units, slots.id(), parent, chain)?),
                label: slots.opt_text(section_slot::DERIVED_LABEL),
            }
        }
        "IFCPROFILEDEF" => return Err(slots.unsupported(GENERIC_PROFILE)),
        _ => return Err(slots.unsupported("profile family is not interpreted")),
    };
    Ok(parameters)
}

/// Is this an `IfcParameterizedProfileDef` family, which carries `Position`?
fn is_parameterized(parameters: &ProfileParameters) -> bool {
    !matches!(
        parameters,
        ProfileParameters::ArbitraryClosed { .. }
            | ProfileParameters::ArbitraryWithVoids { .. }
            | ProfileParameters::ArbitraryOpen { .. }
            | ProfileParameters::OpenCross { .. }
            | ProfileParameters::CenterLine { .. }
            | ProfileParameters::Composite { .. }
            | ProfileParameters::Derived { .. }
            | ProfileParameters::Mirrored { .. }
    )
}

/// Does the model declare IFC2X3, whose asymmetric-I layout differs?
fn is_ifc2x3(model: &Model) -> bool {
    model
        .header()
        .schema_token()
        .is_some_and(|token| token.to_ascii_uppercase().starts_with("IFC2X3"))
}

/// A required curve reference that must resolve.
fn curve_ref(
    model: &Model,
    slots: &Slots<'_>,
    index: usize,
    name: &'static str,
) -> GeometryResult<EntityId> {
    let curve = slots.req_ref(index, name)?;
    slots.resolve(model, curve)?;
    Ok(curve)
}

/// `IfcParameterizedProfileDef.Position` (slot 2), in metres.
fn position(
    model: &Model,
    units: &UnitScale,
    slots: &Slots<'_>,
) -> GeometryResult<Option<ProfilePosition>> {
    let Some(position_id) = slots.opt_ref(slot::POSITION) else {
        return Ok(None);
    };
    let position = slots.resolve(model, position_id)?;
    let position_slots = Slots::new(position_id, position);
    // IfcPlacement.Location (slot 0), IfcAxis2Placement2D.RefDirection (slot 1).
    let location_id = position_slots.req_ref(0, "Location")?;
    let location = position_slots.resolve(model, location_id)?;
    let coordinates = Slots::new(location_id, location).req_f64_list(0, "Coordinates")?;
    if coordinates.len() < 2 {
        return Err(position_slots.degenerate("2D placement location is not 2D"));
    }
    let origin = [units.length(coordinates[0]), units.length(coordinates[1])];
    let x_axis = match position_slots.opt_ref(1) {
        Some(direction_id) => {
            let direction = position_slots.resolve(model, direction_id)?;
            let ratios = Slots::new(direction_id, direction).req_f64_list(0, "DirectionRatios")?;
            if ratios.len() < 2 {
                return Err(position_slots.degenerate("2D reference direction is not 2D"));
            }
            let length = ratios[0].hypot(ratios[1]);
            if length == 0.0 || !length.is_finite() {
                return Err(position_slots.degenerate("2D reference direction has zero length"));
            }
            [ratios[0] / length, ratios[1] / length]
        }
        None => [1.0, 0.0],
    };
    Ok(Some(ProfilePosition { origin, x_axis }))
}

/// `IfcDerivedProfileDef.Operator` (slot 3) as a 2D map, translation in metres.
///
/// Reuses the shared operator reader, which handles the uniform and
/// non-uniform forms. A profile lives in its own xy plane, so any z component
/// means the file used a 3D operator where the schema requires a 2D one;
/// refusing beats silently projecting geometry away.
fn operator(
    model: &Model,
    units: &UnitScale,
    slots: &Slots<'_>,
) -> GeometryResult<ProfileOperator> {
    let operator = slots.req_ref(section_slot::DERIVED_OPERATOR, "Operator")?;
    let entity = slots.resolve(model, operator)?;
    let full = operator_transform(model, operator, entity)?;
    let [x, y, z] = full.basis;
    let z_leak = z[0].abs() + z[1].abs() + x[2].abs() + y[2].abs();
    if z_leak > 1e-9 || full.origin[2].abs() > 1e-9 {
        return Err(Slots::new(operator, entity)
            .unsupported("profile operator has out-of-plane components"));
    }
    Ok(ProfileOperator {
        x_axis: [x[0], x[1]],
        y_axis: [y[0], y[1]],
        origin: [units.length(full.origin[0]), units.length(full.origin[1])],
    })
}
