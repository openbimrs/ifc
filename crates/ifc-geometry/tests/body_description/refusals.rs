//! What cannot be described exactly is refused, whole and typed.
//!
//! Every case asserts the error variant AND the entity it names: a refusal
//! that names the wrong entity sends a user to the wrong record.

use ifc_geometry::{body_description, describe_profile, units, BodyKind, GeometryError};
use ifc_model::{EntityId, Model};

use super::step::{metres, PRODUCT};

fn describe(model: &Model) -> Result<Option<ifc_geometry::BodyDescription>, GeometryError> {
    body_description(model, &units::resolve(model), PRODUCT)
}

const SOLID: &str = "#101=IFCCARTESIANPOINT((0.,0.,0.));
     #102=IFCAXIS2PLACEMENT3D(#101,$,$);
     #103=IFCDIRECTION((0.,0.,1.));";

/// A profile family the crate does not interpret is refused, not guessed.
#[test]
fn an_unsupported_profile_is_refused() {
    let model = metres(
        &format!(
            "{SOLID}
             #100=IFCCRANERAILASHAPEPROFILEDEF(.AREA.,$,$,1.,1.,$,1.,1.,1.,1.,1.,1.,1.,1.,$);
             #104=IFCEXTRUDEDAREASOLID(#100,#102,#103,1.);"
        ),
        "#104",
    );
    let error = describe(&model).expect_err("unknown family");
    assert!(error.is_unsupported(), "{error:?}");
    assert_eq!(error.entity(), Some(EntityId(100)));
}

/// A bare `IfcProfileDef` is a label, not a section.
#[test]
fn a_generic_profile_is_refused() {
    let model = metres("#100=IFCPROFILEDEF(.AREA.,'HEA300');", "");
    let error = describe_profile(&model, &units::resolve(&model), EntityId(100))
        .expect_err("no geometry to describe");
    assert!(error.is_unsupported(), "{error:?}");
}

/// A swept solid whose profile reference dangles names the dangling id.
#[test]
fn a_dangling_profile_is_refused() {
    let model = metres(
        &format!("{SOLID} #104=IFCEXTRUDEDAREASOLID(#999,#102,#103,1.);"),
        "#104",
    );
    assert_eq!(
        describe(&model).unwrap_err(),
        GeometryError::MissingEntity {
            referrer: EntityId(104),
            missing: EntityId(999),
        }
    );
}

/// A representation item that does not exist is refused, naming the
/// representation that lists it.
#[test]
fn a_dangling_item_is_refused() {
    let model = metres("", "#998");
    assert_eq!(
        describe(&model).unwrap_err(),
        GeometryError::MissingEntity {
            referrer: EntityId(20),
            missing: EntityId(998),
        }
    );
}

/// An arbitrary profile whose boundary curve dangles is refused.
#[test]
fn a_dangling_profile_curve_is_refused() {
    let model = metres("#100=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#997);", "");
    assert_eq!(
        describe_profile(&model, &units::resolve(&model), EntityId(100)).unwrap_err(),
        GeometryError::MissingEntity {
            referrer: EntityId(100),
            missing: EntityId(997),
        }
    );
}

/// An open profile swept into a "solid" has no section to report.
#[test]
fn an_extruded_open_profile_is_refused() {
    let model = metres(
        &format!(
            "{SOLID}
             #105=IFCCARTESIANPOINT((0.,0.));
             #106=IFCCARTESIANPOINT((1.,0.));
             #107=IFCPOLYLINE((#105,#106));
             #100=IFCARBITRARYOPENPROFILEDEF(.CURVE.,$,#107);
             #104=IFCEXTRUDEDAREASOLID(#100,#102,#103,1.);"
        ),
        "#104",
    );
    let error = describe(&model).expect_err("open profile");
    assert!(error.is_unsupported(), "{error:?}");
    assert_eq!(error.entity(), Some(EntityId(100)));
}

/// A non-positive depth is refused as lowering refuses it.
#[test]
fn a_non_positive_depth_is_refused() {
    let model = metres(
        &format!(
            "{SOLID}
             #100=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,1.,1.);
             #104=IFCEXTRUDEDAREASOLID(#100,#102,#103,0.);"
        ),
        "#104",
    );
    assert!(matches!(
        describe(&model).unwrap_err(),
        GeometryError::Degenerate { entity, .. } if entity == EntityId(104)
    ));
}

/// A non-physical section dimension is refused, naming the profile.
#[test]
fn a_hollow_section_with_no_hole_is_refused() {
    let model = metres(
        "#100=IFCRECTANGLEHOLLOWPROFILEDEF(.AREA.,$,$,0.2,0.1,0.05,$,$);",
        "",
    );
    assert!(matches!(
        describe_profile(&model, &units::resolve(&model), EntityId(100)).unwrap_err(),
        GeometryError::Degenerate { entity, .. } if entity == EntityId(100)
    ));
}

/// An item family that is not a representation item is refused.
#[test]
fn an_unknown_item_family_is_refused() {
    let model = metres("#100=IFCWALL('w',$,$,$,$,$,$,$,$);", "#100");
    let error = describe(&model).expect_err("a wall is not an item");
    assert!(error.is_unsupported(), "{error:?}");
    assert_eq!(error.entity(), Some(EntityId(100)));
}

/// One unreadable item fails the whole body: no partial answers.
#[test]
fn one_bad_item_fails_the_whole_body() {
    let model = metres(
        &format!(
            "{SOLID}
             #100=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,1.,1.);
             #104=IFCEXTRUDEDAREASOLID(#100,#102,#103,1.);"
        ),
        "#104,#996",
    );
    assert!(describe(&model).is_err());
}

/// A derived profile that is its own parent is a cycle, not a stack overflow.
#[test]
fn a_profile_cycle_is_refused() {
    let model = metres(
        "#100=IFCDERIVEDPROFILEDEF(.AREA.,$,#101,#102,$);
         #101=IFCDERIVEDPROFILEDEF(.AREA.,$,#100,#102,$);
         #103=IFCCARTESIANPOINT((0.,0.));
         #102=IFCCARTESIANTRANSFORMATIONOPERATOR2D($,$,#103,$);",
        "",
    );
    assert_eq!(
        describe_profile(&model, &units::resolve(&model), EntityId(100)).unwrap_err(),
        GeometryError::CyclicChain {
            entity: EntityId(100),
            kind: "profile",
        }
    );
}

/// A map that contains itself is a cycle, not a hang.
#[test]
fn a_mapped_item_cycle_is_refused() {
    let model = metres(
        "#300=IFCSHAPEREPRESENTATION(#10,'Body','MappedRepresentation',(#303));
         #305=IFCCARTESIANPOINT((0.,0.,0.));
         #301=IFCAXIS2PLACEMENT3D(#305,$,$);
         #302=IFCREPRESENTATIONMAP(#301,#300);
         #304=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#305,$,$);
         #303=IFCMAPPEDITEM(#302,#304);",
        "#303",
    );
    assert!(matches!(
        describe(&model).unwrap_err(),
        GeometryError::CyclicChain { entity, .. } if entity == EntityId(303)
    ));
}

/// A scaling map would report parameters the file never authored.
///
/// The B-rep beside it is still reported: a kind does not depend on scale.
#[test]
fn a_scaled_mapped_swept_solid_is_refused_but_a_scaled_brep_is_not() {
    let map = |item: &str| {
        format!(
            "{SOLID}
             #100=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,1.,1.);
             #104=IFCEXTRUDEDAREASOLID(#100,#102,#103,1.);
             #110=IFCFACE(());
             #111=IFCCLOSEDSHELL((#110));
             #112=IFCFACETEDBREP(#111);
             #200=IFCSHAPEREPRESENTATION(#10,'Body','SweptSolid',({item}));
             #201=IFCREPRESENTATIONMAP(#102,#200);
             #202=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#101,2.,$);
             #203=IFCMAPPEDITEM(#201,#202);"
        )
    };
    let error = describe(&metres(&map("#104"), "#203")).expect_err("scaled sweep");
    assert!(error.is_unsupported(), "{error:?}");
    assert_eq!(error.entity(), Some(EntityId(203)), "blame the mapping");

    let body = describe(&metres(&map("#112"), "#203")).unwrap().unwrap();
    assert_eq!(body.sole_item().unwrap().kind, BodyKind::Brep);
}
