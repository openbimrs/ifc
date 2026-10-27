//! Removing a product with what it solely owns, and purging what nothing
//! uses (#478); removing a shape representation with what only it uses
//! (#481).
//!
//! Each answers with a `RemovalReport` record tape: removed (`LIST` of
//! `RemovedEntity`: id `REF`, type name `TEXT`), kept (`LIST` of
//! `KeptEntity`: id, type name, reason `TEXT` -- `shared`,
//! `related-object`, `definition` or `protected` -- and users, a `LIST` of
//! `REF`), detached (`LIST` of `REF`), rebased (`LIST` of `REF`), applied
//! (`BOOL`, false for a dry run).
//!
//! The buffer protocol never applies a removal it cannot report: the call
//! plans it first, and a size query or a short buffer answers
//! `BufferTooSmall` with the sizes and leaves the model unchanged. The
//! applied report has the planned one's size.

use openbim_ifc_binding_core::{
    BindingError, IfcModel, PurgeOptions, RemovalReport, RemoveProductOptions, ToRecord,
};

use crate::buffer::text;
use crate::domains::Out;
use crate::model::{with_model, OpenbimIfcModel};
use crate::status::{boundary, OpenbimIfcStatus};
use crate::tape::{OpenbimIfcValueNode, Tape};

/// `openbim_ifc_v0_1_model_remove_product` flag: re-base placements relative
/// to a removed placement instead of refusing.
pub const OPENBIM_IFC_REMOVE_REBASE_PLACEMENTS: u32 = 1;
/// `openbim_ifc_v0_1_model_remove_product` flag: untype a type object's
/// occurrences instead of refusing.
pub const OPENBIM_IFC_REMOVE_UNTYPE_OCCURRENCES: u32 = 2;
/// `openbim_ifc_v0_1_model_remove_product` flag: report only.
pub const OPENBIM_IFC_REMOVE_DRY_RUN: u32 = 4;
/// `openbim_ifc_v0_1_model_purge` flag: also purge unreferenced `IfcRoot`s of
/// the classes.
pub const OPENBIM_IFC_PURGE_INCLUDE_ROOTS: u32 = 1;
/// `openbim_ifc_v0_1_model_purge` flag: also purge an unreferenced owner
/// history with its actors.
pub const OPENBIM_IFC_PURGE_INCLUDE_OWNER_HISTORY: u32 = 2;
/// `openbim_ifc_v0_1_model_purge` flag: report only.
pub const OPENBIM_IFC_PURGE_DRY_RUN: u32 = 4;

const ALL_FLAGS: u32 = 7;

/// Plan with `run(model, true)`; then write the report, applying with
/// `run(model, false)` first unless `dry_run`, and only when it fits.
///
/// # Safety
/// The buffers as for `Out`.
unsafe fn removal_export(
    model: OpenbimIfcModel,
    out: &Out,
    dry_run: bool,
    run: impl Fn(&mut IfcModel, bool) -> Result<RemovalReport, BindingError>,
) -> OpenbimIfcStatus {
    boundary(|| {
        with_model(model, |m| {
            if let Err(status) = out.check() {
                return Ok(status);
            }
            let planned = run(m, true)?;
            let tape = Tape::encode(&planned.to_record().to_tagged());
            let fits =
                tape.nodes.len() <= out.node_capacity && tape.strings.len() <= out.string_capacity;
            let report = if dry_run || !fits {
                planned
            } else {
                run(m, false)?
            };
            // SAFETY: caller contract.
            unsafe { out.write(&report.to_record().to_tagged()) }
        })
    })
}

/// Remove product (or type object) `product` with everything it solely
/// owns -- its placement, representation, own property and quantity sets,
/// material usage, openings and ports, and relationships left without an
/// end -- keeping what is shared, as one checked transaction. `flags`:
/// `OPENBIM_IFC_REMOVE_REBASE_PLACEMENTS`,
/// `OPENBIM_IFC_REMOVE_UNTYPE_OCCURRENCES`, `OPENBIM_IFC_REMOVE_DRY_RUN`.
/// The answer is a `RemovalReport` tape: removed (`LIST` of
/// `RemovedEntity`: id `REF`, type name `TEXT`), kept (`LIST` of
/// `KeptEntity`: id, type name, reason `TEXT` -- `shared`,
/// `related-object`, `definition`, `protected` -- users `LIST` of `REF`),
/// detached and rebased (`LIST`s of `REF`), applied (`BOOL`, false for a
/// dry run). A size query or a short buffer plans the removal and answers
/// `BufferTooSmall` without applying it.
///
/// `MissingEntity`; `WrongEntityType` for an entity that is neither an
/// `IfcProduct` nor an `IfcTypeObject`; `StillReferenced` for a placement
/// other placements are relative to, a type that still types objects, or
/// an entity that stays and needs something removed; `Unsupported` for a
/// placement that cannot be re-based; `InvalidArgument` for an unknown
/// flag; `FeatureDisabled`.
///
/// # Safety
/// As for `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_remove_product(
    model: OpenbimIfcModel,
    product: u64,
    flags: u32,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    if flags & !ALL_FLAGS != 0 {
        return OpenbimIfcStatus::InvalidArgument;
    }
    let out = Out {
        nodes,
        node_capacity,
        out_nodes_required,
        strings,
        string_capacity,
        out_strings_required,
    };
    let dry_run = flags & OPENBIM_IFC_REMOVE_DRY_RUN != 0;
    // SAFETY: caller contract above.
    unsafe {
        removal_export(model, &out, dry_run, |m, plan| {
            let mut options = RemoveProductOptions::new();
            options.rebase_placements = flags & OPENBIM_IFC_REMOVE_REBASE_PLACEMENTS != 0;
            options.untype_occurrences = flags & OPENBIM_IFC_REMOVE_UNTYPE_OCCURRENCES != 0;
            options.dry_run = plan;
            m.remove_product(product, options)
        })
    }
}

/// Remove shape representation `representation` with what only it uses, as
/// one checked transaction: taken off its product definition shape (which
/// goes when left empty), representation map or shape aspects, then removed
/// with its items down to what nothing else uses; shared items, profiles,
/// points and styles stay. `flags`: `OPENBIM_IFC_REMOVE_DRY_RUN` alone. The
/// answer is the `RemovalReport` tape of
/// `openbim_ifc_v0_1_model_remove_product`; a size query or a short buffer
/// plans the removal and answers `BufferTooSmall` without applying it.
///
/// `MissingEntity`; `WrongEntityType` for an entity that is not an
/// `IfcShapeModel`; `StillReferenced` for a representation map an
/// `IfcMappedItem` still maps; `InvalidArgument` for another flag;
/// `FeatureDisabled`.
///
/// # Safety
/// As for `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_remove_representation(
    model: OpenbimIfcModel,
    representation: u64,
    flags: u32,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    if flags & !OPENBIM_IFC_REMOVE_DRY_RUN != 0 {
        return OpenbimIfcStatus::InvalidArgument;
    }
    let out = Out {
        nodes,
        node_capacity,
        out_nodes_required,
        strings,
        string_capacity,
        out_strings_required,
    };
    let dry_run = flags & OPENBIM_IFC_REMOVE_DRY_RUN != 0;
    // SAFETY: caller contract above.
    unsafe {
        removal_export(model, &out, dry_run, |m, plan| {
            m.remove_representation(representation, plan)
        })
    }
}

/// Purge unreferenced entities as one checked transaction. With
/// `class_count` 0, every resource entity no `IfcRoot` reaches (nor
/// annotates something reached); otherwise entities of the `class_count`
/// classes (UTF-8 names of `class_lengths[i]` bytes, any case, subtypes
/// included) that nothing references. `flags`:
/// `OPENBIM_IFC_PURGE_INCLUDE_ROOTS`,
/// `OPENBIM_IFC_PURGE_INCLUDE_OWNER_HISTORY`, `OPENBIM_IFC_PURGE_DRY_RUN`.
/// The answer is a `RemovalReport` tape: removed (`LIST` of
/// `RemovedEntity`: id `REF`, type name `TEXT`), kept (`LIST` of
/// `KeptEntity`: id, type name, reason `TEXT` -- `shared`,
/// `related-object`, `definition`, `protected` -- users `LIST` of `REF`),
/// detached and rebased (`LIST`s of `REF`), applied (`BOOL`, false for a
/// dry run). A size query or a short buffer plans the removal and answers
/// `BufferTooSmall` without applying it.
///
/// `UnsupportedSchema` for a class the release does not declare;
/// `InvalidArgument` for a class that is not UTF-8 or an unknown flag;
/// `FeatureDisabled`.
///
/// # Safety
/// `classes` and `class_lengths` null (with `class_count` 0) or valid for
/// `class_count` reads, each class valid for its length; the buffers as
/// for `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_purge(
    model: OpenbimIfcModel,
    classes: *const *const u8,
    class_lengths: *const usize,
    class_count: usize,
    flags: u32,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    if flags & !ALL_FLAGS != 0 {
        return OpenbimIfcStatus::InvalidArgument;
    }
    if class_count > 0 && (classes.is_null() || class_lengths.is_null()) {
        return OpenbimIfcStatus::NullPointer;
    }
    let mut names = Vec::with_capacity(class_count);
    for index in 0..class_count {
        // SAFETY: both arrays hold `class_count` items (caller contract).
        let (class, length) = unsafe { (*classes.add(index), *class_lengths.add(index)) };
        // SAFETY: each class is valid for its length (caller contract).
        match unsafe { text(class, length) } {
            Ok(name) => names.push(name.to_owned()),
            Err(status) => return status,
        }
    }
    let out = Out {
        nodes,
        node_capacity,
        out_nodes_required,
        strings,
        string_capacity,
        out_strings_required,
    };
    let dry_run = flags & OPENBIM_IFC_PURGE_DRY_RUN != 0;
    // SAFETY: caller contract above.
    unsafe {
        removal_export(model, &out, dry_run, |m, plan| {
            let mut options = PurgeOptions::new();
            options.classes.clone_from(&names);
            options.include_roots = flags & OPENBIM_IFC_PURGE_INCLUDE_ROOTS != 0;
            options.include_owner_history = flags & OPENBIM_IFC_PURGE_INCLUDE_OWNER_HISTORY != 0;
            options.dry_run = plan;
            m.purge(options)
        })
    }
}
