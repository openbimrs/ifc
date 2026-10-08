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

/** Why one product has no placement, representation or mesh. */
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
