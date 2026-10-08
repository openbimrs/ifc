//! Boundary tests for the geometry exports (#328), called exactly as C
//! would call them: the placement tape is the core's records, and a mesh
//! set hands out the core's arrays, refusals typed per product.

use std::ptr::{null, null_mut};

use openbim_ifc_binding_core::record::to_records;
use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{IfcModel, Record};

use crate::capability_tests::{parse, tape};
use crate::*;

fn fixture() -> Vec<u8> {
    std::fs::read(format!(
        "{}/../../test/fixtures/synthetic-bindings/binding_geometry.ifc",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture")
}

fn list(records: Vec<Record>) -> Tagged {
    Tagged::List(records.iter().map(Record::to_tagged).collect())
}

#[test]
fn placements_cross_as_the_core_records() {
    let bytes = fixture();
    let core = IfcModel::parse(&bytes).unwrap();
    let model = parse(&bytes);
    let mut count = usize::MAX;
    let count_out: *mut usize = &mut count;
    // SAFETY (closure): null ids with count 0 select every product; the
    // buffers are the helper's; `count` outlives every call.
    let all = tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_model_product_placements(model, null(), 0, count_out, n, nc, nr, s, sc, sr)
    })
    .unwrap();
    assert_eq!(count, 4);
    assert_eq!(
        all,
        list(to_records(&core.product_placements(None).unwrap()))
    );

    let ids = [65u64];
    // SAFETY (closure): `ids` holds one id and outlives every call.
    let one = tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_model_product_placements(
            model,
            ids.as_ptr(),
            ids.len(),
            count_out,
            n,
            nc,
            nr,
            s,
            sc,
            sr,
        )
    })
    .unwrap();
    assert_eq!(count, 1);
    assert_eq!(
        one,
        list(to_records(&core.product_placements(Some(&ids)).unwrap()))
    );

    // A count with no ids is a misuse, not "every product".
    // SAFETY: the out-pointer is valid; no buffer is written.
    let status = unsafe {
        openbim_ifc_v0_1_model_product_placements(
            model,
            null(),
            2,
            count_out,
            null_mut(),
            0,
            &mut 0,
            null_mut(),
            0,
            &mut 0,
        )
    };
    assert_eq!(status, OpenbimIfcStatus::NullPointer);
    assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
}

#[cfg(feature = "mesh")]
#[test]
fn a_mesh_set_hands_out_the_core_arrays() {
    use openbim_ifc_binding_core::ToRecord;

    let bytes = fixture();
    let core = IfcModel::parse(&bytes)
        .unwrap()
        .product_meshes(None)
        .unwrap();
    let model = parse(&bytes);
    let mut meshes = 0;
    // SAFETY: valid out-pointer; null ids select every product.
    let status = unsafe { openbim_ifc_v0_1_model_product_meshes(model, null(), 0, &mut meshes) };
    assert_eq!(status, OpenbimIfcStatus::Ok);
    // The set owns its data: it outlives the model.
    assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);

    let mut count = 0;
    let count_out: *mut usize = &mut count;
    // SAFETY (closure): the helper's buffers; `count` outlives every call.
    let records = tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_meshes_records(meshes, count_out, n, nc, nr, s, sc, sr)
    })
    .unwrap();
    assert_eq!(count, core.len());
    assert_eq!(
        records,
        Tagged::List(core.iter().map(|m| m.to_record().to_tagged()).collect())
    );

    for (index, mesh) in core.iter().enumerate() {
        let mut need = 0;
        // SAFETY: the size query, then a buffer of the size it reported.
        let positions = unsafe {
            let _ = openbim_ifc_v0_1_meshes_positions(meshes, index, null_mut(), 0, &mut need);
            let mut buffer = vec![0f32; need];
            let status = openbim_ifc_v0_1_meshes_positions(
                meshes,
                index,
                buffer.as_mut_ptr(),
                buffer.len(),
                &mut need,
            );
            assert_eq!(status, OpenbimIfcStatus::Ok);
            buffer
        };
        assert_eq!(positions, mesh.positions);
        // SAFETY: as above.
        let indices = unsafe {
            let _ = openbim_ifc_v0_1_meshes_indices(meshes, index, null_mut(), 0, &mut need);
            let mut buffer = vec![0u32; need];
            let status = openbim_ifc_v0_1_meshes_indices(
                meshes,
                index,
                buffer.as_mut_ptr(),
                buffer.len(),
                &mut need,
            );
            assert_eq!(status, OpenbimIfcStatus::Ok);
            buffer
        };
        assert_eq!(indices, mesh.indices);
    }
    assert!(!core[0].indices.is_empty(), "the wall meshes");
    assert_eq!(core[3].refusal.as_ref().unwrap().code, "unsupported");

    let mut need = 0;
    // SAFETY: valid out-pointer, no buffer.
    let status =
        unsafe { openbim_ifc_v0_1_meshes_positions(meshes, core.len(), null_mut(), 0, &mut need) };
    assert_eq!(status, OpenbimIfcStatus::OutOfRange);
    assert_eq!(
        openbim_ifc_v0_1_meshes_destroy(meshes),
        OpenbimIfcStatus::Ok
    );
    assert_eq!(
        openbim_ifc_v0_1_meshes_destroy(meshes),
        OpenbimIfcStatus::InvalidHandle
    );
}

#[cfg(not(feature = "mesh"))]
#[test]
fn meshes_refuse_without_the_feature() {
    use crate::capability_tests::last_code;

    let model = parse(&fixture());
    let mut meshes = 0;
    // SAFETY: valid out-pointer.
    let status = unsafe { openbim_ifc_v0_1_model_product_meshes(model, null(), 0, &mut meshes) };
    assert_eq!(status, OpenbimIfcStatus::FeatureDisabled);
    assert_eq!(last_code(model), "feature-disabled");
    assert_eq!(meshes, 0, "no handle written");
    assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
}
