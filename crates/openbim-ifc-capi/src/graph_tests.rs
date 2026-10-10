//! Boundary tests for the graph exports (#367), called exactly as C would
//! call them: the record tape is the core's records, each payload the
//! core's bytes, and every payload reads back through Axiolid's own reader.

use std::ptr::{null, null_mut};

use crate::capability_tests::parse;
use crate::*;

fn fixture() -> Vec<u8> {
    std::fs::read(format!(
        "{}/../../test/fixtures/synthetic-bindings/binding_geometry.ifc",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture")
}

#[test]
fn an_unknown_encoding_is_an_invalid_argument() {
    let model = parse(&fixture());
    let mut graphs = 0;
    // SAFETY: valid out-pointer; null ids select every product.
    let status =
        unsafe { openbim_ifc_v0_1_model_product_geometry(model, null(), 0, 2, &mut graphs) };
    assert_eq!(status, OpenbimIfcStatus::InvalidArgument);
    assert_eq!(graphs, 0, "no handle written");
    assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
}

#[cfg(feature = "graph")]
mod enabled {
    use ifc::geometry::GeometryGraph;
    use openbim_ifc_binding_core::value::Tagged;
    use openbim_ifc_binding_core::{GeometryEncoding, IfcModel, ToRecord};

    use super::*;
    use crate::capability_tests::tape;

    /// Encode every product's graph as `encoding` through the ABI, and
    /// return the record tape and each payload; the set is destroyed after
    /// the model, which it outlives.
    fn read(encoding: u32) -> (usize, Tagged, Vec<Vec<u8>>) {
        let model = parse(&fixture());
        let mut graphs = 0;
        // SAFETY: valid out-pointer; null ids select every product.
        let status = unsafe {
            openbim_ifc_v0_1_model_product_geometry(model, null(), 0, encoding, &mut graphs)
        };
        assert_eq!(status, OpenbimIfcStatus::Ok);
        assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);

        let mut count = 0;
        let count_out: *mut usize = &mut count;
        // SAFETY (closure): the helper's buffers; `count` outlives every call.
        let records = tape(|n, nc, nr, s, sc, sr| unsafe {
            openbim_ifc_v0_1_graphs_records(graphs, count_out, n, nc, nr, s, sc, sr)
        })
        .unwrap();
        let payloads = (0..count)
            .map(|index| {
                let mut need = 0;
                // SAFETY: the size query, then a buffer of the size reported.
                unsafe {
                    let _ =
                        openbim_ifc_v0_1_graphs_payload(graphs, index, null_mut(), 0, &mut need);
                    let mut buffer = vec![0u8; need];
                    let status = openbim_ifc_v0_1_graphs_payload(
                        graphs,
                        index,
                        buffer.as_mut_ptr(),
                        buffer.len(),
                        &mut need,
                    );
                    assert_eq!(status, OpenbimIfcStatus::Ok);
                    buffer
                }
            })
            .collect();
        let mut need = 0;
        // SAFETY: valid out-pointer, no buffer.
        let status =
            unsafe { openbim_ifc_v0_1_graphs_payload(graphs, count, null_mut(), 0, &mut need) };
        assert_eq!(status, OpenbimIfcStatus::OutOfRange);
        assert_eq!(
            openbim_ifc_v0_1_graphs_destroy(graphs),
            OpenbimIfcStatus::Ok
        );
        assert_eq!(
            openbim_ifc_v0_1_graphs_destroy(graphs),
            OpenbimIfcStatus::InvalidHandle
        );
        (count, records, payloads)
    }

    #[test]
    fn a_graph_set_hands_out_the_core_payloads_and_they_read_back() {
        let core = IfcModel::parse(&fixture())
            .unwrap()
            .product_geometry(None, GeometryEncoding::Json)
            .unwrap();
        let (count, records, payloads) = read(OPENBIM_IFC_GEOMETRY_JSON);
        assert_eq!(count, 4);
        assert_eq!(
            records,
            Tagged::List(core.iter().map(|g| g.to_record().to_tagged()).collect())
        );
        for (payload, graph) in payloads.iter().zip(&core) {
            assert_eq!(*payload, graph.payload.clone().unwrap_or_default());
        }

        let wall = std::str::from_utf8(&payloads[0]).expect("UTF-8 JSON");
        let envelope: serde_json::Value = serde_json::from_str(wall).unwrap();
        assert_eq!(envelope["format"], "axiolid-geometry-graph");
        assert_eq!(envelope["version"], "1.0");
        let back = GeometryGraph::from_json(wall).expect("Axiolid reads it back");
        assert_eq!(back.to_json().unwrap(), wall, "re-encodes identically");

        assert!(payloads[2].is_empty(), "an axis-only product has no graph");
        assert!(payloads[3].is_empty(), "the text literal is refused");
        assert_eq!(core[3].refusal.as_ref().unwrap().code, "unsupported");
    }

    #[test]
    fn cbor_payloads_read_the_same_graphs() {
        let (_, _, json) = read(OPENBIM_IFC_GEOMETRY_JSON);
        let (_, records, cbor) = read(OPENBIM_IFC_GEOMETRY_CBOR);
        let Tagged::List(records) = records else {
            panic!("a list");
        };
        assert_eq!(records.len(), 4);
        for (cbor, json) in cbor.iter().zip(&json).take(2) {
            let back = GeometryGraph::from_cbor(cbor).expect("Axiolid reads it back");
            assert_eq!(back.to_json().unwrap().as_bytes(), json.as_slice());
        }
    }
}

#[cfg(not(feature = "graph"))]
#[test]
fn graphs_refuse_without_the_feature() {
    use crate::capability_tests::last_code;

    let model = parse(&fixture());
    let mut graphs = 0;
    // SAFETY: valid out-pointer.
    let status = unsafe {
        openbim_ifc_v0_1_model_product_geometry(
            model,
            null(),
            0,
            OPENBIM_IFC_GEOMETRY_JSON,
            &mut graphs,
        )
    };
    assert_eq!(status, OpenbimIfcStatus::FeatureDisabled);
    assert_eq!(last_code(model), "feature-disabled");
    assert_eq!(graphs, 0, "no handle written");
    assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
}
