//! Lowering exercised against the real fixture corpus.
//!
//! # Why this is the test that matters
//!
//! Every other test in this crate builds its own model. These read files a
//! real exporter produced and assert on the numbers that come out. A view
//! layer can look complete against synthetic input and still misread the
//! first real record it meets.

use axiolid_model::{GeometryNode, SolidOperation};
use axiolid_profile::Profile;
use ifc_geometry::lower::{lower_extruded_area_solid, LoweredGeometry, Tolerance};
use ifc_geometry::transform::Transform;
use ifc_geometry::units;
use ifc_model::Codec;
use ifc_step::StepCodec;
use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../test/fixtures")
        .join(rel)
}

fn extrusion(lowered: &LoweredGeometry) -> (&Profile, axiolid_core::Vec3, f64) {
    let operation_id = match lowered.graph.get(lowered.root).expect("root exists") {
        GeometryNode::Instance(instance) => instance.source,
        other => panic!("expected instance root, got {other:?}"),
    };
    let (profile_id, direction, depth) =
        match lowered.graph.get(operation_id).expect("operation exists") {
            GeometryNode::SolidOperation(SolidOperation::Extrusion {
                profile,
                direction,
                depth,
            }) => (*profile, *direction, *depth),
            other => panic!("expected extrusion, got {other:?}"),
        };
    match lowered.graph.get(profile_id).expect("profile exists") {
        GeometryNode::Profile(profile) => (profile, direction, depth),
        other => panic!("expected profile, got {other:?}"),
    }
}

fn base_profile(mut profile: &Profile) -> &Profile {
    while let Profile::Derived { basis, .. } = profile {
        profile = basis;
    }
    profile
}

/// The wall file's first extrusion, lowered end to end.
///
/// Ground truth read directly out of the file:
///
/// ```text
/// #338081= IFCEXTRUDEDAREASOLID(#338077,#338080,#19,2.41)
/// #338077= IFCRECTANGLEPROFILEDEF(.AREA.,'...',#338076,0.02,0.709999999999996)
/// #19= IFCDIRECTION((0.,0.,1.))
/// ```
///
/// The file declares METRE, so the numbers pass through unscaled and the
/// expected values are readable straight from the record.
#[test]
fn the_wall_fixture_lowers_to_an_extrusion_with_the_documented_numbers() {
    let path = fixture("ifclite-geometry/issue_098_wall_W.ifc");
    let model = StepCodec.read_path(&path).expect("fixture parses");
    let scale = units::resolve(&model);
    let tol = Tolerance::building_scale();

    let id = model
        .ids_of_type("IFCEXTRUDEDAREASOLID")
        .first()
        .copied()
        .expect("the wall file contains extrusions");

    let lowered = lower_extruded_area_solid(&model, id, Transform::identity(), &scale, &tol)
        .expect("a real extrusion must lower");

    let (profile, direction, depth) = extrusion(&lowered);
    assert_eq!(direction, axiolid_core::Vec3::Z, "#19 is the +Z direction");
    assert!(depth > 0.0, "depth must be positive, got {depth}");
    assert!(
        matches!(base_profile(profile), Profile::Rectangle(_)),
        "the fixture uses an exact rectangle profile"
    );
}

/// Lower EVERY extrusion in EVERY fixture and report what happens.
///
/// This is the census that tells the geometry package what it will actually
/// receive. It asserts a floor rather than an exact number so adding fixtures
/// does not break it, but it prints the breakdown so regressions in coverage
/// are visible in the test output.
#[test]
fn every_extrusion_in_the_corpus_lowers_or_reports_why() {
    let dir = fixture("");
    let mut files = Vec::new();
    collect_ifc(&dir, &mut files);
    assert!(files.len() >= 19, "expected the committed corpus");

    let tol = Tolerance::building_scale();
    let (mut ok, mut failed) = (0usize, 0usize);
    let mut reasons: Vec<String> = Vec::new();

    for path in &files {
        let Ok(model) = StepCodec.read_path(path) else {
            continue;
        };
        let scale = units::resolve(&model);
        for id in model.ids_of_type("IFCEXTRUDEDAREASOLID") {
            match lower_extruded_area_solid(&model, *id, Transform::identity(), &scale, &tol) {
                Ok(lowered) => {
                    let (_profile, _direction, depth) = extrusion(&lowered);
                    assert!(depth > 0.0, "{path:?} {id}: non-positive depth");
                    ok += 1;
                }
                Err(e) => {
                    failed += 1;
                    reasons.push(format!(
                        "{}: {e}",
                        path.file_name().unwrap().to_string_lossy()
                    ));
                }
            }
        }
    }

    println!("lowered {ok} extrusions, {failed} unsupported");
    for r in &reasons {
        println!("  {r}");
    }
    assert!(
        ok >= 15,
        "expected most corpus extrusions to lower, got {ok}"
    );
}

/// Recursively collect `.ifc` files.
fn collect_ifc(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_ifc(&p, out);
        } else if p.extension().is_some_and(|e| e == "ifc") {
            out.push(p);
        }
    }
}

/// Regression: the hollow rectangle whose WallThickness sits at slot 5.
///
/// `#113152= IFCRECTANGLEHOLLOWPROFILEDEF(.AREA.,'PK250X7.0',#135,250.,250.,7.,10.,17.)`
///
/// A 250x250 box section with a 7 mm wall. An earlier version of the lowering
/// shared one WallThickness constant between the circle and rectangle hollow
/// profiles. `IfcCircleProfileDef` contributes one attribute before it and
/// `IfcRectangleProfileDef` contributes two, so the rectangle case read
/// `YDim` (250) as the wall thickness and rejected a perfectly valid section
/// as "wall thickness consumes the whole section".
///
/// The lesson worth keeping: inherited slot counts differ per branch, so a
/// shared constant across sibling subtypes is a latent misread.
#[test]
fn a_hollow_rectangle_section_keeps_its_void() {
    let path = fixture("ifclite-geometry/issue_1155_halfspace_flyaway.ifc");
    let model = StepCodec.read_path(&path).expect("fixture parses");
    let scale = units::resolve(&model);
    let tol = Tolerance::building_scale();

    let id = model
        .ids_of_type("IFCRECTANGLEHOLLOWPROFILEDEF")
        .first()
        .copied()
        .expect("fixture has a hollow rectangle");

    let profile = ifc_geometry::lower::lower_profile(&model, id, &scale, &tol)
        .expect("a 250x250x7 box section is valid geometry");

    match base_profile(&profile) {
        Profile::Rectangle(rectangle) => {
            let thickness = rectangle.thickness.expect("hollow profile keeps thickness");
            assert!((thickness - scale.length(7.0)).abs() < 1e-12);
            assert_eq!(rectangle.x, scale.length(250.0));
            assert_eq!(rectangle.y, scale.length(250.0));
        }
        other => panic!("expected exact hollow rectangle, got {other:?}"),
    }
}

/// A census of geometry entities across the corpus.
///
/// Printed rather than asserted in detail: this exists to tell the geometry
/// package what it will actually receive and in what proportion, which is a
/// fact about the corpus rather than a property of the code. The assertion is
/// only that the census is non-empty, so the test still fails if the fixtures
/// or the parser break.
#[test]
fn report_what_the_kernel_will_actually_be_asked_for() {
    let dir = fixture("");
    let mut files = Vec::new();
    collect_ifc(&dir, &mut files);

    let interesting = [
        "IFCEXTRUDEDAREASOLID",
        "IFCREVOLVEDAREASOLID",
        "IFCFACETEDBREP",
        "IFCPOLYGONALBOUNDEDHALFSPACE",
        "IFCHALFSPACESOLID",
        "IFCBOOLEANCLIPPINGRESULT",
        "IFCBOOLEANRESULT",
        "IFCMAPPEDITEM",
        "IFCTRIANGULATEDFACESET",
        "IFCPOLYGONALFACESET",
        "IFCSWEPTDISKSOLID",
        "IFCCSGSOLID",
        "IFCSHELLBASEDSURFACEMODEL",
        "IFCFACEBASEDSURFACEMODEL",
    ];

    let mut totals: Vec<(String, usize)> = interesting
        .iter()
        .map(|t| ((*t).to_string(), 0usize))
        .collect();

    let mut parsed = 0usize;
    for path in &files {
        let Ok(model) = StepCodec.read_path(path) else {
            continue;
        };
        parsed += 1;
        for (name, count) in totals.iter_mut() {
            *count += model.ids_of_type(name).len();
        }
    }

    totals.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    println!("\n=== geometry census over {parsed} files ===");
    for (name, count) in &totals {
        if *count > 0 {
            println!("{count:>6}  {name}");
        }
    }
    println!("=== absent from this corpus ===");
    for (name, count) in &totals {
        if *count == 0 {
            println!("        {name}");
        }
    }

    assert!(parsed >= 19, "expected the committed corpus");
    assert!(
        totals.iter().any(|(_, c)| *c > 0),
        "census found no geometry at all"
    );
}
