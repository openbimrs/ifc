//! Native tests of Level 2 every host binds (#367, ADR 0021): each
//! product's Body as Axiolid's neutral geometry graph in its wire format
//! 1.0, JSON and CBOR, refusals typed per product. The hosts' own suites
//! check their conversion of these records and parse the payloads back.
//!
//! The gate also runs this file without the feature, where the call must
//! refuse with `feature-disabled`.

use openbim_ifc_binding_core::{GeometryEncoding, IfcModel};

#[cfg_attr(not(feature = "graph"), allow(dead_code))]
fn fixture() -> IfcModel {
    let path = format!(
        "{}/../../test/fixtures/synthetic-bindings/binding_geometry.ifc",
        env!("CARGO_MANIFEST_DIR")
    );
    IfcModel::open(std::path::Path::new(&path)).expect("fixture reads")
}

#[cfg(feature = "graph")]
mod graph {
    use super::*;
    use ifc::geometry::wire::{FormatVersion, FORMAT_NAME, FORMAT_VERSION};
    use ifc::geometry::GeometryGraph;
    use ifc::{Codec, StepCodec};
    use openbim_ifc_binding_core::{GEOMETRY_FORMAT, GEOMETRY_FORMAT_VERSION};

    /// The graph the facade lowers for `product`, as the reference.
    fn lowered(product: u64) -> GeometryGraph {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test/fixtures/synthetic-bindings/binding_geometry.ifc"
        );
        let model = StepCodec.read_bytes(&std::fs::read(path).unwrap()).unwrap();
        let mut graphs = ifc::product_graphs(&model, Some(&[ifc::EntityId(product)]));
        let (_, graph) = graphs.remove(0);
        graph.unwrap().graph.expect("a Body")
    }

    /// Two graphs are equal when they have the same nodes in the same order
    /// and the same roots: node handles are branded per graph, so `==`
    /// cannot compare a decoded graph with the original, and the encoding is
    /// canonical, so re-encoding compares them.
    fn same(a: &GeometryGraph, b: &GeometryGraph) {
        assert_eq!(a.len(), b.len());
        assert_eq!(a.roots().len(), b.roots().len());
        assert_eq!(a.to_json().unwrap(), b.to_json().unwrap());
    }

    #[test]
    fn the_json_payload_parses_back_into_the_lowered_graph() {
        let records = fixture()
            .product_geometry(None, GeometryEncoding::Json)
            .unwrap();
        let ids: Vec<u64> = records.iter().map(|r| r.id).collect();
        assert_eq!(ids, [36, 46, 53, 65], "products with a shape, in id order");

        let wall = &records[0];
        assert_eq!(wall.type_name, "IFCWALL");
        assert_eq!(wall.refusal, None);
        let text = wall.json().expect("JSON text");
        let envelope: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(envelope["format"], GEOMETRY_FORMAT);
        // The lowest version the content needs; `axiolid-model` 0.3.9 labels
        // every payload 1.1 (axiolid/kernel#297), so either is accepted.
        let version = envelope["version"].as_str().expect("a version");
        assert!(["1.0", "1.1"].contains(&version), "{version}");
        assert!(FORMAT_VERSION.reads(FormatVersion::parse(version).unwrap()));
        assert_eq!(
            (GEOMETRY_FORMAT, GEOMETRY_FORMAT_VERSION),
            ("axiolid-geometry-graph", "1.1")
        );
        assert_eq!(GEOMETRY_FORMAT, FORMAT_NAME);
        assert_eq!(GEOMETRY_FORMAT_VERSION, FORMAT_VERSION.to_string());

        let back = GeometryGraph::from_json(text).expect("reads back");
        same(&back, &lowered(36));
        assert_eq!(back.to_json().unwrap(), text, "re-encodes identically");

        // The placement rides along, the same as Level 1's.
        let placed = fixture().product_placements(Some(&[36])).unwrap();
        assert_eq!(wall.transform, placed[0].transform);
    }

    #[test]
    fn the_cbor_payload_reads_the_same_graph() {
        let model = fixture();
        let cbor = model
            .product_geometry(Some(&[36, 46]), GeometryEncoding::Cbor)
            .unwrap();
        let json = model
            .product_geometry(Some(&[36, 46]), GeometryEncoding::Json)
            .unwrap();
        for (cbor, json) in cbor.iter().zip(&json) {
            assert_eq!(cbor.encoding, GeometryEncoding::Cbor);
            assert_eq!(cbor.json(), None, "bytes, not text");
            let bytes = cbor.payload.as_deref().unwrap();
            let back = GeometryGraph::from_cbor(bytes).expect("reads back");
            same(&back, &lowered(cbor.id));
            assert_eq!(back.to_json().unwrap(), json.json().unwrap());
            assert!(bytes.len() < json.payload.as_ref().unwrap().len());
        }
    }

    #[test]
    fn a_product_without_a_body_or_refused_stays_a_record() {
        let records = fixture()
            .product_geometry(None, GeometryEncoding::Json)
            .unwrap();
        let axis_only = &records[2];
        assert_eq!(axis_only.payload, None);
        assert_eq!(axis_only.refusal, None, "no Body is not a failure");
        assert!(axis_only.transform.is_some(), "and still placed");

        let text = &records[3];
        let refusal = text.refusal.as_ref().expect("a text literal is no solid");
        assert_eq!(refusal.code, "unsupported");
        assert_eq!(refusal.entity, Some(62));
        assert!(text.payload.is_none() && text.transform.is_none());

        let missing = fixture()
            .product_geometry(Some(&[9999]), GeometryEncoding::Cbor)
            .unwrap();
        assert_eq!(
            missing[0].refusal.as_ref().unwrap().code,
            "missing-reference"
        );
    }

    #[test]
    fn a_record_carries_the_payload_size_not_the_payload() {
        use openbim_ifc_binding_core::record::Field;
        use openbim_ifc_binding_core::ToRecord;
        let records = fixture()
            .product_geometry(Some(&[36]), GeometryEncoding::Cbor)
            .unwrap();
        let record = records[0].to_record();
        assert_eq!(record.get("encoding"), Some(&Field::Text("cbor".into())));
        assert_eq!(
            record.get("payload_size"),
            Some(&Field::Count(records[0].payload.as_ref().unwrap().len()))
        );
        assert_eq!(record.get("refusal"), Some(&Field::Null));
    }
}

#[cfg(not(feature = "graph"))]
#[test]
fn graphs_refuse_without_the_feature() {
    use openbim_ifc_binding_core::BindingError;
    let model = IfcModel::empty();
    assert_eq!(
        model.product_geometry(None, GeometryEncoding::Json),
        Err(BindingError::FeatureDisabled("graph"))
    );
}

#[test]
fn encodings_are_named() {
    assert_eq!(
        GeometryEncoding::parse("json"),
        Some(GeometryEncoding::Json)
    );
    assert_eq!(
        GeometryEncoding::parse("cbor"),
        Some(GeometryEncoding::Cbor)
    );
    assert_eq!(GeometryEncoding::parse("CBOR"), None);
    assert_eq!(GeometryEncoding::Cbor.name(), "cbor");
}
