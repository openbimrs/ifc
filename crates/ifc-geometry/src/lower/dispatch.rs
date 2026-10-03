//! Total representation-item dispatch.
//!
//! # Why totality matters here
//!
//! `GeometryNode` is `#[non_exhaustive]` and the crate contract says an
//! unknown family must become a typed `Unsupported` result, never a panic and
//! never a silently substituted shape. This dispatcher is the single place
//! that decides which IFC representation items are implemented, so coverage is
//! auditable from one table instead of scattered across families.
//!
//! # Coverage is per release
//!
//! [`IMPLEMENTED`], [`PLANNED`] and [`PARTIAL`] together classify every
//! concrete representation item of IFC4 ADD2 TC1 and IFC4X3 ADD2 that is a
//! root item rather than nested input; `tests/schema_coverage.rs` and
//! `tests/lower_dispatch_corpus.rs` derive both inventories from the schemas
//! and fail on an unclassified family.
//!
//! # Subtypes
//!
//! Dispatch matches exact type names, because most subtypes change meaning
//! (`IfcGradientCurve` is an `IfcCompositeCurve` whose segments are heights
//! over a horizontal base curve). A subtype the supertype's lowering handles
//! exactly is routed through [`SPECIALISATIONS`], which names the attributes
//! it adds and why ignoring or checking them is exact.

use axiolid_model::NodeId;
use ifc_model::EntityId;

use crate::error::GeometryResult;
use crate::lower::bbox::lower_bounding_box_node;
use crate::lower::boolean::lower_boolean_result_node;
use crate::lower::brep::{lower_face_surface_node, lower_faceted_brep_node};
use crate::lower::collection::lower_collection_node;
use crate::lower::csg::{
    lower_csg_primitive_node, lower_csg_solid_node, lower_surface_curve_swept_area_solid_node,
    lower_swept_disk_node,
};
use crate::lower::curve::lower_curve_node;
use crate::lower::halfspace::lower_half_space_node;
use crate::lower::mapped::lower_mapped_item_node;
use crate::lower::point::{lower_point_on_curve_node, lower_point_on_surface_node};
use crate::lower::sectioned::{
    lower_sectioned_solid_horizontal_node, lower_sectioned_surface_node,
};
use crate::lower::session::LoweringSession;
use crate::lower::surface::lower_surface_node;
use crate::lower::swept::{
    lower_directrix_derived_reference_sweep_node, lower_extruded_area_solid_node,
    lower_fixed_reference_sweep_node, lower_revolved_area_solid_node, lower_sectioned_spine_node,
    lower_tapered_extrusion_node, lower_tapered_revolution_node,
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
    "IFCPOLYGONALBOUNDEDHALFSPACE",
    "IFCTRIANGULATEDFACESET",
    // IFC4X3: routed through SPECIALISATIONS; voids and holes are refused.
    "IFCTRIANGULATEDIRREGULARNETWORK",
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
    // IFC4X3: tangent-only directrix; a tangent-plane directrix is refused.
    "IFCDIRECTRIXDERIVEDREFERENCESWEPTAREASOLID",
    "IFCSECTIONEDSPINE",
    "IFCSHELLBASEDSURFACEMODEL",
    "IFCFACEBASEDSURFACEMODEL",
    // A face surface is a legal item and a member of IfcSurfaceOrFaceSurface
    // (connection surfaces); it lowers as a single-face open BRep.
    "IFCFACESURFACE",
    "IFCADVANCEDFACE",
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
    // IFC4X3 alignment geometry (#243): placed segments and centrelines.
    "IFCCURVESEGMENT",
    "IFCGRADIENTCURVE",
];

/// Recognized representation items that are not lowered yet.
///
/// Each entry names the concrete reason so a caller building a viewer can
/// report progress instead of a bare failure. Adding a family here is how a
/// stub is declared; implementing it means moving the name to [`IMPLEMENTED`].
/// The dispatcher reports the reason in its typed `Unsupported` refusal.
///
/// Every entry is an IFC4X3 ADD2 family: every IFC4 ADD2 TC1 root item is
/// lowered. Each reason is the runtime refusal text, which
/// `tests/lower_dispatch_corpus.rs` keeps equal.
///
/// The IFC4X3 spirals and `IfcPolynomialCurve` are unbounded on their own;
/// they lower exactly as the `ParentCurve` of an `IfcCurveSegment`, which is
/// where IFC4.3 uses them.
pub const PLANNED: &[(&str, &str)] = &[
    // Alignment curves (IfcGeometryResource, IFC4X3); `lower::curve`.
    ("IFCCLOTHOID", "an IfcSpiral is unbounded (-inf < u < inf) and the neutral intrinsic curve needs a \
         finite arc length; it lowers exactly as the ParentCurve of an IfcCurveSegment"),
    ("IFCSECONDORDERPOLYNOMIALSPIRAL", "an IfcSpiral is unbounded (-inf < u < inf) and the neutral intrinsic curve needs a \
         finite arc length; it lowers exactly as the ParentCurve of an IfcCurveSegment"),
    ("IFCTHIRDORDERPOLYNOMIALSPIRAL", "an IfcSpiral is unbounded (-inf < u < inf) and the neutral intrinsic curve needs a \
         finite arc length; it lowers exactly as the ParentCurve of an IfcCurveSegment"),
    ("IFCSEVENTHORDERPOLYNOMIALSPIRAL", "an IfcSpiral is unbounded (-inf < u < inf) and the neutral intrinsic curve needs a \
         finite arc length; it lowers exactly as the ParentCurve of an IfcCurveSegment"),
    ("IFCCOSINESPIRAL", "an IfcCosineSpiral or IfcSineSpiral law depends on the length L of the IfcCurveSegment \
         using it; it lowers exactly only as the ParentCurve of an IfcCurveSegment"),
    ("IFCSINESPIRAL", "an IfcCosineSpiral or IfcSineSpiral law depends on the length L of the IfcCurveSegment \
         using it; it lowers exactly only as the ParentCurve of an IfcCurveSegment"),
    ("IFCPOLYNOMIALCURVE", "an IfcPolynomialCurve is unbounded (-inf < u < inf) and the neutral vocabulary has no \
         unbounded polynomial curve; it lowers exactly as the ParentCurve of an IfcCurveSegment"),
    ("IFCSEGMENTEDREFERENCECURVE", "IfcSegmentedReferenceCurve states cant through segments placed at stations along its base \
         curve and parent curves with no normative mapping to a cant law; the business cant layout \
         lowers to a banked curve through ifc-alignment (#311)"),
    // Sections along an alignment (IFC4X3); `lower::sectioned` raises these.
    (
        "IFCSECTIONEDSOLIDHORIZONTAL",
        "kernel: sections stand at IfcAxis2PlacementLinear stations (a measure \
         along the directrix plus offsets) and are swept horizontally with \
         tag-matched linear interpolation; the neutral SectionedSpine takes only \
         resolved section frames, and resolving a station is curve evaluation",
    ),
    (
        "IFCSECTIONEDSURFACE",
        "kernel: no neutral sectioned-surface relation exists; its open sections \
         stand at IfcAxis2PlacementLinear stations along the directrix and are \
         joined by tag, and the neutral SectionedSpine is a solid over area \
         profiles",
    ),
    // Distance-along-curve geometry (IFC4X3).
    (
        "IFCOFFSETCURVEBYDISTANCES",
        "offsets are stated at stations along the basis curve as \
         IfcPointByDistanceExpression values; the neutral model has no \
         distance-along-curve point or station-offset curve to hold them",
    ),
    (
        "IFCPOINTBYDISTANCEEXPRESSION",
        "a point at a distance along a basis curve, offset in that curve's \
         frame; the neutral model has no distance-along-curve point relation",
    ),
    (
        "IFCAXIS2PLACEMENTLINEAR",
        "a frame located by an IfcPointByDistanceExpression; the neutral model \
         has no distance-along-curve point relation to anchor it",
    ),
];

/// A subtype the supertype's lowering handles exactly.
///
/// Routing a subtype to its supertype's lowerer is exact only when every
/// attribute and rule the subtype adds either leaves the shape unchanged or
/// is checked by that lowerer. `tests/schema_coverage.rs` asserts each row
/// against the IFC4X3 schema: the subtype relation, the added attributes,
/// and that both names are in [`IMPLEMENTED`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Specialisation {
    /// The subtype as a STEP type name.
    pub subtype: &'static str,
    /// The supertype whose lowering it is routed to.
    pub supertype: &'static str,
    /// The explicit attributes the subtype declares, in schema order.
    pub added_attributes: &'static [&'static str],
    /// Why the supertype's lowering is exact for it.
    pub rationale: &'static str,
}

/// Subtypes routed to their supertype's lowering.
pub const SPECIALISATIONS: &[Specialisation] = &[Specialisation {
    subtype: "IFCTRIANGULATEDIRREGULARNETWORK",
    supertype: "IFCTRIANGULATEDFACESET",
    added_attributes: &["Flags"],
    rationale: "the face-set slots are unchanged; the lowerer checks Flags \
                and refuses voids, holes and undocumented codes, so only \
                breakline codes, which leave the triangles unchanged, lower",
}];

/// A variant within a family that is admitted or refused independently.
///
/// [`IMPLEMENTED`] and [`PLANNED`] classify at *family* granularity, which is
/// too coarse for families whose support depends on how the instance is
/// authored. `IFCPCURVE` is implemented, but only for some reference-curve
/// forms; a flat "implemented" claim hides the refusals inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Variant {
    /// The concrete family this variant belongs to; always in [`IMPLEMENTED`].
    pub family: &'static str,
    /// The distinguishing condition, as a caller would recognize it.
    pub variant: &'static str,
    /// Whether this specific variant lowers or is a typed refusal.
    pub support: Support,
    /// Why it is admitted or refused. Refusals name the missing contract.
    pub rationale: &'static str,
}

/// Whether a [`Variant`] lowers exactly or reports a typed refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Support {
    /// Lowers exactly, with no approximation.
    Admitted,
    /// Reports a typed [`crate::GeometryError::Unsupported`] naming the entity.
    Refused,
}

/// Variant-level dispositions for partially supported families.
///
/// Every family named here must appear in [`IMPLEMENTED`] and must declare at
/// least one `Admitted` and one `Refused` variant -- a family with no refusals
/// is not partial and belongs in `IMPLEMENTED` alone. Enforced by
/// `tests/lower_dispatch_corpus.rs`.
pub const PARTIAL: &[Variant] = &[
    Variant {
        family: "IFCPCURVE",
        variant: "reference curve is an IfcPolyline",
        support: Support::Admitted,
        rationale: "an ordered 2D point sequence needs no evaluation",
    },
    Variant {
        family: "IFCPCURVE",
        variant: "reference curve is an IfcIndexedPolyCurve with no explicit \
                  Segments, or only IfcLineIndex segments",
        support: Support::Admitted,
        rationale: "reads identically to a plain ordered point sequence",
    },
    Variant {
        family: "IFCPCURVE",
        variant: "reference curve is an IfcLine, IfcCircle or IfcEllipse \
                  positioned by an IfcAxis2Placement2D",
        support: Support::Admitted,
        rationale: "defining values are read verbatim in the surface's own \
                    (u, v) domain with no unit conversion",
    },
    Variant {
        family: "IFCPCURVE",
        variant: "reference conic positioned by an IfcAxis2Placement3D",
        support: Support::Refused,
        rationale: "a 3D placement's axis has no meaning in a 2D parameter \
                    domain; admitting it would require inventing a projection",
    },
    Variant {
        family: "IFCPCURVE",
        variant: "reference curve is an IfcIndexedPolyCurve with an explicit \
                  IfcArcIndex segment",
        support: Support::Admitted,
        rationale: "a three-point arc composes exactly from a parameter-space \
                    circumcentre into Circle2 plus a Cartesian trim, mirroring \
                    the 3D path with no approximation",
    },
    Variant {
        family: "IFCPCURVE",
        variant: "reference curve is an explicit-knot IfcBSplineCurveWithKnots \
                  or IfcRationalBSplineCurveWithKnots",
        support: Support::Admitted,
        rationale: "every field is dimensionless or a curve parameter; knots \
                    already pass through the 3D path unscaled, and control \
                    points are read as raw (u, v) pairs",
    },
    Variant {
        family: "IFCPCURVE",
        variant: "reference curve is a trimmed or composite curve",
        support: Support::Admitted,
        rationale: "trim parameters and segments stay in the surface (u, v) \
                    domain, unscaled, so no dimensional contract is needed",
    },
    Variant {
        family: "IFCPCURVE",
        variant: "reference curve is a convention-only IfcBSplineCurve",
        support: Support::Refused,
        rationale: "a base spline carries no authored knot vector to preserve",
    },
    Variant {
        family: "IFCDIRECTRIXDERIVEDREFERENCESWEPTAREASOLID",
        variant: "directrix defines only a tangent (no IfcCurveSegment, \
                  segment-built or surface curve reachable)",
        support: Support::Admitted,
        rationale: "IFC4.3 gives it exactly the behaviour of \
                    IfcFixedReferenceSweptAreaSolid in this case, so it lowers \
                    to the same FixedReferenceSweep",
    },
    Variant {
        family: "IFCDIRECTRIXDERIVEDREFERENCESWEPTAREASOLID",
        variant: "directrix defines a tangent plane",
        support: Support::Refused,
        rationale: "kernel: the directrix defines a tangent plane (it is built from \
                    IfcCurveSegment placements or lies on a surface), so the derived \
                    reference adds that plane's rotation to FixedReference; the neutral \
                    FixedReferenceSweep carries only a constant reference direction",
    },
    Variant {
        family: "IFCTRIANGULATEDIRREGULARNETWORK",
        variant: "every Flags value is a breakline code, 0 to 7",
        support: Support::Admitted,
        rationale: "a breakline marks an edge the triangulation already has, \
                    so the triangle surface is the supertype's; the flags \
                    are not carried into the mesh",
    },
    Variant {
        family: "IFCTRIANGULATEDIRREGULARNETWORK",
        variant: "a Flags value is -1 (hole) or -2 (void)",
        support: Support::Refused,
        rationale: "the triangle is excluded from the surface, and a hole may \
                    fall back on another surface; the neutral mesh has no \
                    face-exclusion or fall-back channel",
    },
    Variant {
        family: "IFCTRIANGULATEDIRREGULARNETWORK",
        variant: "a Flags value is outside -2 to 7",
        support: Support::Refused,
        rationale: "the documentation defines no meaning for it",
    },
    Variant {
        family: "IFCSURFACECURVE",
        variant: "MasterRepresentation is Curve3D, PCurveS1, or PCurveS2 with \
                  the named side present",
        support: Support::Admitted,
        rationale: "each side pairs a surface with its own p-curve, so the \
                    neutral MasterRepresentation names S1 and S2 exactly",
    },
    Variant {
        family: "IFCCURVESEGMENT",
        variant: "ParentCurve is an IfcLine, IfcCircle or 2D IfcPolyline, \
                  measured by IfcLengthMeasure",
        support: Support::Admitted,
        rationale: "a line, arc or polyline cut by arc length and placed rigidly \
                    is elementary: a polyline or an angle-trimmed circle",
    },
    Variant {
        family: "IFCCURVESEGMENT",
        variant: "ParentCurve is an IfcSpiral subtype, measured by \
                  IfcLengthMeasure",
        support: Support::Admitted,
        rationale: "the spiral's curvature law, rebased to the segment in closed \
                    form, on a planar intrinsic curve; nothing is integrated",
    },
    Variant {
        family: "IFCCURVESEGMENT",
        variant: "SegmentLength is zero (the closing segment of a layout)",
        support: Support::Admitted,
        rationale: "its placement exactly: a planar intrinsic curve of length zero",
    },
    Variant {
        family: "IFCCURVESEGMENT",
        variant: "ParentCurve is a 2D IfcPolynomialCurve with a degree-one \
                  coordinate, cut forwards from SegmentStart 0",
        support: Support::Admitted,
        rationale: "its Bezier over a closed-form parameter bound, placed \
                    rigidly and trimmed at TrimSelector::ArcLength: the kernel \
                    inverts the arc length, nothing is integrated here",
    },
    Variant {
        family: "IFCCURVESEGMENT",
        variant: "ParentCurve is an IfcPolynomialCurve cut from a non-zero \
                  SegmentStart, walked backwards, 3D, or with no degree-one \
                  coordinate",
        support: Support::Refused,
        rationale: "the placed start point inverts a non-elementary arc-length \
                    integral, a 3D polynomial has no plane for the placement, \
                    and without a degree-one coordinate the trim has no \
                    closed-form parameter bound",
    },
    Variant {
        family: "IFCCURVESEGMENT",
        variant: "SegmentStart or SegmentLength is an IfcParameterValue",
        support: Support::Refused,
        rationale: "SegmentStart/SegmentLength given as IfcParameterValue: IFC4.3 ADD2 defines no parametric \
         space for IfcCurveSegment parents yet (informal proposition 1 requires IfcLengthMeasure)",
    },
    Variant {
        family: "IFCCURVESEGMENT",
        variant: "Placement is an IfcAxis2PlacementLinear",
        support: Support::Refused,
        rationale: "an IfcAxis2PlacementLinear placement stands at a distance along a basis curve; the neutral \
         model has no distance-along-curve point relation to anchor it (#307)",
    },
    Variant {
        family: "IFCGRADIENTCURVE",
        variant: "horizontal IfcCurveSegments over lines, arcs, spirals and \
                  2D IfcPolynomialCurves; vertical IfcCurveSegments over \
                  IfcLine, IfcCircle, an IfcSpiral subtype, or a degree-2 \
                  IfcPolynomialCurve that keeps its start tangent",
        support: Support::Admitted,
        rationale: "one plan parameterised by arc length (an intrinsic curve, \
                    or an arc-length chain with a polynomial piece) plus a \
                    piecewise elevation law of polynomial, circular and \
                    intrinsic pieces: Curve3::Elevated, exact",
    },
    Variant {
        family: "IFCGRADIENTCURVE",
        variant: "a vertical parabola or spiral with no following segment, \
                  closing segment or EndPoint",
        support: Support::Refused,
        rationale: "its plan extent inverts a non-elementary arc-length \
                    integral, and nothing states it",
    },
    Variant {
        family: "IFCGRADIENTCURVE",
        variant: "a heading kink, a closed-form position gap, or a profile \
                  that does not span the base curve",
        support: Support::Refused,
        rationale: "one plan curve and one elevation law cannot carry a kink or \
                    a gap, and an elevation law must cover the whole plan",
    },
    Variant {
        family: "IFCGRADIENTCURVE",
        variant: "placed by a frame that tilts, scales or mirrors the vertical",
        support: Support::Refused,
        rationale: "a plan plus a height is carried only by frames that keep \
                    the vertical axis",
    },
    Variant {
        family: "IFCSURFACECURVE",
        variant: "MasterRepresentation is PCurveS2 with only one associated \
                  p-curve",
        support: Support::Refused,
        rationale: "the master names a parametric side the curve does not \
                    have; the schema calls this inconsistent, so it is \
                    refused rather than resolved to the remaining p-curve",
    },
];

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
    // IFC4X3 sectioned surface, routed before the inheritance test so the
    // named refusal holds whether or not the subtype table knows the type.
    if type_name == "IFCSECTIONEDSURFACE" {
        return lower_sectioned_surface_node(session, id);
    }
    // An exact specialisation lowers through its supertype's arm. The
    // refusal below still names the entity's own type.
    let routed = SPECIALISATIONS
        .iter()
        .find(|row| row.subtype == type_name)
        .map_or(type_name.as_str(), |row| row.supertype);
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
    match routed {
        "IFCEXTRUDEDAREASOLID" => lower_extruded_area_solid_node(session, id, frame),
        "IFCREVOLVEDAREASOLID" => lower_revolved_area_solid_node(session, id, frame),
        "IFCBOOLEANRESULT" | "IFCBOOLEANCLIPPINGRESULT" => {
            lower_boolean_result_node(session, id, frame)
        }
        "IFCHALFSPACESOLID" | "IFCBOXEDHALFSPACE" | "IFCPOLYGONALBOUNDEDHALFSPACE" => {
            lower_half_space_node(session, id, frame)
        }
        "IFCMAPPEDITEM" => lower_mapped_item_node(session, id, frame),
        // `IfcCurveSegment` is an IfcSegment, not an IfcCurve, so the
        // inheritance test above does not route it; the curve module lowers
        // it. The IFC4X3 curves themselves route by inheritance (#293).
        "IFCCURVESEGMENT" => lower_curve_node(session, id, frame),
        "IFCPOINTONCURVE" => lower_point_on_curve_node(session, id, frame),
        "IFCPOINTONSURFACE" => lower_point_on_surface_node(session, id, frame),
        "IFCFACETEDBREP"
        | "IFCFACETEDBREPWITHVOIDS"
        | "IFCADVANCEDBREP"
        | "IFCADVANCEDBREPWITHVOIDS" => lower_faceted_brep_node(session, id, frame),
        "IFCFACESURFACE" | "IFCADVANCEDFACE" => lower_face_surface_node(session, id, frame),
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
        "IFCDIRECTRIXDERIVEDREFERENCESWEPTAREASOLID" => {
            lower_directrix_derived_reference_sweep_node(session, id, frame)
        }
        "IFCSECTIONEDSOLIDHORIZONTAL" => lower_sectioned_solid_horizontal_node(session, id),
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
        _ => Err(session.unsupported(id, &type_name, detail_for(&type_name))),
    }
}

/// The documented reason a recognized family is not lowered yet.
fn detail_for(type_name: &str) -> &'static str {
    planned_detail(type_name).unwrap_or("representation item family is not lowered yet")
}

/// The documented [`PLANNED`] reason for a family, if it has one.
///
/// Shared with the curve lowerer: once the subtype table routes
/// an IFC4X3 curve there by inheritance (#293), the ledger's reason must
/// still be the one reported.
pub(crate) fn planned_detail(type_name: &str) -> Option<&'static str> {
    PLANNED
        .iter()
        .find(|(name, _)| *name == type_name)
        .map(|(_, detail)| *detail)
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
