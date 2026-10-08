//! Geometry (#328): placements as plain records, meshes with their arrays
//! as typed arrays.
//!
//! A mesh's positions and indices are copied into a fresh `Float32Array`
//! and `Uint32Array`, owned by JavaScript: ready for a WebGL buffer or a
//! three.js `BufferAttribute`, and still valid after the model is freed.

use js_sys::{Array, BigInt, BigUint64Array, Float32Array, Object, Reflect, Uint32Array};
use openbim_ifc_binding_core::{BindingError, ProductMesh, ToRecord};
use wasm_bindgen::{JsCast, JsValue};

use crate::records::record_to_js;

/// `undefined`/`null` (every product with a shape), or the ids as a
/// `bigint[]` or `BigUint64Array`.
pub(crate) fn ids(value: &JsValue) -> Result<Option<Vec<u64>>, BindingError> {
    if value.is_undefined() || value.is_null() {
        return Ok(None);
    }
    if let Some(typed) = value.dyn_ref::<BigUint64Array>() {
        return Ok(Some(typed.to_vec()));
    }
    if !Array::is_array(value) {
        return Err(BindingError::InvalidValue(
            "ids must be an array of bigint, a BigUint64Array or undefined".into(),
        ));
    }
    Array::from(value)
        .iter()
        .map(|id| {
            id.dyn_into::<BigInt>()
                .ok()
                .and_then(|id| u64::try_from(id).ok())
                .ok_or_else(|| {
                    BindingError::InvalidValue("an id must be a non-negative bigint".into())
                })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

/// `ProductMesh` objects: the core's record plus `positions` and
/// `indices`.
pub(crate) fn meshes_to_js(meshes: &[ProductMesh]) -> Array {
    meshes
        .iter()
        .map(|mesh| {
            let object: Object = record_to_js(&mesh.to_record()).unchecked_into();
            // Plain properties on a fresh plain object; setting cannot fail.
            let _ = Reflect::set(
                &object,
                &"positions".into(),
                &Float32Array::from(mesh.positions.as_slice()),
            );
            let _ = Reflect::set(
                &object,
                &"indices".into(),
                &Uint32Array::from(mesh.indices.as_slice()),
            );
            JsValue::from(object)
        })
        .collect()
}
