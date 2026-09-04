//! Total representation-item dispatch.
//!
//! # Why totality matters here
//!
//! `GeometryNode` is `#[non_exhaustive]` and the crate contract says an
//! unknown family must become a typed `Unsupported` result, never a panic and
//! never a silently substituted shape. This dispatcher is the single place
//! that decides which IFC representation items are implemented, so coverage is
//! auditable from one table instead of scattered across families.

use axiolid_model::NodeId;
use ifc_model::EntityId;

use crate::error::GeometryResult;
use crate::lower::bbox::lower_bounding_box_node;
use crate::lower::boolean::lower_boolean_result_node;
use crate::lower::brep::lower_faceted_brep_node;
use crate::lower::collection::lower_collection_node;
use crate::lower::csg::{
    lower_csg_primitive_node, lower_csg_solid_node, lower_surface_curve_swept_area_solid_node,
    lower_swept_disk_node,
};
use crate::lower::curve::lower_curve_node;
use crate::lower::halfspace::lower_half_space_node;
use crate::lower::mapped::lower_mapped_item_node;
use crate::lower::point::{lower_point_on_curve_node, lower_point_on_surface_node};
use crate::lower::session::LoweringSession;
use crate::lower::surface::lower_surface_node;
use crate::lower::swept::{
    lower_extruded_area_solid_node, lower_fixed_reference_sweep_node,
    lower_revolved_area_solid_node, lower_sectioned_spine_node, lower_tapered_extrusion_node,
    lower_tapered_revolution_node,
};
use crate::lower::tessellated::{lower_polygonal_face_set_node, lower_triangulated_face_set_node};
use crate::select::is_a;
use crate::transform::Transform;

/// Families this crate lowers today, paired with what is still missing.
///
/// Kept as data so the census test can assert on it rather than re-deriving
/// the list by scraping source text.
pub const IMPLEMENTED: &[&str] = &[
    "IFCEXTRUDEDAREASOLID",
    "IFCREVOLVEDAREASOLID",
    "IFCBOOLEANRESULT",
    "IFCBOOLEANCLIPPINGRESULT",
    "IFCMAPPEDITEM",
    "IFCFACETEDBREP",
    "IFCFACETEDBREPWITHVOIDS",
    "IFCADVANCEDBREP",
    "IFCADVANCEDBREPWITHVOIDS",
    "IFCHALFSPACESOLID",
    "IFCBOXEDHALFSPACE",
    "IFCTRIANGULATEDFACESET",
    "IFCPOLYGONALFACESET",
    "IFCCSGSOLID",
    "IFCSWEPTDISKSOLID",
    "IFCSWEPTDISKSOLIDPOLYGONAL",
    "IFCSURFACECURVESWEPTAREASOLID",
    "IFCBLOCK",
    "IFCSPHERE",
    "IFCRIGHTCIRCULARCYLINDER",
    "IFCRIGHTCIRCULARCONE",
    "IFCRECTANGULARPYRAMID",
    "IFCBOUNDINGBOX",
    "IFCEXTRUDEDAREASOLIDTAPERED",
    "IFCREVOLVEDAREASOLIDTAPERED",
    "IFCFIXEDREFERENCESWEPTAREASOLID",
    "IFCSECTIONEDSPINE",
    "IFCSHELLBASEDSURFACEMODEL",
    "IFCFACEBASEDSURFACEMODEL",
    "IFCGEOMETRICSET",
    "IFCGEOMETRICCURVESET",
    // Bare curves/surfaces are valid representation items in Curve2D,
    // Curve3D, SurfaceModel, and plan representations.
    "IFCLINE",
    "IFCCIRCLE",
    "IFCELLIPSE",
    "IFCPOLYLINE",
    "IFCINDEXEDPOLYCURVE",
    "IFCCOMPOSITECURVE",
    "IFCCOMPOSITECURVEONSURFACE",
    "IFCBOUNDARYCURVE",
    "IFCOUTERBOUNDARYCURVE",
    "IFCTRIMMEDCURVE",
    "IFCOFFSETCURVE2D",
    "IFCOFFSETCURVE3D",
    "IFCPCURVE",
    "IFCSURFACECURVE",
    "IFCINTERSECTIONCURVE",
    "IFCSEAMCURVE",
    "IFCBSPLINECURVEWITHKNOTS",
    "IFCRATIONALBSPLINECURVEWITHKNOTS",
    "IFCPLANE",
    "IFCCYLINDRICALSURFACE",
    "IFCSPHERICALSURFACE",
    "IFCTOROIDALSURFACE",
    "IFCSURFACEOFLINEAREXTRUSION",
    "IFCSURFACEOFREVOLUTION",
    "IFCRECTANGULARTRIMMEDSURFACE",
    "IFCCURVEBOUNDEDPLANE",
    "IFCCURVEBOUNDEDSURFACE",
    "IFCBSPLINESURFACEWITHKNOTS",
    "IFCRATIONALBSPLINESURFACEWITHKNOTS",
    "IFCPOINTONCURVE",
    "IFCPOINTONSURFACE",
];

/// Recognized representation items that are not lowered yet.
///
/// Each entry names the concrete reason so a caller building a viewer can
/// report progress instead of a bare failure. Adding a family here is how a
/// stub is declared; implementing it means moving the name to [`IMPLEMENTED`].
pub const PLANNED: &[(&str, &str)] = &[(
    "IFCPOLYGONALBOUNDEDHALFSPACE",
    "polygonal boundary cannot be discarded; exact bounded-half-space support is required",
)];

/// Lower any representation item into the caller's session.
///
/// Returns the node for implemented families and a typed
/// [`crate::GeometryError::Unsupported`] naming the source entity otherwise.
pub fn lower_representation_item(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let type_name = session.type_name(id)?;
    // Shape representations may legitimately contain bare curve and surface
    // items (Curve2D/Curve3D/SurfaceModel). Route by generated IFC inheritance
    // before the concrete solid table so plan and surface selections lower
    // through the same total entry point as body geometry.
    if is_a(&type_name, "IFCCURVE") {
        return lower_curve_node(session, id, frame);
    }
    if is_a(&type_name, "IFCSURFACE") {
        return lower_surface_node(session, id, frame);
    }
    match type_name.as_str() {
        "IFCEXTRUDEDAREASOLID" => lower_extruded_area_solid_node(session, id, frame),
        "IFCREVOLVEDAREASOLID" => lower_revolved_area_solid_node(session, id, frame),
        "IFCBOOLEANRESULT" | "IFCBOOLEANCLIPPINGRESULT" => {
            lower_boolean_result_node(session, id, frame)
        }
        "IFCHALFSPACESOLID" | "IFCBOXEDHALFSPACE" | "IFCPOLYGONALBOUNDEDHALFSPACE" => {
            lower_half_space_node(session, id, frame)
        }
        "IFCMAPPEDITEM" => lower_mapped_item_node(session, id, frame),
        "IFCPOINTONCURVE" => lower_point_on_curve_node(session, id, frame),
        "IFCPOINTONSURFACE" => lower_point_on_surface_node(session, id, frame),
        "IFCFACETEDBREP"
        | "IFCFACETEDBREPWITHVOIDS"
        | "IFCADVANCEDBREP"
        | "IFCADVANCEDBREPWITHVOIDS" => lower_faceted_brep_node(session, id, frame),
        "IFCTRIANGULATEDFACESET" => lower_triangulated_face_set_node(session, id, frame),
        "IFCPOLYGONALFACESET" => lower_polygonal_face_set_node(session, id, frame),
        "IFCCSGSOLID" => lower_csg_solid_node(session, id, frame),
        "IFCSWEPTDISKSOLID" | "IFCSWEPTDISKSOLIDPOLYGONAL" => {
            lower_swept_disk_node(session, id, frame)
        }
        "IFCSURFACECURVESWEPTAREASOLID" => {
            lower_surface_curve_swept_area_solid_node(session, id, frame)
        }
        "IFCBOUNDINGBOX" => lower_bounding_box_node(session, id, frame),
        "IFCEXTRUDEDAREASOLIDTAPERED" => lower_tapered_extrusion_node(session, id, frame),
        "IFCREVOLVEDAREASOLIDTAPERED" => lower_tapered_revolution_node(session, id, frame),
        "IFCFIXEDREFERENCESWEPTAREASOLID" => lower_fixed_reference_sweep_node(session, id, frame),
        "IFCSECTIONEDSPINE" => lower_sectioned_spine_node(session, id, frame),
        "IFCSHELLBASEDSURFACEMODEL"
        | "IFCFACEBASEDSURFACEMODEL"
        | "IFCGEOMETRICSET"
        | "IFCGEOMETRICCURVESET" => lower_collection_node(session, id, frame),
        "IFCBLOCK"
        | "IFCSPHERE"
        | "IFCRIGHTCIRCULARCYLINDER"
        | "IFCRIGHTCIRCULARCONE"
        | "IFCRECTANGULARPYRAMID" => lower_csg_primitive_node(session, id, frame),
        other => Err(session.unsupported(id, other, detail_for(other))),
    }
}

/// The documented reason a recognized family is not lowered yet.
fn detail_for(type_name: &str) -> &'static str {
    PLANNED
        .iter()
        .find(|(name, _)| *name == type_name)
        .map(|(_, detail)| *detail)
        .unwrap_or("representation item family is not lowered yet")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_and_planned_families_do_not_overlap() {
        for name in IMPLEMENTED {
            assert!(
                !PLANNED.iter().any(|(planned, _)| planned == name),
                "{name} is listed as both implemented and planned"
            );
        }
    }

    #[test]
    fn every_planned_family_states_a_concrete_reason() {
        for (name, detail) in PLANNED {
            assert!(!detail.is_empty(), "{name} has no stated reason");
            assert_ne!(
                *detail, "unsupported",
                "{name} must say what specifically is missing"
            );
        }
    }
}
