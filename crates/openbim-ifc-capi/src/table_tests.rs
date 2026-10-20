//! Boundary tests for `export_table` and `import_table` (#474), called
//! exactly as C would call them over the selector fixtures. The answers
//! are pinned in `openbim-ifc-binding-core`; these check the calling
//! convention, the record tape, and that a size query writes nothing.

use std::ptr::null_mut;

use openbim_ifc_binding_core::value::Tagged;

use crate::capability_tests::{last_code, parse, tape};
use crate::*;

const RELEASES: [&str; 3] = [
    "selector_ifc2x3.ifc",
    "selector_ifc4.ifc",
    "selector_ifc4x3.ifc",
];

fn fixture(name: &str) -> OpenbimIfcModel {
    let path = format!(
        "{}/../../test/fixtures/synthetic-relations/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    parse(&std::fs::read(path).expect("fixture"))
}

fn export(
    model: OpenbimIfcModel,
    query: &str,
    columns: &[&str],
    format: u32,
    key: u32,
) -> Result<Vec<u8>, OpenbimIfcStatus> {
    let pointers: Vec<*const u8> = columns.iter().map(|column| column.as_ptr()).collect();
    let lengths: Vec<usize> = columns.iter().map(|column| column.len()).collect();
    let call = |buffer: *mut u8, capacity: usize, required: &mut usize| {
        // SAFETY: every slice outlives the call; the buffer is as sized.
        unsafe {
            openbim_ifc_v0_1_model_export_table(
                model,
                query.as_ptr(),
                query.len(),
                pointers.as_ptr(),
                lengths.as_ptr(),
                columns.len(),
                format,
                key,
                b"".as_ptr(),
                0,
                buffer,
                capacity,
                required,
            )
        }
    };
    let mut required = 0;
    let status = call(null_mut(), 0, &mut required);
    if status != OpenbimIfcStatus::BufferTooSmall {
        return Err(status);
    }
    let mut buffer = vec![0u8; required];
    let status = call(buffer.as_mut_ptr(), buffer.len(), &mut required);
    assert_eq!(status, OpenbimIfcStatus::Ok);
    Ok(buffer)
}

fn import(model: OpenbimIfcModel, data: &[u8], flags: u32) -> Result<Tagged, OpenbimIfcStatus> {
    // SAFETY (closure): `data` outlives the call; forwards the buffers.
    tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_model_import_table(
            model,
            data.as_ptr(),
            data.len(),
            OPENBIM_IFC_TABLE_CSV,
            OPENBIM_IFC_TABLE_KEY_AUTO,
            b"".as_ptr(),
            0,
            flags,
            n,
            nc,
            nr,
            s,
            sc,
            sr,
        )
    })
}

fn written(model: OpenbimIfcModel) -> Vec<u8> {
    let mut required = 0;
    // SAFETY: size query, then a buffer of that size.
    unsafe { openbim_ifc_v0_1_model_write(model, null_mut(), 0, &mut required) };
    let mut buffer = vec![0u8; required];
    // SAFETY: as above.
    let status = unsafe {
        openbim_ifc_v0_1_model_write(model, buffer.as_mut_ptr(), buffer.len(), &mut required)
    };
    assert_eq!(status, OpenbimIfcStatus::Ok);
    buffer
}

/// The `applied`, change count and refusal codes of a `TableImport` tape.
fn summary(report: &Tagged) -> (bool, usize, Vec<String>) {
    let Tagged::List(fields) = report else {
        panic!("{report:?}")
    };
    let (Tagged::Bool(applied), Tagged::List(changes), Tagged::List(refusals)) =
        (&fields[0], &fields[1], &fields[2])
    else {
        panic!("{report:?}")
    };
    let codes = refusals
        .iter()
        .map(|refusal| match refusal {
            Tagged::List(fields) => match &fields[2] {
                Tagged::Text(code) => code.clone(),
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        })
        .collect();
    (*applied, changes.len(), codes)
}

#[test]
fn a_table_exports_and_imports_back_through_the_abi() {
    for name in RELEASES {
        let model = fixture(name);
        let csv = export(
            model,
            "IfcWall",
            &["Name", "type.Name"],
            OPENBIM_IFC_TABLE_CSV,
            OPENBIM_IFC_TABLE_KEY_ID,
        )
        .unwrap();
        let text = String::from_utf8(csv.clone()).unwrap();
        assert!(
            text.starts_with("id,Name,type.Name\r\n17,Wall A,WT-200\r\n"),
            "{text}"
        );
        let report = import(model, &csv, 0).unwrap();
        assert_eq!(summary(&report), (false, 0, Vec::new()), "{name}: a no-op");

        let edited = text.replace("17,Wall A,", "17,Wall A1,");
        let before = written(model);
        let dry = import(model, edited.as_bytes(), OPENBIM_IFC_TABLE_DRY_RUN).unwrap();
        assert_eq!(summary(&dry), (false, 1, Vec::new()));
        assert_eq!(written(model), before, "a dry run writes nothing");
        let applied = import(model, edited.as_bytes(), 0).unwrap();
        assert_eq!(summary(&applied), (true, 1, Vec::new()));
        assert_ne!(written(model), before);

        // With a GlobalId column the rows are keyed by it, and both are
        // empty: refused as data, nothing written.
        let refused = import(model, b"id,Name,GlobalId\n17,X,\n99,Y,\n", 0).unwrap();
        assert_eq!(
            summary(&refused),
            (
                false,
                0,
                vec!["empty-key".to_owned(), "empty-key".to_owned()]
            )
        );
        openbim_ifc_v0_1_model_destroy(model);
    }
}

#[test]
fn bad_arguments_are_refused() {
    let model = fixture("selector_ifc4.ifc");
    assert_eq!(
        export(model, "IfcWall", &[], 7, OPENBIM_IFC_TABLE_KEY_AUTO),
        Err(OpenbimIfcStatus::InvalidArgument)
    );
    assert_eq!(
        export(
            model,
            "IfcWal",
            &[],
            OPENBIM_IFC_TABLE_CSV,
            OPENBIM_IFC_TABLE_KEY_AUTO
        ),
        Err(OpenbimIfcStatus::InvalidSelector)
    );
    assert_eq!(last_code(model), "invalid-selector");
    assert_eq!(
        import(model, b"id\n", 8),
        Err(OpenbimIfcStatus::InvalidArgument)
    );
    let xlsx = export(
        model,
        "IfcWall",
        &["Name"],
        OPENBIM_IFC_TABLE_XLSX,
        OPENBIM_IFC_TABLE_KEY_AUTO,
    )
    .unwrap();
    assert!(xlsx.starts_with(b"PK"));
    openbim_ifc_v0_1_model_destroy(model);
}
