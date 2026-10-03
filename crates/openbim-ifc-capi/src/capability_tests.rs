//! Boundary tests for the #244 exports, called exactly as C would call them.

use std::ptr::{null, null_mut};

use openbim_ifc_binding_core::header;
use openbim_ifc_binding_core::value::Tagged;

use crate::errors::openbim_ifc_v0_1_last_error_code;
use crate::tape::{OpenbimIfcValueNode, Reader, Tape};
use crate::*;

const FILE: &[u8] = b"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('c.ifc','2026-10-03T00:00:00',('Ann'),('Org'),'pre','sys','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCWALL('0abc',$,'Wall',$,$,$,$,$,.STANDARD.);
#2=IFCRELDEFINESBYPROPERTIES('0def',$,$,$,(#1),#9);
ENDSEC;
END-ISO-10303-21;
";

fn damaged() -> Vec<u8> {
    let mut text = String::from_utf8(FILE.to_vec()).unwrap();
    text = text.replace("ENDSEC;\nEND", "#3=IFCWALL('x',,;\nENDSEC;\nEND");
    text.into_bytes()
}

fn parse_flags(bytes: &[u8], flags: u32) -> (OpenbimIfcStatus, OpenbimIfcModel, String) {
    let mut model = 0;
    let mut error = vec![0u8; 256];
    // SAFETY: valid slice, out-pointer and error buffer.
    let status = unsafe {
        openbim_ifc_v0_1_model_parse_with_options(
            bytes.as_ptr(),
            bytes.len(),
            flags,
            &mut model,
            error.as_mut_ptr(),
            error.len(),
        )
    };
    let end = error.iter().position(|b| *b == 0).unwrap_or(0);
    (
        status,
        model,
        String::from_utf8_lossy(&error[..end]).into_owned(),
    )
}

fn parse(bytes: &[u8]) -> OpenbimIfcModel {
    let (status, model, error) = parse_flags(bytes, 0);
    assert_eq!(status, OpenbimIfcStatus::Ok, "{error}");
    model
}

/// Size query, then fetch, for any tape export; returns the decoded value.
fn tape(
    call: impl Fn(
        *mut OpenbimIfcValueNode,
        usize,
        *mut usize,
        *mut u8,
        usize,
        *mut usize,
    ) -> OpenbimIfcStatus,
) -> Result<Tagged, OpenbimIfcStatus> {
    let (mut need_nodes, mut need_strings) = (0, 0);
    let status = call(
        null_mut(),
        0,
        &mut need_nodes,
        null_mut(),
        0,
        &mut need_strings,
    );
    if !matches!(
        status,
        OpenbimIfcStatus::BufferTooSmall | OpenbimIfcStatus::Ok
    ) {
        return Err(status);
    }
    let mut nodes = vec![OpenbimIfcValueNode::default(); need_nodes];
    let mut strings = vec![0u8; need_strings];
    let status = call(
        nodes.as_mut_ptr(),
        nodes.len(),
        &mut need_nodes,
        strings.as_mut_ptr(),
        strings.len(),
        &mut need_strings,
    );
    assert_eq!(status, OpenbimIfcStatus::Ok);
    Ok(Reader::new(&nodes, &strings)
        .single()
        .expect("a well-formed tape"))
}

fn last_code(model: OpenbimIfcModel) -> String {
    let mut buffer = vec![0u8; 64];
    let mut need = 0;
    // SAFETY: buffer valid for its length.
    let status = unsafe {
        openbim_ifc_v0_1_last_error_code(model, buffer.as_mut_ptr(), buffer.len(), &mut need)
    };
    assert_eq!(status, OpenbimIfcStatus::Ok);
    String::from_utf8(buffer[..need - 1].to_vec()).unwrap()
}

#[test]
fn a_lenient_read_recovers_and_unknown_flags_are_refused() {
    let damaged = damaged();
    let (status, _, _) = parse_flags(&damaged, 0);
    assert_eq!(status, OpenbimIfcStatus::Parse);

    let (status, model, _) = parse_flags(&damaged, OPENBIM_IFC_PARSE_LENIENT);
    assert_eq!(status, OpenbimIfcStatus::Ok);
    let mut count = 0;
    // SAFETY: valid out-pointer.
    unsafe { openbim_ifc_v0_1_model_diagnostic_count(model, &mut count) };
    assert_eq!(count, 1, "the skipped record is reported");
    assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);

    let (status, _, message) = parse_flags(FILE, 1 << 7);
    assert_eq!(status, OpenbimIfcStatus::InvalidValue);
    assert!(message.contains("flag"), "{message}");
}

#[test]
fn a_lenient_open_reads_from_disk() {
    let dir = std::env::temp_dir().join(format!("capi-244-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("damaged.ifc");
    std::fs::write(&path, damaged()).unwrap();
    let path = path.to_str().unwrap();
    for open in [
        openbim_ifc_v0_1_model_open_with_options,
        openbim_ifc_v0_1_model_open_mapped_with_options,
    ] {
        let mut model = 0;
        // SAFETY: valid path slice and out-pointer; the file stays put
        // while the mapped model lives.
        let status = unsafe {
            open(
                path.as_ptr(),
                path.len(),
                OPENBIM_IFC_PARSE_SKIP_MALFORMED,
                &mut model,
                null_mut(),
                0,
            )
        };
        assert_eq!(status, OpenbimIfcStatus::Ok);
        assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_header_reads_as_a_tape_and_a_replacement_is_written() {
    let model = parse(FILE);
    // SAFETY (closure): forwards the buffers the helper sized.
    let read = || {
        tape(|n, nc, nr, s, sc, sr| unsafe {
            openbim_ifc_v0_1_model_header(model, n, nc, nr, s, sc, sr)
        })
        .unwrap()
    };
    let mut fields = header::from_tagged(read()).unwrap();
    assert_eq!(fields.name, "c.ifc");
    assert_eq!(fields.author, ["Ann"]);
    assert_eq!(fields.schema, ["IFC4"]);

    fields.name = "edited.ifc".into();
    let encoded = Tape::encode(&header::to_tagged(&fields));
    // SAFETY: tape slices are valid for their lengths.
    let status = unsafe {
        openbim_ifc_v0_1_model_set_header(
            model,
            encoded.nodes.as_ptr(),
            encoded.nodes.len(),
            encoded.strings.as_ptr(),
            encoded.strings.len(),
        )
    };
    assert_eq!(status, OpenbimIfcStatus::Ok);
    assert_eq!(header::from_tagged(read()).unwrap(), fields);

    let wrong = Tape::encode(&Tagged::Text("x".into()));
    // SAFETY: tape slices are valid for their lengths.
    let status = unsafe {
        openbim_ifc_v0_1_model_set_header(
            model,
            wrong.nodes.as_ptr(),
            wrong.nodes.len(),
            wrong.strings.as_ptr(),
            wrong.strings.len(),
        )
    };
    assert_eq!(status, OpenbimIfcStatus::InvalidValue);
    assert_eq!(last_code(model), "invalid-value");
    assert_eq!(header::from_tagged(read()).unwrap(), fields, "unchanged");
    assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
}

#[test]
fn validation_crosses_as_a_summary_and_a_tape_of_findings() {
    let model = parse(FILE);
    let mut summary = OpenbimIfcValidationSummary::default();
    let summary_out: *mut OpenbimIfcValidationSummary = &mut summary;
    // SAFETY (closure): forwards the buffers the helper sized; `summary`
    // outlives every call.
    let findings = tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_model_validate(model, 0, summary_out, n, nc, nr, s, sc, sr)
    })
    .unwrap();
    let Tagged::List(rows) = findings else {
        panic!("a list");
    };
    assert_eq!(rows.len(), summary.finding_count);
    assert_eq!(summary.conformant, 0, "#2 references the missing #9");
    assert!(summary.errors >= 1);
    assert_eq!(summary.truncated, 0);
    let on_two = rows.iter().any(|row| {
        matches!(row, Tagged::List(fields)
            if fields.len() == 7
                && fields[0] == Tagged::Text("error".into())
                && fields[2] == Tagged::Ref(2))
    });
    assert!(on_two, "{rows:?}");

    // SAFETY: a null summary pointer is refused before anything is read.
    let status = unsafe {
        openbim_ifc_v0_1_model_validate(
            model,
            0,
            null_mut(),
            null_mut(),
            0,
            null_mut(),
            null_mut(),
            0,
            null_mut(),
        )
    };
    assert_eq!(status, OpenbimIfcStatus::NullPointer);
    assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
}

#[test]
fn ifcxml_round_trips_and_an_unknown_profile_is_refused() {
    let model = parse(FILE);
    let mut need = 0;
    // SAFETY: size query with a null buffer and zero capacity.
    let status =
        unsafe { openbim_ifc_v0_1_model_write_ifcxml(model, null(), 0, null_mut(), 0, &mut need) };
    assert_eq!(status, OpenbimIfcStatus::BufferTooSmall);
    let mut xml = vec![0u8; need];
    // SAFETY: buffer sized from the query.
    let status = unsafe {
        openbim_ifc_v0_1_model_write_ifcxml(
            model,
            null(),
            0,
            xml.as_mut_ptr(),
            xml.len(),
            &mut need,
        )
    };
    assert_eq!(status, OpenbimIfcStatus::Ok);

    let mut back = 0;
    // SAFETY: valid slices and out-pointer.
    let status = unsafe {
        openbim_ifc_v0_1_model_parse_ifcxml(
            xml.as_ptr(),
            xml.len(),
            null(),
            0,
            &mut back,
            null_mut(),
            0,
        )
    };
    assert_eq!(status, OpenbimIfcStatus::Ok);
    let mut count = 0;
    // SAFETY: valid out-pointer.
    unsafe { openbim_ifc_v0_1_model_len(back, &mut count) };
    assert_eq!(count, 2);

    let profile = "IFC2X3";
    // SAFETY: valid profile slice; size query.
    let status = unsafe {
        openbim_ifc_v0_1_model_write_ifcxml(
            model,
            profile.as_ptr(),
            profile.len(),
            null_mut(),
            0,
            &mut need,
        )
    };
    assert_eq!(status, OpenbimIfcStatus::UnsupportedProfile);
    assert_eq!(last_code(model), "unsupported-profile");
    for handle in [model, back] {
        assert_eq!(openbim_ifc_v0_1_model_destroy(handle), OpenbimIfcStatus::Ok);
    }
}

/// A wall with Body geometry that no spatial structure contains.
const UNCONTAINED: &[u8] = b"ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('u.ifc','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#10=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,$,$);
#20=IFCSHAPEREPRESENTATION(#10,'Body','SweptSolid',());
#21=IFCPRODUCTDEFINITIONSHAPE($,$,(#20));
#30=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,$,#21,$,.STANDARD.);
ENDSEC;
END-ISO-10303-21;
";

#[test]
fn unreachable_products_cross_as_a_tape() {
    let model = parse(UNCONTAINED);
    let mut count = usize::MAX;
    let count_out: *mut usize = &mut count;
    // SAFETY (closure): forwards the buffers the helper sized; `count`
    // outlives every call.
    let products = tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_model_unreachable_products(model, count_out, n, nc, nr, s, sc, sr)
    })
    .unwrap();
    assert_eq!(count, 1);
    let Tagged::List(rows) = products else {
        panic!("a list");
    };
    let Tagged::List(fields) = &rows[0] else {
        panic!("a product record");
    };
    assert_eq!(fields[0], Tagged::Ref(30));
    assert_eq!(
        fields[1],
        Tagged::Text("not-contained-in-spatial-structure".into())
    );
    assert_eq!(fields[2], Tagged::List(Vec::new()));
    assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
}
