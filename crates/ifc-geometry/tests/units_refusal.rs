//! A model whose units have no usable scale is refused, never placed in
//! metres (#585).
//!
//! The text is one proxy at the point `(1000, 0, 0)`. Whether that is a
//! kilometre, a metre or a millimetre depends on the project's unit
//! assignment, which the model either declares or does not.

#![cfg(feature = "lowering")]

use ifc_geometry::{product_world_transform, units, GeometryError};
use ifc_model::{Codec, EntityId, Model};

const HEADER: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('ViewDefinition [ReferenceView]'),'2;1');\n\
FILE_NAME('r.ifc','2026-01-01T00:00:00',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n";
const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";
const PLACED: &str = "#2=IFCCARTESIANPOINT((1000.,0.,0.));\n#3=IFCAXIS2PLACEMENT3D(#2,$,$);\n\
#4=IFCLOCALPLACEMENT($,#3);\n\
#5=IFCBUILDINGELEMENTPROXY('1YvctVUKr0kugbFTf53O9L',$,'X',$,$,#4,$,$,$);\n";

/// The #585 repro: `UnitsInContext` is `$`.
fn without_units() -> String {
    format!("{HEADER}#1=IFCPROJECT('3YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,$,$);\n{PLACED}{FOOTER}")
}

/// A project whose `UnitsInContext` is `#7`, which holds `units`.
fn with_assignment(units: &str) -> String {
    format!(
        "{HEADER}#1=IFCPROJECT('3YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,$,#7);\n{PLACED}{units}{FOOTER}"
    )
}

fn parse(text: String) -> Model {
    ifc_step::StepCodec
        .read_bytes(text.as_bytes())
        .expect("text parses")
}

fn origin_in_metres(prefix: &str) -> [f64; 3] {
    let model = parse(with_assignment(&format!(
        "#6=IFCSIUNIT(*,.LENGTHUNIT.,{prefix},.METRE.);\n#7=IFCUNITASSIGNMENT((#6));\n"
    )));
    let scale = units::resolve(&model).expect("units");
    product_world_transform(&model, &scale, EntityId(5))
        .expect("placed")
        .origin
}

#[test]
fn a_project_without_units_in_context_is_refused_not_placed_in_metres() {
    let model = parse(without_units());
    assert_eq!(
        units::resolve(&model),
        Err(GeometryError::LengthUnitUndeclared {
            project: Some(EntityId(1)),
            assignment: None
        })
    );
}

#[test]
fn a_declared_millimetre_places_the_proxy_one_metre_out() {
    assert_eq!(origin_in_metres(".MILLI."), [1.0, 0.0, 0.0]);
}

#[test]
fn a_declared_metre_places_the_proxy_a_thousand_metres_out() {
    assert_eq!(origin_in_metres("$"), [1000.0, 0.0, 0.0]);
}

#[test]
fn an_assignment_with_only_an_angle_unit_is_refused() {
    let model = parse(with_assignment(
        "#6=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);\n#7=IFCUNITASSIGNMENT((#6));\n",
    ));
    assert_eq!(
        units::resolve(&model),
        Err(GeometryError::LengthUnitUndeclared {
            project: Some(EntityId(1)),
            assignment: Some(EntityId(7))
        })
    );
}

#[test]
fn a_conversion_unit_with_a_non_numeric_factor_is_unreadable() {
    let model = parse(with_assignment(
        "#8=IFCMEASUREWITHUNIT(IFCLABEL('x'),#9);\n#9=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n\
         #6=IFCCONVERSIONBASEDUNIT(#10,.LENGTHUNIT.,'INCH',#8);\n#7=IFCUNITASSIGNMENT((#6));\n",
    ));
    assert!(matches!(
        units::resolve(&model),
        Err(GeometryError::UnitUnreadable {
            unit_type: "LENGTHUNIT",
            ..
        })
    ));
}
