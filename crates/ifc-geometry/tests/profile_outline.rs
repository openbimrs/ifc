//! Outline vertices of arbitrary closed profiles, without a kernel (#166).
//!
//! A mitred wall end is the motivating case: its section is a free polyline,
//! and a check of the openings in it needs those vertices exactly, in the
//! profile's coordinates and in metres. Curved boundaries are refused by name
//! rather than chorded.

use ifc_geometry::{profile_outline, units, GeometryError, ProfileOutline};
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

/// Lengths in millimetres, so every vertex must be converted exactly once.
///
/// - `#30`: a 200 mm wall mitred at 45 degrees, closed by repeating `#10`.
/// - `#31`: the same outline closed by a distinct point at `#10`'s place.
/// - `#32`: a 1000 x 600 plate with a 200 x 100 hole: an indexed outer curve
///   without `Segments`, closed by repeating its first point, a polyline
///   hole.
/// - `#33`: an indexed outer curve of two consecutive `IfcLineIndex` runs
///   ending on its first index.
/// - `#34`..`#39`: refusals (arc segment, non-consecutive runs, a circle, a
///   rectangle profile, a 3D point, two distinct vertices).
/// - `#45`, `#46`: open indexed curves, without and with `Segments` (#335).
const FILE: &str = "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#2=IFCUNITASSIGNMENT((#1));
#3=IFCPROJECT('0YvctVUKr0kugbFTf53O9A',$,'P',$,$,$,$,$,#2);
#10=IFCCARTESIANPOINT((0.,0.));
#11=IFCCARTESIANPOINT((4000.,0.));
#12=IFCCARTESIANPOINT((4200.,200.));
#13=IFCCARTESIANPOINT((0.,200.));
#14=IFCCARTESIANPOINT((0.,0.));
#15=IFCCARTESIANPOINT((400.,250.));
#16=IFCCARTESIANPOINT((600.,250.));
#17=IFCCARTESIANPOINT((600.,350.));
#18=IFCCARTESIANPOINT((400.,350.));
#19=IFCCARTESIANPOINT((0.,0.,0.));
#20=IFCPOLYLINE((#10,#11,#12,#13,#10));
#21=IFCPOLYLINE((#10,#11,#12,#13,#14));
#22=IFCPOLYLINE((#15,#16,#17,#18,#15));
#23=IFCCARTESIANPOINTLIST2D(((0.,0.),(1000.,0.),(1000.,600.),(0.,600.)));
#24=IFCINDEXEDPOLYCURVE(#42,$,$);
#42=IFCCARTESIANPOINTLIST2D(((0.,0.),(1000.,0.),(1000.,600.),(0.,600.),(0.,0.)));
#43=IFCINDEXEDPOLYCURVE(#23,$,$);
#44=IFCINDEXEDPOLYCURVE(#23,(IFCLINEINDEX((1,2,3,4))),$);
#45=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#43);
#46=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#44);
#25=IFCINDEXEDPOLYCURVE(#23,(IFCLINEINDEX((1,2,3)),IFCLINEINDEX((3,4,1))),$);
#26=IFCINDEXEDPOLYCURVE(#23,(IFCLINEINDEX((1,2)),IFCARCINDEX((2,3,4)),IFCLINEINDEX((4,1))),$);
#27=IFCINDEXEDPOLYCURVE(#23,(IFCLINEINDEX((1,2)),IFCLINEINDEX((3,4))),$);
#28=IFCAXIS2PLACEMENT2D(#10,$);
#29=IFCCIRCLE(#28,500.);
#40=IFCPOLYLINE((#10,#11,#19,#10));
#41=IFCPOLYLINE((#10,#11,#10));
#30=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,'mitre',#20);
#31=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#21);
#32=IFCARBITRARYPROFILEDEFWITHVOIDS(.AREA.,$,#24,(#22));
#33=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#25);
#34=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#26);
#35=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#27);
#36=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#29);
#37=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,100.,200.);
#38=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#40);
#39=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#41);
ENDSEC;
END-ISO-10303-21;
";

fn outline(id: u64) -> Result<ProfileOutline, GeometryError> {
    let model: Model = StepCodec
        .read_bytes(FILE.as_bytes())
        .expect("fixture parses");
    let scale = units::resolve(&model);
    profile_outline(&model, &scale, EntityId(id))
}

/// `[x, y]` millimetres as the metres the reader must report.
fn mm(points: &[[f64; 2]]) -> Vec<[f64; 2]> {
    points.iter().map(|[x, y]| [x * 0.001, y * 0.001]).collect()
}

fn assert_ring(actual: &[[f64; 2]], expected: &[[f64; 2]]) {
    assert_eq!(actual.len(), expected.len(), "{actual:?}");
    for (a, e) in actual.iter().zip(expected) {
        assert!(
            (a[0] - e[0]).abs() < 1e-12 && (a[1] - e[1]).abs() < 1e-12,
            "{actual:?} vs {expected:?}"
        );
    }
}

fn refused_entity(error: &GeometryError) -> Option<EntityId> {
    match error {
        GeometryError::Unsupported { entity, .. } | GeometryError::Degenerate { entity, .. } => {
            Some(*entity)
        }
        _ => None,
    }
}

#[test]
fn a_mitred_wall_end_reports_its_four_vertices_in_metres() {
    let outline = outline(30).expect("reads");
    assert_eq!(outline.profile, EntityId(30));
    assert_ring(
        &outline.outer,
        &mm(&[[0.0, 0.0], [4000.0, 0.0], [4200.0, 200.0], [0.0, 200.0]]),
    );
    assert!(outline.inner.is_empty());
}

/// A file may close a ring with a second point record at the first
/// vertex's coordinates; the ring is the same.
#[test]
fn a_ring_closed_by_a_distinct_point_is_the_same_ring() {
    assert_eq!(
        outline(31).expect("reads").outer,
        outline(30).unwrap().outer
    );
}

#[test]
fn an_indexed_outer_curve_and_a_polyline_hole_are_both_rings() {
    let outline = outline(32).expect("reads");
    assert_ring(
        &outline.outer,
        &mm(&[[0.0, 0.0], [1000.0, 0.0], [1000.0, 600.0], [0.0, 600.0]]),
    );
    assert_eq!(outline.inner.len(), 1);
    assert_ring(
        &outline.inner[0],
        &mm(&[
            [400.0, 250.0],
            [600.0, 250.0],
            [600.0, 350.0],
            [400.0, 350.0],
        ]),
    );
}

/// Consecutive runs share their joining index once, and a run that ends on
/// the first index closes the ring.
#[test]
fn consecutive_line_index_runs_chain_into_one_ring() {
    assert_eq!(
        outline(33).expect("reads").outer,
        outline(32).unwrap().outer
    );
}

#[test]
fn an_arc_segment_is_refused_not_chorded() {
    let error = outline(34).expect_err("an arc has no vertex outline");
    assert!(
        matches!(error, GeometryError::Unsupported { .. }),
        "{error:?}"
    );
    assert_eq!(refused_entity(&error), Some(EntityId(26)));
}

#[test]
fn runs_that_do_not_join_are_refused() {
    let error = outline(35).expect_err("not one ring");
    assert!(
        matches!(error, GeometryError::Degenerate { .. }),
        "{error:?}"
    );
    assert_eq!(refused_entity(&error), Some(EntityId(27)));
}

#[test]
fn a_curved_boundary_family_is_refused_by_name() {
    let error = outline(36).expect_err("a circle has no vertices");
    assert!(
        matches!(error, GeometryError::Unsupported { .. }),
        "{error:?}"
    );
    assert_eq!(refused_entity(&error), Some(EntityId(29)));
}

#[test]
fn a_parameterised_family_is_refused() {
    let error = outline(37).expect_err("a rectangle is described, not outlined");
    assert!(
        matches!(error, GeometryError::Unsupported { .. }),
        "{error:?}"
    );
    assert_eq!(refused_entity(&error), Some(EntityId(37)));
}

#[test]
fn a_3d_boundary_point_is_refused() {
    let error = outline(38).expect_err("OuterCurve.Dim = 2");
    assert!(
        matches!(error, GeometryError::Degenerate { .. }),
        "{error:?}"
    );
    assert_eq!(refused_entity(&error), Some(EntityId(19)));
}

#[test]
fn fewer_than_three_distinct_vertices_are_refused() {
    let error = outline(39).expect_err("a segment bounds nothing");
    assert!(
        matches!(error, GeometryError::Degenerate { .. }),
        "{error:?}"
    );
    assert_eq!(refused_entity(&error), Some(EntityId(41)));
}

/// An open indexed curve is refused, as the profile lowering refuses it,
/// rather than closed with an edge the file did not author (#335).
#[test]
fn an_open_indexed_curve_is_refused_not_closed() {
    for (profile, curve) in [(45, 43), (46, 44)] {
        let error = outline(profile).expect_err("an open curve bounds no area");
        assert!(
            matches!(&error, GeometryError::Degenerate { detail, .. } if detail.contains("open")),
            "{error:?}"
        );
        assert_eq!(refused_entity(&error), Some(EntityId(curve)));
    }
}
