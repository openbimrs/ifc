//! An opening that the file makes void two hosts voids only the first (#59).
//!
//! `IfcFeatureElementSubtraction.VoidsElements` is a single-valued inverse
//! in IFC2X3 and IFC4: an opening belongs to exactly one element. Here the
//! opening `#30` is claimed by wall `#10` (relation `#40`) and by wall `#20`
//! (relation `#41`, higher id, listed FIRST in the file so file order and id
//! order disagree). Before, it was subtracted from both walls.

use ifc_geometry::{openings_of, voiding_conflicts, VoidingConflict};
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

const WALL_A: EntityId = EntityId(10);
const WALL_B: EntityId = EntityId(20);
const OPENING: EntityId = EntityId(30);

fn wall(id: u64, x: f64) -> String {
    format!(
        "#{id}=IFCWALL('{id}YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,#{p},#{s},$,$);
#{p}=IFCLOCALPLACEMENT($,#{a});
#{a}=IFCAXIS2PLACEMENT3D(#{pt},$,$);
#{pt}=IFCCARTESIANPOINT(({x:?},0.,0.));
#{s}=IFCPRODUCTDEFINITIONSHAPE($,$,(#{r}));
#{r}=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#{b}));
#{b}=IFCEXTRUDEDAREASOLID(#5,#4,#7,3.);
",
        p = id + 1,
        a = id + 2,
        pt = id + 3,
        s = id + 4,
        r = id + 5,
        b = id + 6,
    )
}

fn model(relations: &str) -> Model {
    let data = format!(
        "#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,(#2),#3);
#2=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#4,$);
#3=IFCUNITASSIGNMENT((#8));
#4=IFCAXIS2PLACEMENT3D(#6,$,$);
#5=IFCRECTANGLEPROFILEDEF(.AREA.,$,#9,4.,0.3);
#6=IFCCARTESIANPOINT((0.,0.,0.));
#7=IFCDIRECTION((0.,0.,1.));
#8=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#9=IFCAXIS2PLACEMENT2D(#6,$);
{a}{b}#30=IFCOPENINGELEMENT('3YvctVUKr0kugbFTf53O9L',$,'Opening',$,$,#31,#32,$,.OPENING.);
#31=IFCLOCALPLACEMENT($,#4);
#32=IFCPRODUCTDEFINITIONSHAPE($,$,(#33));
#33=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#34));
#34=IFCEXTRUDEDAREASOLID(#35,#4,#7,2.);
#35=IFCRECTANGLEPROFILEDEF(.AREA.,$,#9,1.,0.5);
{relations}",
        a = wall(10, 0.0),
        b = wall(20, 0.5),
    );
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\n\
         ENDSEC;\nDATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses")
}

fn conflicting() -> Model {
    model(
        "#41=IFCRELVOIDSELEMENT('0YvctVUKr0kugbFTf53O41',$,$,$,#20,#30);
#40=IFCRELVOIDSELEMENT('0YvctVUKr0kugbFTf53O40',$,$,$,#10,#30);",
    )
}

#[test]
fn the_opening_voids_only_its_first_host() {
    let model = conflicting();
    assert_eq!(openings_of(&model, WALL_A), [OPENING]);
    assert!(openings_of(&model, WALL_B).is_empty());
}

#[test]
fn the_rejected_host_is_reported() {
    let conflicts = voiding_conflicts(&conflicting());
    let fields = |c: &VoidingConflict| (c.opening, c.kept_host, c.rejected_host, c.relation);
    assert_eq!(
        conflicts.iter().map(fields).collect::<Vec<_>>(),
        [(OPENING, WALL_A, WALL_B, EntityId(41))]
    );
}

#[test]
fn a_valid_or_redundant_voiding_has_no_conflict() {
    let valid = model("#40=IFCRELVOIDSELEMENT('0YvctVUKr0kugbFTf53O40',$,$,$,#10,#30);");
    assert!(voiding_conflicts(&valid).is_empty());
    assert_eq!(openings_of(&valid, WALL_A), [OPENING]);

    let redundant = model(
        "#40=IFCRELVOIDSELEMENT('0YvctVUKr0kugbFTf53O40',$,$,$,#10,#30);
#41=IFCRELVOIDSELEMENT('0YvctVUKr0kugbFTf53O41',$,$,$,#10,#30);",
    );
    assert!(voiding_conflicts(&redundant).is_empty());
    assert_eq!(openings_of(&redundant, WALL_A), [OPENING]);
}

/// The net body of the rejected host is its gross body: nothing is cut.
#[cfg(feature = "compile-reference-backend")]
#[test]
fn the_rejected_host_keeps_its_gross_body() {
    use axiolid_core::Tolerance;
    use ifc_geometry::compile::compile_product_mesh_net;

    let model = conflicting();
    let a = compile_product_mesh_net(&model, WALL_A, Tolerance::MILLIMETRE)
        .unwrap()
        .unwrap();
    assert_eq!(a.openings, [OPENING]);
    let b = compile_product_mesh_net(&model, WALL_B, Tolerance::MILLIMETRE)
        .unwrap()
        .unwrap();
    assert!(b.openings.is_empty(), "{:?}", b.openings);
}
