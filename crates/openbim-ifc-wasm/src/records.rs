//! Moves the #244 records -- parse options, the header, validation
//! reports and unreachable products -- into and out of JavaScript objects.
//!
//! Field names are camelCase; ids are `bigint`, as everywhere in this
//! binding. Every malformed input is an `invalid-value` [`BindingError`].

use js_sys::{Array, BigInt, Object, Reflect};
use openbim_ifc_binding_core::{
    BindingError, OnMalformed, ParseOptions, UnreachableProduct, ValidationReport,
};
use wasm_bindgen::JsValue;

fn invalid(detail: impl Into<String>) -> BindingError {
    BindingError::InvalidValue(detail.into())
}

fn set(object: &Object, key: &str, value: &JsValue) {
    // Setting a plain property on a fresh plain object cannot fail.
    let _ = Reflect::set(object, &key.into(), value);
}

fn get(object: &JsValue, key: &str) -> Result<JsValue, BindingError> {
    Reflect::get(object, &key.into()).map_err(|_| invalid(format!("cannot read `{key}`")))
}

fn strings(values: &[String]) -> JsValue {
    values
        .iter()
        .map(|s| JsValue::from_str(s))
        .collect::<Array>()
        .into()
}

fn optional<T>(value: Option<T>, map: impl FnOnce(T) -> JsValue) -> JsValue {
    value.map_or(JsValue::UNDEFINED, map)
}

/// `{ onMalformed?, checkReferences?, acceptRealWithoutPoint? }`; a
/// missing field keeps the strict default.
pub(crate) fn parse_options(value: &JsValue) -> Result<ParseOptions, BindingError> {
    if value.is_undefined() || value.is_null() {
        return Ok(ParseOptions::strict());
    }
    if !value.is_object() {
        return Err(invalid("parse options must be an object"));
    }
    let flag = |key: &str| -> Result<bool, BindingError> {
        let field = get(value, key)?;
        if field.is_undefined() {
            return Ok(false);
        }
        field
            .as_bool()
            .ok_or_else(|| invalid(format!("`{key}` must be a boolean")))
    };
    let policy = get(value, "onMalformed")?;
    let on_malformed = if policy.is_undefined() {
        OnMalformed::Abort
    } else {
        OnMalformed::parse(
            &policy
                .as_string()
                .ok_or_else(|| invalid("`onMalformed` must be a string"))?,
        )?
    };
    Ok(ParseOptions {
        on_malformed,
        check_references: flag("checkReferences")?,
        accept_real_without_point: flag("acceptRealWithoutPoint")?,
    })
}

/// The header as an `IfcHeader` object.
pub(crate) fn header_to_js(header: &openbim_ifc_binding_core::header::Header) -> JsValue {
    let object = Object::new();
    set(&object, "description", &strings(&header.description));
    set(
        &object,
        "implementationLevel",
        &header.implementation_level.as_str().into(),
    );
    set(&object, "name", &header.name.as_str().into());
    set(&object, "timeStamp", &header.time_stamp.as_str().into());
    set(&object, "author", &strings(&header.author));
    set(&object, "organization", &strings(&header.organization));
    set(
        &object,
        "preprocessorVersion",
        &header.preprocessor_version.as_str().into(),
    );
    set(
        &object,
        "originatingSystem",
        &header.originating_system.as_str().into(),
    );
    set(
        &object,
        "authorization",
        &header.authorization.as_str().into(),
    );
    set(&object, "schema", &strings(&header.schema));
    object.into()
}

/// An `IfcHeader` object; every field is required.
pub(crate) fn header_from_js(
    value: &JsValue,
) -> Result<openbim_ifc_binding_core::header::Header, BindingError> {
    if !value.is_object() {
        return Err(invalid("a header must be an object"));
    }
    let text = |key: &str| -> Result<String, BindingError> {
        get(value, key)?
            .as_string()
            .ok_or_else(|| invalid(format!("header `{key}` must be a string")))
    };
    let texts = |key: &str| -> Result<Vec<String>, BindingError> {
        let field = get(value, key)?;
        if !Array::is_array(&field) {
            return Err(invalid(format!(
                "header `{key}` must be an array of strings"
            )));
        }
        Array::from(&field)
            .iter()
            .map(|item| {
                item.as_string()
                    .ok_or_else(|| invalid(format!("header `{key}` must hold strings only")))
            })
            .collect()
    };
    Ok(openbim_ifc_binding_core::header::Header {
        description: texts("description")?,
        implementation_level: text("implementationLevel")?,
        name: text("name")?,
        time_stamp: text("timeStamp")?,
        author: texts("author")?,
        organization: texts("organization")?,
        preprocessor_version: text("preprocessorVersion")?,
        originating_system: text("originatingSystem")?,
        authorization: text("authorization")?,
        schema: texts("schema")?,
    })
}

/// A `ValidationReport` object.
pub(crate) fn report_to_js(report: &ValidationReport) -> JsValue {
    let object = Object::new();
    set(&object, "conformant", &report.conformant.into());
    set(&object, "truncated", &report.truncated.into());
    let count = |n: usize| JsValue::from_f64(n as f64);
    set(&object, "errors", &count(report.summary.errors));
    set(
        &object,
        "evaluationErrors",
        &count(report.summary.evaluation_errors),
    );
    set(&object, "warnings", &count(report.summary.warnings));
    set(&object, "unsupported", &count(report.summary.unsupported));
    let findings: Array = report
        .findings
        .iter()
        .map(|finding| {
            let row = Object::new();
            set(&row, "severity", &finding.severity.as_str().into());
            set(&row, "rule", &finding.rule.as_str().into());
            set(
                &row,
                "entity",
                &optional(finding.entity, |id| BigInt::from(id).into()),
            );
            set(
                &row,
                "attributeIndex",
                &optional(finding.attribute_index, count),
            );
            set(
                &row,
                "attributeName",
                &optional(finding.attribute_name.as_deref(), JsValue::from_str),
            );
            set(&row, "path", &finding.path.as_str().into());
            set(&row, "message", &finding.message.as_str().into());
            JsValue::from(row)
        })
        .collect();
    set(&object, "findings", &findings.into());
    object.into()
}

/// An array of `UnreachableProduct` objects.
pub(crate) fn unreachable_to_js(products: &[UnreachableProduct]) -> Array {
    products
        .iter()
        .map(|product| {
            let row = Object::new();
            set(&row, "id", &BigInt::from(product.id).into());
            set(&row, "reason", &product.reason.as_str().into());
            set(&row, "foundViews", &strings(&product.found_views));
            set(&row, "message", &product.message.as_str().into());
            JsValue::from(row)
        })
        .collect()
}
