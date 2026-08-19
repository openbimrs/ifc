//! Units resolved against the real fixture corpus.
//!
//! Unit tests use synthetic models; these prove the resolver survives what
//! actual exporters write, where the unit assignment is buried in a real
//! project structure.

use ifc_geometry::units;
use ifc_model::Codec;
use ifc_step::StepCodec;
use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../test/fixtures")
        .join(rel)
}

/// Every fixture must resolve units without panicking, and the factor must be
/// physically plausible. A building is not 1e12 metres.
#[test]
fn every_fixture_resolves_a_plausible_unit_scale() {
    let dir = fixture("");
    let mut checked = 0;

    fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|e| e == "ifc") {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    walk(&dir, &mut files);
    assert!(!files.is_empty(), "fixture corpus should not be empty");

    for path in files {
        let Ok(model) = StepCodec.read_path(&path) else {
            continue;
        };
        let scale = units::resolve(&model);
        let name = path.file_name().unwrap().to_string_lossy();

        assert!(
            scale.length_to_metres > 1e-9 && scale.length_to_metres < 1e9,
            "{name}: implausible length factor {}",
            scale.length_to_metres
        );
        assert!(
            scale.angle_to_radians > 0.0 && scale.angle_to_radians <= 1.0,
            "{name}: implausible angle factor {}",
            scale.angle_to_radians
        );
        checked += 1;
    }
    assert!(
        checked >= 19,
        "expected the committed corpus, got {checked}"
    );
}

/// Unit resolution must read what the file actually declares.
///
/// This fixture declares `IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.)` — metres with
/// no prefix. An earlier version of this test asserted millimetres on the
/// assumption that Revit exports mm; the file says otherwise. Asserting the
/// file, not the folklore.
///
/// Note the trap this guards: the same file has
/// `IFCSIUNIT(*,.MASSUNIT.,.KILO.,.GRAM.)`. A resolver that matches on the
/// prefix without checking `UnitType` would read KILO off the mass unit and
/// scale every coordinate by 1000.
#[test]
fn reads_the_declared_length_unit_and_ignores_other_unit_types() {
    let path = fixture("ifclite-geometry/issue_098_wall_W.ifc");
    let model = StepCodec.read_path(&path).expect("fixture parses");
    let scale = units::resolve(&model);

    assert_eq!(
        scale.length_to_metres, 1.0,
        "file declares METRE with no prefix, got {scale:?}"
    );
    assert!(
        scale.is_metric_identity(),
        "the KILO on MASSUNIT must not leak into the length scale"
    );

    // The angle unit is IFCCONVERSIONBASEDUNIT(#46,.PLANEANGLEUNIT.,'DEGREE',#47).
    // The file also contains #45 = IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.),
    // but #45 is NOT referenced by the IfcUnitAssignment -- only the degree
    // unit is. Reading the assignment list rather than scanning every
    // IfcSIUnit in the file is what makes this come out right.
    assert!(
        (scale.angle_to_radians - std::f64::consts::PI / 180.0).abs() < 1e-12,
        "file assigns DEGREE, got {}",
        scale.angle_to_radians
    );
    assert!(
        (scale.angle(90.0) - std::f64::consts::FRAC_PI_2).abs() < 1e-12,
        "90 degrees is pi/2 radians"
    );
}

/// A genuinely millimetre-based file scales coordinates down by a thousand.
///
/// Built explicitly rather than hunted for in the corpus, so the assertion
/// stays true regardless of which fixtures are present.
#[test]
fn millimetre_projects_scale_coordinates() {
    use ifc_model::{Entity, EntityId, Model, Value};

    let mut model = Model::new();
    model.insert(
        EntityId(1),
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
        EntityId(2),
        Entity::new(
            "IFCUNITASSIGNMENT",
            vec![Value::List(vec![Value::Ref(EntityId(1))])],
        ),
    );
    let mut project = vec![Value::Null; 9];
    project[8] = Value::Ref(EntityId(2));
    model.insert(EntityId(3), Entity::new("IFCPROJECT", project));

    let scale = units::resolve(&model);
    assert_eq!(scale.length_to_metres, 1e-3);
    assert_eq!(scale.length(3000.0), 3.0, "a 3000 mm wall is 3 m");
}
