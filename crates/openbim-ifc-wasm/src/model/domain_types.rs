//! TypeScript declarations for the domain records (#123).
//!
//! Each interface mirrors one `openbim_ifc_binding_core` record, its fields
//! in camelCase: ids are `bigint`, an absent field is `undefined`, and an
//! IFC value is an `IfcValue` in the tagged encoding.

use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const IFC_DOMAIN_TYPES: &'static str = r#"
/** A property set, quantity set or predefined property set, from `IfcModel.propertySets`. */
export interface PropertySet {
  id: bigint;
  globalId: string | undefined;
  /** `Name`, or the entity name of a predefined set that states none. */
  name: string;
  /** `IFCPROPERTYSET`, `IFCELEMENTQUANTITY` or a predefined set's entity. */
  typeName: string;
  /** Stated on the object, or held by its (or the queried) type object. */
  source: "occurrence" | "type" | "material";
  /** The type object, for `source: "type"`. */
  sourceId: bigint | undefined;
  properties: Property[];
}

/** One object's answer from `IfcModel.propertySetsMany` (#358). */
export interface ObjectPropertySets {
  object: bigint;
  /** Exactly what `propertySets` returns for the object; empty when refused. */
  sets: PropertySet[];
  /** The code and message `propertySets` throws for the object. */
  refusal: PropertyRefusal | undefined;
}

/** Why one object's property sets were refused. */
export interface PropertyRefusal {
  code: IfcErrorCode;
  message: string;
}

/** One property, quantity or predefined-set attribute. */
export interface Property {
  id: bigint;
  name: string;
  /** The entity type: `IFCPROPERTYSINGLEVALUE`, `IFCQUANTITYLENGTH`, ... */
  typeName: string;
  kind: "value" | "enumerated" | "list" | "bounded" | "table" | "reference" | "complex";
  /** The declared IFC value type, e.g. `IFCLENGTHMEASURE`. */
  valueType: string | undefined;
  /** The unit the property states; resolve it with `IfcModel.resolveUnit`. */
  unit: bigint | undefined;
  /** `value`: the typed value; `enumerated`/`list`: a `list`; `reference`: a `ref`. */
  value: IfcValue;
  enumeration: PropertyEnumeration | undefined;
  bounds: PropertyBounds | undefined;
  table: PropertyTable | undefined;
  usage: string | undefined;
  discrimination: string | undefined;
  quality: string | undefined;
  /** A complex property's or quantity's members. */
  members: Property[];
}

/** An `IfcPropertyEnumeration`. */
export interface PropertyEnumeration {
  id: bigint;
  name: string;
  values: IfcValue[];
}

/** An `IfcPropertyBoundedValue`'s values; `{ kind: "null" }` when unstated. */
export interface PropertyBounds {
  lower: IfcValue;
  upper: IfcValue;
  setPoint: IfcValue;
}

/** An `IfcPropertyTableValue`. */
export interface PropertyTable {
  rows: PropertyTableRow[];
  expression: string | undefined;
  definingUnit: bigint | undefined;
  definedUnit: bigint | undefined;
  interpolation: string | undefined;
}

/** One row of a `PropertyTable`. */
export interface PropertyTableRow {
  defining: IfcValue;
  defined: IfcValue;
}

/** A measure's effective unit: `si = value * scale + offset`. */
export interface ResolvedUnit {
  unit: bigint | undefined;
  fromProject: boolean;
  /** SI exponents `[L, M, T, I, Θ, N, J]`. */
  dimensions: bigint[];
  scale: number;
  offset: number;
}

/**
 * One edit for `IfcModel.setProperties`: write `value` to property `name` of
 * set `set` on `object`, or, with `remove: true`, remove it. The value is the
 * read side's `Property.value`: a typed `IfcValue` (or `{ kind: "null" }`), a
 * `list` of them for an enumerated or list value, a typed measure for a
 * quantity.
 */
export type PropertyEdit =
  | {
      object: bigint;
      set: string;
      name: string;
      value: IfcValue;
      /** `"IfcPropertySet"` or `"IfcElementQuantity"`, for a set the edit
       * creates that neither the type object nor the catalog describes. */
      setType?: string;
      remove?: false;
    }
  | { object: bigint; set: string; name: string; remove: true };

/** What a committed `setProperties` batch did. */
export interface PropertyEditResult {
  /** Per edit: the entity holding the property afterwards, if any. */
  properties: (bigint | undefined)[];
  created: bigint[];
  removed: bigint[];
}

/** Attribute values by name, for `IfcModel.author` and `createEntity`. */
export type NamedAttributes = Record<string, IfcValue>;

/** Three coordinates or direction ratios. */
export type Triple = [number, number, number];

/**
 * One operation of `IfcModel.author`. Every id is a `bigint`: an entity's
 * id or `IfcModel.handle(index)`, the entity an earlier operation of the
 * batch produced. Builders take the `ownerHistory` they write on every
 * record they create; none is invented.
 */
export type AuthorOp =
  /** One entity by type and named attributes. */
  | { op: "create"; type: string; attributes?: NamedAttributes }
  /** Replace named attributes; the whole entity is checked again. */
  | { op: "edit"; entity: bigint; attributes: NamedAttributes }
  /** Remove an entity with the relationships that reference it. */
  | { op: "remove"; entity: bigint }
  /** The model's one `IfcProject`. */
  | { op: "project"; attributes?: NamedAttributes; ownerHistory?: bigint }
  /** A spatial element aggregated under `parent` (`IfcRelAggregates`). */
  | {
      op: "spatial";
      type: string;
      parent: bigint;
      attributes?: NamedAttributes;
      placement?: bigint;
      ownerHistory?: bigint;
    }
  /** A product, contained in `container` and typed by `typeObject`. */
  | {
      op: "product";
      type: string;
      container?: bigint;
      attributes?: NamedAttributes;
      placement?: bigint;
      typeObject?: bigint;
      ownerHistory?: bigint;
    }
  /** A type object (`IfcWallType`, ...). */
  | { op: "typeObject"; type: string; attributes?: NamedAttributes; ownerHistory?: bigint }
  /** `IfcRelDefinesByType`. */
  | { op: "assignType"; typeObject: bigint; objects: bigint[]; ownerHistory?: bigint }
  /** `IfcRelContainedInSpatialStructure`. */
  | { op: "contain"; structure: bigint; elements: bigint[]; ownerHistory?: bigint }
  /** `IfcRelAggregates`. */
  | { op: "aggregate"; parent: bigint; parts: bigint[]; ownerHistory?: bigint }
  /** An `IfcLocalPlacement`; `axis` and `refDirection` both or neither. */
  | { op: "placement"; relativeTo?: bigint; location?: Triple; axis?: Triple; refDirection?: Triple }
  /** An `IfcOwnerHistory` with its person, organization and application. */
  | {
      op: "ownerHistory";
      personIdentification?: string;
      familyName?: string;
      givenName?: string;
      organization: string;
      applicationName: string;
      applicationVersion: string;
      applicationIdentifier: string;
      changeAction?: string;
      creationDate: bigint | number;
      lastModifiedDate?: bigint | number;
    };

/** What a committed `IfcModel.author` batch did. */
export interface AuthoringResult {
  /** Per operation, the id of the entity it produced; `undefined` for a removal. */
  ids: (bigint | undefined)[];
  created: bigint[];
  removed: bigint[];
}

/** The containment tree, from `IfcModel.spatialTree`. */
export interface SpatialTree {
  /** The release containers were classified against, e.g. `IFC4_ADD2_TC1`. */
  release: string | undefined;
  roots: bigint[];
  nodes: SpatialNode[];
  orphans: bigint[];
  dangling: SpatialDanglingReference[];
  anomalies: SpatialAnomaly[];
}

/** One spatial container. */
export interface SpatialNode {
  id: bigint;
  globalId: string | undefined;
  name: string | undefined;
  typeName: string;
  kind: "project" | "site" | "building" | "storey" | "space" | "other";
  parent: bigint | undefined;
  children: bigint[];
  /** Elements contained directly. */
  elements: bigint[];
  /** Elements referenced, not contained. */
  referenced: bigint[];
}

/** A relationship naming an entity the file lacks. */
export interface SpatialDanglingReference {
  relation: bigint;
  target: bigint;
}

/** A second parent or a non-container structure the tree rejected. */
export interface SpatialAnomaly {
  kind:
    | "contained-twice"
    | "aggregated-twice"
    | "contained-in-non-container"
    | "referenced-in-non-container";
  relation: bigint;
  subject: bigint;
  kept: bigint | undefined;
}

/** A classification that applies to an object, from `IfcModel.classifications`. */
export interface Classification {
  relationship: bigint;
  globalId: string | undefined;
  source: "occurrence" | "type";
  typeObject: bigint | undefined;
  target: bigint;
  kind: "reference" | "system" | "notation";
  identification: string | undefined;
  name: string | undefined;
  location: string | undefined;
  /** An IFC2X3 notation's facet values. */
  notation: string[];
  /** The references above a `reference` target, nearest first. */
  parents: bigint[];
  system: ClassificationSystem | undefined;
}

/** An `IfcClassification`. */
export interface ClassificationSystem {
  id: bigint;
  name: string;
  source: string | undefined;
  edition: string | undefined;
}

/** The material association of an object, from `IfcModel.material`. */
export interface MaterialAssignment {
  relationship: bigint;
  globalId: string | undefined;
  source: "occurrence" | "type";
  typeObject: bigint | undefined;
  target: bigint;
  typeName: string;
  kind:
    | "material"
    | "list"
    | "layer"
    | "layer-set"
    | "layer-set-usage"
    | "profile"
    | "profile-set"
    | "profile-set-usage"
    | "constituent"
    | "constituent-set";
  set: bigint | undefined;
  name: string | undefined;
  materials: MaterialRef[];
  layers: MaterialLayer[];
  profiles: MaterialProfile[];
  constituents: MaterialConstituent[];
  usage: MaterialUsage | undefined;
}

/** An `IfcMaterial`. */
export interface MaterialRef {
  id: bigint;
  name: string;
  category: string | undefined;
}

/** An `IfcMaterialLayer`; `thickness` in the file's length unit. */
export interface MaterialLayer {
  id: bigint;
  material: MaterialRef | undefined;
  thickness: number;
  /** `bool`, `unknown`, or `null` when unset. */
  isVentilated: IfcValue;
  name: string | undefined;
  category: string | undefined;
  priority: bigint | undefined;
}

/** An `IfcMaterialProfile`. */
export interface MaterialProfile {
  id: bigint;
  material: MaterialRef | undefined;
  profile: bigint;
  name: string | undefined;
  category: string | undefined;
  priority: bigint | undefined;
}

/** An `IfcMaterialConstituent`. */
export interface MaterialConstituent {
  id: bigint;
  material: MaterialRef;
  name: string | undefined;
  category: string | undefined;
  fraction: number | undefined;
}

/** How a layer or profile set is placed. */
export interface MaterialUsage {
  layerSetDirection: string | undefined;
  directionSense: string | undefined;
  offsetFromReferenceLine: number | undefined;
  referenceExtent: number | undefined;
  cardinalPoint: bigint | undefined;
  endSet: bigint | undefined;
  cardinalEndPoint: bigint | undefined;
}

/** Every system, from `IfcModel.systems`. */
export interface Systems {
  systems: System[];
  anomalies: SystemAnomaly[];
}

/** One `IfcSystem`. */
export interface System {
  id: bigint;
  globalId: string | undefined;
  typeName: string;
  name: string | undefined;
  longName: string | undefined;
  predefinedType: string | undefined;
  members: bigint[];
  servicedBuildings: bigint[];
  servicedFacilities: bigint[];
}

/** A membership the systems reader could not honour. */
export interface SystemAnomaly {
  kind: string;
  subject: bigint;
  other: bigint | undefined;
  message: string;
}

/** Every cost schedule and item, from `IfcModel.cost`. */
export interface Cost {
  schedules: CostSchedule[];
  items: CostItem[];
  anomalies: CostAnomaly[];
}

/** An `IfcCostSchedule`. */
export interface CostSchedule {
  id: bigint;
  globalId: string | undefined;
  name: string | undefined;
  identification: string | undefined;
  status: string | undefined;
  predefinedType: string | undefined;
  items: bigint[];
}

/** An `IfcCostItem`. */
export interface CostItem {
  id: bigint;
  globalId: string | undefined;
  name: string | undefined;
  identification: string | undefined;
  description: string | undefined;
  predefinedType: string | undefined;
  parent: bigint | undefined;
  children: bigint[];
  values: CostValue[];
  quantities: bigint[];
  /** Objects the item prices. */
  objects: bigint[];
}

/** An `IfcCostValue`; `appliedValue` as authored, e.g. `typed IFCMONETARYMEASURE`. */
export interface CostValue {
  id: bigint;
  name: string | undefined;
  description: string | undefined;
  category: string | undefined;
  condition: string | undefined;
  appliedValue: IfcValue;
  operator: "ADD" | "DIVIDE" | "MULTIPLY" | "SUBTRACT" | undefined;
  unitBasis: UnitBasis | undefined;
  components: CostValue[];
}

/** The measure a rate is stated per. */
export interface UnitBasis {
  id: bigint;
  value: IfcValue;
  unit: bigint | undefined;
}

/** A cost item nested under two parents. */
export interface CostAnomaly {
  item: bigint;
  kept: bigint;
  rejected: bigint;
  relation: bigint;
}

/** A coordinate operation resolved, from `IfcModel.georeferencing`. */
export interface MapConversion {
  operation: bigint;
  kind: "map-conversion" | "map-conversion-scaled" | "rigid-operation";
  source: bigint;
  sourceKind: "context" | "crs";
  targetCrs: ProjectedCrs;
  eastings: number;
  northings: number;
  orthogonalHeight: number;
  xAxis: [number, number];
  scale: number;
  factors: [number, number, number] | undefined;
  projectUnit: LengthUnit;
  mapUnit: LengthUnit;
  mapUnitDeclared: boolean;
  /** Project metres to map metres, as three columns. */
  linear: [number, number, number][];
  /** Where the project origin lands, in map metres. */
  translation: [number, number, number];
}

/** An `IfcProjectedCRS`. */
export interface ProjectedCrs {
  id: bigint;
  name: string | undefined;
  description: string | undefined;
  geodeticDatum: string | undefined;
  verticalDatum: string | undefined;
  mapProjection: string | undefined;
  mapZone: string | undefined;
  wellKnownText: string | undefined;
}

/** A length unit reduced to metres. */
export interface LengthUnit {
  name: string;
  metresPerUnit: number;
}
"#;
