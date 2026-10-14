//! Boundary tests for the selector exports (#464), called exactly as C
//! would call them, over the IFC2X3, IFC4 and IFC4X3 selector fixtures.
//! The answers are pinned in `openbim-ifc-binding-core`; these check the
//! calling convention.

use std::ptr::null_mut;

use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::SELECTOR_GRAMMAR_VERSION;

use crate::capability_tests::{last_code, parse, tape};
use crate::tape::OPENBIM_IFC_KIND_TEXT;
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
    "selector_ifc2x3.ifc",
    "selector_ifc4.ifc",
    "selector_ifc4x3.ifc",
];

/// Size query, then fetch.
fn select(
    model: OpenbimIfcModel,
    query: &str,
    base: Option<&[u64]>,
) -> Result<Vec<u64>, OpenbimIfcStatus> {
    let call = |buffer: *mut u64, capacity: usize, need: &mut usize| match base {
        // SAFETY: every buffer is valid for its length.
        None => unsafe {
            openbim_ifc_v0_1_model_select(
                model,
                query.as_ptr(),
                query.len(),
                buffer,
                capacity,
                need,
            )
        },
        // SAFETY: every buffer is valid for its length.
        Some(base) => unsafe {
            openbim_ifc_v0_1_model_select_from(
                model,
                query.as_ptr(),
                query.len(),
                base.as_ptr(),
                base.len(),
                buffer,
                capacity,
                need,
            )
        },
    };
    let mut need = 0;
    match call(null_mut(), 0, &mut need) {
        OpenbimIfcStatus::Ok | OpenbimIfcStatus::BufferTooSmall => {}
        status => return Err(status),
    }
    let mut buffer = vec![0u64; need];
    assert_eq!(
        call(buffer.as_mut_ptr(), buffer.len(), &mut need),
        OpenbimIfcStatus::Ok
    );
    Ok(buffer)
}

fn get(model: OpenbimIfcModel, id: u64, path: &str) -> Result<Tagged, OpenbimIfcStatus> {
    tape(
        |nodes, node_capacity, need_nodes, strings, string_capacity, need_strings| {
            // SAFETY: every buffer is valid for its length.
            unsafe {
                openbim_ifc_v0_1_entity_get_value(
                    model,
                    id,
                    path.as_ptr(),
                    path.len(),
                    nodes,
                    node_capacity,
                    need_nodes,
                    strings,
                    string_capacity,
                    need_strings,
                )
            }
        },
    )
}

fn set_text(model: OpenbimIfcModel, id: u64, path: &str, value: &str) -> OpenbimIfcStatus {
    let node = OpenbimIfcValueNode {
        kind: OPENBIM_IFC_KIND_TEXT,
        str_len: value.len() as u64,
        ..OpenbimIfcValueNode::default()
    };
    // SAFETY: every buffer is valid for its length.
    unsafe {
        openbim_ifc_v0_1_entity_set_value(
            model,
            id,
            path.as_ptr(),
            path.len(),
            &node,
            1,
            value.as_ptr(),
            value.len(),
        )
    }
}

fn span(model: OpenbimIfcModel) -> Result<(usize, usize), OpenbimIfcStatus> {
    let (mut start, mut end) = (0, 0);
    // SAFETY: both outputs are valid for one write.
    match unsafe { openbim_ifc_v0_1_last_error_span(model, &mut start, &mut end) } {
        OpenbimIfcStatus::Ok => Ok((start, end)),
        status => Err(status),
    }
}

#[test]
fn selectors_fill_the_caller_s_buffer() {
    assert_eq!(
        format!("{OPENBIM_IFC_SELECTOR_GRAMMAR_MAJOR}.{OPENBIM_IFC_SELECTOR_GRAMMAR_MINOR}"),
        SELECTOR_GRAMMAR_VERSION
    );
    for name in RELEASES {
        let model = fixture(name);
        assert_eq!(
            select(model, "IfcWall, type=WT-200 + IfcDoor", None),
            Ok(vec![17, 18, 28])
        );
        assert_eq!(select(model, "IfcWall", Some(&[17, 28])), Ok(vec![17]));
        assert_eq!(select(model, "IfcWall, Name=x", None), Ok(vec![]));
    }
}

#[test]
fn a_query_that_does_not_parse_reports_its_span() {
    let model = fixture("selector_ifc4.ifc");
    assert_eq!(
        select(model, "IfcWall, colour=red", None),
        Err(OpenbimIfcStatus::InvalidSelector)
    );
    assert_eq!(last_code(model), "invalid-selector");
    assert_eq!(span(model), Ok((9, 15)));
    assert_eq!(select(model, "IfcWall", None), Ok(vec![17, 18, 19]));
    assert_eq!(
        span(model),
        Err(OpenbimIfcStatus::NoValue),
        "a success clears it"
    );
    assert_eq!(
        get(model, 999, "Name"),
        Err(OpenbimIfcStatus::MissingEntity)
    );
    assert_eq!(
        span(model),
        Err(OpenbimIfcStatus::NoValue),
        "no span on other errors"
    );
}

#[test]
fn values_are_read_and_written_by_path() {
    for name in RELEASES {
        let model = fixture(name);
        assert_eq!(
            get(model, 17, "type.Name"),
            Ok(Tagged::Text("WT-200".into()))
        );
        assert_eq!(
            get(model, 17, "Pset_WallCommon"),
            Ok(Tagged::List(vec![
                Tagged::List(vec![Tagged::Text("IsExternal".into()), Tagged::Bool(true)]),
                Tagged::List(vec![
                    Tagged::Text("FireRating".into()),
                    Tagged::Text("F90".into())
                ]),
                Tagged::List(vec![
                    Tagged::Text("AcousticRating".into()),
                    Tagged::Text("R45".into())
                ]),
            ]))
        );
        assert_eq!(
            set_text(model, 17, "type.Name", "WT-300"),
            OpenbimIfcStatus::Ok
        );
        assert_eq!(
            get(model, 18, "type.Name"),
            Ok(Tagged::Text("WT-300".into()))
        );
        assert_eq!(
            set_text(model, 17, "count", "x"),
            OpenbimIfcStatus::InvalidValue
        );
        assert_eq!(
            set_text(model, 17, "Flavour", "x"),
            OpenbimIfcStatus::UnknownAttribute
        );
    }
}
