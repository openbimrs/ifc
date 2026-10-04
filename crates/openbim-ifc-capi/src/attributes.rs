//! Entity attributes by name (#326), resolved against the release the
//! file's header declares.
//!
//! Names match ASCII case-insensitively and answers spell them as the
//! schema does. The positional exports (`openbim_ifc_v0_1_entity_attribute`,
//! `openbim_ifc_v0_1_entity_set_attribute`) are unchanged; these resolve a
//! name to its slot first, in the shared core.

use openbim_ifc_binding_core::record::to_records;

use crate::buffer::{bytes, slice, text};
use crate::domains::{list_export, Out};
use crate::model::{done, with_model, OpenbimIfcModel};
use crate::status::{boundary, OpenbimIfcStatus};
use crate::tape::{OpenbimIfcValueNode, Reader};

/// Every explicit attribute of entity `id` in slot order, inherited first:
/// a `LIST` of `AttributeInfo` records, their number in `out_count`.
/// `AttributeInfo`: name (`TEXT`, the schema's spelling), index
/// (`INTEGER`, the slot), type name (`TEXT`), optional (`BOOL`), aggregate
/// (`BOOL`), derived (`BOOL`: written `*`, not writable), declared by
/// (`TEXT`, the entity introducing it). `INVERSE` attributes hold no slot
/// and are not listed.
///
/// `MissingEntity`; `UnsupportedSchema` when the header names no bundled
/// release or the release does not declare the entity's type.
///
/// # Safety
/// `out_count` valid for one write; otherwise as for
/// `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_entity_attribute_names(
    model: OpenbimIfcModel,
    id: u64,
    out_count: *mut usize,
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
    unsafe {
        list_export(model, out_count, out, |m| {
            Ok(to_records(&m.attribute_names(id)?))
        })
    }
}

/// Attribute `name` (UTF-8, `name_len` bytes, any case, e.g. `Name`) of
/// entity `id` as a value tape; `NULL` when the record stops before its
/// slot.
///
/// `MissingEntity`, `UnknownAttribute`, `UnsupportedSchema` as for
/// [`openbim_ifc_v0_1_entity_attribute_names`].
///
/// # Safety
/// `name` valid for `name_len` reads; otherwise as for
/// `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_entity_attribute_by_name(
    model: OpenbimIfcModel,
    id: u64,
    name: *const u8,
    name_len: usize,
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
    boundary(|| {
        // SAFETY: caller contract above.
        let name = match unsafe { text(name, name_len) } {
            Ok(name) => name,
            Err(status) => return status,
        };
        with_model(model, |m| {
            let value = m.attribute_by_name(id, name)?;
            // SAFETY: caller contract above.
            unsafe { out.write(&value) }
        })
    })
}

/// Set attribute `name` (UTF-8, any case) of entity `id` from a one-value
/// tape. Every check runs before the write, so a refusal changes nothing.
///
/// `DerivedAttribute` for a slot the entity's type derives (written `*`);
/// otherwise as for [`openbim_ifc_v0_1_entity_attribute_by_name`] and
/// `openbim_ifc_v0_1_entity_set_attribute`.
///
/// # Safety
/// `name` valid for `name_len` reads; the tape as for
/// `openbim_ifc_v0_1_entity_set_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_entity_set_attribute_by_name(
    model: OpenbimIfcModel,
    id: u64,
    name: *const u8,
    name_len: usize,
    nodes: *const OpenbimIfcValueNode,
    node_count: usize,
    strings: *const u8,
    string_len: usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        // SAFETY: caller contract above.
        let inputs = unsafe {
            (
                text(name, name_len),
                slice(nodes, node_count),
                bytes(strings, string_len),
            )
        };
        let (name, nodes, strings) = match inputs {
            (Ok(a), Ok(b), Ok(c)) => (a, b, c),
            (Err(status), ..) | (_, Err(status), _) | (.., Err(status)) => return status,
        };
        with_model(model, |m| {
            let value = Reader::new(nodes, strings).single()?;
            m.set_attribute_by_name(id, name, value)?;
            done(OpenbimIfcStatus::Ok)
        })
    })
}
