//! Binding errors as JavaScript `IfcError` objects.

use openbim_ifc_binding_core::BindingError;
use wasm_bindgen::JsValue;

/// A JS `Error` whose `name` is `IfcError` and which carries the stable
/// `code` shared by every binding.
///
/// A function rather than `impl From`: both types are foreign to this crate,
/// so the orphan rule forbids the impl here.
pub(crate) fn js_error(error: BindingError) -> JsValue {
    let js = js_sys::Error::new(&error.to_string());
    js.set_name("IfcError");
    // `Reflect::set` only fails on a frozen or proxied target, which a fresh
    // `Error` is not; the message is still correct without it.
    let _ = js_sys::Reflect::set(&js, &"code".into(), &error.code().into());
    js.into()
}
