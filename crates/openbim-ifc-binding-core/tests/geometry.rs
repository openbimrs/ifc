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
}
