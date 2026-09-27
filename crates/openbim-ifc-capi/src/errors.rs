//! Last-error access, per model.

use crate::buffer::fill_c_string;
use crate::model::OpenbimIfcModel;
use crate::registry::{self, lock};
use crate::status::{boundary, OpenbimIfcStatus};

/// The last error on `model` as its stable code (`parse`, `missing-entity`,
/// ...), NUL-terminated; `NoValue` if the last call succeeded.
///
/// # Safety
/// `buffer` must be null (with `capacity` 0) or valid for `capacity`
/// writes; `out_required` valid for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_last_error_code(
    model: OpenbimIfcModel,
    buffer: *mut u8,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| {
        last_error(
            model,
            |e| e.code().to_owned(),
            buffer,
            capacity,
            out_required,
        )
    })
}

/// The last error on `model` as a human message, NUL-terminated; `NoValue`
/// if the last call succeeded.
///
/// # Safety
/// As for [`openbim_ifc_v0_1_last_error_code`].
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_last_error_message(
    model: OpenbimIfcModel,
    buffer: *mut u8,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| last_error(model, ToString::to_string, buffer, capacity, out_required))
}

fn last_error(
    model: OpenbimIfcModel,
    render: impl FnOnce(&openbim_ifc_binding_core::BindingError) -> String,
    buffer: *mut u8,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    let Some(shared) = registry::get(model) else {
        return OpenbimIfcStatus::InvalidHandle;
    };
    // Read without going through `with_model`, which would clear the error.
    let entry = lock(&shared);
    match &entry.last_error {
        None => OpenbimIfcStatus::NoValue,
        // SAFETY: the exported callers pass their own caller contract here.
        Some(error) => unsafe { fill_c_string(&render(error), buffer, capacity, out_required) },
    }
}
