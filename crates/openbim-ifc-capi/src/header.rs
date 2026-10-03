//! The STEP file header as a value tape (#244).
//!
//! The header crosses as one `LIST` of ten values, in the order a STEP
//! header writes them: description (`LIST` of `TEXT`), implementation
//! level, name, time stamp (`TEXT`), author, organization (`LIST` of
//! `TEXT`), preprocessor version, originating system, authorization
//! (`TEXT`), schema (`LIST` of `TEXT`). Reading and replacing use the same
//! shape, so a host edits a field and writes the tape back.

use openbim_ifc_binding_core::header;

use crate::buffer::{bytes, slice};
use crate::model::{done, fill_tape, with_model, OpenbimIfcModel};
use crate::status::{boundary, OpenbimIfcStatus};
use crate::tape::{OpenbimIfcValueNode, Reader, Tape};

/// The file header as a value tape: one `LIST` of ten values, in STEP
/// header order -- description (`LIST` of `TEXT`), implementation level,
/// name, time stamp, author (`LIST`), organization (`LIST`), preprocessor
/// version, originating system, authorization, schema (`LIST`).
///
/// # Safety
/// As for `openbim_ifc_v0_1_entity_attribute`: each buffer null with
/// capacity 0, or valid for its capacity; both `out_*_required` valid for
/// one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_header(
    model: OpenbimIfcModel,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        with_model(model, |m| {
            let tape = Tape::encode(&header::to_tagged(&m.header()));
            // SAFETY: caller contract above.
            done(unsafe {
                fill_tape(
                    &tape,
                    nodes,
                    node_capacity,
                    out_nodes_required,
                    strings,
                    string_capacity,
                    out_strings_required,
                )
            })
        })
    })
}

/// Replace the file header with the one on the tape. A tape of any other
/// shape is `InvalidValue` and leaves the header unchanged.
///
/// # Safety
/// As for `openbim_ifc_v0_1_entity_set_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_set_header(
    model: OpenbimIfcModel,
    nodes: *const OpenbimIfcValueNode,
    node_count: usize,
    strings: *const u8,
    string_len: usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        // SAFETY: caller contract above.
        let (nodes, strings) =
            match unsafe { (slice(nodes, node_count), bytes(strings, string_len)) } {
                (Ok(n), Ok(s)) => (n, s),
                (Err(status), _) | (_, Err(status)) => return status,
            };
        with_model(model, |m| {
            let value = Reader::new(nodes, strings).single()?;
            m.set_header(header::from_tagged(value)?);
            Ok(OpenbimIfcStatus::Ok)
        })
    })
}
