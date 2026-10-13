//! Entities by `GlobalId`, new `GlobalId`s, and an object's type, a type's
//! occurrences and the openings and fillings of elements (#463).
//!
//! An optional answer (`by_guid`, `type_object`, `voided_element_of`) is
//! written to `out_id`, or is `NoValue` when there is none; a list of ids
//! fills the caller's buffer as `openbim_ifc_v0_1_model_ids` does.

use openbim_ifc_binding_core::{name_based_guid, new_guid, relations, BindingError, IfcModel};

use crate::buffer::{fill, fill_c_string, put, text};
use crate::model::{done, fill_tape, with_model, OpenbimIfcModel};
use crate::status::{boundary, OpenbimIfcStatus};
use crate::tape::{OpenbimIfcValueNode, Tape};

/// Write `id` to `out_id`, or answer `NoValue`.
///
/// # Safety
/// `out_id` valid for one write (checked non-null by the caller).
unsafe fn optional(id: Option<u64>, out_id: *mut u64) -> OpenbimIfcStatus {
    match id {
        // SAFETY: forwarded caller contract.
        Some(id) => match unsafe { put(out_id, id) } {
            Ok(()) => OpenbimIfcStatus::Ok,
            Err(status) => status,
        },
        None => OpenbimIfcStatus::NoValue,
    }
}

/// The entity whose `GlobalId` is `global_id` (UTF-8, `global_len` bytes),
/// written to `out_id`; `NoValue` when none carries it.
///
/// The first lookup indexes every `GlobalId` once; later ones reuse the
/// index until the model changes. `InvalidValue` for text that is not a
/// `GlobalId`, `InvalidModel` when several entities carry it (the last
/// error names them), `UnsupportedSchema`.
///
/// # Safety
/// `global_id` valid for `global_len` reads; `out_id` valid for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_by_guid(
    model: OpenbimIfcModel,
    global_id: *const u8,
    global_len: usize,
    out_id: *mut u64,
) -> OpenbimIfcStatus {
    boundary(|| {
        if out_id.is_null() {
            return OpenbimIfcStatus::NullPointer;
        }
        // SAFETY: caller contract above.
        let global_id = match unsafe { text(global_id, global_len) } {
            Ok(global_id) => global_id,
            Err(status) => return status,
        };
        with_model(model, |m| {
            let found = m.by_guid(global_id)?;
            // SAFETY: checked non-null; caller contract above.
            done(unsafe { optional(found, out_id) })
        })
    })
}

/// Every `GlobalId` several entities carry, in `GlobalId` order, and how
/// many. The tape is a `LIST`; each item a `LIST` of the `GlobalId`
/// (`TEXT`) and its holders in file order (`LIST` of `REF`). Empty for a
/// conformant file. `UnsupportedSchema`.
///
/// # Safety
/// `out_count` valid for one write; otherwise as for
/// `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_duplicate_global_ids(
    model: OpenbimIfcModel,
    out_count: *mut usize,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        if out_count.is_null() {
            return OpenbimIfcStatus::NullPointer;
        }
        with_model(model, |m| {
            let duplicates = m.duplicate_global_ids()?;
            // SAFETY: checked non-null; caller contract above.
            if let Err(status) = unsafe { put(out_count, duplicates.len()) } {
                return done(status);
            }
            let tape = Tape::encode(&relations::duplicates_to_tagged(&duplicates));
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

/// An optional-id query: `out_id` checked, the answer written or `NoValue`.
///
/// # Safety
/// `out_id` valid for one write.
unsafe fn optional_query(
    model: OpenbimIfcModel,
    out_id: *mut u64,
    query: impl FnOnce(&IfcModel) -> Result<Option<u64>, BindingError>,
) -> OpenbimIfcStatus {
    boundary(|| {
        if out_id.is_null() {
            return OpenbimIfcStatus::NullPointer;
        }
        with_model(model, |m| {
            let found = query(m)?;
            // SAFETY: checked non-null; caller contract above.
            done(unsafe { optional(found, out_id) })
        })
    })
}

/// A list-of-ids query, filling the caller's buffer.
///
/// # Safety
/// As for `openbim_ifc_v0_1_model_ids`.
unsafe fn list_query(
    model: OpenbimIfcModel,
    buffer: *mut u64,
    capacity: usize,
    out_required: *mut usize,
    query: impl FnOnce(&IfcModel) -> Result<Vec<u64>, BindingError>,
) -> OpenbimIfcStatus {
    boundary(|| {
        with_model(model, |m| {
            let ids = query(m)?;
            // SAFETY: forwarded caller contract.
            done(unsafe { fill(&ids, buffer, capacity, out_required) })
        })
    })
}

/// The type object of occurrence `id` (`IfcRelDefinesByType`), written to
/// `out_id`; `NoValue` when it has none. `MissingEntity`,
/// `WrongEntityType` (not an `IfcObject`), `InvalidModel` (two types, or a
/// malformed relationship), `MissingReference`, `UnsupportedSchema`.
///
/// # Safety
/// `out_id` valid for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_entity_type_object(
    model: OpenbimIfcModel,
    id: u64,
    out_id: *mut u64,
) -> OpenbimIfcStatus {
    // SAFETY: caller contract above.
    unsafe { optional_query(model, out_id, |m| m.type_object(id)) }
}

/// Every object type object `id` types, in file order. Refusals as for
/// [`openbim_ifc_v0_1_entity_type_object`] (`WrongEntityType`: not an
/// `IfcTypeObject`).
///
/// # Safety
/// As for `openbim_ifc_v0_1_model_ids`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_entity_occurrences(
    model: OpenbimIfcModel,
    id: u64,
    buffer: *mut u64,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    // SAFETY: caller contract above.
    unsafe {
        list_query(model, buffer, capacity, out_required, |m| {
            m.occurrences_of(id)
        })
    }
}

/// Every opening voiding element `id` (`IfcRelVoidsElement`), in file
/// order. Refusals as for [`openbim_ifc_v0_1_entity_type_object`]
/// (`WrongEntityType`: not an `IfcElement`).
///
/// # Safety
/// As for `openbim_ifc_v0_1_model_ids`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_entity_openings(
    model: OpenbimIfcModel,
    id: u64,
    buffer: *mut u64,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    // SAFETY: caller contract above.
    unsafe { list_query(model, buffer, capacity, out_required, |m| m.openings_of(id)) }
}

/// Every element filling opening `id` (`IfcRelFillsElement`), in file
/// order. Refusals as for [`openbim_ifc_v0_1_entity_type_object`]
/// (`WrongEntityType`: not an `IfcOpeningElement`).
///
/// # Safety
/// As for `openbim_ifc_v0_1_model_ids`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_entity_fillings(
    model: OpenbimIfcModel,
    id: u64,
    buffer: *mut u64,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    // SAFETY: caller contract above.
    unsafe { list_query(model, buffer, capacity, out_required, |m| m.fillings_of(id)) }
}

/// The element opening `id` voids (`IfcRelVoidsElement`), written to
/// `out_id`; `NoValue` when it voids none. Refusals as for
/// [`openbim_ifc_v0_1_entity_type_object`] (`WrongEntityType`: not an
/// `IfcFeatureElementSubtraction`; `InvalidModel`: two hosts).
///
/// # Safety
/// `out_id` valid for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_entity_voided_element(
    model: OpenbimIfcModel,
    id: u64,
    out_id: *mut u64,
) -> OpenbimIfcStatus {
    // SAFETY: caller contract above.
    unsafe { optional_query(model, out_id, |m| m.voided_element_of(id)) }
}

/// A new random `GlobalId` (RFC 9562 version 4, from the operating
/// system's generator) in IFC's 22-character form, NUL-terminated: 23
/// bytes. Needs no model. `Unsupported` when the system has no random
/// source.
///
/// # Safety
/// `buffer` null (with `capacity` 0) or valid for `capacity` writes;
/// `out_required` valid for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_new_guid(
    buffer: *mut u8,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| match new_guid() {
        // SAFETY: caller contract above.
        Ok(guid) => unsafe { fill_c_string(&guid, buffer, capacity, out_required) },
        Err(error) => OpenbimIfcStatus::from(&error),
    })
}

/// The `GlobalId` named by `parts` (RFC 9562 version 8), NUL-terminated:
/// the same parts give the same `GlobalId` in every binding and release.
/// `parts` holds `part_count` UTF-8 strings of `part_lengths[i]` bytes
/// each. Needs no model; `InvalidArgument` for a part that is not UTF-8.
///
/// # Safety
/// `parts` and `part_lengths` null (with `part_count` 0) or valid for
/// `part_count` reads, each part valid for its length; `buffer` and
/// `out_required` as for [`openbim_ifc_v0_1_new_guid`].
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_name_based_guid(
    parts: *const *const u8,
    part_lengths: *const usize,
    part_count: usize,
    buffer: *mut u8,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        if part_count > 0 && (parts.is_null() || part_lengths.is_null()) {
            return OpenbimIfcStatus::NullPointer;
        }
        let mut names = Vec::with_capacity(part_count);
        for index in 0..part_count {
            // SAFETY: both arrays hold `part_count` items (caller contract).
            let (part, length) = unsafe { (*parts.add(index), *part_lengths.add(index)) };
            // SAFETY: each part is valid for its length (caller contract).
            match unsafe { text(part, length) } {
                Ok(name) => names.push(name),
                Err(status) => return status,
            }
        }
        let guid = name_based_guid(&names);
        // SAFETY: caller contract above.
        unsafe { fill_c_string(&guid, buffer, capacity, out_required) }
    })
}
