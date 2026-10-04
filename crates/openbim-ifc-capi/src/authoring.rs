//! Schema-checked entity creation (#330) through the C ABI.
//!
//! A batch crosses as one value tape: a `LIST` of operations, each a `LIST`
//! of an `ENUM` naming it (`CREATE`, `EDIT`, `REMOVE`, `PROJECT`, `SPATIAL`,
//! `PRODUCT`, `TYPE_OBJECT`, `ASSIGN_TYPE`, `CONTAIN`, `AGGREGATE`,
//! `PLACEMENT`, `OWNER_HISTORY`), then alternating field names (`TEXT`,
//! snake case) and values; a `NULL` value is an absent field. Ids are `REF`
//! (or non-negative `INTEGER`), points and directions a `LIST` of three
//! `REAL`s, attributes a `LIST` of `LIST(TEXT name, value)`. The core reads
//! it (`AuthorOp::from_tagged`), so the layout is the one the other hosts'
//! objects convert to.
//!
//! An operation names the entity an earlier one produced by its handle,
//! [`OPENBIM_IFC_HANDLE_BASE`] plus the operation's position, anywhere an id
//! goes.

use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{AuthorOp, BindingError};

use crate::buffer::{bytes, fill, put, slice, text};
use crate::model::{with_model, OpenbimIfcModel};
use crate::status::{boundary, OpenbimIfcStatus};
use crate::tape::{OpenbimIfcValueNode, Reader};

/// The first id of the handle range (2^62): `OPENBIM_IFC_HANDLE_BASE + i`
/// names the entity operation `i` of a batch produced.
pub const OPENBIM_IFC_HANDLE_BASE: u64 = 4_611_686_018_427_387_904;

const _: () = assert!(
    OPENBIM_IFC_HANDLE_BASE == openbim_ifc_binding_core::authoring::HANDLE_BASE
);

/// Apply the operations on the tape as one checked transaction against the
/// release the header declares: all of them, in order, or none, and the
/// model unchanged.
///
/// `out_count` gets the number of operations. `out_ids` gets, per
/// operation, the id of the entity it produced, or 0 for a removal; with
/// `ids_capacity` below the operation count the call returns
/// `BufferTooSmall` and applies nothing.
///
/// `UnsupportedSchema` for an unbundled release or an undeclared type;
/// `UnknownAttribute`, `DerivedAttribute`, `MissingAttribute`;
/// `InvalidValue` for a value of the wrong type, form or cardinality, a
/// duplicate `GlobalId`, a malformed tape or handle, or a placement the
/// schema cannot hold; `WrongEntityType` for an abstract type, a builder's
/// type of the wrong kind or a reference the attribute does not accept;
/// `MissingEntity`, `MissingReference`; `InvalidModel` for a second
/// containment, decomposition, typing or `IfcProject`; `StillReferenced`
/// for a removal an entity other than a relationship still needs.
///
/// # Safety
/// `nodes` valid for `node_count` reads and `strings` for `string_len`
/// (either null when its length is 0); `out_ids` null with capacity 0, or
/// valid for `ids_capacity` writes; `out_count` valid for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_author(
    model: OpenbimIfcModel,
    nodes: *const OpenbimIfcValueNode,
    node_count: usize,
    strings: *const u8,
    string_len: usize,
    out_ids: *mut u64,
    ids_capacity: usize,
    out_count: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        if out_count.is_null() || (out_ids.is_null() && ids_capacity != 0) {
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
                    "an authoring batch is one LIST of operations".into(),
                ));
            };
            let ops = items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    AuthorOp::from_tagged(item).map_err(|error| match error {
                        BindingError::InvalidValue(detail) => {
                            BindingError::InvalidValue(format!("op {index}: {detail}"))
                        }
                        other => other,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            // SAFETY: checked non-null; caller contract above.
            if let Err(status) = unsafe { put(out_count, ops.len()) } {
                return Ok(status);
            }
            if ids_capacity < ops.len() {
                return Ok(OpenbimIfcStatus::BufferTooSmall);
            }
            let result = m.author(ops)?;
            let ids: Vec<u64> = result.ids.iter().map(|id| id.unwrap_or(0)).collect();
            let mut written = 0;
            // SAFETY: capacity checked above; caller contract above.
            Ok(unsafe { fill(&ids, out_ids, ids_capacity, &mut written) })
        })
    })
}

/// Create one entity of `type_name` (UTF-8, any case) from named
/// attributes: a batch of one `CREATE`. The tape is one `LIST` of
/// `LIST(TEXT name, value)` pairs (an empty `LIST` for none). An `IfcRoot`
/// without a `GlobalId` gets a fresh one. `out_id` gets the new id.
///
/// Refusals as for [`openbim_ifc_v0_1_model_author`].
///
/// # Safety
/// `type_name` valid for `type_len` reads; the tape as for
/// [`openbim_ifc_v0_1_model_author`]; `out_id` valid for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_create_entity(
    model: OpenbimIfcModel,
    type_name: *const u8,
    type_len: usize,
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
        let (type_name, nodes, strings) = match inputs {
            (Ok(a), Ok(b), Ok(c)) => (a, b, c),
            (Err(status), ..) | (_, Err(status), _) | (.., Err(status)) => return status,
        };
        with_model(model, |m| {
            let Tagged::List(pairs) = Reader::new(nodes, strings).single()? else {
                return Err(BindingError::InvalidValue(
                    "attributes are one LIST of (TEXT name, value) pairs".into(),
                ));
            };
            let attributes = pairs
                .into_iter()
                .map(|pair| match pair {
                    Tagged::List(pair) => match <[Tagged; 2]>::try_from(pair) {
                        Ok([Tagged::Text(name), value]) => Ok((name, value)),
                        _ => Err(()),
                    },
                    _ => Err(()),
                })
                .collect::<Result<Vec<_>, ()>>()
                .map_err(|()| {
                    BindingError::InvalidValue(
                        "attributes are one LIST of (TEXT name, value) pairs".into(),
                    )
                })?;
            let id = m.create_entity(type_name, attributes)?;
            // SAFETY: checked non-null; caller contract above.
            unsafe { out_id.write(id) };
            Ok(OpenbimIfcStatus::Ok)
        })
    })
}

/// Remove entity `id` with its relationships: it is taken out of every
/// relationship holding it, and a relationship left without an end goes
/// too. Unlike `openbim_ifc_v0_1_entity_remove`, nothing is left dangling:
/// `StillReferenced` while an entity other than a relationship needs `id`,
/// `MissingEntity` when there is none.
#[no_mangle]
pub extern "C" fn openbim_ifc_v0_1_entity_remove_with_relationships(
    model: OpenbimIfcModel,
    id: u64,
) -> OpenbimIfcStatus {
    boundary(|| {
        with_model(model, |m| {
            m.remove_with_relationships(id)?;
            Ok(OpenbimIfcStatus::Ok)
        })
    })
}
