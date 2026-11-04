//! Swept disks described in their own item frame (#661).
//!
//! Radii are converted to metres; `StartParam`/`EndParam` are reported as
//! authored. Every refusal names the disk (or, for a dangling directrix, the
//! disk as referrer).

use ifc_geometry::{body_description, units, BodyDescription, BodyKind, GeometryError, SweptDisk};
use ifc_model::{EntityId, Model};

use super::step::{degrees, metres, model, PRODUCT};

fn try_describe(model: &Model) -> Result<BodyDescription, GeometryError> {
    body_description(
        model,
        &units::resolve(model).expect("project units"),
        PRODUCT,
    )
    .map(|body| body.expect("has a body"))
}

fn disk(model: &Model) -> SweptDisk {
    let body = try_describe(model).expect("describes");
    let item = body.sole_item().expect("one item");
    assert_eq!(item.kind, BodyKind::SweptDisk);
    assert!(item.swept.is_none() && item.local_sweep.is_none());
    item.swept_disk.clone().expect("a swept disk")
}

/// Whether the description is refused as `Degenerate` naming the disk, #104.
fn degenerate_disk(model: &Model) -> bool {
    matches!(
        try_describe(model).expect_err("refused"),
        GeometryError::Degenerate {
            entity: EntityId(104),
            ..
        }
    )
}

const LINE: &str = "#101=IFCCARTESIANPOINT((0.,0.,0.));
     #102=IFCCARTESIANPOINT((1.,0.,0.));
     #103=IFCPOLYLINE((#101,#102));";

/// A stirrup-like rod in millimetres: a 3D indexed curve of two lines and an
/// arc, radius 6 mm, no bore, fillet or range.
///
/// ```text
/// #205 line (1,2), arc (2,3,4), line (4,5);  #206 Radius 6.
/// ```
#[test]
fn a_rod_along_an_indexed_polycurve_reports_its_radius_in_metres() {
    let model = model(
        "IFC4",
        ".MILLI.",
        "1.,0.,0.",
        "#204=IFCCARTESIANPOINTLIST3D(((0.,0.,0.),(100.,0.,0.),(150.,50.,0.),(100.,100.,0.),(0.,100.,0.)));
         #205=IFCINDEXEDPOLYCURVE(#204,(IFCLINEINDEX((1,2)),IFCARCINDEX((2,3,4)),IFCLINEINDEX((4,5))),.F.);
         #206=IFCSWEPTDISKSOLID(#205,6.,$,$,$);",
        "#206",
    );
    let disk = disk(&model);
    assert!((disk.radius - 0.006).abs() < 1e-12);
    assert_eq!(disk.inner_radius, None);
    assert_eq!(disk.fillet_radius, None);
    assert_eq!(disk.directrix, EntityId(205));
    assert_eq!(disk.parameter_range, None);
}

/// A pipe along a circle in a degree model: the range is reported as
/// authored (degrees), the radii in metres.
#[test]
fn a_pipe_keeps_its_authored_parameter_range() {
    let model = degrees(
        "#101=IFCCARTESIANPOINT((0.,0.,0.));
         #102=IFCAXIS2PLACEMENT3D(#101,$,$);
         #103=IFCCIRCLE(#102,1.);
         #104=IFCSWEPTDISKSOLID(#103,0.05,0.04,0.,90.);",
        "#104",
    );
    let disk = disk(&model);
    assert_eq!(disk.radius, 0.05);
    assert_eq!(disk.inner_radius, Some(0.04));
    assert_eq!(disk.parameter_range, Some((0., 90.)));
}

/// `IfcSweptDiskSolidPolygonal` adds the fillet radius, in metres.
#[test]
fn a_polygonal_disk_reports_its_fillet_radius() {
    let model = metres(
        &format!("{LINE} #104=IFCSWEPTDISKSOLIDPOLYGONAL(#103,0.02,$,$,$,0.1);"),
        "#104",
    );
    let disk = disk(&model);
    assert_eq!(disk.radius, 0.02);
    assert_eq!(disk.fillet_radius, Some(0.1));
}

/// In a millimetre model radii and the fillet are reported in metres.
#[test]
fn millimetre_radii_and_fillet_are_reported_in_metres() {
    let plain = model(
        "IFC4",
        ".MILLI.",
        "1.,0.,0.",
        &format!("{LINE} #104=IFCSWEPTDISKSOLID(#103,50.,40.,$,$);"),
        "#104",
    );
    let plain = disk(&plain);
    assert!((plain.radius - 0.05).abs() < 1e-12);
    assert!((plain.inner_radius.expect("bore") - 0.04).abs() < 1e-12);
    let polygonal = model(
        "IFC4",
        ".MILLI.",
        "1.,0.,0.",
        &format!("{LINE} #104=IFCSWEPTDISKSOLIDPOLYGONAL(#103,20.,$,$,$,100.);"),
        "#104",
    );
    let polygonal = disk(&polygonal);
    assert!((polygonal.radius - 0.02).abs() < 1e-12);
    assert!((polygonal.fillet_radius.expect("fillet") - 0.1).abs() < 1e-12);
}

/// A scale of 2 from a mapping neither refuses the disk nor changes the
/// authored radius: `item_world` carries the scale.
#[test]
fn a_scaled_mapping_keeps_the_authored_radius() {
    let model = metres(
        &format!(
            "{LINE}
             #104=IFCSWEPTDISKSOLID(#103,0.05,$,$,$);
             #105=IFCAXIS2PLACEMENT3D(#101,$,$);
             #200=IFCSHAPEREPRESENTATION(#10,'Body','SweptSolid',(#104));
             #201=IFCREPRESENTATIONMAP(#105,#200);
             #202=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#101,2.,$);
             #203=IFCMAPPEDITEM(#201,#202);"
        ),
        "#203",
    );
    let body = try_describe(&model).expect("describes");
    let item = body.sole_item().expect("one item");
    assert_eq!(item.swept_disk.as_ref().expect("disk").radius, 0.05);
    assert_eq!(item.item_world.determinant(), 8.0);
}

#[test]
fn a_non_positive_radius_is_refused() {
    let model = metres(
        &format!("{LINE} #104=IFCSWEPTDISKSOLID(#103,0.,$,$,$);"),
        "#104",
    );
    assert!(degenerate_disk(&model));
}

#[test]
fn a_bore_at_least_the_radius_is_refused() {
    let model = metres(
        &format!("{LINE} #104=IFCSWEPTDISKSOLID(#103,0.05,0.05,$,$);"),
        "#104",
    );
    assert!(degenerate_disk(&model));
}

#[test]
fn a_half_open_range_is_refused() {
    for ends in ["0.,$", "$,1."] {
        let model = metres(
            &format!("{LINE} #104=IFCSWEPTDISKSOLID(#103,0.05,$,{ends});"),
            "#104",
        );
        assert!(degenerate_disk(&model), "{ends}");
    }
}

#[test]
fn a_non_positive_fillet_is_refused() {
    for fillet in ["0.", "-0.1"] {
        let model = metres(
            &format!("{LINE} #104=IFCSWEPTDISKSOLIDPOLYGONAL(#103,0.02,$,$,$,{fillet});"),
            "#104",
        );
        assert!(degenerate_disk(&model), "{fillet}");
    }
}

#[test]
fn a_dangling_directrix_is_refused() {
    let model = metres("#104=IFCSWEPTDISKSOLID(#999,0.05,$,$,$);", "#104");
    assert_eq!(
        try_describe(&model).unwrap_err(),
        GeometryError::MissingEntity {
            referrer: EntityId(104),
            missing: EntityId(999),
        }
    );
}
