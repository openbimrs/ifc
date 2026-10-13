//! The two geometry levels the language bindings carry (#328, ADR 0021):
//! `product_placements` (kernel-free) and `product_meshes` (`mesh`), on
//! the fixture the bindings' host tests read.

#![cfg(all(feature = "step", feature = "geometry-select"))]

use ifc::{column_major, product_placements, Codec, EntityId, Model, StepCodec};

fn fixture() -> Model {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test/fixtures/synthetic-bindings/binding_geometry.ifc"
    );
    StepCodec
        .read_bytes(&std::fs::read(path).expect("fixture"))
        .expect("parses")
}

#[test]
fn placements_and_the_selected_body_per_product() {
    let model = fixture();
    let placements = product_placements(&model, None);
    let products: Vec<u64> = placements.iter().map(|p| p.product.0).collect();
    assert_eq!(products, [36, 46, 53, 65]);

    let wall = &placements[0];
    let world = column_major(wall.world.as_ref().expect("placed"));
    // X runs north, Y west; millimetres are metres; the site's 5,403 km
    // offset is kept in f64.
    let expected = [
        0.0,
        1.0,
        0.0,
        0.0,
        -1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        512_002.0,
        5_403_001.0,
        3.0,
        1.0,
    ];
    for (got, want) in world.iter().zip(expected) {
        assert!((got - want).abs() < 1e-6, "{world:?}");
    }
    let body = wall.body.as_ref().unwrap().as_ref().expect("a Body");
    assert_eq!(body.id, EntityId(30));
    assert_eq!(body.representation_type.as_deref(), Some("SweptSolid"));
    let context = body.context.as_ref().expect("its context");
    assert_eq!(
        (context.id, context.target_view.as_deref()),
        (EntityId(7), Some("MODEL_VIEW"))
    );

    // An Axis is never a Body, and that is no failure.
    assert_eq!(placements[2].body, Ok(None));
}

#[cfg(feature = "mesh")]
#[test]
fn meshes_are_relative_to_the_placement_and_refused_per_product() {
    let model = fixture();
    let meshes = ifc::product_meshes(&model, None);
    let wall = meshes[0].1.as_ref().expect("the wall meshes");
    assert_eq!(
        wall.world,
        *product_placements(&model, Some(&[EntityId(36)]))[0]
            .world
            .as_ref()
            .unwrap()
    );
    // Mapping a vertex back through the placement lands it 5,403 km out.
    let first = [wall.positions[0], wall.positions[1], wall.positions[2]].map(f64::from);
    let world = wall.world.apply(first);
    assert!((world[1] - 5_403_001.0).abs() < 3.0, "{world:?}");
    assert!(wall.indices.len() >= 36);

    assert!(
        meshes[2].1.as_ref().unwrap().indices.is_empty(),
        "axis only"
    );
    let refused = meshes[3]
        .1
        .as_ref()
        .expect_err("a text literal is no solid");
    assert!(refused.is_unsupported(), "{refused}");
}

#[cfg(feature = "geometry-wire")]
#[test]
fn graphs_round_trip_through_the_wire_format_and_are_refused_per_product() {
    use ifc::geometry::wire::{FormatVersion, FORMAT_NAME, FORMAT_VERSION};
    use ifc::geometry::GeometryGraph;

    let model = fixture();
    let graphs = ifc::product_graphs(&model, None);
    let products: Vec<u64> = graphs.iter().map(|(id, _)| id.0).collect();
    assert_eq!(products, [36, 46, 53, 65]);

    let wall = graphs[0].1.as_ref().expect("the wall lowers");
    assert_eq!(
        wall.world,
        *product_placements(&model, Some(&[EntityId(36)]))[0]
            .world
            .as_ref()
            .unwrap()
    );
    let graph = wall.graph.as_ref().expect("a Body");
    assert_eq!(graph.roots().len(), 1);

    // JSON: the envelope names the format and version, and the payload
    // reads back into a graph that re-encodes identically.
    let text = graph.to_json().expect("encodes");
    let envelope: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(envelope["format"], FORMAT_NAME);
    // The lowest version the content needs: no station carries a seam
    // window, so 1.0, readable by `axiolid-model` 0.3.8 (axiolid/kernel#297).
    let version = envelope["version"].as_str().expect("a version");
    assert_eq!(version, "1.0");
    assert!(FORMAT_VERSION.reads(FormatVersion::parse(version).unwrap()));
    let back = GeometryGraph::from_json(&text).expect("reads back");
    assert_eq!(back.len(), graph.len());
    assert_eq!(back.roots().len(), graph.roots().len());
    assert_eq!(back.to_json().unwrap(), text);

    // CBOR reads the same graph.
    let bytes = graph.to_cbor().expect("encodes");
    let back = GeometryGraph::from_cbor(&bytes).expect("reads back");
    assert_eq!(back.to_json().unwrap(), text);

    // The site's 5,403 km offset is in the graph, kept bit-exactly.
    assert!(text.contains("5403001"), "{text}");

    // An Axis is no Body, and no failure; a text literal is refused alone.
    assert_eq!(graphs[2].1.as_ref().unwrap().graph, None);
    let refused = graphs[3]
        .1
        .as_ref()
        .expect_err("a text literal is no solid");
    assert!(refused.is_unsupported(), "{refused}");
}

/// A Body swept along a segment placed by a station near an offset's joint
/// carries a seam-snapping window (#423), so its payload is wire format
/// 1.1, the lowest version that holds it (axiolid/kernel#297). A run of
/// sections along the offset carries windows on its sections (#429), so
/// its payload is 1.2. A tube along an offset run beside the offset,
/// continued to the ends the kernel measures (#433), holds stations at its
/// basis's ends, so its payload is 1.3.
#[cfg(feature = "geometry-wire")]
#[test]
fn a_body_with_a_seam_window_is_labelled_wire_format_1_1() {
    use ifc::geometry::GeometryGraph;

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test/fixtures/synthetic-lowering/station_offset_bases_ifc4x3.ifc"
    );
    let model = StepCodec
        .read_bytes(&std::fs::read(path).expect("fixture"))
        .expect("parses");
    // RAIL_NEAR and RAIL_EDGE, swept at a station; DECK and SURFACE, runs.
    let graphs = ifc::product_graphs(
        &model,
        Some(&[EntityId(188), EntityId(199), EntityId(264), EntityId(296)]),
    );
    assert_eq!(graphs.len(), 4);
    for ((product, graph), version) in graphs.into_iter().zip(["1.1", "1.1", "1.2", "1.2"]) {
        let graph = graph
            .unwrap_or_else(|e| panic!("#{}: {e:?}", product.0))
            .graph
            .expect("a Body");
        let text = graph.to_json().expect("encodes");
        let envelope: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        assert_eq!(envelope["version"], version, "#{}: {text}", product.0);
        assert!(text.contains("\"seam_window\""), "#{}: {text}", product.0);
        let back = GeometryGraph::from_json(&text).expect("reads back");
        assert_eq!(back.to_json().unwrap(), text);
    }
    // GUTTER, the tube along the offset run.
    let graphs = ifc::product_graphs(&model, Some(&[EntityId(337)]));
    let (_, graph) = graphs.into_iter().next().expect("GUTTER");
    let graph = graph.expect("lowers").graph.expect("a Body");
    let text = graph.to_json().expect("encodes");
    let envelope: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(envelope["version"], "1.3", "{text}");
    assert!(text.contains("\"basis_end\":\"End\""), "{text}");
    GeometryGraph::from_json(&text).expect("reads back");
}
