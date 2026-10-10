//! Geometry (#328, #367): placements as plain records, graphs with their
//! wire payload as a string, a parsed object or a `Uint8Array`, meshes with
//! their arrays as typed arrays.
//!
//! A mesh's positions and indices are copied into a fresh `Float32Array`
//! and `Uint32Array`, owned by JavaScript: ready for a WebGL buffer or a
//! three.js `BufferAttribute`, and still valid after the model is freed.

use js_sys::{
    Array, BigInt, BigUint64Array, Float32Array, Object, Reflect, Uint32Array, Uint8Array, JSON,
};
use openbim_ifc_binding_core::{
    BindingError, GeometryEncoding, ProductGeometry, ProductMesh, ToRecord,
};
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

/// How `productGeometry` hands each payload over: `json` (the default) a
/// string, `object` that string parsed, `cbor` a `Uint8Array`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PayloadForm {
    Json,
    Object,
    Cbor,
}

impl PayloadForm {
    /// `undefined` or `"json"`, `"object"`, `"cbor"`.
    pub(crate) fn from_js(value: &JsValue) -> Result<Self, BindingError> {
        if value.is_undefined() || value.is_null() {
            return Ok(Self::Json);
        }
        match value.as_string().as_deref() {
            Some("json") => Ok(Self::Json),
            Some("object") => Ok(Self::Object),
            Some("cbor") => Ok(Self::Cbor),
            _ => Err(BindingError::InvalidValue(
                "encoding must be \"json\", \"object\", \"cbor\" or undefined".into(),
            )),
        }
    }

    /// The wire encoding the core writes: an object is parsed JSON.
    pub(crate) fn encoding(self) -> GeometryEncoding {
        match self {
            Self::Json | Self::Object => GeometryEncoding::Json,
            Self::Cbor => GeometryEncoding::Cbor,
        }
    }
}

/// `ProductGeometry` objects: the core's record plus `payload`, in `form`.
pub(crate) fn graphs_to_js(graphs: &[ProductGeometry], form: PayloadForm) -> Array {
    graphs
        .iter()
        .map(|graph| {
            let object: Object = record_to_js(&graph.to_record()).unchecked_into();
            let payload = match (graph.payload.as_deref(), form) {
                (None, _) => JsValue::UNDEFINED,
                (Some(bytes), PayloadForm::Cbor) => Uint8Array::from(bytes).into(),
                (Some(_), PayloadForm::Json) => graph.json().map_or(JsValue::UNDEFINED, Into::into),
                // The writer's JSON always parses; undefined is unreachable.
                (Some(_), PayloadForm::Object) => graph
                    .json()
                    .and_then(|text| JSON::parse(text).ok())
                    .unwrap_or(JsValue::UNDEFINED),
            };
            if form == PayloadForm::Object {
                let _ = Reflect::set(&object, &"encoding".into(), &"object".into());
            }
            // Plain properties on a fresh plain object; setting cannot fail.
            let _ = Reflect::set(&object, &"payload".into(), &payload);
            JsValue::from(object)
        })
        .collect()
}
