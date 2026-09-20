//! Surfaces: authored, then read back through the crate's own views.
//!
//! A B-spline surface carries two independent knot vectors and a grid
//! whose rows must agree in length. None of that is expressible in the
//! EXPRESS types, so all of it is the writer's to enforce.

use ifc_geometry::authoring::{
    axis2_placement_3d, bspline_surface_with_knots, cartesian_point, circle, curve_bounded_plane,
    curve_bounded_surface, plane, rational_bspline_surface_with_knots, rectangular_trimmed_surface,
    spherical_surface, toroidal_surface, SurfaceBasis, SurfaceKnots,
};
use ifc_geometry::surface::bounded::{
    CurveBoundedPlane, CurveBoundedSurface, RectangularTrimmedSurface,
};
use ifc_geometry::surface::bspline::BSplineSurface;
use ifc_geometry::surface::elementary::{SphericalSurface, ToroidalSurface};
use ifc_model::{EntityId, Model, Transaction};

/// A 3D placement at the origin.
fn origin(tx: &mut Transaction) -> EntityId {
    let point = cartesian_point(tx, &[0.0, 0.0, 0.0]).expect("point");
    axis2_placement_3d(tx, point, None, None)
}

/// The analytic surfaces round-trip, with distinct radii so a slot
/// transposition on the torus is visible.
#[test]
fn analytic_surfaces_round_trip() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = origin(&mut tx);

    let sphere = spherical_surface(&mut tx, at, 2.5).expect("sphere");
    let torus = toroidal_surface(&mut tx, at, 5.0, 1.25).expect("torus");

    assert!(spherical_surface(&mut tx, at, 0.0).is_err(), "zero radius");
    assert!(
        toroidal_surface(&mut tx, at, 5.0, 0.0).is_err(),
        "zero minor radius"
    );
    assert!(
        toroidal_surface(&mut tx, at, f64::NAN, 1.0).is_err(),
        "non-finite major radius"
    );

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let view = SphericalSurface::new(sphere, model.get(sphere).expect("sphere"));
    assert_eq!(view.radius().expect("radius"), 2.5);
    assert_eq!(view.position_ref().expect("position"), at);

    let view = ToroidalSurface::new(torus, model.get(torus).expect("torus"));
    assert_eq!(view.major_radius().expect("major"), 5.0);
    assert_eq!(view.minor_radius().expect("minor"), 1.25);
}

/// A B-spline surface round-trips its grid, and each knot identity is
/// checked against its own direction.
///
/// The grid here is deliberately non-square (3 rows, 2 columns) so a
/// writer that applied the u identity to v, or transposed the grid,
/// fails rather than coincidentally passing.
#[test]
fn a_bspline_surface_checks_each_direction_separately() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    let mut grid_ids = Vec::new();
    for u in 0..3 {
        let mut row = Vec::new();
        for v in 0..2 {
            row.push(cartesian_point(&mut tx, &[f64::from(u), f64::from(v), 0.0]).expect("point"));
        }
        grid_ids.push(row);
    }
    let grid: Vec<&[EntityId]> = grid_ids.iter().map(Vec::as_slice).collect();

    // u: degree 2 over 3 rows    -> 2 + 3 + 1 = 6
    // v: degree 1 over 2 columns -> 1 + 2 + 1 = 4
    let u = SurfaceKnots {
        multiplicities: &[3, 3],
        knots: &[0.0, 1.0],
    };
    let v = SurfaceKnots {
        multiplicities: &[2, 2],
        knots: &[0.0, 1.0],
    };

    let basis = SurfaceBasis {
        degree: (2, 1),
        u,
        v,
        form: "UNSPECIFIED",
        knot_spec: "UNSPECIFIED",
    };
    let id = bspline_surface_with_knots(&mut tx, &grid, basis)
        .expect("a consistent surface is accepted");

    // Swapping the two knot vectors breaks both identities at once.
    assert!(
        bspline_surface_with_knots(
            &mut tx,
            &grid,
            SurfaceBasis {
                u: v,
                v: u,
                ..basis
            }
        )
        .is_err(),
        "the u knots do not describe the v direction"
    );

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("surface");
    let view = BSplineSurface::new(id, entity);
    assert_eq!(view.u_degree().expect("u degree"), 2);
    assert_eq!(view.v_degree().expect("v degree"), 1);

    let points = view.control_points().expect("grid");
    assert_eq!(points.u_count(), 3, "rows run along u");
    assert_eq!(points.v_count(), 2, "columns run along v");
    assert_eq!(
        points.get(2, 1),
        Some(grid_ids[2][1]),
        "grid is not transposed"
    );

    assert!(view.has_knots());
    assert!(!view.is_rational());
    // Neither closure was evaluated, so neither may be claimed.
    assert_eq!(view.u_closed(), None);
    assert_eq!(view.self_intersect(), None);
}

/// A ragged control grid, and the weight grid that must match it.
#[test]
fn a_control_grid_must_be_rectangular() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let p = |tx: &mut Transaction, x: f64| cartesian_point(tx, &[x, 0.0, 0.0]).expect("point");
    let a0 = p(&mut tx, 0.0);
    let a1 = p(&mut tx, 1.0);
    let a2 = p(&mut tx, 2.0);

    let long: Vec<EntityId> = vec![a0, a1, a2];
    let short: Vec<EntityId> = vec![a0, a1];
    let ragged: Vec<&[EntityId]> = vec![long.as_slice(), short.as_slice()];

    // u: degree 1 over 2 rows -> 4. v would be 1 + 3 + 1 = 5.
    let u = SurfaceKnots {
        multiplicities: &[2, 2],
        knots: &[0.0, 1.0],
    };
    let v = SurfaceKnots {
        multiplicities: &[3, 2],
        knots: &[0.0, 1.0],
    };
    assert!(
        bspline_surface_with_knots(
            &mut tx,
            &ragged,
            SurfaceBasis {
                degree: (1, 1),
                u,
                v,
                form: "UNSPECIFIED",
                knot_spec: "UNSPECIFIED"
            }
        )
        .is_err(),
        "LIST OF LIST allows a ragged grid; it denotes no surface"
    );

    // A square grid, with a weight grid one entry short.
    let square: Vec<&[EntityId]> = vec![short.as_slice(), short.as_slice()];
    let both = SurfaceKnots {
        multiplicities: &[2, 2],
        knots: &[0.0, 1.0],
    };
    let full_weights: Vec<&[f64]> = vec![&[1.0, 1.0], &[1.0, 1.0]];
    let short_weights: Vec<&[f64]> = vec![&[1.0, 1.0], &[1.0]];

    assert!(
        rational_bspline_surface_with_knots(
            &mut tx,
            &square,
            SurfaceBasis {
                degree: (1, 1),
                u: both,
                v: both,
                form: "UNSPECIFIED",
                knot_spec: "UNSPECIFIED"
            },
            &short_weights
        )
        .is_err(),
        "a weight row shorter than its control row"
    );

    let id = rational_bspline_surface_with_knots(
        &mut tx,
        &square,
        SurfaceBasis {
            degree: (1, 1),
            u: both,
            v: both,
            form: "UNSPECIFIED",
            knot_spec: "UNSPECIFIED",
        },
        &full_weights,
    )
    .expect("matched weights");

    let mut model = model;
    tx.commit(&mut model).expect("commit");
    let entity = model.get(id).expect("rational");
    assert_eq!(entity.attributes.len(), 13, "WeightsData is slot 12");
    let view = BSplineSurface::new(id, entity);
    assert!(view.is_rational());
}

/// Bounded surfaces keep their outer boundary distinct from the holes.
#[test]
fn bounded_surfaces_keep_outer_and_inner_apart() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = origin(&mut tx);
    let flat = plane(&mut tx, at);
    let outer = circle(&mut tx, at, 5.0).expect("outer");
    let hole = circle(&mut tx, at, 1.0).expect("hole");

    let bounded = curve_bounded_plane(&mut tx, flat, outer, &[hole]);
    // SET [0:?]: a plane with no holes is an empty set, not `$`.
    let solid = curve_bounded_plane(&mut tx, flat, outer, &[]);
    let general = curve_bounded_surface(&mut tx, flat, &[outer], true).expect("bounded");
    assert!(
        curve_bounded_surface(&mut tx, flat, &[], true).is_err(),
        "SET [1:?] needs a boundary"
    );

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let view = CurveBoundedPlane::new(bounded, model.get(bounded).expect("plane"));
    assert_eq!(view.basis_surface_ref().expect("basis"), flat);
    assert_eq!(view.outer_boundary_ref().expect("outer"), outer);
    assert_eq!(view.inner_boundary_refs(), vec![hole]);

    let view = CurveBoundedPlane::new(solid, model.get(solid).expect("solid"));
    assert!(
        view.inner_boundary_refs().is_empty(),
        "no holes is an empty set, and reads back as one"
    );

    let view = CurveBoundedSurface::new(general, model.get(general).expect("surface"));
    assert_eq!(view.boundary_refs().expect("boundaries"), vec![outer]);
    assert!(view.implicit_outer());
}

/// A rectangular trim derives its senses from its parameters.
///
/// The schema requires `Vsense = (V2 > V1)`, so the two can never
/// disagree -- the writer does not offer a way to make them.
#[test]
fn a_rectangular_trim_derives_its_senses() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = origin(&mut tx);
    let flat = plane(&mut tx, at);

    let forward =
        rectangular_trimmed_surface(&mut tx, flat, (0.0, 1.0), (0.0, 2.0)).expect("forward trim");
    // Reversed in v: the sense must follow, not be asserted separately.
    let reversed =
        rectangular_trimmed_surface(&mut tx, flat, (0.0, 1.0), (2.0, 0.0)).expect("reversed trim");

    assert!(
        rectangular_trimmed_surface(&mut tx, flat, (1.0, 1.0), (0.0, 2.0)).is_err(),
        "U1 and U2 must differ"
    );
    assert!(
        rectangular_trimmed_surface(&mut tx, flat, (0.0, 1.0), (2.0, 2.0)).is_err(),
        "V1 and V2 must differ"
    );

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let view = RectangularTrimmedSurface::new(forward, model.get(forward).expect("trim"));
    assert_eq!(view.basis_surface_ref().expect("basis"), flat);
    let rect = view.rectangle().expect("rectangle");
    assert!(rect.usense, "u runs forward");
    assert!(rect.vsense, "v runs forward");

    let view = RectangularTrimmedSurface::new(reversed, model.get(reversed).expect("trim"));
    let rect = view.rectangle().expect("rectangle");
    assert!(!rect.vsense, "v runs backward, and the sense says so");
}

/// The u sense follows the u parameters, not the v ones.
///
/// The earlier trim test moves u forward in both cases, so a writer
/// that computed `usense` from the v parameters would pass it. Here u
/// runs backward while v runs forward, which separates them.
#[test]
fn each_trim_sense_follows_its_own_direction() {
    use ifc_model::Value;

    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = origin(&mut tx);
    let flat = plane(&mut tx, at);

    // u descends, v ascends: the two senses must disagree.
    let id =
        rectangular_trimmed_surface(&mut tx, flat, (3.0, 1.0), (0.0, 2.0)).expect("mixed trim");
    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("trim");
    let view = RectangularTrimmedSurface::new(id, entity);
    let rect = view.rectangle().expect("rectangle");
    assert!(!rect.usense, "u descends, so usense is false");
    assert!(rect.vsense, "v ascends, so vsense is true");
    assert_ne!(rect.usense, rect.vsense, "the senses are independent");

    // Each parameter keeps its measure type. The reader tolerates a
    // bare real, so only the stored value shows the wrapper survived.
    for (index, name) in [(1, "U1"), (2, "V1"), (3, "U2"), (4, "V2")] {
        match &entity.attributes[index] {
            Value::Typed { type_name, .. } => {
                assert_eq!(type_name.as_ref(), "IFCPARAMETERVALUE", "{name}");
            }
            other => panic!("{name} lost its measure wrapper: {other:?}"),
        }
    }
}

/// Surface knots must strictly increase, in both directions.
///
/// A repeated value belongs in the multiplicity list, not the knot
/// list; writing it twice describes a different basis.
#[test]
fn surface_knots_must_strictly_increase() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let mut rows = Vec::new();
    for u in 0..2 {
        let mut row = Vec::new();
        for v in 0..2 {
            row.push(cartesian_point(&mut tx, &[f64::from(u), f64::from(v), 0.0]).expect("point"));
        }
        rows.push(row);
    }
    let grid: Vec<&[EntityId]> = rows.iter().map(Vec::as_slice).collect();

    let good = SurfaceKnots {
        multiplicities: &[2, 2],
        knots: &[0.0, 1.0],
    };
    // Same sum, but the knots repeat rather than increase.
    let repeated = SurfaceKnots {
        multiplicities: &[2, 2],
        knots: &[1.0, 1.0],
    };
    let descending = SurfaceKnots {
        multiplicities: &[2, 2],
        knots: &[1.0, 0.0],
    };

    assert!(
        bspline_surface_with_knots(
            &mut tx,
            &grid,
            SurfaceBasis {
                degree: (1, 1),
                u: repeated,
                v: good,
                form: "UNSPECIFIED",
                knot_spec: "UNSPECIFIED"
            }
        )
        .is_err(),
        "a repeated u knot belongs in the multiplicity"
    );
    assert!(
        bspline_surface_with_knots(
            &mut tx,
            &grid,
            SurfaceBasis {
                degree: (1, 1),
                u: good,
                v: descending,
                form: "UNSPECIFIED",
                knot_spec: "UNSPECIFIED"
            }
        )
        .is_err(),
        "v knots must not descend"
    );
    assert!(
        bspline_surface_with_knots(
            &mut tx,
            &grid,
            SurfaceBasis {
                degree: (1, 1),
                u: good,
                v: good,
                form: "UNSPECIFIED",
                knot_spec: "UNSPECIFIED"
            }
        )
        .is_ok(),
        "the well formed case still passes"
    );
}
