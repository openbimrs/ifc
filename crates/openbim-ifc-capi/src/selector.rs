//! Selectors and values by path (#464).
//!
//! A selector such as `IfcWall, Pset_WallCommon.IsExternal=TRUE` selects
//! ids into the caller's buffer as `openbim_ifc_v0_1_model_ids` fills it;
//! a value path such as `type.Name` reads one value as a tape and writes
//! one from a tape of plain values. A query that does not parse, or names
//! a class or attribute the declared release lacks, is `InvalidSelector`;
//! `openbim_ifc_v0_1_last_error_span` then gives the byte range it refuses.

use crate::buffer::{bytes, fill, put, slice, text};
use crate::domains::Out;
use crate::model::{done, with_model, OpenbimIfcModel};
use crate::registry::{self, lock};
use crate::status::{boundary, OpenbimIfcStatus};
use crate::tape::{OpenbimIfcValueNode, Reader};

/// The selector grammar's major version (#464); a selector written for one
/// major version parses the same in every build of it.
pub const OPENBIM_IFC_SELECTOR_GRAMMAR_MAJOR: u32 = 1;
/// The selector grammar's minor version: it grows when the grammar accepts
/// more.
pub const OPENBIM_IFC_SELECTOR_GRAMMAR_MINOR: u32 = 0;

/// The ids selector `query` (UTF-8, `query_len` bytes) selects, ascending,
/// filling `buffer` as `openbim_ifc_v0_1_model_ids` does.
///
/// Class facets (subtypes included) add entities, `!` removes them;
/// attribute, property and `type=`, `material=`, `classification=`,
/// `location=` and `group=` facets filter; `,` chains facets and `+`
/// unites lists. `InvalidSelector` (span through
/// `openbim_ifc_v0_1_last_error_span`), `UnsupportedSchema`,
/// `InvalidModel` for an ambiguous or malformed relationship or a
/// duplicated `GlobalId` an instance facet names, `MissingReference`, and
/// the property refusals of `openbim_ifc_v0_1_entity_property_sets`.
///
/// # Safety
/// `query` valid for `query_len` reads; otherwise as for
/// `openbim_ifc_v0_1_model_ids`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_select(
    model: OpenbimIfcModel,
    query: *const u8,
    query_len: usize,
    buffer: *mut u64,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        // SAFETY: caller contract above.
        let query = match unsafe { text(query, query_len) } {
            Ok(query) => query,
            Err(status) => return status,
        };
        with_model(model, |m| {
            let ids = m.select(query)?;
            // SAFETY: caller contract above.
            done(unsafe { fill(&ids, buffer, capacity, out_required) })
        })
    })
}

/// As [`openbim_ifc_v0_1_model_select`], selecting from the `base_count`
/// ids at `base` alone; `MissingEntity` for one the model lacks.
///
/// # Safety
/// `base` valid for `base_count` reads; otherwise as for
/// [`openbim_ifc_v0_1_model_select`].
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_select_from(
    model: OpenbimIfcModel,
    query: *const u8,
    query_len: usize,
    base: *const u64,
    base_count: usize,
    buffer: *mut u64,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        // SAFETY: caller contract above.
        let inputs = unsafe { (text(query, query_len), slice(base, base_count)) };
        let (query, base) = match inputs {
            (Ok(query), Ok(base)) => (query, base),
            (Err(status), _) | (_, Err(status)) => return status,
        };
        with_model(model, |m| {
            let ids = m.select_from(query, base)?;
            // SAFETY: caller contract above.
            done(unsafe { fill(&ids, buffer, capacity, out_required) })
        })
    })
}

/// The value `path` (UTF-8, e.g. `type.Name`, `Pset_WallCommon.FireRating`)
/// reads from entity `id`, as a value tape: a scalar or reference as its
/// node, several values as a `LIST`, a property set as a `LIST` of
/// two-item `LIST`s, each the member's name (`TEXT`) and value.
///
/// `InvalidSelector` for a path that does not parse, `MissingEntity`, and
/// as for [`openbim_ifc_v0_1_model_select`].
///
/// # Safety
/// `path` valid for `path_len` reads; otherwise as for
/// `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_entity_get_value(
    model: OpenbimIfcModel,
    id: u64,
    path: *const u8,
    path_len: usize,
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
        let path = match unsafe { text(path, path_len) } {
            Ok(path) => path,
            Err(status) => return status,
        };
        with_model(model, |m| {
            let value = m.get_value(id, path)?.to_tagged();
            // SAFETY: caller contract above.
            unsafe { out.write(&value) }
        })
    })
}

/// Write a one-value tape of plain values at `path` (UTF-8) of entity
/// `id`, as one transaction: an attribute (`Name`, `type.Name`) coerced
/// as `openbim_ifc_v0_1_entity_set_attribute_by_name_plain` coerces it,
/// or a property (`Pset_WallCommon.FireRating`) from a `TYPED` value, or a
/// plain one when the property already states its type. A refusal
/// changes nothing.
///
/// `InvalidValue` for a path that names no single value (a pattern,
/// `count`, a prefix reaching no entity), `UnknownAttribute`,
/// `DerivedAttribute`, `TypeMismatch` (also for a plain value no stated
/// property types), `AmbiguousValue`, and the property edit's refusals.
///
/// # Safety
/// `path` valid for `path_len` reads; the tape as for
/// `openbim_ifc_v0_1_entity_set_attribute_by_name_plain`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_entity_set_value(
    model: OpenbimIfcModel,
    id: u64,
    path: *const u8,
    path_len: usize,
    nodes: *const OpenbimIfcValueNode,
    node_count: usize,
    strings: *const u8,
    string_len: usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        // SAFETY: caller contract above.
        let inputs = unsafe {
            (
                text(path, path_len),
                slice(nodes, node_count),
                bytes(strings, string_len),
            )
        };
        let (path, nodes, strings) = match inputs {
            (Ok(a), Ok(b), Ok(c)) => (a, b, c),
            (Err(status), ..) | (_, Err(status), _) | (.., Err(status)) => return status,
        };
        with_model(model, |m| {
            let value = Reader::new(nodes, strings).plain_single()?;
            m.set_value(id, path, value)?;
            done(OpenbimIfcStatus::Ok)
        })
    })
}

/// The byte range in the query the last error on `model` refuses, written
/// to `out_start` and `out_end`; `NoValue` when the last call succeeded or
/// its error names no span (only `InvalidSelector` does).
///
/// # Safety
/// `out_start` and `out_end` valid for one write each.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_last_error_span(
    model: OpenbimIfcModel,
    out_start: *mut usize,
    out_end: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        if out_start.is_null() || out_end.is_null() {
            return OpenbimIfcStatus::NullPointer;
        }
        let Some(shared) = registry::get(model) else {
            return OpenbimIfcStatus::InvalidHandle;
        };
        // Read without going through `with_model`, which would clear the
        // error.
        let entry = lock(&shared);
        let Some((start, end)) = entry.last_error.as_ref().and_then(|error| error.span()) else {
            return OpenbimIfcStatus::NoValue;
        };
        // SAFETY: checked non-null; caller contract above.
        match unsafe { (put(out_start, start), put(out_end, end)) } {
            (Ok(()), Ok(())) => OpenbimIfcStatus::Ok,
            (Err(status), _) | (_, Err(status)) => status,
        }
    })
}
