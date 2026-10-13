//! Boundary tests for the `GlobalId` and relation exports (#463), called
//! exactly as C would call them, over the IFC2X3, IFC4 and IFC4X3 relation
//! fixtures. The answers are pinned in `openbim-ifc-binding-core`; these
//! check the calling convention.

use std::ptr::null_mut;

use openbim_ifc_binding_core::value::Tagged;

use crate::capability_tests::{last_code, parse, tape};
use crate::*;

fn fixture(name: &str) -> OpenbimIfcModel {
    parse(
        &std::fs::read(format!(
            "{}/../../test/fixtures/synthetic-relations/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("fixture"),
    )
}

const RELEASES: [&str; 3] = [
    "relations_ifc2x3.ifc",
    "relations_ifc4.ifc",
    "relations_ifc4x3.ifc",
];

/// Size query, then fetch, for an id-list export.
fn ids(call: impl Fn(*mut u64, usize, *mut usize) -> OpenbimIfcStatus) -> Vec<u64> {
    let mut need = 0;
    let status = call(null_mut(), 0, &mut need);
    assert!(matches!(
        status,
        OpenbimIfcStatus::Ok | OpenbimIfcStatus::BufferTooSmall
    ));
    let mut buffer = vec![0u64; need];
    assert_eq!(
        call(buffer.as_mut_ptr(), buffer.len(), &mut need),
        OpenbimIfcStatus::Ok
    );
    buffer
}

/// An optional-id export's answer.
fn optional(call: impl Fn(*mut u64) -> OpenbimIfcStatus) -> Result<Option<u64>, OpenbimIfcStatus> {
    let mut id = 0;
    match call(&mut id) {
        OpenbimIfcStatus::Ok => Ok(Some(id)),
        OpenbimIfcStatus::NoValue => Ok(None),
        status => Err(status),
    }
}

/// Size query, then fetch, for a C-string export.
fn string(call: impl Fn(*mut u8, usize, *mut usize) -> OpenbimIfcStatus) -> String {
    let mut need = 0;
    assert_eq!(
        call(null_mut(), 0, &mut need),
        OpenbimIfcStatus::BufferTooSmall
    );
    let mut buffer = vec![0u8; need];
    assert_eq!(
        call(buffer.as_mut_ptr(), buffer.len(), &mut need),
        OpenbimIfcStatus::Ok
    );
    String::from_utf8(buffer[..need - 1].to_vec()).unwrap()
}

fn by_guid(model: OpenbimIfcModel, guid: &str) -> Result<Option<u64>, OpenbimIfcStatus> {
    // SAFETY (closure): `guid` outlives the call; `out` is a valid pointer.
    optional(|out| unsafe { openbim_ifc_v0_1_model_by_guid(model, guid.as_ptr(), guid.len(), out) })
}

fn global_id(model: OpenbimIfcModel, id: u64) -> String {
    // SAFETY (closure): forwards the buffers the helper sized.
    let value = tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_entity_attribute(model, id, 0, n, nc, nr, s, sc, sr)
    })
    .unwrap();
    match value {
        Tagged::Text(text) => text,
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_entity_is_found_by_its_global_id_in_every_release() {
    for name in RELEASES {
        let model = fixture(name);
        let guid = global_id(model, 17);
        assert_eq!(by_guid(model, &guid), Ok(Some(17)), "{name}");
        assert_eq!(by_guid(model, "3MtNd0Uaz8Ix2OPtRq_3Kk"), Ok(None), "{name}");
        assert_eq!(by_guid(model, "nope"), Err(OpenbimIfcStatus::InvalidValue));
        assert_eq!(last_code(model), "invalid-value");
        // SAFETY: a null out-pointer is refused before it is used.
        let status =
            unsafe { openbim_ifc_v0_1_model_by_guid(model, guid.as_ptr(), guid.len(), null_mut()) };
        assert_eq!(status, OpenbimIfcStatus::NullPointer);
    }
}

#[test]
fn a_copied_global_id_is_refused_and_listed() {
    let model = fixture("relations_ifc4.ifc");
    let guid = global_id(model, 17);
    // Copy Wall A through the C value tape, GlobalId and all.
    let mut nodes = vec![OpenbimIfcValueNode::default(); 64];
    let mut strings = vec![0u8; 256];
    let (mut count, mut need_nodes, mut need_strings) = (0, 0, 0);
    // SAFETY: buffers valid for their lengths.
    let status = unsafe {
        openbim_ifc_v0_1_entity_attributes(
            model,
            17,
            &mut count,
            nodes.as_mut_ptr(),
            nodes.len(),
            &mut need_nodes,
            strings.as_mut_ptr(),
            strings.len(),
            &mut need_strings,
        )
    };
    assert_eq!(status, OpenbimIfcStatus::Ok);
    let type_name = "IFCWALL";
    let mut copy = 0;
    // SAFETY: the tape this crate just wrote; `copy` valid for one write.
    let status = unsafe {
        openbim_ifc_v0_1_entity_add(
            model,
            type_name.as_ptr(),
            type_name.len(),
            count,
            nodes.as_ptr(),
            need_nodes,
            strings.as_ptr(),
            need_strings,
            &mut copy,
        )
    };
    assert_eq!(status, OpenbimIfcStatus::Ok);
    assert_eq!(by_guid(model, &guid), Err(OpenbimIfcStatus::InvalidModel));
    assert_eq!(last_code(model), "invalid-model");

    let mut duplicates = 0;
    let count_out: *mut usize = &mut duplicates;
    // SAFETY (closure): forwards the buffers the helper sized; `duplicates`
    // outlives every call.
    let listed = tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_model_duplicate_global_ids(model, count_out, n, nc, nr, s, sc, sr)
    })
    .unwrap();
    assert_eq!(duplicates, 1);
    assert_eq!(
        listed,
        Tagged::List(vec![Tagged::List(vec![
            Tagged::Text(guid),
            Tagged::List(vec![Tagged::Ref(17), Tagged::Ref(copy)]),
        ])])
    );
}

#[test]
fn types_openings_and_fillings_cross_in_every_release() {
    for name in RELEASES {
        let model = fixture(name);
        // SAFETY (closures): each forwards a valid out-pointer or the
        // buffers the helper sized.
        unsafe {
            assert_eq!(
                optional(|out| openbim_ifc_v0_1_entity_type_object(model, 17, out)),
                Ok(Some(16))
            );
            assert_eq!(
                optional(|out| openbim_ifc_v0_1_entity_type_object(model, 19, out)),
                Ok(None)
            );
            assert_eq!(
                ids(|b, c, r| openbim_ifc_v0_1_entity_occurrences(model, 16, b, c, r)),
                [17, 18]
            );
            assert_eq!(
                ids(|b, c, r| openbim_ifc_v0_1_entity_openings(model, 17, b, c, r)),
                [21, 23]
            );
            assert_eq!(
                ids(|b, c, r| openbim_ifc_v0_1_entity_fillings(model, 21, b, c, r)),
                [28]
            );
            assert_eq!(
                ids(|b, c, r| openbim_ifc_v0_1_entity_fillings(model, 23, b, c, r)),
                Vec::<u64>::new()
            );
            assert_eq!(
                optional(|out| openbim_ifc_v0_1_entity_voided_element(model, 22, out)),
                Ok(Some(18))
            );
            assert_eq!(
                optional(|out| openbim_ifc_v0_1_entity_type_object(model, 8, out)),
                Err(OpenbimIfcStatus::WrongEntityType),
                "{name}"
            );
            let mut need = 0;
            assert_eq!(
                openbim_ifc_v0_1_entity_openings(model, 999, null_mut(), 0, &mut need),
                OpenbimIfcStatus::MissingEntity
            );
        }
        assert_eq!(last_code(model), "missing-entity");
    }
}

#[test]
fn new_and_name_based_global_ids_need_no_model() {
    // SAFETY (closure): forwards the buffers the helper sized.
    let drawn: std::collections::HashSet<String> = (0..100)
        .map(|_| string(|b, c, r| unsafe { openbim_ifc_v0_1_new_guid(b, c, r) }))
        .collect();
    assert_eq!(drawn.len(), 100);
    assert!(drawn.iter().all(|guid| guid.len() == 22));

    let parts = ["openbim-ifc", "example"];
    let pointers: Vec<*const u8> = parts.iter().map(|part| part.as_ptr()).collect();
    let lengths: Vec<usize> = parts.iter().map(|part| part.len()).collect();
    // SAFETY (closure): `pointers` and `lengths` hold `parts.len()` items,
    // each part valid for its length.
    let named = string(|b, c, r| unsafe {
        openbim_ifc_v0_1_name_based_guid(pointers.as_ptr(), lengths.as_ptr(), parts.len(), b, c, r)
    });
    // Pinned in the facade: every binding derives the same GlobalId.
    assert_eq!(named, "347eFU5Y69bPY9V9tumQuV");
    // SAFETY: null arrays with a non-zero count are refused before use.
    let status = unsafe {
        openbim_ifc_v0_1_name_based_guid(
            std::ptr::null(),
            std::ptr::null(),
            1,
            null_mut(),
            0,
            null_mut(),
        )
    };
    assert_eq!(status, OpenbimIfcStatus::NullPointer);
}
