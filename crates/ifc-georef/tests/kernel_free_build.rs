//! Proof that the kernel-free build is real, not decorative (#268).
//!
//! A semantic adapter reads a file's CRS, map conversion and true north and
//! has no use for a geometry kernel. A single unconditional
//! `use axiolid_core` anywhere in the crate silently relinks it, and nothing
//! in a normal test run would notice, because the default build links it
//! anyway.
//!
//! This checks the **resolved dependency graph** under
//! `--no-default-features`, not the manifest: an optional dependency can be
//! re-enabled by accident through a feature edge, and only the resolver
//! knows. Same technique as `ifc-geometry`'s `kernel_free_build.rs`.

#[path = "support/step.rs"]
mod support;

use std::process::Command;

use ifc_georef::{
    coordinate_operation_for, grid_north_direction, resolve_project_to_map_in, resolve_true_north,
    GeorefView, NorthReference, OperationKind, OperationSource,
};
use ifc_model::EntityId;
use support::{step, BASE};

/// Every crate linked into `ifc-georef` for a given feature selection.
fn dependency_tree(extra: &[&str]) -> String {
    let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let mut cmd = Command::new(env!("CARGO"));
    cmd.args([
        "tree",
        "--manifest-path",
        manifest,
        "--edges",
        "normal",
        "--prefix",
        "none",
    ]);
    cmd.args(extra);
    let out = cmd.output().expect("cargo tree should run");
    assert!(
        out.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("tree output is utf-8")
}

/// The crate names in a `--prefix none` tree.
fn crate_names(tree: &str) -> impl Iterator<Item = &str> {
    tree.lines()
        .filter_map(|line| line.split_whitespace().next())
}

/// Without `transform`, no `axiolid-*` crate and not the math library
/// behind them may be linked.
#[test]
fn the_kernel_free_build_links_no_geometry_crate() {
    let tree = dependency_tree(&["--no-default-features"]);
    let forbidden: Vec<&str> = crate_names(&tree)
        .filter(|name| name.starts_with("axiolid-") || *name == "glam")
        .collect();
    assert!(
        forbidden.is_empty(),
        "a kernel-free ifc-georef links {forbidden:?}. Something uses the \
         neutral geometry vocabulary unconditionally; gate it behind \
         `#[cfg(feature = \"transform\")]`.\n{tree}"
    );

    // ...while keeping what resolution actually needs, so an empty or
    // failed tree cannot pass the check above.
    for required in ["ifc-georef", "ifc-model", "ifc-schema"] {
        assert!(
            crate_names(&tree).any(|name| name == required),
            "the kernel-free tree lost {required}:\n{tree}"
        );
    }
}

/// With `transform` (the default), `axiolid-core` must be present.
///
/// The inverse assertion: a feature table that gated the dependency off
/// entirely would pass the test above while breaking every default user.
#[test]
fn the_default_build_still_links_the_transform_vocabulary() {
    let tree = dependency_tree(&[]);
    assert!(
        crate_names(&tree).any(|name| name == "axiolid-core"),
        "the default build lost axiolid-core; `ProjectToMap::transform` cannot exist.\n{tree}"
    );
}

/// An IFC4 model with a map conversion and a rotated true north.
///
/// `#6` points true north 30 degrees west of project north; `#60` puts
/// the project origin at E 500 000, N 5 800 000, H 100 with project X
/// rotated by atan2(0.8, 0.6) onto map XY and a 1:1000 scale.
const IFC4_GEOREFERENCED: &str = "\
#6=IFCDIRECTION((-0.5,0.8660254037844386));
#11=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-6,#5,#6);
#60=IFCMAPCONVERSION(#11,#50,500000.,5800000.,100.,0.6,0.8,0.001);
";

/// Map conversion and true north, read from an IFC4 file. Meaningful in
/// every column; the gate runs it under `--no-default-features`, where it
/// proves the parameters are reachable without the kernel.
#[test]
fn reads_an_ifc4_map_conversion_and_true_north_without_the_kernel() {
    let model = step("IFC4", &format!("{BASE}{IFC4_GEOREFERENCED}"));
    let view = GeorefView::for_model(&model).expect("IFC4 header");

    let operation_id = coordinate_operation_for(&view, EntityId(11))
        .expect("context resolves")
        .expect("context has a map conversion");
    assert_eq!(operation_id, EntityId(60));

    // Project length unit: millimetres, so the scale composes with units.
    let operation = resolve_project_to_map_in(&view, operation_id, 0.001).expect("resolves");
    assert_eq!(operation.kind, OperationKind::MapConversion);
    assert!(matches!(
        operation.source,
        OperationSource::Context(EntityId(11))
    ));
    assert_eq!(operation.target_crs.name.as_deref(), Some("EPSG:25832"));
    assert_eq!(
        operation.target_crs.vertical_datum.as_deref(),
        Some("DHHN2016")
    );
    assert_eq!(operation.map_unit.metres_per_unit, 1.0);
    assert_eq!(operation.project_unit.metres_per_unit, 0.001);
    assert_eq!(
        (
            operation.eastings,
            operation.northings,
            operation.orthogonal_height
        ),
        (500_000.0, 5_800_000.0, 100.0)
    );
    assert_eq!(operation.declared_scale, 0.001);
    assert_eq!(operation.x_axis_direction, (0.6, 0.8));
    assert_eq!(operation.translation(), [500_000.0, 5_800_000.0, 100.0]);

    // Scale 0.001 maps project millimetres to map metres; the project unit
    // is also millimetres, so in metres the operation is a pure rotation.
    let mapped = operation.map_point([1.0, 0.0, 2.0]);
    let expected = [500_000.6, 5_800_000.8, 102.0];
    for axis in 0..3 {
        assert!(
            (mapped[axis] - expected[axis]).abs() < 1e-9,
            "{mapped:?} != {expected:?}"
        );
    }

    let NorthReference::True { direction } =
        resolve_true_north(&model, EntityId(11)).expect("true north resolves")
    else {
        panic!("a context's TrueNorth is a true-north reference");
    };
    assert!(
        (direction.0 + 0.5).abs() < 1e-12 && (direction.1 - 0.866_025_403_784_438_6).abs() < 1e-12
    );

    // Grid north is the map's northing axis pulled back into the project.
    assert_eq!(grid_north_direction(&operation).direction(), (0.8, 0.6));
}
