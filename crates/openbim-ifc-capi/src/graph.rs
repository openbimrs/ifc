//! Geometry, Level 2 (#367, ADR 0021): each product's Body as Axiolid's
//! neutral geometry graph, in Axiolid's versioned wire format 1.0.
//!
//! As for meshes, lowering and encoding are the expensive step and the C
//! protocol sizes every buffer with a first call, so the graphs are
//! encoded once into an opaque set ([`OpenbimIfcGraphs`], a non-zero `u64`
//! like a model handle) that the host reads record by record and payload
//! by payload, then destroys. The set owns its data: it stays valid after
//! the model changes or is destroyed.

use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{GeometryEncoding, ProductGeometry, ToRecord};

use crate::buffer::{fill, put};
use crate::domains::Out;
use crate::geometry::selection;
use crate::model::{with_model, OpenbimIfcModel};
use crate::registry;
use crate::status::{boundary, OpenbimIfcStatus};
use crate::tape::OpenbimIfcValueNode;

/// Opaque handle to an encoded graph set. Zero is never a valid handle.
pub type OpenbimIfcGraphs = u64;

/// `encoding` for `openbim_ifc_v0_1_model_product_geometry`: JSON text
/// (RFC 8259), UTF-8, not NUL-terminated.
pub const OPENBIM_IFC_GEOMETRY_JSON: u32 = 0;
/// `encoding` for `openbim_ifc_v0_1_model_product_geometry`: CBOR bytes
/// (RFC 8949).
pub const OPENBIM_IFC_GEOMETRY_CBOR: u32 = 1;

/// Lower the Body of each of `ids` (`id_count` of them) or, with `ids` null
/// and `id_count` 0, of every product with a shape, encode each graph in
/// Axiolid's wire format 1.0 as `encoding` (`OPENBIM_IFC_GEOMETRY_JSON` or
/// `_CBOR`), and write the new set's handle to `out_graphs`. Read it with
/// `openbim_ifc_v0_1_graphs_records` and `_graphs_payload`; destroy it with
/// `openbim_ifc_v0_1_graphs_destroy`.
///
/// The payload is the envelope `{"format":"axiolid-geometry-graph",
/// "version":"1.0","graph":{"nodes":[...],"roots":[...]}}`, in world
/// coordinates, metres. A product that cannot be lowered is a record with
/// a refusal, not a failed call. `InvalidArgument` for another `encoding`;
/// `UnsupportedSchema`; `FeatureDisabled` in a library built without the
/// `graph` feature.
///
/// # Safety
/// `ids` valid for `id_count` reads when non-null; `out_graphs` valid for
/// one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_product_geometry(
    model: OpenbimIfcModel,
    ids: *const u64,
    id_count: usize,
    encoding: u32,
    out_graphs: *mut OpenbimIfcGraphs,
) -> OpenbimIfcStatus {
    boundary(|| {
        if out_graphs.is_null() {
            return OpenbimIfcStatus::NullPointer;
        }
        let encoding = match encoding {
            OPENBIM_IFC_GEOMETRY_JSON => GeometryEncoding::Json,
            OPENBIM_IFC_GEOMETRY_CBOR => GeometryEncoding::Cbor,
            _ => return OpenbimIfcStatus::InvalidArgument,
        };
        // SAFETY: caller contract above.
        let ids = match unsafe { selection(ids, id_count) } {
            Ok(ids) => ids,
            Err(status) => return status,
        };
        with_model(model, |m| {
            let handle = registry::insert_graphs(m.product_geometry(ids, encoding)?);
            // SAFETY: checked non-null; caller contract above.
            Ok(unsafe { put(out_graphs, handle) }
                .err()
                .unwrap_or(OpenbimIfcStatus::Ok))
        })
    })
}

/// Run `read` on the live set behind `graphs`.
fn with_graphs(
    graphs: OpenbimIfcGraphs,
    read: impl FnOnce(&[ProductGeometry]) -> OpenbimIfcStatus,
) -> OpenbimIfcStatus {
    boundary(|| match registry::graphs(graphs) {
        Some(set) => read(&set),
        None => OpenbimIfcStatus::InvalidHandle,
    })
}

/// The set's `ProductGeometry` records as a tape, in the order lowered;
/// `out_count` gets their number. `ProductGeometry`: id (`REF`), global
/// id, type name, transform (`LIST` of 16 `REAL`s, column-major, metres,
/// already applied to the graph, or `NULL`), encoding (`json` or `cbor`),
/// payload size (`INTEGER`, bytes), refusal (`GeometryRefusal` or `NULL`).
/// A payload size of 0 and no refusal is a product with no Body
/// representation.
///
/// # Safety
/// `out_count` valid for one write; otherwise as for
/// `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_graphs_records(
    graphs: OpenbimIfcGraphs,
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
    with_graphs(graphs, |set| {
        // SAFETY: caller contract above.
        if let Err(status) = unsafe { put(out_count, set.len()) } {
            return status;
        }
        let tape = Tagged::List(
            set.iter()
                .map(|graph| graph.to_record().to_tagged())
                .collect(),
        );
        // SAFETY: caller contract above.
        unsafe { out.write(&tape) }.unwrap_or_else(|error| OpenbimIfcStatus::from(&error))
    })
}

/// Copy the wire payload of graph `index` -- JSON text without a trailing
/// NUL, or CBOR bytes -- into `buffer` (`capacity` bytes), after writing
/// the size needed (its record's payload size) to `out_required`; 0 for a
/// product with no graph. `OutOfRange` for an index past the set.
///
/// # Safety
/// As for the other buffer calls: `buffer` valid for `capacity` writes
/// when non-null, `out_required` for one.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_graphs_payload(
    graphs: OpenbimIfcGraphs,
    index: usize,
    buffer: *mut u8,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    with_graphs(graphs, |set| match set.get(index) {
        // SAFETY: caller contract above.
        Some(graph) => unsafe {
            fill(
                graph.payload.as_deref().unwrap_or_default(),
                buffer,
                capacity,
                out_required,
            )
        },
        None => OpenbimIfcStatus::OutOfRange,
    })
}

/// Destroy a graph set. A stale or repeated handle is `InvalidHandle`.
#[no_mangle]
pub extern "C" fn openbim_ifc_v0_1_graphs_destroy(graphs: OpenbimIfcGraphs) -> OpenbimIfcStatus {
    boundary(|| {
        if registry::remove_graphs(graphs) {
            OpenbimIfcStatus::Ok
        } else {
            OpenbimIfcStatus::InvalidHandle
        }
    })
}
