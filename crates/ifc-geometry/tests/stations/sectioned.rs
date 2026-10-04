//! `IfcSectionedSolidHorizontal` and `IfcSectionedSurface` (IFC4.3 ADD2
//! 8.8.3.35, 8.8.3.37) as `SectionsAtStations` and `OpenSectionsAtStations`.

use axiolid_model::{
    GeometryNode, SectionAtStation, SolidOperation, StationFrame, SurfaceRelation,
};

use super::common::{close_vec, families, lower, only, refused, root, step};

/// A 3D polyline directrix with a corner at 10 m, a straight one, two
/// rectangles and a circle.
const BASE: &str = "#10=IFCPOLYLINE((#11,#12,#13));
#11=IFCCARTESIANPOINT((0.,0.,0.));
#12=IFCCARTESIANPOINT((10.,0.,0.));
#13=IFCCARTESIANPOINT((10.,10.,0.));
#14=IFCPOLYLINE((#11,#15));
#15=IFCCARTESIANPOINT((20.,0.,0.));
#40=IFCRECTANGLEPROFILEDEF(.AREA.,'a',#43,2.,1.);
#41=IFCRECTANGLEPROFILEDEF(.AREA.,'b',#43,4.,1.);
#42=IFCCIRCLEPROFILEDEF(.AREA.,'c',#43,1.);
#43=IFCAXIS2PLACEMENT2D(#44,$);
#44=IFCCARTESIANPOINT((0.,0.));
#45=IFCDIRECTION((0.,0.,1.));
#46=IFCDIRECTION((0.9,0.1,0.));";

/// A position `id` (its expression `id + 1`) at `along` on `basis`, with
/// raw offset fields and axis references.
fn position(id: u64, basis: u64, along: f64, offsets: &str, axes: &str) -> String {
    format!(
        "#{id}=IFCAXIS2PLACEMENTLINEAR(#{},{axes});\n\
         #{}=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE({along:?}),{offsets},#{basis});",
        id + 1,
        id + 1
    )
}

fn plain(id: u64, basis: u64, along: f64) -> String {
    position(id, basis, along, "$,$,$", "$,$")
}

/// An open crowned section, 3.5 m each side, with `tags`.
fn open(id: u64, tags: &str) -> String {
    format!("#{id}=IFCOPENCROSSPROFILEDEF(.CURVE.,$,.T.,(3.5,3.5),(-0.02,0.02),{tags},$);")
}

fn solid_sections(lowered: &ifc_geometry::lower::LoweredGeometry) -> Vec<SectionAtStation> {
    let GeometryNode::SolidOperation(SolidOperation::SectionsAtStations {
        sections, frame, ..
    }) = root(lowered)
    else {
        panic!("SectionsAtStations: {:?}", root(lowered));
    };
    assert_eq!(*frame, StationFrame::Section);
    sections.clone()
}

fn surface_sections(lowered: &ifc_geometry::lower::LoweredGeometry) -> Vec<SectionAtStation> {
    let GeometryNode::SurfaceRelation(SurfaceRelation::OpenSectionsAtStations {
        sections,
        frame,
        ..
    }) = root(lowered)
    else {
        panic!("OpenSectionsAtStations: {:?}", root(lowered));
    };
    assert_eq!(*frame, StationFrame::Section);
    sections.clone()
}

/// The committed solid: two rectangles at 0 and 40 m along the gradient
/// curve, untagged, in the curve frame.
#[test]
fn the_fixture_sectioned_solid_lowers_to_its_stations() {
    let model = families();
    let lowered = lower(&model, only(&model, "IFCSECTIONEDSOLIDHORIZONTAL")).expect("lowers");
    let sections = solid_sections(&lowered);
    let along: Vec<f64> = sections.iter().map(|s| s.station.distance).collect();
    assert_eq!(along, [0.0, 40.0]);
    assert!(sections
        .iter()
        .all(|s| s.tags.is_empty() && s.orientation.is_base()));
}

/// The committed surface: its `IfcOpenCrossProfileDef.Tags` become the
/// section tags.
#[test]
fn the_fixture_sectioned_surface_carries_its_tags() {
    let model = families();
    let lowered = lower(&model, only(&model, "IFCSECTIONEDSURFACE")).expect("lowers");
    for section in surface_sections(&lowered) {
        assert_eq!(section.tags, ["left", "axis", "right"]);
    }
}

/// Offsets other than longitudinal, Axis and RefDirection pass through:
/// RefDirection is the section normal and profile X = Axis x RefDirection
/// (#344; the conversion itself is checked through the evaluator in
/// `points`).
#[test]
fn a_solid_section_keeps_its_offsets_and_axes() {
    let records = format!(
        "{BASE}\n{}\n{}\n#30=IFCSECTIONEDSOLIDHORIZONTAL(#14,(#40,#41),(#20,#22));",
        position(20, 14, 0.0, "1.,0.5,$", "#45,#46"),
        plain(22, 14, 10.0),
    );
    let model = step(&records, false);
    let sections = solid_sections(&lower(&model, 30).expect("lowers"));
    let first = &sections[0];
    assert_eq!(first.station.offsets.lateral, 1.0);
    assert_eq!(first.station.offsets.vertical, 0.5);
    close_vec(
        first.orientation.axis.expect("axis"),
        [0.0, 0.0, 1.0],
        "axis",
    );
    close_vec(
        first.orientation.ref_direction.expect("normal"),
        [0.9, 0.1, 0.0],
        "normal",
    );
    assert!(sections[1].orientation.is_base());
}

/// A corner outside the run does not matter; one inside it does.
#[test]
fn only_a_corner_within_the_run_is_refused() {
    let records = format!(
        "{BASE}\n{}\n{}\n{}\n\
         #30=IFCSECTIONEDSOLIDHORIZONTAL(#10,(#40,#41),(#20,#22));\n\
         #31=IFCSECTIONEDSOLIDHORIZONTAL(#10,(#40,#41),(#20,#24));",
        plain(20, 10, 2.0),
        plain(22, 10, 8.0),
        plain(24, 10, 12.0),
    );
    let model = step(&records, false);
    lower(&model, 30).expect("clear of the corner");
    refused(lower(&model, 31), true, "tangent discontinuity");
}

#[test]
fn solids_ifc_and_axiolid_read_differently_are_refused_by_name() {
    let records = format!(
        "{BASE}\n{}\n{}\n{}\n{}\n\
         #30=IFCSECTIONEDSOLIDHORIZONTAL(#14,(#40,#41),(#20,#24));\n\
         #31=IFCSECTIONEDSOLIDHORIZONTAL(#14,(#40,#41),(#22,#26));\n\
         #32=IFCSECTIONEDSOLIDHORIZONTAL(#14,(#40,#41),(#24,#22));\n\
         #33=IFCSECTIONEDSOLIDHORIZONTAL(#14,(#40,#41,#41),(#22,#24));\n\
         #34=IFCSECTIONEDSOLIDHORIZONTAL(#14,(#40,#42),(#22,#24));",
        position(20, 14, 0.0, "$,$,0.", "$,$"),
        plain(22, 14, 0.0),
        plain(24, 14, 10.0),
        plain(26, 10, 5.0),
    );
    let model = step(&records, false);
    // A WHERE rule: EXISTS, so even a zero longitudinal offset.
    refused(lower(&model, 30), false, "NoLongitudinalOffsets");
    refused(lower(&model, 31), true, "another BasisCurve");
    refused(lower(&model, 32), false, "strictly increasing");
    refused(lower(&model, 33), false, "CorrespondingSectionPositions");
    refused(lower(&model, 34), false, "SectionsSameType");
}

/// Tags are matched as sets: a section may run them in the first section's
/// order or reversed; Axis and RefDirection pass through on a surface too.
#[test]
fn a_surface_joins_by_tag_forwards_or_reversed() {
    let records = format!(
        "{BASE}\n{}\n{}\n{}\n{}\n{}\n\
         #30=IFCSECTIONEDSURFACE(#14,(#20,#22),(#50,#51));",
        position(20, 14, 0.0, "$,$,$", "#45,#46"),
        plain(22, 14, 10.0),
        open(50, "('l','c','r')"),
        open(51, "('r','c','l')"),
        "",
    );
    let model = step(&records, false);
    let sections = surface_sections(&lower(&model, 30).expect("reversed tags"));
    assert_eq!(sections[0].tags, ["l", "c", "r"]);
    assert_eq!(sections[1].tags, ["r", "c", "l"]);
    assert!(sections[0].orientation.ref_direction.is_some());
}

#[test]
fn surfaces_ifc_and_axiolid_read_differently_are_refused_by_name() {
    let records = format!(
        "{BASE}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n\
         #30=IFCSECTIONEDSURFACE(#14,(#20,#22),(#50,#51));\n\
         #31=IFCSECTIONEDSURFACE(#14,(#20,#22),(#50,#52));\n\
         #32=IFCSECTIONEDSURFACE(#14,(#20,#22),(#50,#53));\n\
         #33=IFCSECTIONEDSURFACE(#14,(#24,#22),(#50,#50));\n\
         #34=IFCSECTIONEDSURFACE(#10,(#26,#28),(#50,#50));",
        plain(20, 14, 0.0),
        plain(22, 14, 10.0),
        position(24, 14, 0.0, "0.,$,$", "$,$"),
        plain(26, 10, 5.0),
        plain(28, 10, 15.0),
        open(50, "('l','c','r')"),
        open(51, "$"),
        open(52, "('l','c','x')") + "\n" + &open(53, "('c','l','r')"),
    );
    let model = step(&records, false);
    refused(lower(&model, 30), true, "others do not");
    refused(lower(&model, 31), true, "branching");
    refused(lower(&model, 32), true, "orders its tags");
    refused(lower(&model, 33), false, "NoOffsets");
    refused(lower(&model, 34), true, "tangent discontinuity");
}
