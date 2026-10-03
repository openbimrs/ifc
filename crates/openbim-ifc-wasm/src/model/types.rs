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
  | "unsupported-schema"
  | "io"
  | "unsupported-profile"
  | "feature-disabled"
  | "invalid-model"
  | "missing-reference"
  | "budget-exceeded"
  | "unsupported"
  | "wrong-entity-type"
  | "template-violation"
  | "missing-property"
  | "catalog-not-loaded";

/**
 * Where `IfcModel.loadCatalog` reads a catalog snapshot from. By default
 * Node reads `catalog/<file>` from the package directory, and a browser or
 * bundle fetches it relative to the module (`new URL(..., import.meta.url)`).
 */
export interface CatalogLoadOptions {
  /** The snapshot bytes themselves, for one release; nothing is read. */
  bytes?: Uint8Array | ArrayBuffer;
  /** A directory URL holding the `*.bin` files, used instead of the package's. */
  baseUrl?: string | URL;
}

export declare namespace IfcModel {
  /**
   * Load the PSD/QTO catalog of `release` (`"IFC2X3"`, `"IFC4"`,
   * `"IFC4X3"`), or of all three when omitted, into this module instance.
   * Each edition is read once, checked against its pinned SHA-256 and
   * cached; loading it again is a no-op. Until its release is loaded, a
   * write to a `Pset_`/`Qto_` set throws `catalog-not-loaded`.
   */
  function loadCatalog(release?: string, options?: CatalogLoadOptions): Promise<void>;
}

/** How `IfcModel.parseWithOptions` treats damaged input; omitted fields are strict. */
export interface ParseOptions {
  /** `"skip"` drops a malformed record and reports it in `diagnostics()`. */
  onMalformed?: "abort" | "skip";
  /** Report duplicate ids and references to undefined ids as diagnostics. */
  checkReferences?: boolean;
  /** Read `1E-05` (no decimal point) as a real, with a diagnostic. */
  acceptRealWithoutPoint?: boolean;
}

/** The STEP file header (`FILE_DESCRIPTION`, `FILE_NAME`, `FILE_SCHEMA`). */
export interface IfcHeader {
  description: string[];
  implementationLevel: string;
  name: string;
  timeStamp: string;
  author: string[];
  organization: string[];
  preprocessorVersion: string;
  originatingSystem: string;
  authorization: string;
  schema: string[];
}

/** One validation finding. `entity` is `undefined` for the file as a whole. */
export interface ValidationFinding {
  severity: "error" | "evaluation-error" | "warning" | "unsupported";
  rule: string;
  entity: bigint | undefined;
  attributeIndex: number | undefined;
  attributeName: string | undefined;
  path: string;
  message: string;
}

/** The result of `IfcModel.validate`. */
export interface ValidationReport {
  /** No errors and no evaluation errors; unsupported rules do not count. */
  conformant: boolean;
  /** The finding budget was reached: counts are lower bounds. */
  truncated: boolean;
  errors: number;
  evaluationErrors: number;
  warnings: number;
  unsupported: number;
  findings: ValidationFinding[];
}

/** A product no viewer will draw, from `IfcModel.unreachableProducts`. */
export interface UnreachableProduct {
  id: bigint;
  reason:
    | "not-contained-in-spatial-structure"
    | "no-representation-in-model-context"
    | "representation-without-context";
  /** Target views the geometry was found in instead, for the second reason. */
  foundViews: string[];
  message: string;
}
"#;
