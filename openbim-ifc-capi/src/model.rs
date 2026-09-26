//! The model exports: create, parse, query, edit, write, destroy.
//!
//! Every function follows one shape: validate pointers, look up the handle,
//! call the shared core, record a failure as the model's last error, and
//! copy the result into caller memory. No IFC behaviour lives here.

use openbim_ifc_binding_core::{BindingError, IfcModel};

use crate::buffer::{bytes, fill, fill_c_string, put, slice, text};
use crate::registry::{self, lock, Entry};
use crate::status::{boundary, OpenbimIfcStatus, OpenbimIfcVersion};
use crate::tape::{OpenbimIfcValueNode, Reader, Tape};

/// ABI version implemented by this crate; also the `v0_1` symbol prefix.
const ABI: (u16, u16, u16) = (0, 1, 0);

/// Opaque model handle. Zero is never a valid handle.
pub type OpenbimIfcModel = u64;

/// Run `operation` against the live model behind `handle`.
///
/// A [`BindingError`] is stored as the model's last error and mapped to its
/// status; a successful call clears the last error, so the error a host
/// reads always belongs to the call that just failed.
fn with_model(
    handle: OpenbimIfcModel,
    operation: impl FnOnce(&mut IfcModel) -> Result<OpenbimIfcStatus, BindingError>,
) -> OpenbimIfcStatus {
    let Some(shared) = registry::get(handle) else {
        return OpenbimIfcStatus::InvalidHandle;
    };
    let mut entry = lock(&shared);
    let Entry { model, last_error } = &mut *entry;
    match operation(model) {
        Ok(status) => {
            *last_error = None;
            status
        }
        Err(error) => {
            let status = OpenbimIfcStatus::from(&error);
            *last_error = Some(error);
            status
        }
    }
}

/// Status from a buffer helper, which already speaks the C protocol.
fn done(status: OpenbimIfcStatus) -> Result<OpenbimIfcStatus, BindingError> {
    Ok(status)
}

/// Write the ABI and crate versions.
///
/// # Safety
/// `out_version` must be null or valid for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_version(
    out_version: *mut OpenbimIfcVersion,
) -> OpenbimIfcStatus {
    boundary(|| {
        let version = OpenbimIfcVersion {
            abi_major: ABI.0,
            abi_minor: ABI.1,
            abi_patch: ABI.2,
            crate_major: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap_or(0),
            crate_minor: env!("CARGO_PKG_VERSION_MINOR").parse().unwrap_or(0),
            crate_patch: env!("CARGO_PKG_VERSION_PATCH").parse().unwrap_or(0),
        };
        // SAFETY: caller contract above.
        unsafe { put(out_version, version) }
            .err()
            .unwrap_or(OpenbimIfcStatus::Ok)
    })
}

/// Create an empty model and write its handle to `out_model`.
///
/// # Safety
/// `out_model` must be null or valid for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_create(
    out_model: *mut OpenbimIfcModel,
) -> OpenbimIfcStatus {
    boundary(|| {
        if out_model.is_null() {
            return OpenbimIfcStatus::NullPointer;
        }
        let handle = registry::insert(IfcModel::empty());
        // SAFETY: checked non-null; caller contract above.
        unsafe { put(out_model, handle) }
            .err()
            .unwrap_or(OpenbimIfcStatus::Ok)
    })
}

/// Parse `len` bytes of STEP and write the new model's handle.
///
/// A parse failure has no model to hold its error, so the message is written
/// to the optional `error_buffer` (NUL-terminated, truncated to `capacity`).
///
/// # Safety
/// `data` must be valid for `len` reads; `out_model` for one write;
/// `error_buffer`, if non-null, for `capacity` writes.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_parse(
    data: *const u8,
    len: usize,
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
        // SAFETY: caller contract above.
        unsafe { finish_load(IfcModel::parse(input), out_model, error_buffer, capacity) }
    })
}

/// Completes a load: registers the model and writes its handle, or writes
/// the error message to the optional `error_buffer` (NUL-terminated,
/// truncated on a character boundary to `capacity`) and returns its status.
///
/// # Safety
/// `out_model` must be non-null and valid for one write; `error_buffer`,
/// if `capacity` is non-zero, non-null and valid for `capacity` writes.
pub(crate) unsafe fn finish_load(
    loaded: Result<IfcModel, BindingError>,
    out_model: *mut OpenbimIfcModel,
    error_buffer: *mut u8,
    capacity: usize,
) -> OpenbimIfcStatus {
    match loaded {
        Ok(model) => {
            let handle = registry::insert(model);
            // SAFETY: caller contract above.
            unsafe { put(out_model, handle) }
                .err()
                .unwrap_or(OpenbimIfcStatus::Ok)
        }
        Err(error) => {
            if capacity != 0 {
                let message = error.to_string();
                let count = message.len().min(capacity - 1);
                // Truncate on a char boundary so C sees valid UTF-8.
                let count = (0..=count)
                    .rev()
                    .find(|i| message.is_char_boundary(*i))
                    .unwrap_or(0);
                // SAFETY: non-null with `capacity` > count bytes (caller contract).
                unsafe {
                    std::ptr::copy_nonoverlapping(message.as_ptr(), error_buffer, count);
                    error_buffer.add(count).write(0);
                }
            }
            OpenbimIfcStatus::from(&error)
        }
    }
}

/// Destroy a model. A stale or repeated handle is `InvalidHandle`.
#[no_mangle]
pub extern "C" fn openbim_ifc_v0_1_model_destroy(model: OpenbimIfcModel) -> OpenbimIfcStatus {
    boundary(|| {
        if registry::remove(model) {
            OpenbimIfcStatus::Ok
        } else {
            OpenbimIfcStatus::InvalidHandle
        }
    })
}

/// Number of live models, for leak checks.
///
/// # Safety
/// `out_count` must be null or valid for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_live_models(out_count: *mut usize) -> OpenbimIfcStatus {
    boundary(|| {
        // SAFETY: caller contract above.
        unsafe { put(out_count, registry::live()) }
            .err()
            .unwrap_or(OpenbimIfcStatus::Ok)
    })
}

/// Serialize as STEP into a caller buffer; `out_required` gets the size.
///
/// # Safety
/// `buffer` must be null (with `capacity` 0) or valid for `capacity`
/// writes; `out_required` valid for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_write(
    model: OpenbimIfcModel,
    buffer: *mut u8,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        with_model(model, |m| {
            let bytes = m.write()?;
            // SAFETY: caller contract above.
            done(unsafe { fill(&bytes, buffer, capacity, out_required) })
        })
    })
}

/// Number of entities.
///
/// # Safety
/// `out_count` must be null or valid for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_len(
    model: OpenbimIfcModel,
    out_count: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        with_model(model, |m| {
            // SAFETY: caller contract above.
            done(
                unsafe { put(out_count, m.len()) }
                    .err()
                    .unwrap_or(OpenbimIfcStatus::Ok),
            )
        })
    })
}

/// The first `FILE_SCHEMA` token as a NUL-terminated string, or `NoValue`.
///
/// # Safety
/// As for [`openbim_ifc_v0_1_model_write`].
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_schema(
    model: OpenbimIfcModel,
    buffer: *mut u8,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        with_model(model, |m| {
            Ok(match m.schema() {
                // SAFETY: caller contract above.
                Some(token) => unsafe { fill_c_string(token, buffer, capacity, out_required) },
                None => OpenbimIfcStatus::NoValue,
            })
        })
    })
}

/// Every entity id, in file order.
///
/// # Safety
/// `buffer` must be null (with `capacity` 0) or valid for `capacity` `u64`
/// writes; `out_required` valid for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_ids(
    model: OpenbimIfcModel,
    buffer: *mut u64,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        with_model(model, |m| {
            // SAFETY: caller contract above.
            done(unsafe { fill(&m.ids(), buffer, capacity, out_required) })
        })
    })
}

/// Ids of every entity of exactly `type_name` (UTF-8, `type_len` bytes,
/// case-insensitive). Subtypes are not included.
///
/// # Safety
/// `type_name` must be valid for `type_len` reads; otherwise as for
/// [`openbim_ifc_v0_1_model_ids`].
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_ids_of_type(
    model: OpenbimIfcModel,
    type_name: *const u8,
    type_len: usize,
    buffer: *mut u64,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        // SAFETY: caller contract above.
        let name = match unsafe { text(type_name, type_len) } {
            Ok(name) => name,
            Err(status) => return status,
        };
        with_model(model, |m| {
            // SAFETY: caller contract above.
            done(unsafe { fill(&m.ids_of_type(name), buffer, capacity, out_required) })
        })
    })
}

/// Ids of every entity of `type_name` or any of its subtypes, using the
/// schema the file's header declares. `UnsupportedSchema` if none is bundled.
///
/// # Safety
/// As for [`openbim_ifc_v0_1_model_ids_of_type`].
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_ids_of_type_including_subtypes(
    model: OpenbimIfcModel,
    type_name: *const u8,
    type_len: usize,
    buffer: *mut u64,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        // SAFETY: caller contract above.
        let name = match unsafe { text(type_name, type_len) } {
            Ok(name) => name,
            Err(status) => return status,
        };
        with_model(model, |m| {
            let ids = m.ids_of_type_including_subtypes(name)?;
            // SAFETY: caller contract above.
            done(unsafe { fill(&ids, buffer, capacity, out_required) })
        })
    })
}

/// The type name of entity `id`, upper-case, NUL-terminated.
///
/// # Safety
/// As for [`openbim_ifc_v0_1_model_write`].
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_entity_type(
    model: OpenbimIfcModel,
    id: u64,
    buffer: *mut u8,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        with_model(model, |m| {
            let name = m.type_of(id)?;
            // SAFETY: caller contract above.
            done(unsafe { fill_c_string(name, buffer, capacity, out_required) })
        })
    })
}

/// Copy a tape into the caller's two buffers.
///
/// Both required sizes are written before either capacity is checked, so a
/// single size query (both buffers null, capacities 0) sizes both.
///
/// # Safety
/// Each buffer must be null (with its capacity 0) or valid for its capacity
/// in writes; both `out_*_required` valid for one write.
unsafe fn fill_tape(
    tape: &Tape,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    // SAFETY: forwarded caller contract.
    let node_status = unsafe { fill(&tape.nodes, nodes, node_capacity, out_nodes_required) };
    // SAFETY: forwarded caller contract.
    let string_status = unsafe {
        fill(
            &tape.strings,
            strings,
            string_capacity,
            out_strings_required,
        )
    };
    // Report the more serious of the two: a null pointer beats a short buffer.
    match (node_status, string_status) {
        (OpenbimIfcStatus::Ok, other) | (other, OpenbimIfcStatus::Ok) => other,
        (OpenbimIfcStatus::BufferTooSmall, other) => other,
        (first, _) => first,
    }
}

/// Attribute `index` of entity `id` as a value tape (`$` past the end).
///
/// # Safety
/// As for `fill_tape`: each buffer null with capacity 0, or valid for its
/// capacity; both `out_*_required` valid for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_entity_attribute(
    model: OpenbimIfcModel,
    id: u64,
    index: usize,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        with_model(model, |m| {
            let tape = Tape::encode(&m.attribute(id, index)?);
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

/// Every attribute of entity `id`, back to back on one tape, plus how many
/// top-level values it holds.
///
/// # Safety
/// As for [`openbim_ifc_v0_1_entity_attribute`]; `out_count` valid for one
/// write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_entity_attributes(
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
    boundary(|| {
        if out_count.is_null() {
            return OpenbimIfcStatus::NullPointer;
        }
        with_model(model, |m| {
            let values = m.attributes(id)?;
            let tape = Tape::encode_all(&values);
            // SAFETY: checked non-null; caller contract above.
            unsafe { out_count.write(values.len()) };
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

/// Set attribute `index` of entity `id` from a one-value tape. Writing past
/// the end pads the gap with `$`.
///
/// # Safety
/// `nodes` must be valid for `node_count` reads and `strings` for
/// `string_len` reads (either may be null when its length is 0).
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_entity_set_attribute(
    model: OpenbimIfcModel,
    id: u64,
    index: usize,
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
            m.set_attribute(id, index, value)?;
            Ok(OpenbimIfcStatus::Ok)
        })
    })
}

/// Append an entity of `type_name` whose `attribute_count` attributes are
/// on the tape back to back, and write its new id to `out_id`.
///
/// # Safety
/// As for [`openbim_ifc_v0_1_entity_set_attribute`]; `type_name` valid for
/// `type_len` reads; `out_id` valid for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_entity_add(
    model: OpenbimIfcModel,
    type_name: *const u8,
    type_len: usize,
    attribute_count: usize,
    nodes: *const OpenbimIfcValueNode,
    node_count: usize,
    strings: *const u8,
    string_len: usize,
    out_id: *mut u64,
) -> OpenbimIfcStatus {
    boundary(|| {
        if out_id.is_null() {
            return OpenbimIfcStatus::NullPointer;
        }
        // SAFETY: caller contract above.
        let inputs = unsafe {
            (
                text(type_name, type_len),
                slice(nodes, node_count),
                bytes(strings, string_len),
            )
        };
        let (name, nodes, strings) = match inputs {
            (Ok(name), Ok(n), Ok(s)) => (name, n, s),
            (Err(status), _, _) | (_, Err(status), _) | (_, _, Err(status)) => return status,
        };
        with_model(model, |m| {
            let values = Reader::new(nodes, strings).many(attribute_count)?;
            let id = m.add(name, values)?;
            // SAFETY: checked non-null; caller contract above.
            unsafe { out_id.write(id) };
            Ok(OpenbimIfcStatus::Ok)
        })
    })
}

/// Remove entity `id`; references to it are left dangling.
#[no_mangle]
pub extern "C" fn openbim_ifc_v0_1_entity_remove(
    model: OpenbimIfcModel,
    id: u64,
) -> OpenbimIfcStatus {
    boundary(|| {
        with_model(model, |m| {
            m.remove(id)?;
            Ok(OpenbimIfcStatus::Ok)
        })
    })
}

/// Every dangling reference as `(from, to)` pairs, flattened: element
/// `2k` is a referencing id, `2k+1` the missing id it points to.
///
/// # Safety
/// As for [`openbim_ifc_v0_1_model_ids`]; sizes count `u64`s, not pairs.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_dangling_references(
    model: OpenbimIfcModel,
    buffer: *mut u64,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        with_model(model, |m| {
            let flat: Vec<u64> = m
                .dangling_references()
                .into_iter()
                .flat_map(|(from, to)| [from, to])
                .collect();
            // SAFETY: caller contract above.
            done(unsafe { fill(&flat, buffer, capacity, out_required) })
        })
    })
}

/// Number of non-fatal parse diagnostics.
///
/// # Safety
/// `out_count` must be null or valid for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_diagnostic_count(
    model: OpenbimIfcModel,
    out_count: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        with_model(model, |m| {
            let count = m.diagnostics().len();
            // SAFETY: caller contract above.
            done(
                unsafe { put(out_count, count) }
                    .err()
                    .unwrap_or(OpenbimIfcStatus::Ok),
            )
        })
    })
}

/// Diagnostic `index` as a NUL-terminated string; `OutOfRange` past the end.
///
/// # Safety
/// As for [`openbim_ifc_v0_1_model_write`].
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_diagnostic(
    model: OpenbimIfcModel,
    index: usize,
    buffer: *mut u8,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        with_model(model, |m| {
            let all = m.diagnostics();
            let message = all.get(index).ok_or_else(|| {
                BindingError::OutOfRange(format!("diagnostic {index} of {}", all.len()))
            })?;
            // SAFETY: caller contract above.
            done(unsafe { fill_c_string(message, buffer, capacity, out_required) })
        })
    })
}

#[cfg(test)]
mod tests;
