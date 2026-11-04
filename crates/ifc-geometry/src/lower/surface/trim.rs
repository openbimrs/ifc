//! The unit of a surface's `(u, v)` parameters, for trims and points on a surface.
//!
//! Each direction has its own unit, so one factor for the pair is wrong on any
//! surface that mixes angles and lengths.

use ifc_model::EntityId;

use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;
use crate::surface::swept::SurfaceOfRevolution;

/// Choose the unit factors for the `(u, v)` parameters of a basis surface.
///
/// Each direction has its own unit, following the parameterisation of the
/// surface type:
///
/// - a plane takes lengths in both directions;
/// - a cylinder takes an angle in `u` and a length along the axis in `v`;
/// - a sphere and a torus take angles in both;
/// - a B-spline surface is parameterised in knot space and stays unscaled;
/// - a surface of revolution takes the revolution angle in `u`, and `v` is
///   the parameter of its generatrix, so its factor is the generatrix's own
///   (see `generatrix_parameter_factor`).
///
/// Any other basis is refused, never given a default factor: a length factor
/// on an angle, or the reverse, still yields a plausible patch. Returning
/// closures keeps the decision at the one place that knows the basis type.
pub(crate) fn trim_converter(
    session: &LoweringSession<'_>,
    entity: EntityId,
    basis_id: EntityId,
) -> GeometryResult<(impl Fn(f64) -> f64, impl Fn(f64) -> f64)> {
    let kind = session.type_name(basis_id)?.to_ascii_uppercase();
    let length = session.units().length_to_metres;
    let (u_factor, v_factor) = match kind.as_str() {
        "IFCPLANE" => (length, length),
        "IFCCYLINDRICALSURFACE" => (session.units().radians_per_unit(entity)?, length),
        "IFCSPHERICALSURFACE" | "IFCTOROIDALSURFACE" => {
            let angle = session.units().radians_per_unit(entity)?;
            (angle, angle)
        }
        "IFCBSPLINESURFACEWITHKNOTS" | "IFCRATIONALBSPLINESURFACEWITHKNOTS" => (1.0, 1.0),
        "IFCSURFACEOFREVOLUTION" => (
            session.units().radians_per_unit(entity)?,
            generatrix_parameter_factor(session, entity, basis_id)?,
        ),
        other => return Err(session.unsupported(
            entity,
            other,
            "trim or surface parameters on this basis surface (parameter units are not modelled)",
        )),
    };
    Ok((
        move |value: f64| value * u_factor,
        move |value: f64| value * v_factor,
    ))
}

/// The unit factor of a surface of revolution's `v`: its generatrix's own
/// curve parameter.
///
/// The profile is unwrapped as [`swept_generatrix`] does and an
/// `IfcTrimmedCurve` is followed to its basis. A conic parameterises by angle,
/// a line or polyline is unscaled; any other curve family has a parameter unit
/// this crate does not model and is refused.
fn generatrix_parameter_factor(
    session: &LoweringSession<'_>,
    entity: EntityId,
    revolution: EntityId,
) -> GeometryResult<f64> {
    let view = SurfaceOfRevolution::new(revolution, session.entity(entity, revolution)?);
    let swept = view.swept_curve_ref()?;
    let kind = session.type_name(swept)?.to_ascii_uppercase();
    let mut curve =
        if kind == "IFCARBITRARYOPENPROFILEDEF" || kind == "IFCARBITRARYCLOSEDPROFILEDEF" {
            session.slots(swept)?.req_ref(2, "Curve")?
        } else {
            swept
        };
    loop {
        let kind = session.type_name(curve)?.to_ascii_uppercase();
        match kind.as_str() {
            "IFCTRIMMEDCURVE" => curve = session.slots(curve)?.req_ref(0, "BasisCurve")?,
            "IFCCIRCLE" | "IFCELLIPSE" | "IFCLINE" | "IFCPOLYLINE" => {
                return crate::lower::curve::parameter_factor(session, entity, &kind)
            }
            other => {
                return Err(session.unsupported(
                    curve,
                    other,
                    "generatrix curve whose parameter unit is not modelled",
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests;
