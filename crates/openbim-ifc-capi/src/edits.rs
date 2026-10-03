//! Writing property sets (#123) through the C ABI.
//!
//! A batch crosses as one value tape: a `LIST` of edits, each a `LIST` of
//! an `ENUM` (`SET` or `REMOVE`), the object (`REF` or `INTEGER`), the set
//! name and the property name (`TEXT`), then, for `SET`, the value (the read
//! side's `value` field, typed) and an optional set type (`TEXT` or
//! `NULL`). The core reads it (`PropertyEdit::from_tagged`), so the layout
//! is the same one the other hosts' objects carry.
//!
//! A batch is applied once: unlike a read, it has no size query that runs
//! it. The caller passes a buffer for one id per edit, which is checked
//! before anything is written.

use openbim_ifc_binding_core::property_edit::PropertyEdit;
use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::BindingError;

use crate::buffer::{bytes, fill, put, slice, text};
use crate::model::{with_model, OpenbimIfcModel};
use crate::status::{boundary, OpenbimIfcStatus};
use crate::tape::{OpenbimIfcValueNode, Reader};

/// Apply the edits on the tape as one checked transaction: all of them, in
/// order, or none, and the model unchanged.
///
/// `out_count` gets the number of edits. `out_properties` gets, per edit,
/// the id of the entity holding the property afterwards, or 0 when the
/// batch leaves none; with `properties_capacity` below the edit count the
/// call returns `BufferTooSmall` and applies nothing.
///
/// `InvalidArgument` or `InvalidValue` for a malformed tape;
/// `MissingEntity`, `WrongEntityType`, `UnsupportedSchema`, `InvalidValue`,
/// `TemplateViolation`, `MissingProperty`, `Unsupported`, `InvalidModel`,
/// `FeatureDisabled` as the shared codes say.
///
/// # Safety
/// `nodes` valid for `node_count` reads and `strings` for `string_len`
/// (either null when its length is 0); `out_properties` null with capacity
/// 0, or valid for `properties_capacity` writes; `out_count` valid for one
/// write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_set_properties(
    model: OpenbimIfcModel,
    nodes: *const OpenbimIfcValueNode,
    node_count: usize,
    strings: *const u8,
    string_len: usize,
    out_properties: *mut u64,
    properties_capacity: usize,
    out_count: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        if out_count.is_null() || (out_properties.is_null() && properties_capacity != 0) {
            return OpenbimIfcStatus::NullPointer;
        }
        // SAFETY: caller contract above.
        let (nodes, strings) =
            match unsafe { (slice(nodes, node_count), bytes(strings, string_len)) } {
                (Ok(n), Ok(s)) => (n, s),
                (Err(status), _) | (_, Err(status)) => return status,
            };
        with_model(model, |m| {
            let Tagged::List(items) = Reader::new(nodes, strings).single()? else {
                return Err(BindingError::InvalidValue(
                    "property edits are one LIST of edits".into(),
                ));
            };
            let edits = items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    PropertyEdit::from_tagged(item).map_err(|error| match error {
                        BindingError::InvalidValue(detail) => {
                            BindingError::InvalidValue(format!("edit {index}: {detail}"))
                        }
                        other => other,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            // SAFETY: checked non-null; caller contract above.
            if let Err(status) = unsafe { put(out_count, edits.len()) } {
                return Ok(status);
            }
            if properties_capacity < edits.len() {
                return Ok(OpenbimIfcStatus::BufferTooSmall);
            }
            let result = m.set_properties(edits)?;
            let ids: Vec<u64> = result.properties.iter().map(|id| id.unwrap_or(0)).collect();
            let mut written = 0;
            // SAFETY: capacity checked above; caller contract above.
            Ok(unsafe { fill(&ids, out_properties, properties_capacity, &mut written) })
        })
    })
}

/// Write one value: [`openbim_ifc_v0_1_model_set_properties`] with one
/// `SET` edit. `set` and `name` are UTF-8; the value is a one-value tape;
/// `set_type` (`IfcPropertySet` or `IfcElementQuantity`) may be null with
/// length 0. `out_id` gets the entity holding the property.
///
/// # Safety
/// `set`, `name` and `set_type` valid for their lengths (null only for 0);
/// the tape as for `openbim_ifc_v0_1_entity_set_attribute`; `out_id` valid
/// for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_set_property(
    model: OpenbimIfcModel,
    object: u64,
    set: *const u8,
    set_len: usize,
    name: *const u8,
    name_len: usize,
    nodes: *const OpenbimIfcValueNode,
    node_count: usize,
    strings: *const u8,
    string_len: usize,
    set_type: *const u8,
    set_type_len: usize,
    out_id: *mut u64,
) -> OpenbimIfcStatus {
    boundary(|| {
        if out_id.is_null() {
            return OpenbimIfcStatus::NullPointer;
        }
        // SAFETY: caller contract above.
        let inputs = unsafe {
            (
                text(set, set_len),
                text(name, name_len),
                text(set_type, set_type_len),
                slice(nodes, node_count),
                bytes(strings, string_len),
            )
        };
        let (set, name, set_type, nodes, strings) = match inputs {
            (Ok(a), Ok(b), Ok(c), Ok(d), Ok(e)) => (a, b, c, d, e),
            (Err(status), ..)
            | (_, Err(status), ..)
            | (_, _, Err(status), ..)
            | (_, _, _, Err(status), _)
            | (.., Err(status)) => return status,
        };
        let set_type = (!set_type.is_empty()).then(|| set_type.to_owned());
        with_model(model, |m| {
            let value = Reader::new(nodes, strings).single()?;
            let id = m.set_property(object, set, name, value, set_type)?;
            // SAFETY: checked non-null; caller contract above.
            unsafe { out_id.write(id) };
            Ok(OpenbimIfcStatus::Ok)
        })
    })
}

/// Remove one property from `object`'s own set:
/// [`openbim_ifc_v0_1_model_set_properties`] with one `REMOVE` edit.
///
/// # Safety
/// `set` and `name` valid for their lengths (null only for 0).
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_remove_property(
    model: OpenbimIfcModel,
    object: u64,
    set: *const u8,
    set_len: usize,
    name: *const u8,
    name_len: usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        // SAFETY: caller contract above.
        let (set, name) = match unsafe { (text(set, set_len), text(name, name_len)) } {
            (Ok(set), Ok(name)) => (set, name),
            (Err(status), _) | (_, Err(status)) => return status,
        };
        with_model(model, |m| {
            m.remove_property(object, set, name)?;
            Ok(OpenbimIfcStatus::Ok)
        })
    })
}
