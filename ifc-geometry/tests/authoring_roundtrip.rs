//! Author geometry, then lower it back (ADR 0011).
//!
//! This is the test a separate authoring crate could not write. Both
//! directions compile together here, so a slot layout that authoring and
//! lowering disagree about fails immediately rather than silently producing
//! a file that parses and means something else.

use ifc_geometry::authoring::{
    axis2_placement_3d, boolean_result, cartesian_point, direction, extruded_area_solid,
    rectangle_profile,
};
use ifc_geometry::solid::boolean::IfcBooleanOperator;
use ifc_model::{Model, Transaction};

/// A 6m x 0.3m wall extruded 2.4m up, authored from plain numbers.
fn wall(tx: &mut Transaction) -> ifc_model::EntityId {
    let origin = cartesian_point(tx, &[0.0, 0.0, 0.0]).expect("origin");
    let up = direction(tx, &[0.0, 0.0, 1.0]).expect("up");
    let placement = axis2_placement_3d(tx, origin, Some(up), None);
    let profile = rectangle_profile(tx, Some("Wall"), None, 6.0, 0.3).expect("profile");
    extruded_area_solid(tx, profile, Some(placement), up, 2.4).expect("solid")
}

#[test]
fn an_authored_extrusion_is_read_back_by_the_same_crate() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let solid = wall(&mut tx);
    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(solid).expect("solid present");
    assert_eq!(entity.type_name.as_ref(), "IFCEXTRUDEDAREASOLID");

    // Read through the crate's own view, which indexes the same slot
    // constants the writer used.
    let view = ifc_geometry::solid::swept::ExtrudedAreaSolid::new(solid, entity);
    assert_eq!(view.depth().expect("depth"), 2.4);
}

/// The strongest form: authored entities survive the full lowering path
/// into the neutral DAG. If authoring wrote a slot the lowerer reads
/// differently, this fails rather than producing a wrong graph.
///
/// Requires `lowering`: authoring itself does not, which is the point.
#[test]
#[cfg(feature = "lowering")]
fn an_authored_extrusion_lowers_into_the_neutral_graph() {
    use ifc_geometry::lower::{lower_representation_item, LoweringSession};
    use ifc_geometry::transform::Transform;
    use ifc_geometry::units;

    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let solid = wall(&mut tx);
    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale);
    let node = lower_representation_item(&mut session, solid, Transform::identity())
        .expect("authored solid lowers");
    let lowered = session.finish(node).expect("graph finishes");
    assert_eq!(lowered.root, node);
}

#[test]
fn values_that_would_parse_but_mean_nothing_are_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    // A zero-length direction denotes no direction; normalising it downstream
    // yields NaN rather than an error.
    assert!(direction(&mut tx, &[0.0, 0.0, 0.0]).is_err());
    // NaN propagates silently through placement composition.
    assert!(cartesian_point(&mut tx, &[f64::NAN, 0.0, 0.0]).is_err());
    // IfcPositiveLengthMeasure excludes zero: a zero-depth extrusion is a
    // face claiming to be a solid.
    let p = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("point");
    let d = direction(&mut tx, &[0.0, 0.0, 1.0]).expect("dir");
    let prof = rectangle_profile(&mut tx, None, None, 1.0, 1.0).expect("profile");
    assert!(extruded_area_solid(&mut tx, prof, None, d, 0.0).is_err());
    assert!(rectangle_profile(&mut tx, None, None, 0.0, 1.0).is_err());
    // A boolean against itself is never meaningful.
    assert!(boolean_result(&mut tx, IfcBooleanOperator::Difference, p, p).is_err());
}

/// Authored geometry survives serialisation to STEP text and lowers from
/// the reparsed bytes.
///
/// The round-trips above stay in memory, so a value authored correctly but
/// written or reparsed wrongly passes them. A consumer receives a file, so
/// this drives author -> write -> reparse -> lower and asserts on the
/// geometry that comes out the far end.
#[test]
#[cfg(feature = "lowering")]
fn an_authored_extrusion_survives_step_text_and_lowers() {
    use ifc_geometry::lower::{lower_representation_item, LoweringSession};
    use ifc_geometry::transform::Transform;
    use ifc_model::codec::Codec;
    use ifc_step::StepCodec;

    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let solid = wall(&mut tx);
    let mut model = model;
    *model.header_mut() = ifc_model::Header {
        schema: vec!["IFC4X3_ADD2".to_owned()],
        ..ifc_model::Header::default()
    };
    tx.commit(&mut model).expect("commit");

    let mut bytes = Vec::new();
    StepCodec
        .write(&model, &mut bytes)
        .expect("authored model serialises");
    let reparsed = StepCodec
        .read_bytes(&bytes)
        .expect("serialised text reparses");

    // The depth is the value most likely to be mangled by a writer that
    // loses precision: it is the one non-integer dimension authored.
    let entity = reparsed.get(solid).expect("solid survives the round trip");
    assert_eq!(entity.type_name.as_ref(), "IFCEXTRUDEDAREASOLID");
    let view = ifc_geometry::solid::swept::ExtrudedAreaSolid::new(solid, entity);
    assert_eq!(view.depth().expect("depth"), 2.4);

    let units = ifc_geometry::units::UnitScale::default();
    let mut session = LoweringSession::new(&reparsed, &units);
    lower_representation_item(&mut session, solid, Transform::identity())
        .expect("reparsed solid lowers into the neutral graph");
}
