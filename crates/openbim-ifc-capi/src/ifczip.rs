//! ifcZIP (#450): a STEP or ifcXML model in a ZIP archive.
//!
//! Reading needs no call of its own: `openbim_ifc_v0_1_model_parse` and
//! `openbim_ifc_v0_1_model_open` recognize an archive by its ZIP signature.

use openbim_ifc_binding_core::IfcModel;

use crate::buffer::fill;
use crate::model::{done, with_model, OpenbimIfcModel};
use crate::status::{boundary, OpenbimIfcStatus};
use crate::xml::profile;

/// Serialize as an ifcZIP archive into a caller buffer; `out_required` gets
/// the size.
///
/// `inner` (UTF-8, `inner_len` bytes) is null with length 0 or `step` for
/// STEP inside (`model.ifc`), or `ifcxml` (any case) for ifcXML
/// (`model.ifcXML`) in the layout `profile` names as for
/// `openbim_ifc_v0_1_model_write_ifcxml`. `InvalidValue` for another
/// `inner` or a `profile` with STEP inside; otherwise as the inner write,
/// and `FeatureDisabled` without the `zip` feature.
///
/// # Safety
/// As `openbim_ifc_v0_1_model_write`; `inner` and `profile`, if non-null,
/// valid for `inner_len` and `profile_len` reads.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_write_ifczip(
    model: OpenbimIfcModel,
    inner: *const u8,
    inner_len: usize,
    profile: *const u8,
    profile_len: usize,
    buffer: *mut u8,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        // SAFETY: caller contract above; a null, empty text is `None` as
        // for the profile.
        let texts = unsafe {
            (
                self::profile(inner, inner_len),
                self::profile(profile, profile_len),
            )
        };
        let (inner, profile) = match texts {
            (Ok(inner), Ok(profile)) => (inner, profile),
            (Err(status), _) | (_, Err(status)) => return status,
        };
        with_model(model, |m: &mut IfcModel| {
            let bytes = m.write_ifczip(inner, profile)?;
            // SAFETY: caller contract above.
            done(unsafe { fill(&bytes, buffer, capacity, out_required) })
        })
    })
}
