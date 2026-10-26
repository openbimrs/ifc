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

/// Curve representations as polylines (#458): the wall's and the proxy's
/// `Axis` is an `IfcPolyline` from (-2000, 0) to (2000, 0) mm, so (-2, 0, 0)
/// to (2, 0, 0) m in the product's frame, its own curve.
#[cfg(feature = "mesh")]
#[test]
fn axis_curves_are_polylines_relative_to_the_placement() {
    use ifc::RepresentationPurpose;

    let model = fixture();
    let curves = ifc::product_curves(&model, None, RepresentationPurpose::Axis);
    let products: Vec<u64> = curves.iter().map(|(id, _)| id.0).collect();
    assert_eq!(products, [36, 46, 53, 65]);
    for index in [0, 2] {
        let product = curves[index].1.as_ref().expect("the axis compiles");
        assert_eq!(product.polylines.len(), 1);
        let axis = &product.polylines[0];
        assert_eq!(axis.curve, Some(EntityId(33)));
        assert!(!axis.closed);
        assert_eq!(axis.chord_bound, 0.0);
        assert_eq!(axis.chord_requested, 1e-3);
        let expected = [-2.0, 0.0, 0.0, 2.0, 0.0, 0.0];
        assert_eq!(axis.positions.len(), 6);
        for (got, want) in axis.positions.iter().zip(expected) {
            assert!(
                (f64::from(*got) - want).abs() < 1e-6,
                "{:?}",
                axis.positions
            );
        }
        // Mapped back, the axis lies 5,403 km out, as the wall does.
        let start = [axis.positions[0], axis.positions[1], axis.positions[2]].map(f64::from);
        let world = product.world.apply(start);
        assert!((world[1] - 5_403_001.0).abs() < 3.0, "{world:?}");
    }
    // A slab has no Axis and the text member none either: no failure.
    for index in [1, 3] {
        assert!(curves[index].1.as_ref().unwrap().polylines.is_empty());
    }
    // The drawing selector picks the same Axis when nothing else is drawn.
    let plan = ifc::product_curves(&model, Some(&[EntityId(36)]), RepresentationPurpose::Plan);
    assert_eq!(plan[0].1, curves[0].1);
    // The Body is a solid and has no curves.
    let body = ifc::product_curves(&model, Some(&[EntityId(36)]), RepresentationPurpose::Body);
    assert!(body[0].1.as_ref().unwrap().polylines.is_empty());
}

/// A mesh reports whether it bounds a solid; an `Axis` has nothing to mesh.
#[cfg(feature = "mesh")]
#[test]
fn meshes_report_their_closure_and_select_a_representation() {
    use ifc::geometry::compile::MeshClosure;
    use ifc::RepresentationPurpose;

    let model = fixture();
    let meshes = ifc::product_meshes(&model, None);
    assert_eq!(meshes[0].1.as_ref().unwrap().closure, MeshClosure::Solid);
    assert_eq!(meshes[2].1.as_ref().unwrap().closure, MeshClosure::Unknown);
    let body = ifc::product_meshes_for(&model, None, RepresentationPurpose::Body);
    assert_eq!(body.len(), meshes.len());
    for ((_, a), (_, b)) in body.iter().zip(&meshes) {
        assert_eq!(a.as_ref().ok(), b.as_ref().ok(), "Body is the default");
    }
    let axis = ifc::product_meshes_for(&model, None, RepresentationPurpose::Axis);
    for (product, mesh) in &axis {
        let mesh = mesh.as_ref().expect("no Axis mesh is no failure");
        assert!(mesh.indices.is_empty(), "#{}", product.0);
        assert_eq!(mesh.colour, Ok(None));
    }
}

/// A slab whose `Surface` is a rectangular trimmed plane, 2 x 3 m, is a
/// surface mesh of 6 m2; a wall whose `Axis` is an `IfcTrimmedCurve` of a
/// unit circle from 0 to 1.5 rad compiles to a certified polyline on that
/// arc (#506).
#[cfg(feature = "mesh")]
#[test]
fn a_trimmed_surface_meshes_as_a_surface_and_a_trimmed_axis_is_a_polyline() {
    use ifc::geometry::compile::MeshClosure;
    use ifc::RepresentationPurpose;

    let text = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;
#1=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#2,$);
#2=IFCAXIS2PLACEMENT3D(#3,$,$);
#3=IFCCARTESIANPOINT((0.,0.,0.));
#4=IFCLOCALPLACEMENT($,#2);
#10=IFCSLAB('0000000000000000000010',$,'S',$,$,#4,#11,$,$);
#11=IFCPRODUCTDEFINITIONSHAPE($,$,(#12));
#12=IFCSHAPEREPRESENTATION(#1,'Surface','Surface3D',(#13));
#13=IFCRECTANGULARTRIMMEDSURFACE(#14,0.,0.,2.,3.,.T.,.T.);
#14=IFCPLANE(#2);
#20=IFCWALL('0000000000000000000020',$,'W',$,$,#4,#21,$,$);
#21=IFCPRODUCTDEFINITIONSHAPE($,$,(#22));
#22=IFCSHAPEREPRESENTATION(#1,'Axis','Curve2D',(#23));
#23=IFCTRIMMEDCURVE(#24,(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(1.5)),.T.,.PARAMETER.);
#24=IFCCIRCLE(#25,1.);
#25=IFCAXIS2PLACEMENT2D(#26,$);
#26=IFCCARTESIANPOINT((0.,0.));
ENDSEC;\nEND-ISO-10303-21;\n";
    let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");

    let surfaces = ifc::product_meshes_for(
        &model,
        Some(&[EntityId(10)]),
        RepresentationPurpose::Surface,
    );
    let slab = surfaces[0].1.as_ref().expect("the surface meshes");
    assert_eq!(slab.closure, MeshClosure::Surface);
    let p = |i: u32| {
        let i = 3 * i as usize;
        [
            slab.positions[i],
            slab.positions[i + 1],
            slab.positions[i + 2],
        ]
        .map(f64::from)
    };
    let area: f64 = slab
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| {
            let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
            let (u, v) = ([b[0] - a[0], b[1] - a[1]], [c[0] - a[0], c[1] - a[1]]);
            0.5 * (u[0] * v[1] - u[1] * v[0]).abs()
        })
        .sum();
    assert!((area - 6.0).abs() < 1e-6, "{area}");

    let axis = ifc::product_curves(&model, Some(&[EntityId(20)]), RepresentationPurpose::Axis);
    let curves = axis[0].1.as_ref().expect("the trimmed axis compiles");
    assert_eq!(curves.polylines.len(), 1);
    let arc = &curves.polylines[0];
    assert_eq!(arc.curve, Some(EntityId(23)));
    assert!(!arc.closed);
    assert!(
        arc.chord_bound > 0.0 && arc.chord_bound <= arc.chord_requested,
        "{arc:?}"
    );
    let vertices: Vec<[f64; 3]> = arc
        .positions
        .as_chunks::<3>()
        .0
        .iter()
        .map(|v| [v[0], v[1], v[2]].map(f64::from))
        .collect();
    let (first, last) = (vertices[0], vertices[vertices.len() - 1]);
    assert!(
        (first[0] - 1.0).abs() < 1e-6 && first[1].abs() < 1e-6,
        "{first:?}"
    );
    assert!(
        (last[0] - 1.5_f64.cos()).abs() < 1e-6 && (last[1] - 1.5_f64.sin()).abs() < 1e-6,
        "{last:?}"
    );
    for v in &vertices {
        let angle = v[1].atan2(v[0]);
        // f32 positions: on the unit circle and within the trim.
        assert!((v[0].hypot(v[1]) - 1.0).abs() < 1e-6, "{v:?}");
        assert!((-1e-6..=1.5 + 1e-6).contains(&angle), "{v:?}");
    }
}

/// A dimension `IfcAnnotation` (IFC4X3, `PredefinedType = DIMENSION`)
/// whose `Annotation2D` holds its dimension line, an extension line and
/// its value as an `IfcTextLiteralWithExtent`, beside an annotation fill
/// area: the two lines come back as polylines, the text and the fill area
/// as skipped items, not as a refusal (#512).
#[cfg(feature = "mesh")]
#[test]
fn a_dimension_annotation_gives_its_lines_and_skips_its_text() {
    use ifc::{NonCurveItem, RepresentationPurpose};

    let text = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4X3_ADD2'));\nENDSEC;\nDATA;
#1=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Plan',2,1.E-05,#2,$);
#2=IFCAXIS2PLACEMENT3D(#3,$,$);
#3=IFCCARTESIANPOINT((0.,0.,0.));
#4=IFCLOCALPLACEMENT($,#5);
#5=IFCAXIS2PLACEMENT3D(#6,$,$);
#6=IFCCARTESIANPOINT((10.,0.,0.));
#10=IFCANNOTATION('0000000000000000000010',$,'Dimension',$,$,#4,#11,.DIMENSION.);
#11=IFCPRODUCTDEFINITIONSHAPE($,$,(#12));
#12=IFCSHAPEREPRESENTATION(#1,'Annotation','Annotation2D',(#13,#14,#15,#18));
#13=IFCPOLYLINE((#20,#21));
#14=IFCPOLYLINE((#20,#22));
#15=IFCTEXTLITERALWITHEXTENT('2.00',#16,.LEFT.,#17,'bottom-middle');
#16=IFCAXIS2PLACEMENT2D(#23,$);
#17=IFCPLANAREXTENT(0.5,0.1);
#18=IFCANNOTATIONFILLAREA(#19,$);
#19=IFCPOLYLINE((#20,#21,#22,#20));
#20=IFCCARTESIANPOINT((0.,0.));
#21=IFCCARTESIANPOINT((2.,0.));
#22=IFCCARTESIANPOINT((0.,-0.5));
#23=IFCCARTESIANPOINT((1.,0.1));
ENDSEC;\nEND-ISO-10303-21;\n";
    let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");

    let curves = ifc::product_curves(
        &model,
        Some(&[EntityId(10)]),
        RepresentationPurpose::Annotation,
    );
    let dimension = curves[0].1.as_ref().expect("the lines compile");
    let lines: Vec<_> = dimension.polylines.iter().map(|p| p.curve).collect();
    assert_eq!(lines, [Some(EntityId(13)), Some(EntityId(14))]);
    let end = &dimension.polylines[0].positions[3..6];
    assert_eq!(end, [2.0, 0.0, 0.0]);
    let skipped: Vec<_> = dimension
        .skipped
        .iter()
        .map(|skipped| (skipped.item, skipped.kind))
        .collect();
    assert_eq!(
        skipped,
        [
            (EntityId(15), NonCurveItem::TextLiteral),
            (EntityId(18), NonCurveItem::AnnotationFillArea),
        ]
    );
    // Plan, the default drawing purpose, reads the same representation.
    let plan = ifc::product_curves(&model, Some(&[EntityId(10)]), RepresentationPurpose::Plan);
    assert_eq!(plan[0].1.as_ref(), Ok(dimension));
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

/// A wall clipped by polygonal bounded half-spaces, each bounded along its
/// `Position`'s Z (`BoundedHalfSpaceAlongZ`, axiolid/kernel#321; #515), is
/// wire format 1.4.
#[cfg(feature = "geometry-wire")]
#[test]
fn a_polygonal_bounded_clip_is_labelled_wire_format_1_4() {
    use ifc::geometry::GeometryGraph;

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test/fixtures/synthetic-compile/gable_wall_polygonal_clip_ifc2x3.ifc"
    );
    let model = StepCodec
        .read_bytes(&std::fs::read(path).expect("fixture"))
        .expect("parses");
    let graphs = ifc::product_graphs(&model, Some(&[EntityId(9031)]));
    let (_, graph) = graphs.into_iter().next().expect("the wall");
    let graph = graph.expect("lowers").graph.expect("a Body");
    let text = graph.to_json().expect("encodes");
    let envelope: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(envelope["version"], "1.4", "{text}");
    assert!(text.contains("\"BoundedHalfSpaceAlongZ\""), "{text}");
    let back = GeometryGraph::from_json(&text).expect("reads back");
    assert_eq!(back.to_json().unwrap(), text);
}
