//! Reads under explicit STEP parse options (#244).
//!
//! The options cross as flag bits rather than a struct, so a later option
//! is a new bit, not a new struct layout. An unknown bit is refused with
//! `InvalidValue`, the same code the other hosts give a malformed options
//! object, and the message reaches the optional error buffer.

use std::path::Path;

use openbim_ifc_binding_core::{BindingError, IfcModel, OnMalformed, ParseOptions};

use crate::buffer::{bytes, text};
use crate::model::{finish_load, OpenbimIfcModel};
use crate::status::{boundary, OpenbimIfcStatus};

/// Skip a data record that cannot be parsed and report it as a diagnostic,
/// instead of failing the read.
pub const OPENBIM_IFC_PARSE_SKIP_MALFORMED: u32 = 1;
/// Report duplicate instance ids and references to undefined ids as
/// diagnostics. Nothing is dropped.
pub const OPENBIM_IFC_PARSE_CHECK_REFERENCES: u32 = 2;
/// Read a real written without its decimal point (`1E-05`) as that real,
/// with a diagnostic.
pub const OPENBIM_IFC_PARSE_ACCEPT_REAL_WITHOUT_POINT: u32 = 4;
/// The lenient preset: `SKIP_MALFORMED | ACCEPT_REAL_WITHOUT_POINT`.
pub const OPENBIM_IFC_PARSE_LENIENT: u32 = 5;

const KNOWN: u32 = OPENBIM_IFC_PARSE_SKIP_MALFORMED
    | OPENBIM_IFC_PARSE_CHECK_REFERENCES
    | OPENBIM_IFC_PARSE_ACCEPT_REAL_WITHOUT_POINT;

/// Options from flag bits; an unknown bit is `invalid-value`.
pub(crate) fn options(flags: u32) -> Result<ParseOptions, BindingError> {
    if flags & !KNOWN != 0 {
        return Err(BindingError::InvalidValue(format!(
            "unknown parse flag bits {:#x}",
            flags & !KNOWN
        )));
    }
    Ok(ParseOptions {
        on_malformed: if flags & OPENBIM_IFC_PARSE_SKIP_MALFORMED != 0 {
            OnMalformed::Skip
        } else {
            OnMalformed::Abort
        },
        check_references: flags & OPENBIM_IFC_PARSE_CHECK_REFERENCES != 0,
        accept_real_without_point: flags & OPENBIM_IFC_PARSE_ACCEPT_REAL_WITHOUT_POINT != 0,
    })
}

/// As `openbim_ifc_v0_1_model_parse`, under the `OPENBIM_IFC_PARSE_*`
/// `flags` (0 is the strict read). What a lenient read recovers from is
/// listed by `openbim_ifc_v0_1_model_diagnostic`.
///
/// # Safety
/// As `openbim_ifc_v0_1_model_parse`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_parse_with_options(
    data: *const u8,
    len: usize,
    flags: u32,
    out_model: *mut OpenbimIfcModel,
    error_buffer: *mut u8,
    capacity: usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        if out_model.is_null() || (capacity != 0 && error_buffer.is_null()) {
            return OpenbimIfcStatus::NullPointer;
        }
        // SAFETY: caller contract above.
        let input = match unsafe { bytes(data, len) } {
            Ok(input) => input,
            Err(status) => return status,
        };
        let loaded = options(flags).and_then(|options| IfcModel::parse_with(input, options));
        // SAFETY: checked above; caller contract above.
        unsafe { finish_load(loaded, out_model, error_buffer, capacity) }
    })
}

/// As `openbim_ifc_v0_1_model_open`, under the `OPENBIM_IFC_PARSE_*`
/// `flags`.
///
/// # Safety
/// As `openbim_ifc_v0_1_model_open`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_open_with_options(
    path: *const u8,
    path_len: usize,
    flags: u32,
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
        let loaded =
            options(flags).and_then(|options| IfcModel::open_with(Path::new(path), options));
        // SAFETY: checked above; caller contract above.
        unsafe { finish_load(loaded, out_model, error_buffer, capacity) }
    })
}

/// As `openbim_ifc_v0_1_model_open_mapped`, under the
/// `OPENBIM_IFC_PARSE_*` `flags`.
///
/// # Safety
/// As `openbim_ifc_v0_1_model_open_mapped`: the file must not be modified
/// or truncated until the model is destroyed.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_open_mapped_with_options(
    path: *const u8,
    path_len: usize,
    flags: u32,
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
        let loaded = options(flags).and_then(|options| {
            // SAFETY: the caller guarantees the file stays unchanged while
            // the model lives.
            unsafe { IfcModel::open_mapped_with(Path::new(path), options) }
        });
        // SAFETY: checked above; caller contract above.
        unsafe { finish_load(loaded, out_model, error_buffer, capacity) }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_bits_map_to_the_options_and_unknown_bits_are_refused() {
        assert_eq!(options(0), Ok(ParseOptions::strict()));
        assert_eq!(
            options(OPENBIM_IFC_PARSE_LENIENT),
            Ok(ParseOptions::lenient())
        );
        assert_eq!(
            OPENBIM_IFC_PARSE_LENIENT,
            OPENBIM_IFC_PARSE_SKIP_MALFORMED | OPENBIM_IFC_PARSE_ACCEPT_REAL_WITHOUT_POINT
        );
        assert!(
            options(OPENBIM_IFC_PARSE_CHECK_REFERENCES)
                .unwrap()
                .check_references
        );
        assert!(matches!(options(8), Err(BindingError::InvalidValue(_))));
    }
}
