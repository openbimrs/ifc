//! `IfcPlanarExtent` / `IfcPlanarBox` presentation-extent projections.
//!
//! The geometry disposition ledger classifies both `non-shape` with
//! `ifc-style` as owner. These tests make that ownership real rather than
//! merely declared.

use ifc_model::{Codec, Model};
use ifc_schema::{ifc2x3, ifc4, ifc4x3};
use ifc_step::StepCodec;
use ifc_style::{StyleError, StyleView};

const SOURCE: &[u8] = br#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('Planar extent contract'),'2;1');
FILE_NAME('extent.ifc','2026-09-14T00:00:00',(''),(''),'openbim','openbim','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCCARTESIANPOINT((0.,0.));
#2=IFCAXIS2PLACEMENT2D(#1,$);
#3=IFCPLANAREXTENT(2.,1.);
#4=IFCPLANARBOX(4.,3.,#2);
ENDSEC;
END-ISO-10303-21;
"#;

#[test]
fn planar_box_reads_its_inherited_extent_and_placement_in_every_schema() {
    for schema in [ifc2x3(), ifc4(), ifc4x3()] {
        let model = StepCodec.read_bytes(SOURCE).unwrap();
        let view = StyleView::new(&model, schema);
        let box_id = model.of_type("IFCPLANARBOX").next().unwrap().0;

        let planar_box = view.planar_box(box_id).unwrap();
        // SizeInX/SizeInY are inherited slots: reading them through the box
        // proves they resolve by name, not by a hardcoded subtype offset.
        assert_eq!(
            planar_box.planar_extent().size_in_x().unwrap(),
            4.0,
            "SizeInX in {}",
            schema.name()
        );
        assert_eq!(planar_box.planar_extent().size_in_y().unwrap(), 3.0);
        assert_eq!(
            planar_box.placement().unwrap(),
            model.of_type("IFCAXIS2PLACEMENT2D").next().unwrap().0
        );
    }
}

#[test]
fn a_bare_planar_extent_is_not_readable_as_a_box() {
    let model = StepCodec.read_bytes(SOURCE).unwrap();
    let view = StyleView::new(&model, ifc4());
    let extent_id = model.of_type("IFCPLANAREXTENT").next().unwrap().0;

    // IfcPlanarExtent is the supertype, so it projects as an extent...
    let extent = view.planar_extent(extent_id).unwrap();
    assert_eq!(extent.size_in_x().unwrap(), 2.0);
    assert_eq!(extent.size_in_y().unwrap(), 1.0);

    // ...but must not be silently readable as its subtype, which would make
    // `placement()` report a missing attribute instead of a wrong type.
    assert!(matches!(
        view.planar_box(extent_id).unwrap_err(),
        StyleError::WrongEntityType {
            expected: "IfcPlanarBox",
            ..
        }
    ));
}

#[test]
fn a_box_placement_pointing_at_a_non_placement_is_refused() {
    let malformed = String::from_utf8(SOURCE.to_vec())
        .unwrap()
        .replace("#4=IFCPLANARBOX(4.,3.,#2);", "#4=IFCPLANARBOX(4.,3.,#3);");
    let model: Model = StepCodec.read_bytes(malformed.as_bytes()).unwrap();
    let view = StyleView::new(&model, ifc4());
    let box_id = model.of_type("IFCPLANARBOX").next().unwrap().0;

    // #3 is an IfcPlanarExtent, not a member of IfcAxis2Placement.
    assert!(matches!(
        view.planar_box(box_id).unwrap().placement().unwrap_err(),
        StyleError::ReferenceType {
            expected: "IfcAxis2Placement",
            ..
        }
    ));
}
