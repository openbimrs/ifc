//! The colour of every product on every committed fixture is pinned (#608):
//! a hash of `product_colours(model, None)` per file, so a change to how
//! style, material or aspect records are found cannot move a colour or a
//! refusal unnoticed. On a mismatch the test prints the whole table.

#![cfg(all(feature = "step", feature = "colour"))]

use std::path::{Path, PathBuf};

use ifc::{product_colours, Codec, StepCodec};

/// Fixture path under `test/fixtures` -> FNV-1a 64 of the `Debug` text of
/// `product_colours(&model, None)` after a lenient read.
const EXPECTED: &[(&str, u64)] = &[
    ("costing/costing_schedule.ifc", 0x09612b07b5ecb5a5),
    ("hostile/string_literals.ifc", 0x09612b07b5ecb5a5),
    ("ifclite-geometry/bath_csg_solid.ifc", 0x1a22605ac4617d93),
    ("ifclite-geometry/issue_098_wall_W.ifc", 0x55e38d7f8e0ae86a),
    (
        "ifclite-geometry/issue_1155_halfspace_flyaway.ifc",
        0x119c39e5904c4384,
    ),
    (
        "ifclite-geometry/issue_1485_duct_elbow_surface_curve_swept.ifc",
        0x81afb0267064e02f,
    ),
    (
        "ifclite-geometry/issue_1985_scaled_kinds.ifc",
        0x1e58ce3a52d40fb0,
    ),
    (
        "ifclite-geometry/issue_2019_wall_two_overlapping_openings.ifc",
        0x227745e4f4498b6b,
    ),
    (
        "ifclite-geometry/mapped_instances_indexed_colour.ifc",
        0x71f166ad096cae15,
    ),
    (
        "ifclite-geometry/mapped_instances_indexed_colour_uniform.ifc",
        0x71f166ad096cae15,
    ),
    (
        "ifclite-geometry/mapped_instances_multi_item.ifc",
        0x4f9a7d1c8545f226,
    ),
    (
        "ifclite-geometry/mapped_instances_nested.ifc",
        0x3a0da9edd1dac3d4,
    ),
    (
        "ifclite-geometry/nested_mapped_item.ifc",
        0x0182e7cc1b7e02c2,
    ),
    (
        "ifclite-geometry/nested_mapped_item_cycle.ifc",
        0x07a66b0f84940fbb,
    ),
    (
        "ifclite-geometry/shared_point_faceted_brep.ifc",
        0x8f623f0abdd65d0e,
    ),
    (
        "ifclite-geometry/swept_disk_composite_arc_crankbar.ifc",
        0x28ebe37ec73dcd02,
    ),
    (
        "ifcopenshell-validate/fail-duplicated-guids-ifc4.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "ifcopenshell-validate/fail-header-attr-too-many.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "ifcopenshell-validate/fail-header-wrong-type-list.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "ifcopenshell-validate/fail-invalid-selected-simple-type.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "ifcopenshell-validate/fail-nil-as-derived.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "ifcopenshell-validate/pass-complex-number-ifc4.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "ifcopenshell-validate/pass-duplicated-guids-ifc4.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "ifcopenshell-validate/pass-selected-simple-type.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "nurbs/ifc4_rational_bspline_curve_surface.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "nurbs/invalid_abstract_base_splines.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "synthetic-bindings/binding_geometry.ifc",
        0xd3fb71b40df46293,
    ),
    ("synthetic-colour/material_colour.ifc", 0x4865b7c21b3b2cff),
    (
        "synthetic-colour/material_colour_ifc2x3.ifc",
        0xe06ea0e844ed1b69,
    ),
    (
        "synthetic-colour/material_colour_ifc4x3.ifc",
        0x4865b7c21b3b2cff,
    ),
    (
        "synthetic-compile/gable_wall_polygonal_clip_ifc2x3.ifc",
        0xc0a220cff57e0a65,
    ),
    (
        "synthetic-compile/gable_wall_polygonal_clip_ifc4.ifc",
        0xc0a220cff57e0a65,
    ),
    (
        "synthetic-compile/tall_wall_small_bounded_clip_ifc2x3.ifc",
        0x81e2ecac8c42292f,
    ),
    (
        "synthetic-compile/union_over_halfspace_unbounded.ifc",
        0x119c39e5904c4384,
    ),
    (
        "synthetic-compile/vertex_loop_zero_area.ifc",
        0x8f623f0abdd65d0e,
    ),
    (
        "synthetic-cost-schedule/synthetic_cost_schedule.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "synthetic-coverage/meshing_coverage.ifc",
        0xd006bc7824c13d00,
    ),
    (
        "synthetic-georef/epset_malformed_ifc2x3.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "synthetic-georef/epset_missing_northings_ifc2x3.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "synthetic-georef/epset_project_ifc2x3.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "synthetic-georef/epset_sites_ifc2x3.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "synthetic-georef/epset_split_pair_ifc2x3.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "synthetic-georef/map_conversion_ifc4.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "synthetic-lowering/curve_bounded_plane_composite.ifc",
        0xd2e9db04c4a6771f,
    ),
    (
        "synthetic-lowering/flush_openings_site_placements.ifc",
        0xe4d8131a3183932f,
    ),
    ("synthetic-lowering/grid_placement.ifc", 0x49d5fb9944acb905),
    (
        "synthetic-lowering/grid_placement_ifc4x3.ifc",
        0x49d5fb9944acb905,
    ),
    (
        "synthetic-lowering/halfspace_boundaries_ifc4x3.ifc",
        0xff0a13e9e2d3a2bb,
    ),
    (
        "synthetic-lowering/indexed_curve_arcs.ifc",
        0x5221a088fec402f4,
    ),
    (
        "synthetic-lowering/indexed_profile_boundaries.ifc",
        0xd53bf0650ee23852,
    ),
    (
        "synthetic-lowering/indexed_profile_boundaries_ifc4x3.ifc",
        0xd53bf0650ee23852,
    ),
    (
        "synthetic-lowering/linear_placement_alignment_frame.ifc",
        0x0c9211eda74c00a4,
    ),
    (
        "synthetic-lowering/linear_placement_cache_check.ifc",
        0x9131a2c8a5e9950f,
    ),
    (
        "synthetic-lowering/linear_placement_uncached.ifc",
        0xbce30bfe87157877,
    ),
    (
        "synthetic-lowering/reference_view_openings.ifc",
        0xf7eadb064884ac14,
    ),
    (
        "synthetic-lowering/station_offset_bases_ifc4x3.ifc",
        0x46c98a17fdb488c4,
    ),
    (
        "synthetic-lowering/station_offsets_ifc4x3.ifc",
        0x402bf21a787d13a6,
    ),
    (
        "synthetic-lowering/station_relations_ifc4x3.ifc",
        0x38cff6e50135cdd1,
    ),
    (
        "synthetic-lowering/station_seams_ifc4x3.ifc",
        0x71e49b48d5e9fefe,
    ),
    (
        "synthetic-properties/synthetic_properties.ifc",
        0x09612b07b5ecb5a5,
    ),
    ("synthetic-relations/cascade_ifc2x3.ifc", 0x18799b553a3cac4c),
    ("synthetic-relations/cascade_ifc4.ifc", 0x18799b553a3cac4c),
    ("synthetic-relations/cascade_ifc4x3.ifc", 0x18799b553a3cac4c),
    (
        "synthetic-relations/relations_ifc2x3.ifc",
        0x09612b07b5ecb5a5,
    ),
    ("synthetic-relations/relations_ifc4.ifc", 0x09612b07b5ecb5a5),
    (
        "synthetic-relations/relations_ifc4x3.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "synthetic-relations/selector_ifc2x3.ifc",
        0x09612b07b5ecb5a5,
    ),
    ("synthetic-relations/selector_ifc4.ifc", 0x09612b07b5ecb5a5),
    (
        "synthetic-relations/selector_ifc4x3.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "synthetic-surfaces/advanced_brep_3d_edges_ifc4.ifc",
        0xa6edbb00a6147cc5,
    ),
    (
        "synthetic-surfaces/advanced_brep_3d_edges_ifc4x3.ifc",
        0xa6edbb00a6147cc5,
    ),
    (
        "synthetic-surfaces/synthetic_advanced_brep.ifc",
        0xf5280e6fb9116254,
    ),
    (
        "synthetic-surfaces/synthetic_alignment_layout.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "synthetic-surfaces/synthetic_alignment_spiral.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "synthetic-surfaces/synthetic_bspline_surface.ifc",
        0xa347b5947c35e449,
    ),
    (
        "synthetic-surfaces/synthetic_collections.ifc",
        0x3b5c7aaad39d696d,
    ),
    (
        "synthetic-surfaces/synthetic_conic_offset_bounded.ifc",
        0x7eeafe706a0bbea3,
    ),
    (
        "synthetic-surfaces/synthetic_curve_bounded_plane.ifc",
        0xbec51ad09d67290f,
    ),
    (
        "synthetic-surfaces/synthetic_elementary_surfaces.ifc",
        0x80785dbbe5de07a6,
    ),
    (
        "synthetic-surfaces/synthetic_ifc4x3_alignment_curves.ifc",
        0x67a2d3c0fd9d524c,
    ),
    (
        "synthetic-surfaces/synthetic_ifc4x3_geometry_families.ifc",
        0x178804fd1029629b,
    ),
    (
        "synthetic-surfaces/synthetic_parameter_space_conics.ifc",
        0x80785dbbe5de07a6,
    ),
    (
        "synthetic-surfaces/synthetic_point_references.ifc",
        0x21511b67254a7ce7,
    ),
    (
        "synthetic-surfaces/synthetic_primitives_and_bbox.ifc",
        0x8bbdba13064fe3c5,
    ),
    (
        "synthetic-surfaces/synthetic_profile_families.ifc",
        0x8a24039f3819f40b,
    ),
    (
        "synthetic-surfaces/synthetic_sectioned_spine.ifc",
        0x0182e7cc1b7e02c2,
    ),
    (
        "synthetic-surfaces/synthetic_space_boundary_face_surface.ifc",
        0x09612b07b5ecb5a5,
    ),
    (
        "synthetic-surfaces/synthetic_surface_of_revolution.ifc",
        0x5866cab712d6c5de,
    ),
    (
        "synthetic-surfaces/synthetic_tapered_sweeps.ifc",
        0x110b59c480477e53,
    ),
    (
        "synthetic-systems/synthetic_systems.ifc",
        0x09612b07b5ecb5a5,
    ),
];

fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn fixtures(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .expect("fixture directory")
        .map(|entry| entry.expect("entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            fixtures(root, &path, out);
        } else if path.extension().is_some_and(|ext| ext == "ifc") {
            out.push(path.strip_prefix(root).expect("under root").to_path_buf());
        }
    }
}

#[test]
fn product_colours_are_pinned_on_every_fixture() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test/fixtures");
    let mut names = Vec::new();
    fixtures(&root, &root, &mut names);
    assert!(
        names.len() >= 50,
        "the fixture walk found {} files",
        names.len()
    );
    let actual: Vec<(String, u64)> = names
        .iter()
        .map(|name| {
            let model = StepCodec::lenient()
                .read_bytes(&std::fs::read(root.join(name)).expect("fixture"))
                .unwrap_or_else(|e| panic!("{} must parse leniently: {e:?}", name.display()));
            let colours = product_colours(&model, None);
            (
                name.to_string_lossy().replace('\\', "/"),
                fnv1a(&format!("{colours:?}")),
            )
        })
        .collect();
    let matches = actual.len() == EXPECTED.len()
        && actual
            .iter()
            .zip(EXPECTED)
            .all(|((name, hash), (want_name, want_hash))| name == want_name && hash == want_hash);
    if !matches {
        let table: String = actual
            .iter()
            .map(|(name, hash)| format!("    (\"{name}\", {hash:#018x}),\n"))
            .collect();
        panic!("the colour table changed; actual:\n{table}");
    }
}
