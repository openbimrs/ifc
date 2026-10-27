//! Undo and redo (#465).
//!
//! History is off until `openbim_ifc_v0_1_model_set_history_limit` gives it
//! a bound; then each mutating export is one undo entry. `undo` and `redo`
//! answer with a `HistoryChange` record tape: created (`LIST` of `REF`),
//! removed (`LIST` of `REF`), modified (`LIST` of `REF`), header (`BOOL`).
//!
//! The buffer protocol never applies a step it cannot report: the call
//! peeks first, and a size query or a short buffer answers `BufferTooSmall`
//! with the sizes and leaves the model unchanged. The applied change has the
//! peeked one's size.

use openbim_ifc_binding_core::{BindingError, HistoryChange, IfcModel, ToRecord};

use crate::buffer::put;
use crate::domains::Out;
use crate::model::{done, with_model, OpenbimIfcModel};
use crate::status::{boundary, OpenbimIfcStatus};
use crate::tape::{OpenbimIfcValueNode, Tape};

/// How much history a model holds (`HistoryState`).
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OpenbimIfcHistoryState {
    /// The bound on undo entries; 0 records nothing.
    pub limit: usize,
    /// Entries `openbim_ifc_v0_1_model_undo` can step back through.
    pub undo: usize,
    /// Entries `openbim_ifc_v0_1_model_redo` can step forward through.
    pub redo: usize,
}

/// Bound the undo entries a model keeps. 0 turns history off and drops what
/// is held; a lower limit drops the oldest entries. History is off (0) for a
/// new model.
#[no_mangle]
pub extern "C" fn openbim_ifc_v0_1_model_set_history_limit(
    model: OpenbimIfcModel,
    limit: usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        with_model(model, |m| {
            m.set_history_limit(limit);
            Ok(OpenbimIfcStatus::Ok)
        })
    })
}

/// Write the history bound and how many entries each direction holds.
///
/// # Safety
/// `out_state` must be null or valid for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_history(
    model: OpenbimIfcModel,
    out_state: *mut OpenbimIfcHistoryState,
) -> OpenbimIfcStatus {
    boundary(|| {
        if out_state.is_null() {
            return OpenbimIfcStatus::NullPointer;
        }
        with_model(model, |m| {
            let state = m.history();
            let state = OpenbimIfcHistoryState {
                limit: state.limit,
                undo: state.undo,
                redo: state.redo,
            };
            // SAFETY: checked non-null; caller contract above.
            done(match unsafe { put(out_state, state) } {
                Ok(()) => OpenbimIfcStatus::Ok,
                Err(status) => status,
            })
        })
    })
}

/// Drop every undo and redo entry, keeping the limit.
#[no_mangle]
pub extern "C" fn openbim_ifc_v0_1_model_clear_history(model: OpenbimIfcModel) -> OpenbimIfcStatus {
    boundary(|| {
        with_model(model, |m| {
            m.clear_history();
            Ok(OpenbimIfcStatus::Ok)
        })
    })
}

/// Peek with `peek`; then write the change, applying it with `apply` first
/// only when it fits.
///
/// # Safety
/// The buffers as for `Out`.
unsafe fn step_export(
    model: OpenbimIfcModel,
    out: &Out,
    peek: impl Fn(&IfcModel) -> Result<HistoryChange, BindingError>,
    apply: impl Fn(&mut IfcModel) -> Result<HistoryChange, BindingError>,
) -> OpenbimIfcStatus {
    boundary(|| {
        with_model(model, |m| {
            if let Err(status) = out.check() {
                return done(status);
            }
            let planned = peek(m)?;
            let tape = Tape::encode(&planned.to_record().to_tagged());
            let fits =
                tape.nodes.len() <= out.node_capacity && tape.strings.len() <= out.string_capacity;
            let change = if fits { apply(m)? } else { planned };
            // SAFETY: caller contract.
            unsafe { out.write(&change.to_record().to_tagged()) }
        })
    })
}

/// Restore the model to the state before the last recorded change: entities
/// under their ids and at their file positions, the header included. The
/// answer is a `HistoryChange` tape: created, removed and modified (`LIST`s
/// of `REF`) and header (`BOOL`). A size query or a short buffer peeks the
/// step and answers `BufferTooSmall` without applying it.
///
/// `HistoryEmpty` with nothing to undo (history off, or the stack empty);
/// `HistoryStale` when the model changed outside the history.
///
/// # Safety
/// As for `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_undo(
    model: OpenbimIfcModel,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    let out = Out {
        nodes,
        node_capacity,
        out_nodes_required,
        strings,
        string_capacity,
        out_strings_required,
    };
    // SAFETY: caller contract above.
    unsafe { step_export(model, &out, IfcModel::peek_undo, IfcModel::undo) }
}

/// Reverse the last undo. The answer and the buffer protocol are those of
/// `openbim_ifc_v0_1_model_undo`; `HistoryEmpty` with nothing to redo.
///
/// # Safety
/// As for `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_redo(
    model: OpenbimIfcModel,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    let out = Out {
        nodes,
        node_capacity,
        out_nodes_required,
        strings,
        string_capacity,
        out_strings_required,
    };
    // SAFETY: caller contract above.
    unsafe { step_export(model, &out, IfcModel::peek_redo, IfcModel::redo) }
}
