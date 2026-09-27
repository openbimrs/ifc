//! Opening a model from a file path.
//!
//! A strict read keeps the file as the model's source and decodes entities
//! on access. Reading the file here, straight into that source, saves the
//! host a read into its own buffer plus the copy `model_parse` makes of it.

use std::path::Path;

use openbim_ifc_binding_core::IfcModel;

use crate::buffer::text;
use crate::model::{finish_load, OpenbimIfcModel};
use crate::status::{boundary, OpenbimIfcStatus};

/// Read the STEP file at `path` (UTF-8, `path_len` bytes, no NUL needed)
/// into a model that owns its bytes, and write the new model's handle.
///
/// Errors as `openbim_ifc_v0_1_model_parse`, plus `Io` when the file cannot
/// be opened or read.
///
/// # Safety
/// `path` must be valid for `path_len` reads; `out_model` for one write;
/// `error_buffer`, if non-null, for `capacity` writes.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_open(
    path: *const u8,
    path_len: usize,
    out_model: *mut OpenbimIfcModel,
    error_buffer: *mut u8,
    capacity: usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        if out_model.is_null() || (capacity != 0 && error_buffer.is_null()) {
            return OpenbimIfcStatus::NullPointer;
        }
        // SAFETY: caller contract above.
        let path = match unsafe { text(path, path_len) } {
            Ok(path) => path,
            Err(status) => return status,
        };
        let loaded = IfcModel::open(Path::new(path));
        // SAFETY: checked above; caller contract above.
        unsafe { finish_load(loaded, out_model, error_buffer, capacity) }
    })
}

/// Read the STEP file at `path` through a memory mapping, and write the new
/// model's handle. No copy of the file is made, and its pages belong to the
/// page cache rather than the process heap.
///
/// # Safety
/// As `openbim_ifc_v0_1_model_open`, and additionally: the file must not
/// be modified or truncated until the model is destroyed. The model decodes
/// entities from the mapping on access; a changed file makes that fail
/// (reported as `Panic`), end the process (`SIGBUS` on truncation), or read
/// other content.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_open_mapped(
    path: *const u8,
    path_len: usize,
    out_model: *mut OpenbimIfcModel,
    error_buffer: *mut u8,
    capacity: usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        if out_model.is_null() || (capacity != 0 && error_buffer.is_null()) {
            return OpenbimIfcStatus::NullPointer;
        }
        // SAFETY: caller contract above.
        let path = match unsafe { text(path, path_len) } {
            Ok(path) => path,
            Err(status) => return status,
        };
        // SAFETY: the caller guarantees the file stays unchanged while the
        // model lives.
        let loaded = unsafe { IfcModel::open_mapped(Path::new(path)) };
        // SAFETY: checked above; caller contract above.
        unsafe { finish_load(loaded, out_model, error_buffer, capacity) }
    })
}
