//! A missing `PLANEANGLEUNIT` is refused where an angle is read, and only
//! there (#585).
//!
//! A model that declares a length unit and no plane-angle unit is valid IFC.
//! Length-only work (a linearly placed product with a cached position, a
//! Cartesian trim) must resolve; reading an angle (a revolution, a horizontal
//! alignment segment, a parameter trim of a conic) names the entity.

#![cfg(feature = "lowering")]

use ifc_geometry::lower::{lower_representation_item, LoweringSession};
use ifc_geometry::transform::Transform;
use ifc_geometry::{product_world_transform, units, GeometryError};
use ifc_model::{Codec, Entity, EntityId, Model, Value};
use std::path::PathBuf;

fn parse(text: &str) -> Model {
    ifc_step::StepCodec
        .read_bytes(text.as_bytes())
        .unwrap_or_else(|e| panic!("text parses: {e:?}"))
}

fn fixture(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures")
        .join(rel);
    std::fs::read_to_string(path).expect("fixture reads")
}

/// The shape of `linear_placement.rs::model_with_linear_placement(true)` (a
/// cached `CartesianPosition`, an `IfcPolyline` basis) in a model whose unit
/// assignment holds only a millimetre length unit.
fn linear_placement_in_millimetres() -> (Model, EntityId) {
    let mut model = Model::default();
    let r = |n| Value::Ref(EntityId(n));
    model.insert(
        EntityId(1),
        Entity::new("IFCPOLYLINE", vec![Value::List(vec![])]),
    );
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCPOINTBYDISTANCEEXPRESSION",
            vec![
                Value::Real(1240.0),
                Value::Real(2.0),
                Value::Real(3.0),
                Value::Null,
                r(1),
            ],
        ),
    );
    model.insert(
        EntityId(3),
        Entity::new(
            "IFCAXIS2PLACEMENTLINEAR",
            vec![r(2), Value::Null, Value::Null],
        ),
    );
    model.insert(
        EntityId(4),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![
                Value::Real(100.0),
                Value::Real(200.0),
                Value::Real(5.0),
            ])],
        ),
    );
    model.insert(
        EntityId(5),
        Entity::new("IFCAXIS2PLACEMENT3D", vec![r(4), Value::Null, Value::Null]),
    );
    model.insert(
        EntityId(6),
        Entity::new("IFCLINEARPLACEMENT", vec![Value::Null, r(3), r(5)]),
    );
    let mut sign = vec![Value::Null; 7];
    sign[5] = r(6);
    model.insert(EntityId(7), Entity::new("IFCSIGN", sign));
    model.insert(
        EntityId(10),
        Entity::new(
            "IFCSIUNIT",
            vec![
                Value::Derived,
                Value::Enum("LENGTHUNIT".into()),
                Value::Enum("MILLI".into()),
                Value::Enum("METRE".into()),
            ],
        ),
    );
    model.insert(
        EntityId(11),
        Entity::new("IFCUNITASSIGNMENT", vec![Value::List(vec![r(10)])]),
    );
    let mut project = vec![Value::Null; 9];
    project[8] = r(11);
    model.insert(EntityId(12), Entity::new("IFCPROJECT", project));
    (model, EntityId(7))
}

#[test]
fn a_linear_placement_resolves_without_an_angle_unit() {
    let (model, sign) = linear_placement_in_millimetres();
    let scale = units::resolve(&model).expect("a length unit is enough");
    assert_eq!(scale.angle_to_radians, None);
    let world = product_world_transform(&model, &scale, sign)
        .expect("length-only placement must not need an angle unit");
    let expected = [0.1, 0.2, 0.005];
    for (got, want) in world.origin.iter().zip(expected) {
        assert!((got - want).abs() < 1e-12, "{:?}", world.origin);
    }
}

#[cfg(feature = "compile-reference-backend")]
mod alignment {
    use super::*;
    use axiolid_evaluate::ReferenceCurveEvaluator;
    use ifc_geometry::{product_world_transform_with_evaluator, CachedPositionPolicy};

    const EXTRA: &str = "#900=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(5.),0.,$,$,#401);\n\
#901=IFCAXIS2PLACEMENTLINEAR(#900,$,$);\n#902=IFCLINEARPLACEMENT($,#901,$);\n\
#903=IFCBUILDINGELEMENTPROXY('1Y0f2uCbr0gfm5cmtZmh01',$,'ON_ALIGNMENT',$,$,#902,$,$,$);\n";

    fn layout(with_angle_unit: bool) -> Model {
        let text = fixture("synthetic-surfaces/synthetic_alignment_layout.ifc");
        // The vertical profile spans the whole horizontal plan.
        let text = text.replacen("0.,15.,100.", "0.,17.853981633974483,100.", 1);
        let text = if with_angle_unit {
            text
        } else {
            let without = text.replacen("IFCUNITASSIGNMENT((#1,#2))", "IFCUNITASSIGNMENT((#1))", 1);
            assert_ne!(without, text, "the angle unit is dropped");
            without
        };
        let text = text.replacen("ENDSEC;\nEND-ISO", &format!("{EXTRA}ENDSEC;\nEND-ISO"), 1);
        assert!(text.contains("ON_ALIGNMENT"), "records appended");
        parse(&text)
    }

    fn place(model: &Model) -> Result<ifc_geometry::Transform, GeometryError> {
        let scale = units::resolve(model).expect("project units");
        product_world_transform_with_evaluator(
            model,
            &scale,
            EntityId(903),
            &ReferenceCurveEvaluator::default(),
            CachedPositionPolicy::Verify,
        )
    }

    #[test]
    fn a_horizontal_alignment_basis_read_refuses_without_an_angle_unit() {
        place(&layout(true)).expect("with an angle unit the placement resolves");
        let model = layout(false);
        let GeometryError::AngleUnitUndeclared { entity } =
            place(&model).expect_err("a StartDirection needs an angle unit")
        else {
            panic!("expected AngleUnitUndeclared");
        };
        assert!(
            model
                .get(entity)
                .is_some_and(|e| e.is_type("IFCALIGNMENTHORIZONTALSEGMENT")),
            "names the segment that holds the angle, got {entity:?}"
        );
    }
}

const TRIMMED: &str =
    "#1=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);\n#2=IFCUNITASSIGNMENT((#1));\n\
#3=IFCPROJECT('3YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,$,#2);\n\
#10=IFCCARTESIANPOINT((0.,0.,0.));\n#11=IFCAXIS2PLACEMENT3D(#10,$,$);\n\
#12=IFCCIRCLE(#11,1000.);\n#13=IFCCARTESIANPOINT((1000.,0.,0.));\n\
#14=IFCCARTESIANPOINT((0.,1000.,0.));\n";

fn trimmed_model(preference: &str) -> Model {
    parse(&format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('ViewDefinition [ReferenceView]'),'2;1');\n\
FILE_NAME('t.ifc','2026-01-01T00:00:00',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n\
{TRIMMED}#15=IFCTRIMMEDCURVE(#12,(IFCPARAMETERVALUE(0.),#13),(IFCPARAMETERVALUE(90.),#14),.T.,{preference});\n\
ENDSEC;\nEND-ISO-10303-21;\n"
    ))
}

fn lower_trimmed(model: &Model) -> Result<(), GeometryError> {
    let scale = units::resolve(model).expect("project units");
    let mut session = LoweringSession::new(model, &scale);
    lower_representation_item(&mut session, EntityId(15), Transform::identity()).map(|_| ())
}

#[test]
fn a_cartesian_trim_of_a_circle_lowers_without_an_angle_unit() {
    lower_trimmed(&trimmed_model(".CARTESIAN.")).expect("the points carry the trim");
}

#[test]
fn a_parameter_trim_of_a_circle_needs_the_angle_unit() {
    assert!(matches!(
        lower_trimmed(&trimmed_model(".PARAMETER.")),
        Err(GeometryError::AngleUnitUndeclared { entity }) if entity == EntityId(15)
    ));
}

#[test]
fn a_revolved_solid_refuses_without_an_angle_unit() {
    let text = fixture("synthetic-surfaces/synthetic_tapered_sweeps.ifc");
    let without = text.replacen("IFCUNITASSIGNMENT((#1,#5))", "IFCUNITASSIGNMENT((#1))", 1);
    assert_ne!(without, text, "the angle unit is dropped");
    let model = parse(&without);
    let solid = model.ids_of_type("IFCREVOLVEDAREASOLID")[0];
    let scale = units::resolve(&model).expect("project units");
    let mut session = LoweringSession::new(&model, &scale);
    let error = lower_representation_item(&mut session, solid, Transform::identity())
        .expect_err("an Angle needs an angle unit");
    assert_eq!(error, GeometryError::AngleUnitUndeclared { entity: solid });
}
