//! TypeScript declarations for the geometry records (#328).
//!
//! Each interface mirrors one `openbim_ifc_binding_core::geometry` record,
//! its fields in camelCase; a mesh adds its typed arrays.

use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const IFC_GEOMETRY_TYPES: &'static str = r#"
/**
 * A 4x4 column-major matrix in metres, the layout WebGL and three.js
 * (`Matrix4.fromArray`) read: three basis columns, then the origin.
 */
export type Matrix4 = number[];

/** Why one product has no placement, representation, graph or mesh. */
export interface GeometryRefusal {
  code: "unsupported" | "invalid-model" | "missing-reference" | "budget-exceeded";
  /** The entity at fault, when the refusal names one. */
  entity: bigint | undefined;
  message: string;
}

/** The `IfcShapeRepresentation` selected as a product's Body. */
export interface SelectedRepresentation {
  id: bigint;
  /** `RepresentationIdentifier`, e.g. `Body`. */
  identifier: string | undefined;
  /** `RepresentationType`, e.g. `SweptSolid`. */
  representationType: string | undefined;
  /** `ContextOfItems`. */
  context: bigint | undefined;
  /** The context's `ContextType`, e.g. `Model`. */
  contextType: string | undefined;
  /** The context's `ContextIdentifier`, e.g. `Body`. */
  contextIdentifier: string | undefined;
  /** A sub-context's `TargetView`, e.g. `MODEL_VIEW`. */
  targetView: string | undefined;
}

/** One product's world placement and Body, from `IfcModel.productPlacements`. */
export interface ProductPlacement {
  id: bigint;
  globalId: string | undefined;
  typeName: string;
  /** `undefined` when the placement is refused. */
  transform: Matrix4 | undefined;
  /** `undefined` for a product with no solid representation (an axis only). */
  representation: SelectedRepresentation | undefined;
  refusal: GeometryRefusal | undefined;
}

/**
 * How `IfcModel.productGeometry` hands each payload over: `"json"` the wire
 * text, `"object"` that text parsed, `"cbor"` the CBOR bytes.
 */
export type GeometryPayloadEncoding = "json" | "object" | "cbor";

/**
 * One node of an Axiolid geometry graph, externally tagged by its kind
 * (`Profile`, `SolidOperation`, `Instance`, `Collection`, ...), with the
 * kind's variant tagged inside it. A reference to another node is the
 * index of an earlier node; points and vectors are `[x, y, z]` arrays; a
 * transform is its linear part's three columns, then the translation.
 * Axiolid's ADR 0085 defines every kind.
 */
export type GeometryGraphNode = { [kind: string]: unknown };

/**
 * Axiolid's versioned wire format of a geometry graph, 1.0
 * (Axiolid ADR 0085). A reader refuses another `format`, a major `version`
 * other than its own, a newer minor, and any kind, variant or field it
 * does not know.
 */
export interface GeometryGraphEnvelope {
  format: "axiolid-geometry-graph";
  /** `"MAJOR.MINOR"`; this build writes `"1.0"`. */
  version: string;
  graph: {
    /** In insertion order, which is topological. */
    nodes: GeometryGraphNode[];
    /** Indices into `nodes`; a product's graph has one. */
    roots: number[];
  };
}

/** One product's Body as a geometry graph, from `IfcModel.productGeometry`. */
export interface ProductGeometry {
  id: bigint;
  globalId: string | undefined;
  typeName: string;
  /**
   * World placement, as `productPlacements` gives it. The graph is already
   * in world coordinates: never apply this to it again.
   */
  transform: Matrix4 | undefined;
  encoding: GeometryPayloadEncoding;
  /** Bytes of the wire payload (UTF-8 for JSON); 0 without one. */
  payloadSize: number;
  /** No payload and no refusal: a product with no Body representation. */
  refusal: GeometryRefusal | undefined;
  /**
   * The wire payload: a string for `"json"`, a `GeometryGraphEnvelope`
   * for `"object"`, a `Uint8Array` for `"cbor"`.
   */
  payload: string | GeometryGraphEnvelope | Uint8Array | undefined;
}

/** One product's Body as triangles, from `IfcModel.productMeshes`. */
export interface ProductMesh {
  id: bigint;
  globalId: string | undefined;
  typeName: string;
  /** World placement; a vertex's world position is `transform * position`. */
  transform: Matrix4 | undefined;
  vertexCount: number;
  triangleCount: number;
  /** Empty arrays and no refusal: a product with no Body representation. */
  refusal: GeometryRefusal | undefined;
  /** `x, y, z` per vertex, metres, relative to `transform`. */
  positions: Float32Array;
  /** Three vertex indices per triangle. */
  indices: Uint32Array;
}
"#;
