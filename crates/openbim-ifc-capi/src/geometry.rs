//! Geometry (#328, ADR 0021): placements as a record tape, meshes as a
//! handle to a compiled set whose arrays the caller copies out.
//!
//! Compiling meshes is the expensive step, and the C protocol sizes every
//! buffer with a first call, so meshes are compiled once into an opaque
//! set ([`OpenbimIfcMeshes`], a non-zero `u64` like a model handle) that the
//! host reads record by record and array by array, then destroys. The set
//! owns its data: it stays valid after the model changes or is destroyed.

use openbim_ifc_binding_core::record::to_records;
use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{ProductMesh, ToRecord};

use crate::buffer::{fill, put, slice};
use crate::domains::{list_export, Out};
use crate::model::{with_model, OpenbimIfcModel};
use crate::registry;
use crate::status::{boundary, OpenbimIfcStatus};
use crate::tape::OpenbimIfcValueNode;

/// Opaque handle to a compiled mesh set. Zero is never a valid handle.
pub type OpenbimIfcMeshes = u64;

/// An id selection: null with `id_count` 0 is the call's default (every
/// product with a shape, every object definition); otherwise `id_count`
/// ids.
///
/// # Safety
/// `ids`, if non-null, valid for `id_count` reads.
pub(crate) unsafe fn selection<'a>(
    ids: *const u64,
    id_count: usize,
) -> Result<Option<&'a [u64]>, OpenbimIfcStatus> {
    if ids.is_null() {
        return if id_count == 0 {
            Ok(None)
        } else {
            Err(OpenbimIfcStatus::NullPointer)
        };
    }
    // SAFETY: forwarded caller contract.
    unsafe { slice(ids, id_count) }.map(Some)
}

/// Each product's world placement and selected Body representation, for
/// `ids` (`id_count` of them) or, with `ids` null and `id_count` 0, every
/// product with a shape, in id order. The tape is a `LIST` of
/// `ProductPlacement` records; `out_count` gets their number.
/// `ProductPlacement`: id (`REF`), global id, type name, transform (`LIST`
/// of 16 `REAL`s, a column-major 4x4 in metres, or `NULL`), representation
/// (`SelectedRepresentation` or `NULL`), refusal (`GeometryRefusal` or
/// `NULL`). `SelectedRepresentation`: id, identifier, representation type,
/// context (`REF` or `NULL`), context type, context identifier, target
/// view. `GeometryRefusal`: code (`unsupported`, `invalid-model`,
/// `missing-reference` or `budget-exceeded`), entity (`REF` or `NULL`),
/// message.
///
/// A product that cannot be placed is a record with a refusal, not a
/// failed call. `UnsupportedSchema`, `FeatureDisabled`.
///
/// # Safety
/// `ids` valid for `id_count` reads when non-null; `out_count` valid for
/// one write; otherwise as for `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_product_placements(
    model: OpenbimIfcModel,
    ids: *const u64,
    id_count: usize,
    out_count: *mut usize,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    // SAFETY: caller contract above.
    let ids = match unsafe { selection(ids, id_count) } {
        Ok(ids) => ids,
        Err(status) => return status,
    };
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
            Ok(to_records(&m.product_placements(ids)?))
        })
    }
}

/// Compile the Body mesh of each of `ids` (`id_count` of them) or, with
/// `ids` null and `id_count` 0, of every product with a shape, and write
/// the new set's handle to `out_meshes`. Read it with
/// `openbim_ifc_v0_1_meshes_records`, `_meshes_positions` and
/// `_meshes_indices`; destroy it with `openbim_ifc_v0_1_meshes_destroy`.
///
/// A product that cannot be meshed is a record with a refusal, not a
/// failed call. `UnsupportedSchema`; `FeatureDisabled` in a library built
/// without the `mesh` feature (the default build).
///
/// # Safety
/// `ids` valid for `id_count` reads when non-null; `out_meshes` valid for
/// one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_product_meshes(
    model: OpenbimIfcModel,
    ids: *const u64,
    id_count: usize,
    out_meshes: *mut OpenbimIfcMeshes,
) -> OpenbimIfcStatus {
    boundary(|| {
        if out_meshes.is_null() {
            return OpenbimIfcStatus::NullPointer;
        }
        // SAFETY: caller contract above.
        let ids = match unsafe { selection(ids, id_count) } {
            Ok(ids) => ids,
            Err(status) => return status,
        };
        with_model(model, |m| {
            let handle = registry::insert_meshes(m.product_meshes(ids)?);
            // SAFETY: checked non-null; caller contract above.
            Ok(unsafe { put(out_meshes, handle) }
                .err()
                .unwrap_or(OpenbimIfcStatus::Ok))
        })
    })
}

/// Run `read` on the live set behind `meshes`.
fn with_meshes(
    meshes: OpenbimIfcMeshes,
    read: impl FnOnce(&[ProductMesh]) -> OpenbimIfcStatus,
) -> OpenbimIfcStatus {
    boundary(|| match registry::meshes(meshes) {
        Some(set) => read(&set),
        None => OpenbimIfcStatus::InvalidHandle,
    })
}

/// The set's `ProductMesh` records as a tape, in the order compiled;
/// `out_count` gets their number. `ProductMesh`: id (`REF`), global id,
/// type name, transform (`LIST` of 16 `REAL`s, column-major, metres, or
/// `NULL`), vertex count, triangle count (`INTEGER`s), refusal
/// (`GeometryRefusal` or `NULL`). Empty arrays and no refusal is a
/// product with no Body representation.
///
/// # Safety
/// `out_count` valid for one write; otherwise as for
/// `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_meshes_records(
    meshes: OpenbimIfcMeshes,
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
    with_meshes(meshes, |set| {
        // SAFETY: caller contract above.
        if let Err(status) = unsafe { put(out_count, set.len()) } {
            return status;
        }
        let tape = Tagged::List(
            set.iter()
                .map(|mesh| mesh.to_record().to_tagged())
                .collect(),
        );
        // SAFETY: caller contract above.
        unsafe { out.write(&tape) }.unwrap_or_else(|error| OpenbimIfcStatus::from(&error))
    })
}

/// Copy the positions of mesh `index` -- `x, y, z` per vertex, metres,
/// relative to its record's transform -- into `buffer` (`capacity`
/// floats), after writing the count needed (3 x vertex count) to
/// `out_required`. `OutOfRange` for an index past the set.
///
/// # Safety
/// As for the other buffer calls: `buffer` valid for `capacity` writes
/// when non-null, `out_required` for one.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_meshes_positions(
    meshes: OpenbimIfcMeshes,
    index: usize,
    buffer: *mut f32,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    with_meshes(meshes, |set| match set.get(index) {
        // SAFETY: caller contract above.
        Some(mesh) => unsafe { fill(&mesh.positions, buffer, capacity, out_required) },
        None => OpenbimIfcStatus::OutOfRange,
    })
}

/// Copy the triangle indices of mesh `index` -- three vertex indices per
/// triangle -- into `buffer` (`capacity` values), after writing the count
/// needed (3 x triangle count) to `out_required`. `OutOfRange` for an
/// index past the set.
///
/// # Safety
/// As for `openbim_ifc_v0_1_meshes_positions`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_meshes_indices(
    meshes: OpenbimIfcMeshes,
    index: usize,
    buffer: *mut u32,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    with_meshes(meshes, |set| match set.get(index) {
        // SAFETY: caller contract above.
        Some(mesh) => unsafe { fill(&mesh.indices, buffer, capacity, out_required) },
        None => OpenbimIfcStatus::OutOfRange,
    })
}

/// Destroy a mesh set. A stale or repeated handle is `InvalidHandle`.
#[no_mangle]
pub extern "C" fn openbim_ifc_v0_1_meshes_destroy(meshes: OpenbimIfcMeshes) -> OpenbimIfcStatus {
    boundary(|| {
        if registry::remove_meshes(meshes) {
            OpenbimIfcStatus::Ok
        } else {
            OpenbimIfcStatus::InvalidHandle
        }
    })
}
