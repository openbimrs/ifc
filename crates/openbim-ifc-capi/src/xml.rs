//! The ifcXML codec (#244).
//!
//! A null `profile` with length 0 selects the library's own lossless
//! layout; `IFC4` or `IFC4X3_ADD2` selects the buildingSMART XSD
//! configuration of that release.

use openbim_ifc_binding_core::IfcModel;

use crate::buffer::{bytes, fill, text};
use crate::model::{done, finish_load, with_model, OpenbimIfcModel};
use crate::status::{boundary, OpenbimIfcStatus};

/// The profile argument: `None` for the native layout.
///
/// # Safety
/// As `buffer::text`.
unsafe fn profile<'a>(
    profile: *const u8,
    profile_len: usize,
) -> Result<Option<&'a str>, OpenbimIfcStatus> {
    if profile.is_null() && profile_len == 0 {
        return Ok(None);
    }
    // SAFETY: forwarded caller contract.
    unsafe { text(profile, profile_len) }.map(Some)
}

/// Parse `len` bytes of ifcXML and write the new model's handle.
///
/// `profile` (UTF-8, `profile_len` bytes) is null with length 0 for the
/// native layout, else `IFC4` or `IFC4X3_ADD2` (any case) for that
/// release's XSD configuration. `UnsupportedProfile` for any other name,
/// `UnsupportedSchema` when the release is not bundled, `Parse` when the
/// document does not read, `FeatureDisabled` without the `ifcxml` feature.
/// The message goes to the optional `error_buffer` as for
/// `openbim_ifc_v0_1_model_parse`.
///
/// # Safety
/// As `openbim_ifc_v0_1_model_parse`; `profile`, if non-null, valid for
/// `profile_len` reads.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_parse_ifcxml(
    data: *const u8,
    len: usize,
    profile: *const u8,
    profile_len: usize,
    out_model: *mut OpenbimIfcModel,
    error_buffer: *mut u8,
    capacity: usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        if out_model.is_null() || (capacity != 0 && error_buffer.is_null()) {
            return OpenbimIfcStatus::NullPointer;
        }
        // SAFETY: caller contract above.
        let inputs = unsafe { (bytes(data, len), self::profile(profile, profile_len)) };
        let (input, profile) = match inputs {
            (Ok(input), Ok(profile)) => (input, profile),
            (Err(status), _) | (_, Err(status)) => return status,
        };
        let loaded = IfcModel::parse_ifcxml(input, profile);
        // SAFETY: checked above; caller contract above.
        unsafe { finish_load(loaded, out_model, error_buffer, capacity) }
    })
}

/// Serialize as ifcXML into a caller buffer; `out_required` gets the size.
/// `profile` as for `openbim_ifc_v0_1_model_parse_ifcxml`. An XSD-layout
/// write needs the header to declare the profile's schema, and refuses
/// with `Write` what the configuration cannot carry.
///
/// # Safety
/// As `openbim_ifc_v0_1_model_write`; `profile`, if non-null, valid for
/// `profile_len` reads.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_write_ifcxml(
    model: OpenbimIfcModel,
    profile: *const u8,
    profile_len: usize,
    buffer: *mut u8,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        // SAFETY: caller contract above.
        let profile = match unsafe { self::profile(profile, profile_len) } {
            Ok(profile) => profile,
            Err(status) => return status,
        };
        with_model(model, |m| {
            let bytes = m.write_ifcxml(profile)?;
            // SAFETY: caller contract above.
            done(unsafe { fill(&bytes, buffer, capacity, out_required) })
        })
    })
}
