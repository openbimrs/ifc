//! Boundary tests for undo and redo (#465), called exactly as C would call
//! them. The answers are pinned in `openbim-ifc-binding-core`; these check
//! the calling convention: statuses, the history state, and that a size
//! query or a short buffer applies nothing.

use std::ptr::null_mut;

use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{IfcModel, ToRecord};

use crate::capability_tests::{last_code, parse, tape};
use crate::*;

fn fixture() -> OpenbimIfcModel {
    parse(
        &std::fs::read(format!(
            "{}/../../test/fixtures/synthetic-relations/cascade_ifc4.ifc",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("fixture"),
    )
}

fn bytes(model: OpenbimIfcModel) -> Vec<u8> {
    let mut need = 0;
    // SAFETY: size query with a null buffer.
    unsafe { openbim_ifc_v0_1_model_write(model, null_mut(), 0, &mut need) };
    let mut out = vec![0u8; need];
    // SAFETY: buffer valid for `need` writes.
    let status = unsafe { openbim_ifc_v0_1_model_write(model, out.as_mut_ptr(), need, &mut need) };
    assert_eq!(status, OpenbimIfcStatus::Ok);
    out
}

fn state(model: OpenbimIfcModel) -> OpenbimIfcHistoryState {
    let mut state = OpenbimIfcHistoryState::default();
    // SAFETY: valid out-pointer.
    let status = unsafe { openbim_ifc_v0_1_model_history(model, &mut state) };
    assert_eq!(status, OpenbimIfcStatus::Ok);
    state
}

fn undo(model: OpenbimIfcModel) -> Result<Tagged, OpenbimIfcStatus> {
    // SAFETY (closure): forwards the buffers the helper sized.
    tape(|n, nc, nr, s, sc, sr| unsafe { openbim_ifc_v0_1_model_undo(model, n, nc, nr, s, sc, sr) })
}

fn redo(model: OpenbimIfcModel) -> Result<Tagged, OpenbimIfcStatus> {
    // SAFETY (closure): forwards the buffers the helper sized.
    tape(|n, nc, nr, s, sc, sr| unsafe { openbim_ifc_v0_1_model_redo(model, n, nc, nr, s, sc, sr) })
}

#[test]
fn undo_and_redo_cross_as_the_core_record() {
    let model = fixture();
    assert_eq!(state(model), OpenbimIfcHistoryState::default());
    assert_eq!(undo(model), Err(OpenbimIfcStatus::HistoryEmpty));
    assert_eq!(OpenbimIfcStatus::HistoryEmpty as i32, 38);
    assert_eq!(last_code(model), "history-empty");

    assert_eq!(
        openbim_ifc_v0_1_model_set_history_limit(model, 4),
        OpenbimIfcStatus::Ok
    );
    let before = bytes(model);
    assert_eq!(
        openbim_ifc_v0_1_entity_remove(model, 80),
        OpenbimIfcStatus::Ok
    );
    let after = bytes(model);
    assert_ne!(before, after);
    assert_eq!(
        state(model),
        OpenbimIfcHistoryState {
            limit: 4,
            undo: 1,
            redo: 0
        }
    );

    // What the core answers for the same change.
    let mut core = IfcModel::parse(&before).expect("parses");
    core.set_history_limit(4);
    core.remove(80).unwrap();
    let undone = core.undo().unwrap().to_record().to_tagged();
    let redone = core.redo().unwrap().to_record().to_tagged();

    assert_eq!(undo(model), Ok(undone));
    assert_eq!(bytes(model), before);
    assert_eq!((state(model).undo, state(model).redo), (0, 1));
    assert_eq!(redo(model), Ok(redone));
    assert_eq!(bytes(model), after);
    assert_eq!(redo(model), Err(OpenbimIfcStatus::HistoryEmpty));
}

#[test]
fn a_size_query_or_a_short_buffer_applies_nothing() {
    let model = fixture();
    openbim_ifc_v0_1_model_set_history_limit(model, 4);
    let before = bytes(model);
    openbim_ifc_v0_1_entity_remove(model, 80);
    let after = bytes(model);

    let (mut nodes, mut strings) = (0, 0);
    // SAFETY: the size query passes null buffers with zero capacity.
    let status = unsafe {
        openbim_ifc_v0_1_model_undo(
            model,
            null_mut(),
            0,
            &mut nodes,
            null_mut(),
            0,
            &mut strings,
        )
    };
    assert_eq!(status, OpenbimIfcStatus::BufferTooSmall);
    assert!(nodes > 0);
    assert_eq!(bytes(model), after, "a size query applies nothing");
    assert_eq!(state(model).undo, 1);

    let mut node = [crate::tape::OpenbimIfcValueNode::default(); 1];
    // SAFETY: one node of capacity; string buffer absent with zero capacity.
    let status = unsafe {
        openbim_ifc_v0_1_model_undo(
            model,
            node.as_mut_ptr(),
            1,
            &mut nodes,
            null_mut(),
            0,
            &mut strings,
        )
    };
    assert_eq!(status, OpenbimIfcStatus::BufferTooSmall);
    assert_eq!(bytes(model), after, "a short buffer applies nothing");

    assert!(undo(model).is_ok());
    assert_eq!(bytes(model), before);
}

#[test]
fn clear_and_a_null_state_pointer() {
    let model = fixture();
    openbim_ifc_v0_1_model_set_history_limit(model, 4);
    openbim_ifc_v0_1_entity_remove(model, 80);
    assert_eq!(
        openbim_ifc_v0_1_model_clear_history(model),
        OpenbimIfcStatus::Ok
    );
    assert_eq!((state(model).limit, state(model).undo), (4, 0));
    // SAFETY: a null out-pointer is refused before anything is written.
    let status = unsafe { openbim_ifc_v0_1_model_history(model, null_mut()) };
    assert_eq!(status, OpenbimIfcStatus::NullPointer);
}

#[test]
fn a_null_output_pointer_is_refused_before_anything_is_applied() {
    let model = fixture();
    openbim_ifc_v0_1_model_set_history_limit(model, 4);
    let before = bytes(model);
    openbim_ifc_v0_1_entity_remove(model, 80);
    let after = bytes(model);
    let mut node = [crate::tape::OpenbimIfcValueNode::default(); 64];
    let mut string = [0u8; 256];
    let (mut nodes, mut strings) = (0usize, 0usize);
    for (required_nodes, required_strings, node_buffer, string_buffer) in [
        (
            null_mut(),
            &mut strings as *mut usize,
            node.as_mut_ptr(),
            string.as_mut_ptr(),
        ),
        (
            &mut nodes as *mut usize,
            null_mut(),
            node.as_mut_ptr(),
            string.as_mut_ptr(),
        ),
        (
            &mut nodes as *mut usize,
            &mut strings as *mut usize,
            null_mut(),
            string.as_mut_ptr(),
        ),
        (
            &mut nodes as *mut usize,
            &mut strings as *mut usize,
            node.as_mut_ptr(),
            null_mut(),
        ),
    ] {
        // SAFETY: the null pointers are the case under test; the capacities
        // describe the buffers that are passed.
        let status = unsafe {
            openbim_ifc_v0_1_model_undo(
                model,
                node_buffer,
                64,
                required_nodes,
                string_buffer,
                256,
                required_strings,
            )
        };
        assert_eq!(status, OpenbimIfcStatus::NullPointer);
        assert_eq!(bytes(model), after, "a refused call applies nothing");
        assert_eq!((state(model).undo, state(model).redo), (1, 0));
    }
    assert!(undo(model).is_ok(), "the retry undoes once");
    assert_eq!(bytes(model), before);
}
