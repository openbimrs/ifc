//! Copying a product with what it solely owns, and appending entities from
//! another model (#479).
//!
//! Each answers with a `CopyReport` record tape: copy (`REF`, the product's
//! copy, or `NULL` for an append), copied and reused (`LIST`s of
//! `CopiedEntity`: original `REF`, copy `REF`, type name `TEXT`), joined,
//! skipped and regenerated (`LIST`s of `REF`), applied (`BOOL`, false for a
//! dry run).
//!
//! As for the removals, the buffer protocol never applies a copy it cannot
//! report: the call plans it first, and a size query or a short buffer
//! answers `BufferTooSmall` with the sizes and leaves the model unchanged.
//! The applied report has the planned one's size: planning and applying
//! allocate the same ids.

use openbim_ifc_binding_core::{
    AppendOptions, BindingError, CopyProductOptions, CopyReport, IfcModel, ToRecord,
};

use crate::domains::Out;
use crate::model::{with_model, OpenbimIfcModel};
use crate::registry::{self, lock, Entry};
use crate::status::{boundary, OpenbimIfcStatus};
use crate::tape::{OpenbimIfcValueNode, Tape};

/// `openbim_ifc_v0_1_model_copy_product` flag: do not copy the openings.
pub const OPENBIM_IFC_COPY_NO_OPENINGS: u32 = 1;
/// `openbim_ifc_v0_1_model_copy_product` flag: do not copy the fillings of
/// the copied openings.
pub const OPENBIM_IFC_COPY_NO_FILLINGS: u32 = 2;
/// `openbim_ifc_v0_1_model_copy_product` flag: do not contain the copy in
/// the original's container.
pub const OPENBIM_IFC_COPY_NO_CONTAINER: u32 = 4;
/// `openbim_ifc_v0_1_model_copy_product` flag: do not make the copy a part
/// of the original's whole.
pub const OPENBIM_IFC_COPY_NO_AGGREGATE: u32 = 8;
/// `openbim_ifc_v0_1_model_copy_product` flag: report only.
pub const OPENBIM_IFC_COPY_DRY_RUN: u32 = 16;

/// `openbim_ifc_v0_1_model_append` `GlobalId` mode: keep, refusing one the
/// target holds.
pub const OPENBIM_IFC_GLOBAL_IDS_KEEP: u32 = 0;
/// `openbim_ifc_v0_1_model_append` `GlobalId` mode: regenerate every one.
pub const OPENBIM_IFC_GLOBAL_IDS_REGENERATE: u32 = 1;
/// `openbim_ifc_v0_1_model_append` `GlobalId` mode: regenerate the ones the
/// target holds.
pub const OPENBIM_IFC_GLOBAL_IDS_REGENERATE_COLLIDING: u32 = 2;
/// `openbim_ifc_v0_1_model_append` `GlobalId` mode: an `IfcRoot` the target
/// holds by `GlobalId` and type is reused, its relationships joined.
pub const OPENBIM_IFC_GLOBAL_IDS_MERGE: u32 = 3;

/// `openbim_ifc_v0_1_model_append` flag: copy units equal to the target's.
pub const OPENBIM_IFC_APPEND_NO_REUSE_UNITS: u32 = 1;
/// `openbim_ifc_v0_1_model_append` flag: copy contexts equal to the
/// target's.
pub const OPENBIM_IFC_APPEND_NO_REUSE_CONTEXTS: u32 = 2;
/// `openbim_ifc_v0_1_model_append` flag: copy an owner history equal to
/// the target's.
pub const OPENBIM_IFC_APPEND_NO_REUSE_OWNER_HISTORY: u32 = 4;
/// `openbim_ifc_v0_1_model_append` flag: copy materials equal to the
/// target's.
pub const OPENBIM_IFC_APPEND_NO_REUSE_MATERIALS: u32 = 8;
/// `openbim_ifc_v0_1_model_append` flag: copy styles equal to the target's.
pub const OPENBIM_IFC_APPEND_NO_REUSE_STYLES: u32 = 16;
/// `openbim_ifc_v0_1_model_append` flag: report only.
pub const OPENBIM_IFC_APPEND_DRY_RUN: u32 = 32;

const COPY_FLAGS: u32 = 31;
const APPEND_FLAGS: u32 = 63;

/// Plan with `run(model, true)`; then write the report, applying with
/// `run(model, false)` first unless `dry_run`, and only when it fits.
///
/// # Safety
/// The buffers as for `Out`.
unsafe fn copy_export(
    model: &mut IfcModel,
    out: &Out,
    dry_run: bool,
    run: impl Fn(&mut IfcModel, bool) -> Result<CopyReport, BindingError>,
) -> Result<OpenbimIfcStatus, BindingError> {
    if let Err(status) = out.check() {
        return Ok(status);
    }
    let planned = run(model, true)?;
    let tape = Tape::encode(&planned.to_record().to_tagged());
    let fits = tape.nodes.len() <= out.node_capacity && tape.strings.len() <= out.string_capacity;
    let report = if dry_run || !fits {
        planned
    } else {
        run(model, false)?
    };
    // SAFETY: caller contract.
    unsafe { out.write(&report.to_record().to_tagged()) }
}

/// Copy product (or type object) `product` with everything it solely owns,
/// as one checked transaction: its placement, representation, own property
/// and quantity sets, material usage, ports and, unless a flag says
/// otherwise, its openings and the doors and windows filling them; joining
/// its type, shared property sets, classification and, unless a flag says
/// otherwise, its container and whole. `placement` non-zero places the
/// copy by that `IfcObjectPlacement`; `offset` non-null (three doubles)
/// moves it in its parent placement's frame; not both. `flags`:
/// `OPENBIM_IFC_COPY_NO_OPENINGS`, `OPENBIM_IFC_COPY_NO_FILLINGS`,
/// `OPENBIM_IFC_COPY_NO_CONTAINER`, `OPENBIM_IFC_COPY_NO_AGGREGATE`,
/// `OPENBIM_IFC_COPY_DRY_RUN`. The answer is a `CopyReport` tape (see the
/// module docs); a size query or a short buffer plans the copy and answers
/// `BufferTooSmall` without applying it.
///
/// `MissingEntity`; `WrongEntityType` for neither an `IfcProduct` nor an
/// `IfcTypeObject`; `Unsupported` for an offset onto a placement that is
/// not a local placement with a Cartesian location, or a moved copy whose
/// openings are not placed relative to it; `InvalidValue` for both a
/// placement and an offset, or a non-finite offset; `InvalidArgument` for
/// an unknown flag; `FeatureDisabled`.
///
/// # Safety
/// `offset` null or valid for three reads; the buffers as for
/// `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_copy_product(
    model: OpenbimIfcModel,
    product: u64,
    placement: u64,
    offset: *const f64,
    flags: u32,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    if flags & !COPY_FLAGS != 0 {
        return OpenbimIfcStatus::InvalidArgument;
    }
    let mut options = CopyProductOptions::new();
    if !offset.is_null() {
        // SAFETY: three reads (caller contract).
        options.offset = Some(unsafe { [*offset, *offset.add(1), *offset.add(2)] });
    }
    options.placement = (placement != 0).then_some(placement);
    options.openings = flags & OPENBIM_IFC_COPY_NO_OPENINGS == 0;
    options.fillings = flags & OPENBIM_IFC_COPY_NO_FILLINGS == 0;
    options.container = flags & OPENBIM_IFC_COPY_NO_CONTAINER == 0;
    options.aggregate = flags & OPENBIM_IFC_COPY_NO_AGGREGATE == 0;
    let dry_run = flags & OPENBIM_IFC_COPY_DRY_RUN != 0;
    let out = Out {
        nodes,
        node_capacity,
        out_nodes_required,
        strings,
        string_capacity,
        out_strings_required,
    };
    boundary(|| {
        with_model(model, |m| {
            // SAFETY: caller contract above.
            unsafe {
                copy_export(m, &out, dry_run, |m, plan| {
                    let mut options = options;
                    options.dry_run = plan;
                    m.copy_product(product, options)
                })
            }
        })
    })
}

/// Copy the `id_count` entities `ids` of model `source` into model
/// `target` with what belongs to them, as one checked transaction: what
/// they reference, their property sets, type, material and classification,
/// their openings, fillings and ports, and what attaches to their geometry.
/// Units, contexts, the owner history, materials and styles equal to the
/// target's are reused unless a flag says otherwise. Relationships to
/// objects not copied (a spatial container) are skipped and reported.
/// `global_ids`: `OPENBIM_IFC_GLOBAL_IDS_KEEP`, `_REGENERATE`,
/// `_REGENERATE_COLLIDING` or `_MERGE`. `flags`:
/// `OPENBIM_IFC_APPEND_NO_REUSE_UNITS`, `_CONTEXTS`, `_OWNER_HISTORY`,
/// `_MATERIALS`, `_STYLES`, `OPENBIM_IFC_APPEND_DRY_RUN`. The answer is a
/// `CopyReport` tape (see the module docs); a size query or a short buffer
/// plans the append and answers `BufferTooSmall` without applying it.
///
/// `InvalidHandle`; `InvalidArgument` for `target` and `source` the same
/// model, an unknown mode or flag; `MissingEntity` for an id `source`
/// lacks; `Unsupported` when the models' length or plane angle units
/// differ; `UnsupportedSchema` for an entity the target's release cannot
/// hold; `InvalidValue` for a kept `GlobalId` the target holds;
/// `FeatureDisabled`.
///
/// # Safety
/// `ids` null (with `id_count` 0) or valid for `id_count` reads; the
/// buffers as for `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_append(
    target: OpenbimIfcModel,
    source: OpenbimIfcModel,
    ids: *const u64,
    id_count: usize,
    global_ids: u32,
    flags: u32,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    if flags & !APPEND_FLAGS != 0 {
        return OpenbimIfcStatus::InvalidArgument;
    }
    let mode = match global_ids {
        OPENBIM_IFC_GLOBAL_IDS_KEEP => "keep",
        OPENBIM_IFC_GLOBAL_IDS_REGENERATE => "regenerate",
        OPENBIM_IFC_GLOBAL_IDS_REGENERATE_COLLIDING => "regenerate-colliding",
        OPENBIM_IFC_GLOBAL_IDS_MERGE => "merge",
        _ => return OpenbimIfcStatus::InvalidArgument,
    };
    if id_count > 0 && ids.is_null() {
        return OpenbimIfcStatus::NullPointer;
    }
    let roots: Vec<u64> = if id_count == 0 {
        Vec::new()
    } else {
        // SAFETY: `id_count` reads (caller contract).
        unsafe { std::slice::from_raw_parts(ids, id_count) }.to_vec()
    };
    let mut options = AppendOptions::new();
    options.global_ids = mode.to_owned();
    options.reuse_units = flags & OPENBIM_IFC_APPEND_NO_REUSE_UNITS == 0;
    options.reuse_contexts = flags & OPENBIM_IFC_APPEND_NO_REUSE_CONTEXTS == 0;
    options.reuse_owner_history = flags & OPENBIM_IFC_APPEND_NO_REUSE_OWNER_HISTORY == 0;
    options.reuse_materials = flags & OPENBIM_IFC_APPEND_NO_REUSE_MATERIALS == 0;
    options.reuse_styles = flags & OPENBIM_IFC_APPEND_NO_REUSE_STYLES == 0;
    let dry_run = flags & OPENBIM_IFC_APPEND_DRY_RUN != 0;
    let out = Out {
        nodes,
        node_capacity,
        out_nodes_required,
        strings,
        string_capacity,
        out_strings_required,
    };
    if target == source {
        return OpenbimIfcStatus::InvalidArgument;
    }
    let (Some(into), Some(from)) = (registry::get(target), registry::get(source)) else {
        return OpenbimIfcStatus::InvalidHandle;
    };
    boundary(|| {
        // Both locks, always in handle order, so two appends between the
        // same models in opposite directions cannot deadlock.
        let (mut into, from) = if target < source {
            let into = lock(&into);
            (into, lock(&from))
        } else {
            let from = lock(&from);
            (lock(&into), from)
        };
        let Entry { model, last_error } = &mut *into;
        // SAFETY: caller contract above.
        let result = unsafe {
            copy_export(model, &out, dry_run, |m, plan| {
                let mut options = options.clone();
                options.dry_run = plan;
                m.append(&from.model, &roots, &options)
            })
        };
        match result {
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
    })
}
