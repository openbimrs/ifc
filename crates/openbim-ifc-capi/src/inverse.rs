//! `INVERSE` attributes and the entities referencing one (#462).
//!
//! `openbim_ifc_v0_1_entity_attribute_by_name` reads an `INVERSE` name
//! (`IsDefinedBy`) as well: a `LIST` of `REF`s for a `SET` or `BAG`
//! (`SET [0:1]` included: zero or one element), a `REF` or `NULL` for a
//! single-valued inverse. These list the declarations and the referrers,
//! through the shared core's reverse index, built on first use and again
//! only after the model changes.

use openbim_ifc_binding_core::record::to_records;

use crate::buffer::put;
use crate::domains::{list_export, Out};
use crate::model::{done, with_model, OpenbimIfcModel};
use crate::status::{boundary, OpenbimIfcStatus};
use crate::tape::OpenbimIfcValueNode;

/// Every `INVERSE` attribute of entity `id`, inherited ones first: a
/// `LIST` of `InverseAttributeInfo` records, their number in `out_count`.
/// `InverseAttributeInfo`: name (`TEXT`, the schema's spelling), entity
/// (`TEXT`, the referencing type before `FOR`), for attribute (`TEXT`, its
/// attribute after `FOR`), aggregate (`BOOL`: a `SET` or `BAG`, read as a
/// `LIST`), lower (`INTEGER`), upper (`INTEGER`, or `NULL` for `?`),
/// declared by (`TEXT`, the entity introducing it).
///
/// `MissingEntity`; `UnsupportedSchema` as for
/// `openbim_ifc_v0_1_entity_attribute_names`.
///
/// # Safety
/// As for `openbim_ifc_v0_1_entity_attribute_names`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_entity_inverse_attribute_names(
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
            Ok(to_records(&m.inverse_attribute_names(id)?))
        })
    }
}

/// Every reference to entity `id`, in ascending order: a `LIST` of
/// `InverseReference` records, their number in `out_count`, one per
/// referencing entity and attribute. `InverseReference`: id (`REF`, the
/// referencing entity), index (`INTEGER`, its attribute holding the
/// reference, directly or inside an aggregate). Needs no schema.
///
/// `MissingEntity`.
///
/// # Safety
/// As for `openbim_ifc_v0_1_entity_attribute_names`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_entity_inverse_references(
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
            Ok(to_records(&m.inverse_references(id)?))
        })
    }
}

/// How many distinct entities reference entity `id`, through any
/// attribute, written to `out_count`. Needs no schema. `MissingEntity`.
///
/// # Safety
/// `out_count` must be null or valid for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_entity_total_inverses(
    model: OpenbimIfcModel,
    id: u64,
    out_count: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        with_model(model, |m| {
            let total = m.total_inverses(id)?;
            // SAFETY: caller contract above.
            done(
                unsafe { put(out_count, total) }
                    .err()
                    .unwrap_or(OpenbimIfcStatus::Ok),
            )
        })
    })
}
