//! Swept solids and surfaces: authored, then read back.
//!
//! The layouts do not generalise across the family, so each test pins
//! the slots its type actually uses. The swept disk solid is the one
//! that breaks the pattern -- `Directrix` at slot 0, no inherited
//! `SweptArea` -- and it gets its own test for that reason.

use ifc_geometry::authoring::{
    axis1_placement, axis2_placement_3d, cartesian_point, cylindrical_surface, direction,
    extruded_area_solid_tapered, fixed_reference_swept_area_solid, plane, polyline,
    rectangle_profile, revolved_area_solid_tapered, surface_curve_swept_area_solid,
    surface_of_linear_extrusion, surface_of_revolution, swept_disk_solid,
    swept_disk_solid_polygonal, SweepTrim,
};
use ifc_geometry::solid::swept::{
    ExtrudedAreaSolid, RevolvedAreaSolid, SweptAreaSolid, SweptDiskSolid,
};
use ifc_model::{EntityId, Model, Transaction, Value};

/// A profile and a placement, which most sweeps need.
fn setup(tx: &mut Transaction) -> (EntityId, EntityId, EntityId) {
    let origin = cartesian_point(tx, &[0.0, 0.0, 0.0]).expect("origin");
    let placement = axis2_placement_3d(tx, origin, None, None);
    let profile = rectangle_profile(tx, Some("Section"), None, 0.3, 0.2).expect("profile");
    (origin, placement, profile)
}

/// The tapered sweeps carry a second profile at the end of the sweep.
#[test]
fn tapered_sweeps_carry_both_profiles() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let (origin, placement, start_profile) = setup(&mut tx);
    let end_profile = rectangle_profile(&mut tx, Some("End"), None, 0.15, 0.1).expect("end");
    let up = direction(&mut tx, &[0.0, 0.0, 1.0]).expect("up");
    let axis = axis1_placement(&mut tx, origin, Some(up));

    let extruded = extruded_area_solid_tapered(
        &mut tx,
        start_profile,
        Some(placement),
        up,
        2.4,
        end_profile,
    )
    .expect("tapered extrusion");
    let revolved = revolved_area_solid_tapered(
        &mut tx,
        start_profile,
        Some(placement),
        axis,
        90.0,
        end_profile,
    )
    .expect("tapered revolution");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(extruded).expect("extruded");
    assert_eq!(entity.attributes.len(), 5, "EndSweptArea is slot 4");
    let view = ExtrudedAreaSolid::new(extruded, entity);
    assert_eq!(view.depth().expect("depth"), 2.4);
    let base = SweptAreaSolid::new(extruded, entity);
    assert_eq!(base.swept_area().expect("swept area"), start_profile);
    // The end profile is the distinguishing attribute: it must not be
    // confused with the start one.
    assert_eq!(entity.attributes[4], Value::Ref(end_profile));

    let entity = model.get(revolved).expect("revolved");
    let view = RevolvedAreaSolid::new(revolved, entity);
    // The angle is in the file's plane-angle unit, written unconverted.
    assert_eq!(view.angle_raw().expect("angle"), 90.0);
    assert_eq!(view.axis().expect("axis"), axis);
    assert_eq!(entity.attributes[4], Value::Ref(end_profile));
}

/// The swept disk solid puts `Directrix` at slot 0.
///
/// It subtypes `IfcSolidModel` directly, so there is no inherited
/// `SweptArea`/`Position` pair. Writing it like the other sweeps would
/// shift every attribute by two and still parse.
#[test]
fn a_swept_disk_solid_starts_at_its_directrix() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let a = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("a");
    let b = cartesian_point(&mut tx, &[5.0, 0.0, 0.0]).expect("b");
    let directrix = polyline(&mut tx, &[a, b]).expect("directrix");

    let id = swept_disk_solid(
        &mut tx,
        directrix,
        0.1,
        Some(0.08),
        SweepTrim {
            start: Some(0.0),
            end: Some(1.0),
        },
    )
    .expect("disk solid");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("disk");
    assert_eq!(
        entity.attributes[0],
        Value::Ref(directrix),
        "Directrix is slot 0"
    );
    let view = SweptDiskSolid::new(id, entity);
    assert_eq!(view.directrix().expect("directrix"), directrix);
    assert_eq!(view.radius().expect("radius"), 0.1);
    assert_eq!(view.inner_radius(), Some(0.08));
}

/// A bore at least as wide as its tube leaves no solid.
#[test]
fn a_disk_solid_bore_must_be_narrower_than_the_tube() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let a = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("a");
    let b = cartesian_point(&mut tx, &[5.0, 0.0, 0.0]).expect("b");
    let directrix = polyline(&mut tx, &[a, b]).expect("directrix");

    assert!(
        swept_disk_solid(&mut tx, directrix, 0.1, Some(0.1), SweepTrim::default()).is_err(),
        "an inner radius equal to the outer one leaves nothing"
    );
    assert!(
        swept_disk_solid(&mut tx, directrix, 0.1, Some(0.2), SweepTrim::default()).is_err(),
        "a bore wider than the tube is not a solid"
    );
    assert!(
        swept_disk_solid(&mut tx, directrix, 0.0, None, SweepTrim::default()).is_err(),
        "a zero radius disk sweeps nothing"
    );

    // Zero is a legal fillet: IfcNonNegativeLengthMeasure means a sharp
    // joint, not an error.
    assert!(
        swept_disk_solid_polygonal(
            &mut tx,
            directrix,
            0.1,
            None,
            SweepTrim::default(),
            Some(0.0),
        )
        .is_ok(),
        "a sharp joint is legal"
    );
    assert!(
        swept_disk_solid_polygonal(
            &mut tx,
            directrix,
            0.1,
            None,
            SweepTrim::default(),
            Some(-0.01),
        )
        .is_err(),
        "a negative fillet is not"
    );
}

/// The two directrix sweeps share slots 0-4 and differ only at slot 5.
#[test]
fn directrix_sweeps_differ_only_in_their_last_slot() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let (origin, placement, profile) = setup(&mut tx);
    let a = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("a");
    let b = cartesian_point(&mut tx, &[5.0, 0.0, 0.0]).expect("b");
    let directrix = polyline(&mut tx, &[a, b]).expect("directrix");
    let surface = plane(&mut tx, placement);
    let up = direction(&mut tx, &[0.0, 0.0, 1.0]).expect("up");
    let _ = origin;

    let trim = SweepTrim {
        start: Some(0.0),
        end: Some(4.0),
    };
    let on_surface =
        surface_curve_swept_area_solid(&mut tx, profile, Some(placement), directrix, trim, surface)
            .expect("surface curve sweep");
    let fixed =
        fixed_reference_swept_area_solid(&mut tx, profile, Some(placement), directrix, trim, up)
            .expect("fixed reference sweep");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    for id in [on_surface, fixed] {
        let entity = model.get(id).expect("sweep");
        assert_eq!(entity.attributes.len(), 6);
        assert_eq!(entity.attributes[0], Value::Ref(profile));
        assert_eq!(entity.attributes[2], Value::Ref(directrix));
        // Trim parameters keep their measure type.
        match &entity.attributes[3] {
            Value::Typed { type_name, .. } => {
                assert_eq!(type_name.as_ref(), "IFCPARAMETERVALUE");
            }
            other => panic!("StartParam lost its measure: {other:?}"),
        }
    }

    // Slot 5 is the only difference between the two types.
    assert_eq!(
        model.get(on_surface).expect("a").attributes[5],
        Value::Ref(surface)
    );
    assert_eq!(model.get(fixed).expect("b").attributes[5], Value::Ref(up));
}

/// Swept surfaces: a linear extrusion takes a signed depth.
#[test]
fn swept_surfaces_round_trip() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let (origin, placement, profile) = setup(&mut tx);
    let up = direction(&mut tx, &[0.0, 0.0, 1.0]).expect("up");
    let axis = axis1_placement(&mut tx, origin, Some(up));

    // Depth is IfcLengthMeasure on the surface, not the positive form.
    let extruded = surface_of_linear_extrusion(&mut tx, profile, Some(placement), up, -1.5)
        .expect("a negative depth is legal on a surface");
    let revolved = surface_of_revolution(&mut tx, profile, Some(placement), axis);
    let cylinder = cylindrical_surface(&mut tx, placement, 0.5).expect("cylinder");
    assert!(
        cylindrical_surface(&mut tx, placement, 0.0).is_err(),
        "a zero radius cylinder is not a surface"
    );

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    assert_eq!(
        model.get(extruded).expect("e").attributes[3],
        Value::Real(-1.5)
    );
    assert_eq!(
        model.get(revolved).expect("r").attributes[2],
        Value::Ref(axis)
    );
    assert_eq!(
        model.get(cylinder).expect("c").attributes[1],
        Value::Real(0.5)
    );
}

/// A tapered extrusion needs a real depth.
///
/// `IfcSweptAreaSolid.Depth` is `IfcPositiveLengthMeasure`, unlike the
/// surface form where a signed length is legal. Without this the depth
/// guard on the tapered writer is never exercised.
#[test]
fn a_tapered_extrusion_needs_a_positive_depth() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let (_, placement, profile) = setup(&mut tx);
    let end = rectangle_profile(&mut tx, Some("End"), None, 0.15, 0.1).expect("end");
    let up = direction(&mut tx, &[0.0, 0.0, 1.0]).expect("up");

    assert!(
        extruded_area_solid_tapered(&mut tx, profile, Some(placement), up, 0.0, end).is_err(),
        "a zero depth extrudes nothing"
    );
    assert!(
        extruded_area_solid_tapered(&mut tx, profile, Some(placement), up, -2.0, end).is_err(),
        "a negative depth is not a positive length"
    );
    assert!(
        extruded_area_solid_tapered(&mut tx, profile, Some(placement), up, f64::NAN, end).is_err(),
        "a non-finite depth is refused"
    );

    // The revolved form takes a plain angle, so a negative one is legal.
    let origin = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("origin");
    let axis = axis1_placement(&mut tx, origin, Some(up));
    assert!(
        revolved_area_solid_tapered(&mut tx, profile, Some(placement), axis, -90.0, end).is_ok(),
        "a negative revolution angle is legal"
    );
    assert!(
        revolved_area_solid_tapered(&mut tx, profile, Some(placement), axis, f64::INFINITY, end,)
            .is_err(),
        "a non-finite angle is refused"
    );
}
