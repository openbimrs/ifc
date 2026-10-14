//! Polylines (#458): the curves of a curve representation as a handle to a
//! compiled set, read as a mesh set is.
//!
//! One record per product, whose `ProductPolyline` records each name a
//! slice of the product's one position array: the host copies the array
//! once and draws each slice as a line strip. The set owns its data, as a
//! mesh set does.

use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{ProductCurves, ToRecord};

use crate::buffer::{fill, put, text};
use crate::domains::Out;
use crate::geometry::selection;
use crate::model::{with_model, OpenbimIfcModel};
use crate::registry;
use crate::status::{boundary, OpenbimIfcStatus};
use crate::tape::OpenbimIfcValueNode;

/// Opaque handle to a compiled polyline set. Zero is never a valid handle.
pub type OpenbimIfcCurves = u64;

/// Compile the curves of each product's `representation` as polylines
/// (#458), for `ids` (`id_count` of them) or, with `ids` null and
/// `id_count` 0, every product with a shape, and write the new set's
/// handle to `out_curves`. `representation` is `representation_len` bytes
/// of UTF-8: `Plan` (the best drawing curves: `Plan`, `Annotation`,
/// `FootPrint`, then `Axis`, preferring a `PLAN_VIEW` context), `Axis`,
/// `FootPrint`, `Annotation`, `Body` or `Surface`. Read the set with
/// `openbim_ifc_v0_1_curves_records` and `_curves_positions`; destroy it
/// with `openbim_ifc_v0_1_curves_destroy`. The chord budget is one
/// millimetre.
///
/// A product whose curves cannot be compiled is a record with a refusal,
/// not a failed call: a trimmed, composite, offset or intrinsic curve is
/// `unsupported`, naming the curve, until the reference compiler
/// polylines it (#506). `InvalidValue` for another representation name,
/// `InvalidArgument` for text that is not UTF-8, `UnsupportedSchema`;
/// `FeatureDisabled` in a library built without the `mesh` feature.
///
/// # Safety
/// `ids` valid for `id_count` reads when non-null; `representation` valid
/// for `representation_len` reads; `out_curves` valid for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_product_curves(
    model: OpenbimIfcModel,
    ids: *const u64,
    id_count: usize,
    representation: *const u8,
    representation_len: usize,
    out_curves: *mut OpenbimIfcCurves,
) -> OpenbimIfcStatus {
    boundary(|| {
        if out_curves.is_null() {
            return OpenbimIfcStatus::NullPointer;
        }
        // SAFETY: caller contract above.
        let inputs = unsafe {
            (
                selection(ids, id_count),
                text(representation, representation_len),
            )
        };
        let (ids, representation) = match inputs {
            (Ok(ids), Ok(representation)) => (ids, representation),
            (Err(status), _) | (_, Err(status)) => return status,
        };
        with_model(model, |m| {
            let handle = registry::insert_curves(m.product_curves(ids, representation)?);
            // SAFETY: checked non-null; caller contract above.
            Ok(unsafe { put(out_curves, handle) }
                .err()
                .unwrap_or(OpenbimIfcStatus::Ok))
        })
    })
}

/// Run `read` on the live set behind `curves`.
fn with_curves(
    curves: OpenbimIfcCurves,
    read: impl FnOnce(&[ProductCurves]) -> OpenbimIfcStatus,
) -> OpenbimIfcStatus {
    boundary(|| match registry::curves(curves) {
        Some(set) => read(&set),
        None => OpenbimIfcStatus::InvalidHandle,
    })
}

/// The set's `ProductCurves` records as a tape, in the order compiled;
/// `out_count` gets their number. `ProductCurves`: id (`REF`), global id,
/// type name, transform (`LIST` of 16 `REAL`s, column-major, metres, or
/// `NULL`), vertex count (`INTEGER`), polylines (`LIST` of
/// `ProductPolyline`), refusal (`GeometryRefusal` or `NULL`).
/// `ProductPolyline`: curve (`REF` of the IFC curve, or `NULL`), first
/// vertex and vertex count (`INTEGER`s: its slice of the positions),
/// closed (`BOOL`: its last vertex repeats its first), chord bound and
/// chord requested (`REAL`s, metres: the certified bound on how far the
/// exact curve lies from the polyline, `0` for a polyline the file
/// authored, and the budget). No polylines and no refusal is a product
/// without the representation, or with no curves.
///
/// # Safety
/// `out_count` valid for one write; otherwise as for
/// `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_curves_records(
    curves: OpenbimIfcCurves,
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
    with_curves(curves, |set| {
        // SAFETY: caller contract above.
        if let Err(status) = unsafe { put(out_count, set.len()) } {
            return status;
        }
        let tape = Tagged::List(
            set.iter()
                .map(|product| product.to_record().to_tagged())
                .collect(),
        );
        // SAFETY: caller contract above.
        unsafe { out.write(&tape) }.unwrap_or_else(|error| OpenbimIfcStatus::from(&error))
    })
}

/// Copy the positions of product `index` -- `x, y, z` per vertex, metres,
/// relative to its record's transform, every polyline's vertices in turn
/// -- into `buffer` (`capacity` floats), after writing the count needed
/// (3 x vertex count) to `out_required`. `OutOfRange` for an index past
/// the set.
///
/// # Safety
/// As for `openbim_ifc_v0_1_meshes_positions`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_curves_positions(
    curves: OpenbimIfcCurves,
    index: usize,
    buffer: *mut f32,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    with_curves(curves, |set| match set.get(index) {
        // SAFETY: caller contract above.
        Some(product) => unsafe { fill(&product.positions, buffer, capacity, out_required) },
        None => OpenbimIfcStatus::OutOfRange,
    })
}

/// Destroy a polyline set. A stale or repeated handle is `InvalidHandle`.
#[no_mangle]
pub extern "C" fn openbim_ifc_v0_1_curves_destroy(curves: OpenbimIfcCurves) -> OpenbimIfcStatus {
    boundary(|| {
        if registry::remove_curves(curves) {
            OpenbimIfcStatus::Ok
        } else {
            OpenbimIfcStatus::InvalidHandle
        }
    })
}
