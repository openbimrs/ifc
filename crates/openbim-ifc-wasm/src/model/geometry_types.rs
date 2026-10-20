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
 * Axiolid's versioned wire format of a geometry graph, 1.0 to 1.4
 * (Axiolid ADR 0085). A reader refuses another `format`, a major `version`
 * other than its own, a newer minor, and any kind, variant or field it
 * does not know.
 */
export interface GeometryGraphEnvelope {
  format: "axiolid-geometry-graph";
  /**
   * `"MAJOR.MINOR"`: the lowest version the content needs, `"1.0"` unless
   * a station carries a seam-snapping window (#423), then `"1.1"`, or a
   * run's sections do (#429), then `"1.2"`, or an offset run continues to
   * a basis end the kernel measures (#433), then `"1.3"`, or a polygonal
   * bounded half-space clips (#515), then `"1.4"` (axiolid/kernel#297,
   * #301, #303, #321).
   * This build writes at most `"1.4"`.
   */
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

/**
 * Why a mesh's colour could not be resolved (#449). `ambiguous`: two styled
 * items or two colours at one level, a layer or profile set of several
 * materials, a constituent set no shape aspect resolves, or Body items of
 * different colours. Never refuses the mesh itself.
 */
export interface ColourRefusal {
  code:
    | "ambiguous"
    | "unsupported"
    | "invalid-model"
    | "missing-reference"
    | "budget-exceeded";
  /** The entity at fault, when the refusal names one. */
  entity: bigint | undefined;
  message: string;
}

/**
 * The surface colour a mesh is painted with (#449): its representation
 * item's `IfcStyledItem`, else its styled presentation layer, else its
 * material's `IfcMaterialDefinitionRepresentation`.
 */
export interface SurfaceColour {
  /** Each channel in `[0, 1]`. */
  red: number;
  green: number;
  blue: number;
  /** In `[0, 1]`, 0 opaque; `undefined` when the file states none (opaque). */
  transparency: number | undefined;
  /** Which assignment supplied the colour, in that precedence. */
  source: "item" | "layer" | "material";
  /** The `IfcStyledItem`, the layer assignment or the `IfcMaterial`. */
  sourceEntity: bigint;
  /** The `IfcSurfaceStyle`. */
  surfaceStyle: bigint;
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
  /**
   * The colour the Body's items agree on; `undefined` when nothing colours
   * the product or `colourRefusal` says why it is not resolved.
   */
  colour: SurfaceColour | undefined;
  colourRefusal: ColourRefusal | undefined;
  /**
   * `"solid"` when the mesh bounds a volume the file declared, `"surface"`
   * for a surface (an area, no volume), `"unknown"` for an empty or refused
   * mesh (#458).
   */
  closure: "solid" | "surface" | "unknown";
  /** `x, y, z` per vertex, metres, relative to `transform`. */
  positions: Float32Array;
  /** Three vertex indices per triangle. */
  indices: Uint32Array;
}

/**
 * A representation a product's geometry is read from (#458): `"Body"` (the
 * meshes' default), `"Plan"` (the curves' default: `Plan`, `Annotation`,
 * `FootPrint`, then `Axis`, preferring a `PLAN_VIEW` context), or one
 * `RepresentationIdentifier`.
 */
export type RepresentationName =
  | "Body"
  | "Plan"
  | "Axis"
  | "FootPrint"
  | "Annotation"
  | "Surface";

/** One curve of a `ProductCurves`, as a slice of its `positions` (#458). */
export interface ProductPolyline {
  /** The IFC curve it was compiled from, when the lowering names one. */
  curve: bigint | undefined;
  /** Its first vertex in the product's `positions`. */
  firstVertex: number;
  vertexCount: number;
  /** Whether it is closed: its last vertex repeats its first. */
  closed: boolean;
  /**
   * Certified bound, metres, on how far the exact curve lies from the
   * polyline: `0` for a polyline the file authored.
   */
  chordBound: number;
  /** The chord budget, metres: one millimetre. */
  chordRequested: number;
}

/** One product's curves as polylines, from `IfcModel.productCurves` (#458). */
export interface ProductCurves {
  id: bigint;
  globalId: string | undefined;
  typeName: string;
  /** World placement; a vertex's world position is `transform * position`. */
  transform: Matrix4 | undefined;
  vertexCount: number;
  /** In authored order; empty with no refusal: no such curves. */
  polylines: ProductPolyline[];
  refusal: GeometryRefusal | undefined;
  /**
   * The representation's text literals and annotation fill areas, by
   * entity id in authored order: not drawn, and not a refusal (#512).
   */
  skipped: bigint[];
  /** `x, y, z` per vertex, metres, relative to `transform`: every polyline in turn. */
  positions: Float32Array;
}
"#;
