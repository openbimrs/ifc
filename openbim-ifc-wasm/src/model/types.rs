//! TypeScript declarations for the tagged value encoding.
//!
//! wasm-bindgen types a `JsValue` as `any`; this section declares the real
//! shape so TypeScript callers are checked against the encoding in
//! [`crate::value`].

use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const IFC_VALUE_TYPES: &'static str = r#"
/** One IFC attribute value, in the lossless tagged encoding (ADR 0013). */
export type IfcValue =
  | { kind: "null" }
  | { kind: "derived" }
  | { kind: "bool"; value: boolean }
  | { kind: "unknown" }
  | { kind: "integer"; value: bigint }
  | { kind: "real"; value: number }
  | { kind: "text"; value: string }
  | { kind: "binary"; value: string }
  | { kind: "enum"; value: string }
  | { kind: "ref"; id: bigint }
  | { kind: "list"; items: IfcValue[] }
  | { kind: "typed"; type: string; value: IfcValue };

/** The `code` of an `IfcError`. */
export type IfcErrorCode =
  | "parse"
  | "write"
  | "missing-entity"
  | "invalid-value"
  | "out-of-range"
  | "unsupported-schema";
"#;
