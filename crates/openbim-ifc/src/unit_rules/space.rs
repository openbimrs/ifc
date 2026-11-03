//! The parameter space of a curve or surface instance: unitless, a length,
//! a plane angle, a mix of angles and other ranges, or undefined for these
//! conversions.
//!
//! The parameterisations are ISO 10303-42's, as the IFC entity definitions
//! adopt them (IFC4 ADD2 TC1, IFC4.3 ADD2):
//!
//! - `IfcLine`: `Pnt + u * Dir`, `Dir` an `IfcVector` whose `Magnitude` is
//!   a length, so `u` is a multiple of the vector: unitless.
//! - `IfcCircle`, `IfcEllipse`: `u` is an angle in the plane-angle unit.
//! - `IfcPolyline`: segment `i` spans `i - 1 <= u <= i`. `IfcIndexedPolyCurve`:
//!   the sum of its segments' parametric lengths, 1 per line segment and an
//!   angle per arc. Both unitless.
//! - `IfcBSplineCurve`: the knot space.
//! - `IfcClothoid` (IFC4.3): `s = A u sqrt(pi)`, so `u` is unitless.
//! - `IfcCompositeCurve`: "an accumulation of the parametric ranges of the
//!   referenced bounded curves", unitless when each is, and a reparametrised
//!   segment's `ParamLength` is unitless by itself.
//! - `IfcTrimmedCurve`, `IfcOffsetCurve2D`, `IfcOffsetCurve3D`: the basis
//!   curve's ("takes its parameterization from the basis curve").
//! - `IfcSurfaceCurve` and its subtypes: `Curve3D`'s when that is the master
//!   representation.
//! - `IfcPlane`: `C + u x + v y`, "the length unit for the unit vectors x
//!   and y is derived from the context": both lengths.
//! - `IfcCylindricalSurface`: `u` an angle, `v` along the axis "lower bound
//!   in z": a length.
//! - `IfcSphericalSurface`, `IfcToroidalSurface`: both angles.
//! - `IfcBSplineSurface`: knot spaces.
//! - `IfcSurfaceOfLinearExtrusion`: `lambda(u) + v V`, `V` the extrusion
//!   vector of length `Depth`, so `v` is unitless and `u` the swept curve's.
//! - `IfcSurfaceOfRevolution`: `u` an angle, `v` the swept curve's.
//! - `IfcRectangularTrimmedSurface`, `IfcCurveBoundedPlane`,
//!   `IfcCurveBoundedSurface`: the basis surface's.
//!
//! Any other curve or surface (spirals other than the clothoid, whose
//! parameterisations IFC4.3 states only in figures; `IfcPolynomialCurve`;
//! `IfcCurveSegment`, whose `IfcParameterValue` IFC4.3 leaves undefined;
//! `IfcPcurve`; `IfcOffsetCurveByDistances`; sectioned surfaces) is
//! undefined here, and a parameter value on it is refused.

use std::sync::Arc;

use ifc_model::{EntityId, Value};
use ifc_schema::Schema;

use super::Records;

/// A parameter direction's unit.
///
/// Length conversion (#469) rescales [`Space::Length`] and keeps every
/// other space; SI input (#638) also converts [`Space::PlaneAngle`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Space {
    /// Unitless (a knot, a multiple, an ordinal): kept.
    Unitless,
    /// A length in the project unit: rescaled.
    Length,
    /// An angle in the project plane-angle unit.
    PlaneAngle,
    /// Ranges of angles and of other units along one curve (a composite
    /// curve with an arc segment, an indexed poly curve with arcs): no single
    /// factor applies.
    AngleRanges,
}

/// Why a parameter space is undefined: the host and the reason.
pub(crate) type Undefined = (EntityId, Arc<str>, Arc<str>);

const DEPTH: usize = 64;

/// The value of attribute `name` of `entity`, by the release's layout.
pub(crate) fn attribute<'m, R: Records + ?Sized>(
    schema: &Schema,
    model: &'m R,
    entity: EntityId,
    name: &str,
) -> Option<&'m Value> {
    let record = model.record(entity)?;
    let slot = (0..schema.attribute_count(&record.type_name)).find(|slot| {
        schema
            .attribute_at(&record.type_name, *slot)
            .is_some_and(|attribute| attribute.name.eq_ignore_ascii_case(name))
    })?;
    record.attributes.get(slot)
}

/// The entity attribute `name` of `entity` refers to.
pub(crate) fn reference<R: Records + ?Sized>(
    schema: &Schema,
    model: &R,
    entity: EntityId,
    name: &str,
) -> Option<EntityId> {
    match attribute(schema, model, entity, name)? {
        Value::Ref(id) => Some(*id),
        _ => None,
    }
}

fn type_of<R: Records + ?Sized>(model: &R, id: EntityId) -> Arc<str> {
    model
        .record(id)
        .map_or_else(|| "(missing)".into(), |entity| entity.type_name.clone())
}

fn undefined<R: Records + ?Sized>(model: &R, id: EntityId, detail: &str) -> Undefined {
    (id, type_of(model, id), detail.into())
}

/// The parameter space of curve `curve`.
pub(crate) fn curve<R: Records + ?Sized>(
    schema: &Schema,
    model: &R,
    curve: EntityId,
) -> Result<Space, Undefined> {
    curve_at(schema, model, curve, 0)
}

fn curve_at<R: Records + ?Sized>(
    schema: &Schema,
    model: &R,
    id: EntityId,
    depth: usize,
) -> Result<Space, Undefined> {
    if depth > DEPTH {
        return Err(undefined(
            model,
            id,
            "the curve chain is deeper than the budget",
        ));
    }
    let Some(entity) = model.record(id) else {
        return Err(undefined(model, id, "the curve does not exist"));
    };
    let is_a = |ancestor| schema.is_a(&entity.type_name, ancestor);
    let basis = |name| {
        reference(schema, model, id, name)
            .ok_or_else(|| undefined(model, id, "the basis curve is not a reference"))
            .and_then(|basis| curve_at(schema, model, basis, depth + 1))
    };
    if is_a("IFCCONIC") {
        Ok(Space::PlaneAngle)
    } else if is_a("IFCINDEXEDPOLYCURVE") {
        Ok(poly_curve(schema, model, id))
    } else if is_a("IFCLINE")
        || is_a("IFCPOLYLINE")
        || is_a("IFCBSPLINECURVE")
        || is_a("IFCCLOTHOID")
    {
        Ok(Space::Unitless)
    } else if is_a("IFCTRIMMEDCURVE") || is_a("IFCOFFSETCURVE2D") || is_a("IFCOFFSETCURVE3D") {
        basis("BasisCurve")
    } else if is_a("IFCSURFACECURVE") {
        match attribute(schema, model, id, "MasterRepresentation") {
            Some(Value::Enum(master)) if master.eq_ignore_ascii_case("CURVE3D") => basis("Curve3D"),
            _ => Err(undefined(
                model,
                id,
                "a surface curve parameterised by a p-curve, whose space is the surface's",
            )),
        }
    } else if is_a("IFCCOMPOSITECURVE") {
        composite(schema, model, id, depth)
    } else {
        Err(undefined(
            model,
            id,
            "IFC states no parameterisation of this curve that the conversion can carry",
        ))
    }
}

/// An indexed poly curve's space: its parameter runs 1 per line segment and
/// an angle per arc.
fn poly_curve<R: Records + ?Sized>(schema: &Schema, model: &R, id: EntityId) -> Space {
    let arcs = matches!(
        attribute(schema, model, id, "Segments"),
        Some(Value::List(segments)) if segments.iter().any(|segment| matches!(
            segment,
            Value::Typed { type_name, .. } if type_name.eq_ignore_ascii_case("IFCARCINDEX")
        ))
    );
    if arcs {
        Space::AngleRanges
    } else {
        Space::Unitless
    }
}

fn composite<R: Records + ?Sized>(
    schema: &Schema,
    model: &R,
    id: EntityId,
    depth: usize,
) -> Result<Space, Undefined> {
    let Some(Value::List(segments)) = attribute(schema, model, id, "Segments") else {
        return Err(undefined(model, id, "the segments are not a list"));
    };
    let mut space = Space::Unitless;
    let mut angles = false;
    for segment in segments {
        let Value::Ref(segment) = segment else {
            return Err(undefined(model, id, "a segment is not a reference"));
        };
        let Some(record) = model.record(*segment) else {
            return Err(undefined(model, *segment, "the segment does not exist"));
        };
        if schema.is_a(&record.type_name, "IFCREPARAMETRISEDCOMPOSITECURVESEGMENT") {
            continue;
        }
        if !schema.is_a(&record.type_name, "IFCCOMPOSITECURVESEGMENT") {
            return Err(undefined(
                model,
                *segment,
                "IFC4.3 ADD2 defines no parametric space for an IfcCurveSegment's parent curve",
            ));
        }
        let parent = reference(schema, model, *segment, "ParentCurve")
            .ok_or_else(|| undefined(model, *segment, "the parent curve is not a reference"))?;
        match curve_at(schema, model, parent, depth + 1)? {
            Space::Length => space = Space::Length,
            Space::PlaneAngle | Space::AngleRanges => angles = true,
            Space::Unitless => {}
        }
    }
    if space == Space::Length {
        // A mix would need a factor per segment range.
        return Err(undefined(
            model,
            id,
            "segments mix length and unitless parameters",
        ));
    }
    Ok(if angles { Space::AngleRanges } else { space })
}

/// Which direction of a surface's parameter space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    /// `u`.
    U,
    /// `v`.
    V,
}

/// The parameter space of surface `surface` in `direction`.
pub(crate) fn surface<R: Records + ?Sized>(
    schema: &Schema,
    model: &R,
    surface: EntityId,
    direction: Direction,
) -> Result<Space, Undefined> {
    surface_at(schema, model, surface, direction, 0)
}

fn surface_at<R: Records + ?Sized>(
    schema: &Schema,
    model: &R,
    id: EntityId,
    direction: Direction,
    depth: usize,
) -> Result<Space, Undefined> {
    if depth > DEPTH {
        return Err(undefined(
            model,
            id,
            "the surface chain is deeper than the budget",
        ));
    }
    let Some(entity) = model.record(id) else {
        return Err(undefined(model, id, "the surface does not exist"));
    };
    let is_a = |ancestor| schema.is_a(&entity.type_name, ancestor);
    let basis = || {
        reference(schema, model, id, "BasisSurface")
            .ok_or_else(|| undefined(model, id, "the basis surface is not a reference"))
            .and_then(|basis| surface_at(schema, model, basis, direction, depth + 1))
    };
    let swept_curve = || {
        let profile = reference(schema, model, id, "SweptCurve")
            .ok_or_else(|| undefined(model, id, "the swept curve is not a reference"))?;
        let curve = profile_curve(schema, model, profile)
            .ok_or_else(|| undefined(model, profile, "a profile with no curve of its own"))?;
        curve_at(schema, model, curve, depth + 1)
    };
    match direction {
        _ if is_a("IFCPLANE") => Ok(Space::Length),
        Direction::U if is_a("IFCCYLINDRICALSURFACE") => Ok(Space::PlaneAngle),
        Direction::V if is_a("IFCCYLINDRICALSURFACE") => Ok(Space::Length),
        _ if is_a("IFCSPHERICALSURFACE") || is_a("IFCTOROIDALSURFACE") => Ok(Space::PlaneAngle),
        _ if is_a("IFCBSPLINESURFACE") => Ok(Space::Unitless),
        Direction::U if is_a("IFCSURFACEOFLINEAREXTRUSION") => swept_curve(),
        Direction::V if is_a("IFCSURFACEOFLINEAREXTRUSION") => Ok(Space::Unitless),
        Direction::U if is_a("IFCSURFACEOFREVOLUTION") => Ok(Space::PlaneAngle),
        Direction::V if is_a("IFCSURFACEOFREVOLUTION") => swept_curve(),
        _ if is_a("IFCRECTANGULARTRIMMEDSURFACE")
            || is_a("IFCCURVEBOUNDEDPLANE")
            || is_a("IFCCURVEBOUNDEDSURFACE") =>
        {
            basis()
        }
        _ => Err(undefined(
            model,
            id,
            "IFC states no parameterisation of this surface that the conversion can carry",
        )),
    }
}

/// The curve an arbitrary profile is drawn by.
fn profile_curve<R: Records + ?Sized>(
    schema: &Schema,
    model: &R,
    profile: EntityId,
) -> Option<EntityId> {
    let record = model.record(profile)?;
    if schema.is_a(&record.type_name, "IFCARBITRARYOPENPROFILEDEF") {
        reference(schema, model, profile, "Curve")
    } else if schema.is_a(&record.type_name, "IFCARBITRARYCLOSEDPROFILEDEF") {
        reference(schema, model, profile, "OuterCurve")
    } else {
        None
    }
}

/// The surface whose parameter space an `IfcPcurve` is drawn in, when its
/// coordinates are all lengths (a plane, also through a trimmed or bounded
/// plane); otherwise that surface and its type.
pub(crate) fn pcurve_surface<R: Records + ?Sized>(
    schema: &Schema,
    model: &R,
    pcurve: EntityId,
) -> Result<(), (EntityId, Arc<str>)> {
    let Some(surface) = reference(schema, model, pcurve, "BasisSurface") else {
        return Err((pcurve, type_of(model, pcurve)));
    };
    let both = surface_at(schema, model, surface, Direction::U, 0)
        .ok()
        .zip(surface_at(schema, model, surface, Direction::V, 0).ok());
    match both {
        Some((Space::Length, Space::Length)) => Ok(()),
        _ => Err((surface, type_of(model, surface))),
    }
}

#[cfg(all(test, feature = "step"))]
mod tests {
    use super::*;
    use crate::{Codec, Model, StepCodec};
    use ifc_properties::SchemaVersion;

    fn ifc4(data: &str) -> (Model, &'static Schema) {
        let text = format!(
            "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
             FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\n\
             ENDSEC;\nDATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
        );
        let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
        let schema = ifc_schema::for_version(SchemaVersion::Ifc4).expect("IFC4 is bundled");
        (model, schema)
    }

    const BASE: &str = "#1=IFCCARTESIANPOINT((0.,0.,0.));
#2=IFCAXIS2PLACEMENT3D(#1,$,$);
#3=IFCCIRCLE(#2,1.);
#4=IFCCYLINDRICALSURFACE(#2,1.);
#5=IFCDIRECTION((1.,0.,0.));
#6=IFCVECTOR(#5,1.);
#7=IFCLINE(#1,#6);
#8=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#3);
#9=IFCCOMPOSITECURVE((#8),.F.);
#10=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#7);
#11=IFCCOMPOSITECURVE((#10),.F.);
#12=IFCTRIMMEDCURVE(#3,(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(1.)),.T.,.PARAMETER.);
#13=IFCSPHERICALSURFACE(#2,1.);
#14=IFCINDEXEDPOLYCURVE(#15,(IFCARCINDEX((1,2,3))),.F.);
#15=IFCCARTESIANPOINTLIST2D(((0.,0.),(1.,1.),(2.,0.)));
#16=IFCINDEXEDPOLYCURVE(#15,(IFCLINEINDEX((1,2))),.F.);";

    #[test]
    fn angles_are_planes_angles_and_lengths_are_lengths() {
        let (model, schema) = ifc4(BASE);
        let at = |id| EntityId(id);
        assert_eq!(curve(schema, &model, at(3)), Ok(Space::PlaneAngle));
        assert_eq!(curve(schema, &model, at(12)), Ok(Space::PlaneAngle));
        assert_eq!(curve(schema, &model, at(7)), Ok(Space::Unitless));
        let u = surface(schema, &model, at(4), Direction::U);
        let v = surface(schema, &model, at(4), Direction::V);
        assert_eq!((u, v), (Ok(Space::PlaneAngle), Ok(Space::Length)));
        let sphere = (
            surface(schema, &model, at(13), Direction::U),
            surface(schema, &model, at(13), Direction::V),
        );
        assert_eq!(sphere, (Ok(Space::PlaneAngle), Ok(Space::PlaneAngle)));
    }

    #[test]
    fn a_composite_of_arcs_is_a_mix_of_ranges() {
        let (model, schema) = ifc4(BASE);
        assert_eq!(curve(schema, &model, EntityId(9)), Ok(Space::AngleRanges));
        assert_eq!(curve(schema, &model, EntityId(11)), Ok(Space::Unitless));
        assert_eq!(curve(schema, &model, EntityId(14)), Ok(Space::AngleRanges));
        assert_eq!(curve(schema, &model, EntityId(16)), Ok(Space::Unitless));
    }
}
