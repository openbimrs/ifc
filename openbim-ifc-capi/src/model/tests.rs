//! Boundary tests: the exports called exactly as C would call them.

use super::*;
use crate::errors::{openbim_ifc_v0_1_last_error_code, openbim_ifc_v0_1_last_error_message};
use crate::tape::{OPENBIM_IFC_KIND_LIST, OPENBIM_IFC_KIND_TEXT, OPENBIM_IFC_KIND_UNKNOWN};
use std::ptr::{null, null_mut};

const FILE: &[u8] = b"ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCWALL('0abc',$,'Wall',$,$,$,$,$,.STANDARD.);
#2=IFCPROPERTYSINGLEVALUE('Flag',$,IFCLOGICAL(.U.),$);
#3=IFCRELDEFINESBYPROPERTIES('0def',$,$,$,(#1),#9);
ENDSEC;
END-ISO-10303-21;
";

fn parse(bytes: &[u8]) -> OpenbimIfcModel {
    let mut model = 0;
    // SAFETY: valid slice and out-pointer; no error buffer.
    let status = unsafe {
        openbim_ifc_v0_1_model_parse(bytes.as_ptr(), bytes.len(), &mut model, null_mut(), 0)
    };
    assert_eq!(status, OpenbimIfcStatus::Ok);
    assert_ne!(model, 0, "zero is never a valid handle");
    model
}

/// Size query, then fetch: the two-call protocol every buffer export uses.
fn c_string(call: impl Fn(*mut u8, usize, *mut usize) -> OpenbimIfcStatus) -> String {
    let mut required = 0;
    assert_eq!(
        call(null_mut(), 0, &mut required),
        OpenbimIfcStatus::BufferTooSmall
    );
    let mut buffer = vec![0u8; required];
    assert_eq!(
        call(buffer.as_mut_ptr(), buffer.len(), &mut required),
        OpenbimIfcStatus::Ok
    );
    assert_eq!(buffer.pop(), Some(0), "NUL-terminated");
    String::from_utf8(buffer).unwrap()
}

fn attribute(model: OpenbimIfcModel, id: u64, index: usize) -> (Vec<OpenbimIfcValueNode>, Vec<u8>) {
    let (mut need_nodes, mut need_strings) = (0, 0);
    // SAFETY: size query with null buffers and zero capacities.
    let status = unsafe {
        openbim_ifc_v0_1_entity_attribute(
            model,
            id,
            index,
            null_mut(),
            0,
            &mut need_nodes,
            null_mut(),
            0,
            &mut need_strings,
        )
    };
    assert!(matches!(
        status,
        OpenbimIfcStatus::BufferTooSmall | OpenbimIfcStatus::Ok
    ));
    let mut nodes = vec![OpenbimIfcValueNode::default(); need_nodes];
    let mut strings = vec![0u8; need_strings];
    // SAFETY: buffers sized from the query.
    let status = unsafe {
        openbim_ifc_v0_1_entity_attribute(
            model,
            id,
            index,
            nodes.as_mut_ptr(),
            nodes.len(),
            &mut need_nodes,
            strings.as_mut_ptr(),
            strings.len(),
            &mut need_strings,
        )
    };
    assert_eq!(status, OpenbimIfcStatus::Ok);
    (nodes, strings)
}

#[test]
fn a_file_parses_and_reads_back_through_the_abi() {
    let model = parse(FILE);
    let mut count = 0;
    // SAFETY: valid out-pointer.
    assert_eq!(
        unsafe { openbim_ifc_v0_1_model_len(model, &mut count) },
        OpenbimIfcStatus::Ok
    );
    assert_eq!(count, 3);

    // SAFETY (closures): each forwards a buffer the helper sized.
    let schema = c_string(|b, c, r| unsafe { openbim_ifc_v0_1_model_schema(model, b, c, r) });
    assert_eq!(schema, "IFC4");
    let kind = c_string(|b, c, r| unsafe { openbim_ifc_v0_1_entity_type(model, 1, b, c, r) });
    assert_eq!(kind, "IFCWALL");

    let (nodes, strings) = attribute(model, 1, 2);
    assert_eq!(nodes[0].kind, OPENBIM_IFC_KIND_TEXT);
    assert_eq!(&strings[..], b"Wall");

    // IFCLOGICAL(.U.) stays unknown, not false.
    let (nodes, _) = attribute(model, 2, 2);
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[1].kind, OPENBIM_IFC_KIND_UNKNOWN);

    assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
}

#[test]
fn an_edit_and_an_added_entity_survive_writing() {
    let model = parse(FILE);
    let tape = Tape::encode(&openbim_ifc_binding_core::value::Tagged::Text(
        "Renamed".into(),
    ));
    // SAFETY: tape slices are valid for their lengths.
    let status = unsafe {
        openbim_ifc_v0_1_entity_set_attribute(
            model,
            1,
            2,
            tape.nodes.as_ptr(),
            tape.nodes.len(),
            tape.strings.as_ptr(),
            tape.strings.len(),
        )
    };
    assert_eq!(status, OpenbimIfcStatus::Ok);

    let point = Tape::encode(&openbim_ifc_binding_core::value::Tagged::List(vec![
        openbim_ifc_binding_core::value::Tagged::Real(1.0),
        openbim_ifc_binding_core::value::Tagged::Real(2.0),
    ]));
    let mut id = 0;
    let name = b"IfcCartesianPoint";
    // SAFETY: all slices valid for their lengths; `id` is writable.
    let status = unsafe {
        openbim_ifc_v0_1_entity_add(
            model,
            name.as_ptr(),
            name.len(),
            1,
            point.nodes.as_ptr(),
            point.nodes.len(),
            point.strings.as_ptr(),
            point.strings.len(),
            &mut id,
        )
    };
    assert_eq!(status, OpenbimIfcStatus::Ok);
    assert_eq!(id, 4);

    let mut required = 0;
    // SAFETY: size query.
    unsafe { openbim_ifc_v0_1_model_write(model, null_mut(), 0, &mut required) };
    let mut bytes = vec![0u8; required];
    // SAFETY: buffer sized from the query.
    let status = unsafe {
        openbim_ifc_v0_1_model_write(model, bytes.as_mut_ptr(), bytes.len(), &mut required)
    };
    assert_eq!(status, OpenbimIfcStatus::Ok);

    let reparsed = parse(&bytes);
    let (nodes, strings) = attribute(reparsed, 1, 2);
    assert_eq!(
        (nodes[0].kind, &strings[..]),
        (OPENBIM_IFC_KIND_TEXT, &b"Renamed"[..])
    );
    let (nodes, _) = attribute(reparsed, 4, 0);
    assert_eq!(
        (nodes[0].kind, nodes[0].child_count),
        (OPENBIM_IFC_KIND_LIST, 2)
    );
    assert_eq!((nodes[1].real_value, nodes[2].real_value), (1.0, 2.0));

    openbim_ifc_v0_1_model_destroy(model);
    openbim_ifc_v0_1_model_destroy(reparsed);
}

#[test]
fn misuse_is_a_status_never_a_crash() {
    let mut out = 0;
    // SAFETY: each call passes null or dangling-free arguments on purpose.
    unsafe {
        assert_eq!(
            openbim_ifc_v0_1_model_create(null_mut()),
            OpenbimIfcStatus::NullPointer
        );
        assert_eq!(
            openbim_ifc_v0_1_model_parse(null(), 5, &mut out, null_mut(), 0),
            OpenbimIfcStatus::NullPointer
        );
        assert_eq!(
            openbim_ifc_v0_1_model_len(0, &mut 0),
            OpenbimIfcStatus::InvalidHandle
        );
        assert_eq!(
            openbim_ifc_v0_1_model_len(u64::MAX, &mut 0),
            OpenbimIfcStatus::InvalidHandle
        );
    }
    let model = parse(FILE);
    assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
    assert_eq!(
        openbim_ifc_v0_1_model_destroy(model),
        OpenbimIfcStatus::InvalidHandle,
        "double destroy"
    );
    // SAFETY: stale handle, valid out-pointer.
    assert_eq!(
        unsafe { openbim_ifc_v0_1_model_len(model, &mut 0) },
        OpenbimIfcStatus::InvalidHandle
    );

    let model = parse(FILE);
    let mut required = 0;
    let mut tiny = [0u8; 2];
    // SAFETY: a real 2-byte buffer, honestly declared.
    let status = unsafe {
        openbim_ifc_v0_1_model_schema(model, tiny.as_mut_ptr(), tiny.len(), &mut required)
    };
    assert_eq!((status, required), (OpenbimIfcStatus::BufferTooSmall, 5));
    assert_eq!(tiny, [0, 0], "a short buffer is left untouched");
    // SAFETY: non-zero capacity with a null buffer is refused.
    let status = unsafe { openbim_ifc_v0_1_model_schema(model, null_mut(), 8, &mut required) };
    assert_eq!(status, OpenbimIfcStatus::NullPointer);
    openbim_ifc_v0_1_model_destroy(model);
}

#[test]
fn a_failed_call_leaves_its_code_and_message_until_the_next_success() {
    let model = parse(FILE);
    // SAFETY: valid out-pointer; entity 99 does not exist.
    assert_eq!(
        openbim_ifc_v0_1_entity_remove(model, 99),
        OpenbimIfcStatus::MissingEntity
    );
    // SAFETY (closures): buffers sized by the helper.
    let code = c_string(|b, c, r| unsafe { openbim_ifc_v0_1_last_error_code(model, b, c, r) });
    assert_eq!(code, "missing-entity");
    let message =
        c_string(|b, c, r| unsafe { openbim_ifc_v0_1_last_error_message(model, b, c, r) });
    assert_eq!(message, "no entity #99");

    // SAFETY: valid out-pointer.
    assert_eq!(
        unsafe { openbim_ifc_v0_1_model_len(model, &mut 0) },
        OpenbimIfcStatus::Ok
    );
    let mut required = 0;
    // SAFETY: size query.
    let status = unsafe { openbim_ifc_v0_1_last_error_code(model, null_mut(), 0, &mut required) };
    assert_eq!(
        status,
        OpenbimIfcStatus::NoValue,
        "a success clears the error"
    );
    openbim_ifc_v0_1_model_destroy(model);
}

#[test]
fn a_parse_error_is_reported_without_a_model() {
    let mut model = 7;
    let mut message = [0xAAu8; 16];
    let bad = b"not a step file";
    // SAFETY: valid slices and out-pointer.
    let status = unsafe {
        openbim_ifc_v0_1_model_parse(
            bad.as_ptr(),
            bad.len(),
            &mut model,
            message.as_mut_ptr(),
            message.len(),
        )
    };
    assert_eq!(status, OpenbimIfcStatus::Parse);
    assert_eq!(model, 7, "no handle is written on failure");
    let end = message
        .iter()
        .position(|b| *b == 0)
        .expect("NUL within capacity");
    assert!(end <= 15);
    assert!(std::str::from_utf8(&message[..end])
        .unwrap()
        .starts_with("cannot parse"));
}

#[test]
fn a_malformed_input_tape_does_not_change_the_model() {
    let model = parse(FILE);
    let bad = [OpenbimIfcValueNode {
        kind: 42,
        ..OpenbimIfcValueNode::default()
    }];
    // SAFETY: a valid one-node slice; no strings.
    let status =
        unsafe { openbim_ifc_v0_1_entity_set_attribute(model, 1, 2, bad.as_ptr(), 1, null(), 0) };
    assert_eq!(status, OpenbimIfcStatus::InvalidValue);
    let (_, strings) = attribute(model, 1, 2);
    assert_eq!(&strings[..], b"Wall");
    openbim_ifc_v0_1_model_destroy(model);
}

#[test]
fn ids_types_and_dangling_references_use_the_size_protocol() {
    let model = parse(FILE);
    let mut required = 0;
    let mut ids = [0u64; 8];
    // SAFETY: a real 8-element buffer.
    let status =
        unsafe { openbim_ifc_v0_1_model_ids(model, ids.as_mut_ptr(), ids.len(), &mut required) };
    assert_eq!(
        (status, &ids[..required]),
        (OpenbimIfcStatus::Ok, &[1, 2, 3][..])
    );

    let name = b"ifcwall";
    // SAFETY: valid name slice and buffer.
    let status = unsafe {
        openbim_ifc_v0_1_model_ids_of_type(
            model,
            name.as_ptr(),
            name.len(),
            ids.as_mut_ptr(),
            ids.len(),
            &mut required,
        )
    };
    assert_eq!((status, &ids[..required]), (OpenbimIfcStatus::Ok, &[1][..]));

    let root = b"IfcBuildingElement";
    // SAFETY: valid name slice and buffer.
    let status = unsafe {
        openbim_ifc_v0_1_model_ids_of_type_including_subtypes(
            model,
            root.as_ptr(),
            root.len(),
            ids.as_mut_ptr(),
            ids.len(),
            &mut required,
        )
    };
    assert_eq!((status, &ids[..required]), (OpenbimIfcStatus::Ok, &[1][..]));

    // SAFETY: valid buffer; #3 references a missing #9.
    let status = unsafe {
        openbim_ifc_v0_1_model_dangling_references(
            model,
            ids.as_mut_ptr(),
            ids.len(),
            &mut required,
        )
    };
    assert_eq!(
        (status, &ids[..required]),
        (OpenbimIfcStatus::Ok, &[3, 9][..])
    );

    let invalid_utf8 = [0xFFu8];
    // SAFETY: valid one-byte slice, not UTF-8.
    let status = unsafe {
        openbim_ifc_v0_1_model_ids_of_type(
            model,
            invalid_utf8.as_ptr(),
            1,
            ids.as_mut_ptr(),
            ids.len(),
            &mut required,
        )
    };
    assert_eq!(status, OpenbimIfcStatus::InvalidArgument);
    openbim_ifc_v0_1_model_destroy(model);
}

#[test]
fn the_version_reports_abi_and_crate_separately() {
    let mut version = OpenbimIfcVersion::default();
    // SAFETY: valid out-pointer.
    assert_eq!(
        unsafe { openbim_ifc_v0_1_version(&mut version) },
        OpenbimIfcStatus::Ok
    );
    assert_eq!((version.abi_major, version.abi_minor), (0, 1));
    assert_eq!(
        version.crate_minor,
        env!("CARGO_PKG_VERSION_MINOR").parse::<u16>().unwrap()
    );
}

#[test]
fn destroyed_models_are_released() {
    let before = {
        let mut n = 0;
        // SAFETY: valid out-pointer.
        unsafe { openbim_ifc_v0_1_live_models(&mut n) };
        n
    };
    let models: Vec<_> = (0..5).map(|_| parse(FILE)).collect();
    for model in models {
        assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
    }
    let mut after = 0;
    // SAFETY: valid out-pointer.
    unsafe { openbim_ifc_v0_1_live_models(&mut after) };
    // Other tests run in parallel, so only this test's models are asserted:
    // the count must not have grown by the five just destroyed.
    assert!(after < before + 5, "before {before}, after {after}");
}
