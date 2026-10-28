//! Native tests of the geometry every host binds (#328, ADR 0021):
//! placements with the selected Body (feature `placements`) and meshes
//! (feature `mesh`), each refusal typed per product. The hosts' own suites
//! only check their conversion of these records.
//!
//! The gate also runs this file with both features left out
//! (`--no-default-features --features ifc4`), where each operation must
//! refuse with `feature-disabled`.

use openbim_ifc_binding_core::IfcModel;

/// Millimetres, a site 512 km east and 5,403 km north of the origin, a
/// storey 3 m up, and four products: a wall turned 90 degrees with an Axis
/// beside its Body, a slab, a proxy with an Axis only, and a member whose
/// Body is a text literal.
#[cfg_attr(not(feature = "placements"), allow(dead_code))]
fn fixture() -> IfcModel {
    let path = format!(
        "{}/../../test/fixtures/synthetic-bindings/binding_geometry.ifc",
        env!("CARGO_MANIFEST_DIR")
    );
    IfcModel::open(std::path::Path::new(&path)).expect("fixture reads")
}

/// A placement chain broken three ways: a product placed by a missing
/// entity, one in a two-link cycle, and one whose relative placement is
/// a point.
#[cfg_attr(not(feature = "placements"), allow(dead_code))]
const BROKEN: &str = "ISO-10303-21;
HEADER;FILE_DESCRIPTION((''),'2;1');FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;
DATA;
#1=IFCCARTESIANPOINT((0.,0.,0.));
#2=IFCAXIS2PLACEMENT3D(#1,$,$);
#3=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',());
#4=IFCPRODUCTDEFINITIONSHAPE($,$,(#3));
#10=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,'dangling',$,$,#99,#4,$,$);
#20=IFCLOCALPLACEMENT(#21,#2);
#21=IFCLOCALPLACEMENT(#20,#2);
#22=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'cyclic',$,$,#20,#4,$,$);
#30=IFCLOCALPLACEMENT($,#1);
#31=IFCWALL('2YvctVUKr0kugbFTf53O9L',$,'point',$,$,#30,#4,$,$);
#90=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#91=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#92=IFCUNITASSIGNMENT((#90,#91));
#93=IFCPROJECT('3YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,$,#92);
ENDSEC;
END-ISO-10303-21;
";

#[cfg(feature = "placements")]
mod placements {
    use super::*;

    fn close(actual: &[f64; 16], expected: [f64; 16]) {
        for (a, e) in actual.iter().zip(expected) {
            assert!((a - e).abs() < 1e-6, "{actual:?} != {expected:?}");
        }
    }

    #[test]
    fn every_product_with_a_shape_is_placed_and_its_body_selected() {
        let placements = fixture().product_placements(None).unwrap();
        let ids: Vec<u64> = placements.iter().map(|p| p.id).collect();
        assert_eq!(ids, [36, 46, 53, 65], "products with a shape, in id order");

        let wall = &placements[0];
        assert_eq!(wall.type_name, "IFCWALL");
        assert_eq!(wall.global_id.as_deref(), Some("2nR5uK8Lw3eT6yH1aJ9sD0"));
        assert_eq!(wall.refusal, None);
        // Turned 90 degrees about Z: X runs north, Y west. Millimetres to
        // metres, the site's offset kept in f64.
        close(
            wall.transform.as_ref().unwrap(),
            [
                0.0,
                1.0,
                0.0,
                0.0, //
                -1.0,
                0.0,
                0.0,
                0.0, //
                0.0,
                0.0,
                1.0,
                0.0, //
                512_002.0,
                5_403_001.0,
                3.0,
                1.0,
            ],
        );
        let body = wall.representation.as_ref().unwrap();
        assert_eq!(body.id, 30, "the Body, not the Axis listed first");
        assert_eq!(body.identifier.as_deref(), Some("Body"));
        assert_eq!(body.representation_type.as_deref(), Some("SweptSolid"));
        assert_eq!(body.context, Some(7));
        assert_eq!(body.context_type.as_deref(), Some("Model"));
        assert_eq!(body.context_identifier.as_deref(), Some("Body"));
        assert_eq!(body.target_view.as_deref(), Some("MODEL_VIEW"));

        let axis_only = &placements[2];
        assert_eq!(axis_only.representation, None, "an Axis is not a body");
        assert_eq!(axis_only.refusal, None, "and that is not a failure");
        close(
            axis_only.transform.as_ref().unwrap(),
            [
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                1.0,
                0.0,
                512_000.0,
                5_403_000.0,
                3.0,
                1.0,
            ],
        );

        let text = &placements[3];
        assert_eq!(text.representation.as_ref().unwrap().id, 63);
        assert_eq!(text.refusal, None, "selection does not lower the items");
    }

    #[test]
    fn a_selection_names_only_the_requested_products() {
        let placements = fixture().product_placements(Some(&[65, 36])).unwrap();
        let ids: Vec<u64> = placements.iter().map(|p| p.id).collect();
        assert_eq!(ids, [65, 36], "in the order asked");
    }

    #[test]
    fn each_broken_placement_is_one_typed_refusal() {
        let model = IfcModel::parse(BROKEN.as_bytes()).unwrap();
        let placements = model.product_placements(None).unwrap();
        let codes: Vec<(u64, &str, Option<u64>)> = placements
            .iter()
            .map(|p| {
                let refusal = p.refusal.as_ref().expect("refused");
                assert!(p.transform.is_none(), "#{}: no placement", p.id);
                (p.id, refusal.code.as_str(), refusal.entity)
            })
            .collect();
        assert_eq!(
            codes,
            [
                // The resolver names the placement it could not find.
                (10, "missing-reference", Some(99)),
                (22, "budget-exceeded", Some(20)),
                (31, "invalid-model", Some(1)),
            ]
        );
        // The Body is still reported beside a refused placement.
        assert_eq!(placements[0].representation.as_ref().unwrap().id, 3);
    }

    #[test]
    fn a_model_without_a_length_unit_refuses_each_placement() {
        let model = IfcModel::parse(
            b"ISO-10303-21;
HEADER;FILE_DESCRIPTION((''),'2;1');FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;
DATA;
#1=IFCPROJECT('3YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,$,$);
#2=IFCCARTESIANPOINT((1000.,0.,0.));
#3=IFCAXIS2PLACEMENT3D(#2,$,$);
#4=IFCLOCALPLACEMENT($,#3);
#5=IFCBUILDINGELEMENTPROXY('1YvctVUKr0kugbFTf53O9L',$,'X',$,$,#4,#7,$,$);
#6=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',());
#7=IFCPRODUCTDEFINITIONSHAPE($,$,(#6));
ENDSEC;
END-ISO-10303-21;
",
        )
        .unwrap();
        let placements = model.product_placements(None).unwrap();
        assert_eq!(placements.len(), 1);
        let refusal = placements[0].refusal.as_ref().expect("refused");
        assert!(placements[0].transform.is_none());
        assert_eq!(refusal.code, "invalid-model");
        assert_eq!(refusal.entity, Some(1));
        assert!(
            refusal.message.contains("length unit"),
            "{}",
            refusal.message
        );
    }

    #[test]
    fn an_id_the_model_lacks_is_refused_not_dropped() {
        let placements = fixture().product_placements(Some(&[9999])).unwrap();
        assert_eq!(placements.len(), 1);
        assert_eq!(
            placements[0].refusal.as_ref().unwrap().code,
            "missing-reference"
        );
    }

    #[test]
    fn a_record_crosses_with_its_matrix_as_sixteen_reals() {
        use openbim_ifc_binding_core::record::Field;
        use openbim_ifc_binding_core::ToRecord;
        let placements = fixture().product_placements(Some(&[36])).unwrap();
        let record = placements[0].to_record();
        let Some(Field::List(matrix)) = record.get("transform") else {
            panic!("a transform list: {record:?}");
        };
        assert_eq!(matrix.len(), 16);
        assert!(matches!(
            record.get("representation"),
            Some(Field::Record(_))
        ));
        assert_eq!(record.get("refusal"), Some(&Field::Null));
    }
}

#[cfg(not(feature = "placements"))]
#[test]
fn placements_refuse_without_the_feature() {
    use openbim_ifc_binding_core::BindingError;
    let model = IfcModel::empty();
    assert_eq!(
        model.product_placements(None),
        Err(BindingError::FeatureDisabled("placements"))
    );
}

#[cfg(feature = "mesh")]
mod mesh {
    use super::*;

    #[test]
    fn the_wall_meshes_in_its_own_frame_and_the_text_is_refused() {
        let meshes = fixture().product_meshes(None).unwrap();
        let ids: Vec<u64> = meshes.iter().map(|m| m.id).collect();
        assert_eq!(ids, [36, 46, 53, 65]);

        let wall = &meshes[0];
        assert_eq!(wall.refusal, None);
        assert!(
            wall.indices.len() >= 36,
            "a box: {} indices",
            wall.indices.len()
        );
        assert_eq!(wall.indices.len() % 3, 0);
        assert_eq!(wall.positions.len() % 3, 0);
        let vertices = wall.positions.len() / 3;
        assert!(wall.indices.iter().all(|&i| (i as usize) < vertices));
        // Relative to the wall's placement, not 5,403 km out: the profile
        // is 4 m by 0.2 m about the origin, extruded 2.8 m up.
        let (mut min, mut max) = ([f32::MAX; 3], [f32::MIN; 3]);
        for p in wall.positions.chunks(3) {
            for axis in 0..3 {
                min[axis] = min[axis].min(p[axis]);
                max[axis] = max[axis].max(p[axis]);
            }
        }
        for (got, want) in min.iter().chain(&max).zip([-2.0, -0.1, 0.0, 2.0, 0.1, 2.8]) {
            assert!((got - want).abs() < 1e-4, "bounds {min:?} {max:?}");
        }
        let placed = fixture().product_placements(Some(&[36])).unwrap();
        assert_eq!(wall.transform, placed[0].transform, "one placement");

        let axis_only = &meshes[2];
        assert!(axis_only.positions.is_empty() && axis_only.indices.is_empty());
        assert_eq!(axis_only.refusal, None, "no Body is not a failure");
        assert!(axis_only.transform.is_some(), "and still placed");

        let text = &meshes[3];
        let refusal = text.refusal.as_ref().expect("a text literal is no solid");
        assert_eq!(refusal.code, "unsupported");
        assert_eq!(refusal.entity, Some(62));
        assert!(text.positions.is_empty() && text.transform.is_none());
    }

    #[test]
    fn a_mesh_reports_its_closure_and_another_representation_meshes_on_request() {
        let model = fixture();
        let meshes = model.product_meshes(None).unwrap();
        let closures: Vec<&str> = meshes.iter().map(|m| m.closure.as_str()).collect();
        assert_eq!(closures, ["solid", "solid", "unknown", "unknown"]);
        assert_eq!(model.product_meshes_for(None, "Body").unwrap(), meshes);
        // An Axis has nothing to mesh, and that is no failure.
        for mesh in model.product_meshes_for(None, "Axis").unwrap() {
            assert!(
                mesh.indices.is_empty() && mesh.refusal.is_none(),
                "#{}",
                mesh.id
            );
        }
        let error = model.product_meshes_for(None, "body").unwrap_err();
        assert_eq!(error.code(), "invalid-value");
    }

    /// The wall's and the proxy's `Axis`: an `IfcPolyline` from (-2000, 0)
    /// to (2000, 0) mm, so (-2, 0, 0) to (2, 0, 0) m in their frames.
    #[test]
    fn axis_curves_cross_as_polylines_relative_to_the_placement() {
        use openbim_ifc_binding_core::record::Field;
        use openbim_ifc_binding_core::ToRecord;

        let model = fixture();
        let curves = model.product_curves(None, "Axis").unwrap();
        let ids: Vec<u64> = curves.iter().map(|c| c.id).collect();
        assert_eq!(ids, [36, 46, 53, 65]);
        for index in [0, 2] {
            let axis = &curves[index];
            assert_eq!(axis.refusal, None);
            assert_eq!(axis.polylines.len(), 1);
            let line = &axis.polylines[0];
            assert_eq!(
                (
                    line.curve,
                    line.first_vertex,
                    line.vertex_count,
                    line.closed
                ),
                (Some(33), 0, 2, false)
            );
            assert_eq!((line.chord_bound, line.chord_requested), (0.0, 1e-3));
            for (got, want) in axis.positions.iter().zip([-2.0, 0.0, 0.0, 2.0, 0.0, 0.0]) {
                assert!((got - want).abs() < 1e-5, "{:?}", axis.positions);
            }
            let placed = model.product_placements(Some(&[axis.id])).unwrap();
            assert_eq!(axis.transform, placed[0].transform, "one placement");
        }
        for index in [1, 3] {
            assert!(curves[index].polylines.is_empty() && curves[index].refusal.is_none());
        }
        // `Plan` picks the same Axis; `Body` has no curves.
        assert_eq!(
            model.product_curves(Some(&[36]), "Plan").unwrap()[0],
            curves[0]
        );
        assert!(model.product_curves(Some(&[36]), "Body").unwrap()[0]
            .polylines
            .is_empty());
        // A Body of a text literal alone has no curves and is no refusal:
        // the literal is skipped by id (#512).
        let text = &model.product_curves(Some(&[65]), "Body").unwrap()[0];
        assert_eq!(
            (
                text.polylines.len(),
                text.refusal.as_ref(),
                text.skipped.as_slice()
            ),
            (0, None, &[62][..])
        );
        assert_eq!(
            text.to_record().get("skipped"),
            Some(&Field::List(vec![Field::Id(62)]))
        );
        assert!(curves[0].skipped.is_empty());

        let record = curves[0].to_record();
        assert_eq!(record.get("vertex_count"), Some(&Field::Count(2)));
        let Some(Field::List(polylines)) = record.get("polylines") else {
            panic!("a polyline list: {record:?}");
        };
        assert!(
            matches!(&polylines[0], Field::Record(r) if r.get("closed") == Some(&Field::Bool(false)))
        );
        assert_eq!(
            model.product_curves(None, "Centreline").unwrap_err().code(),
            "invalid-value"
        );
    }

    /// A trimmed axis is one certified polyline (#506); an offset of an
    /// offset, which the reference compiler has no certified polyline for,
    /// is an `unsupported` refusal naming the product, its message the
    /// kernel's reason.
    #[test]
    fn a_trimmed_axis_is_a_polyline_and_an_offset_of_an_offset_is_refused() {
        let model = IfcModel::parse(
            b"ISO-10303-21;
HEADER;FILE_DESCRIPTION((''),'2;1');FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;
DATA;
#1=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#2,$);
#2=IFCAXIS2PLACEMENT3D(#3,$,$);
#3=IFCCARTESIANPOINT((0.,0.,0.));
#4=IFCLOCALPLACEMENT($,#2);
#20=IFCWALL('0000000000000000000020',$,'W',$,$,#4,#21,$,$);
#21=IFCPRODUCTDEFINITIONSHAPE($,$,(#22));
#22=IFCSHAPEREPRESENTATION(#1,'Axis','Curve2D',(#23));
#23=IFCTRIMMEDCURVE(#24,(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(1.5)),.T.,.PARAMETER.);
#24=IFCCIRCLE(#25,1.);
#25=IFCAXIS2PLACEMENT2D(#26,$);
#26=IFCCARTESIANPOINT((0.,0.));
#30=IFCWALL('0000000000000000000030',$,'W',$,$,#4,#31,$,$);
#31=IFCPRODUCTDEFINITIONSHAPE($,$,(#32));
#32=IFCSHAPEREPRESENTATION(#1,'Axis','Curve2D',(#33));
#33=IFCOFFSETCURVE2D(#34,0.1,.F.);
#34=IFCOFFSETCURVE2D(#23,0.1,.F.);
#90=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#91=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#92=IFCUNITASSIGNMENT((#90,#91));
#93=IFCPROJECT('3YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,$,#92);
ENDSEC;
END-ISO-10303-21;
",
        )
        .unwrap();
        let curves = model.product_curves(None, "Axis").unwrap();
        let arc = &curves[0];
        assert_eq!(arc.id, 20);
        assert!(arc.refusal.is_none(), "{:?}", arc.refusal);
        assert_eq!(arc.polylines.len(), 1);
        let line = &arc.polylines[0];
        assert_eq!((line.curve, line.closed), (Some(23), false));
        assert!(line.chord_bound > 0.0 && line.chord_bound <= line.chord_requested);
        for vertex in arc.positions.as_chunks::<3>().0 {
            let radius = f64::from(vertex[0]).hypot(f64::from(vertex[1]));
            assert!((radius - 1.0).abs() < 1e-6, "{vertex:?}");
        }

        let refused = &curves[1];
        assert_eq!(refused.id, 30);
        let refusal = refused.refusal.as_ref().expect("refused");
        assert_eq!(
            (refusal.code.as_str(), refusal.entity),
            ("unsupported", Some(30))
        );
        assert!(
            refusal.message.contains("an offset of an offset curve"),
            "{}",
            refusal.message
        );
        assert!(refused.positions.is_empty() && refused.polylines.is_empty());
    }

    #[test]
    fn a_broken_placement_refuses_the_mesh_with_the_same_code() {
        let model = IfcModel::parse(BROKEN.as_bytes()).unwrap();
        let codes: Vec<String> = model
            .product_meshes(None)
            .unwrap()
            .into_iter()
            .map(|m| m.refusal.expect("refused").code)
            .collect();
        assert_eq!(
            codes,
            ["missing-reference", "budget-exceeded", "invalid-model"]
        );
    }

    #[test]
    fn the_binding_fixture_states_no_colour() {
        for mesh in fixture().product_meshes(None).unwrap() {
            assert_eq!(mesh.colour, None, "{}", mesh.id);
            assert_eq!(mesh.colour_refusal, None, "{}", mesh.id);
        }
    }

    /// The generated #449 fixture: one cube per colour source.
    fn colour_fixture(name: &str) -> IfcModel {
        let path = format!(
            "{}/../../test/fixtures/synthetic-colour/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        IfcModel::open(std::path::Path::new(&path)).expect("fixture reads")
    }

    #[test]
    fn meshes_carry_the_item_layer_or_material_colour() {
        use openbim_ifc_binding_core::ToRecord;
        for (name, ids) in [
            // MATERIAL, ITEM_OVER_MATERIAL, LAYER_OVER_MATERIAL, MULTI_LAYER
            ("material_colour.ifc", [58, 89, 107, 125]),
            ("material_colour_ifc4x3.ifc", [58, 89, 107, 125]),
        ] {
            let model = colour_fixture(name);
            let meshes = model.product_meshes(Some(&ids)).unwrap();
            let colour = |index: usize| meshes[index].colour.as_ref().expect(name);

            let material = colour(0);
            assert_eq!(
                (material.red, material.green, material.blue),
                (0.8, 0.1, 0.1)
            );
            assert_eq!(material.transparency, Some(0.25));
            assert_eq!(material.source, "material");
            assert_eq!(colour(1).source, "item");
            assert_eq!(colour(1).blue, 0.9);
            assert_eq!(colour(2).source, "layer");
            assert_eq!(colour(2).red, 0.9);

            let multi = &meshes[3];
            assert_eq!(multi.colour, None);
            assert!(
                !multi.indices.is_empty(),
                "the colour never refuses the mesh"
            );
            let refusal = multi.colour_refusal.as_ref().expect("two layers");
            assert_eq!(refusal.code, "ambiguous");

            // The record crosses with the colour fields (#449), then the
            // closure (#458), last.
            let record = meshes[0].to_record();
            let names: Vec<&str> = record.fields.iter().map(|(name, _)| *name).collect();
            assert_eq!(
                names[names.len() - 3..],
                ["colour", "colour_refusal", "closure"]
            );
        }

        let meshes = colour_fixture("material_colour_ifc2x3.ifc")
            .product_meshes(Some(&[57]))
            .unwrap();
        let colour = meshes[0].colour.as_ref().expect("IFC2X3 material colour");
        assert_eq!(
            (colour.source.as_str(), colour.transparency),
            ("material", Some(0.25))
        );
    }
}

#[cfg(not(feature = "mesh"))]
#[test]
fn meshes_refuse_without_the_feature() {
    use openbim_ifc_binding_core::BindingError;
    let model = IfcModel::empty();
    assert_eq!(
        model.product_meshes(None),
        Err(BindingError::FeatureDisabled("mesh"))
    );
    assert_eq!(
        model.product_meshes_for(None, "Surface"),
        Err(BindingError::FeatureDisabled("mesh"))
    );
    assert_eq!(
        model.product_curves(None, "Axis"),
        Err(BindingError::FeatureDisabled("mesh"))
    );
}
