//! Errors raised by the bindings.
//!
//! Each error carries a stable machine-readable `code` and a human message,
//! so a JavaScript caller can branch on the code without parsing prose.

use std::fmt;

/// Why a binding call failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingError {
    /// The STEP input could not be parsed.
    Parse(String),
    /// The model could not be serialized.
    Write(String),
    /// No entity has the given id.
    MissingEntity(u64),
    /// A JavaScript value did not follow the tagged value encoding.
    InvalidValue(String),
    /// An id or index was outside the range the model can represent.
    OutOfRange(String),
    /// The file's header names no schema this crate bundles.
    UnsupportedSchema(String),
}

impl BindingError {
    /// Stable code for programmatic handling.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Parse(_) => "parse",
            Self::Write(_) => "write",
            Self::MissingEntity(_) => "missing-entity",
            Self::InvalidValue(_) => "invalid-value",
            Self::OutOfRange(_) => "out-of-range",
            Self::UnsupportedSchema(_) => "unsupported-schema",
        }
    }
}

impl fmt::Display for BindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(detail) => write!(f, "cannot parse STEP: {detail}"),
            Self::Write(detail) => write!(f, "cannot write STEP: {detail}"),
            Self::MissingEntity(id) => write!(f, "no entity #{id}"),
            Self::InvalidValue(detail) => write!(f, "invalid IFC value: {detail}"),
            Self::OutOfRange(detail) => write!(f, "out of range: {detail}"),
            Self::UnsupportedSchema(token) => {
                write!(
                    f,
                    "no bundled schema for {token:?}; expected IFC2X3, IFC4 or IFC4X3"
                )
            }
        }
    }
}

impl std::error::Error for BindingError {}

#[cfg(target_arch = "wasm32")]
impl From<BindingError> for wasm_bindgen::JsValue {
    /// A JS `Error` whose `name` is `IfcError` and which carries `code`.
    fn from(error: BindingError) -> Self {
        let js = js_sys::Error::new(&error.to_string());
        js.set_name("IfcError");
        // `Reflect::set` only fails on a frozen or proxied target, which a
        // fresh `Error` is not; the message is still correct without it.
        let _ = js_sys::Reflect::set(&js, &"code".into(), &error.code().into());
        js.into()
    }
}
