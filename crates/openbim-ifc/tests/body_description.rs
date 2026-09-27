//! The facade exposes body description in the kernel-free geometry size.
//!
//! `geometry-select` links no geometry kernel (`thin_build.rs` proves that
//! from the dependency graph); this proves the body-description entry points
//! are reachable at the facade root in that size and answer on a real file.

#![cfg(all(feature = "step", feature = "geometry-select"))]

use std::path::PathBuf;

use ifc::{body_description, geometry::units, BodyKind, ProfileParameters, SweepPath};
use ifc_model::EntityId;

/// `synthetic_profile_families.ifc` product #89: thirteen extrusions, the
/// first an I-section 290 mm deep, extruded 1000 mm along +Z.
#[test]
fn the_facade_describes_a_steel_section_body() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-surfaces/synthetic_profile_families.ifc");
    let model = ifc::read_path(&path).expect("fixture parses");
    let body = body_description(&model, &units::resolve(&model), EntityId(89))
        .expect("describes")
        .expect("has a body");
    assert_eq!(body.items.len(), 13);

    let first = &body.items[0];
    assert_eq!(first.kind, BodyKind::Extrusion);
    let swept = first.swept.as_ref().expect("an extrusion is swept");
    let ProfileParameters::IShape { overall_depth, .. } = swept.profile.parameters else {
        panic!("the first item is the I-section");
    };
    assert!((overall_depth - 0.29).abs() < 1e-12, "mm to m");
    let SweepPath::Extrusion {
        direction_world,
        depth,
        ..
    } = swept.path
    else {
        panic!("an extrusion path");
    };
    assert_eq!(direction_world, [0.0, 0.0, 1.0]);
    assert!((depth - 1.0).abs() < 1e-12);
}
