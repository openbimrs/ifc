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
  /**
   * `IfcRelDefinesByType`; an object already typed is refused unless
   * `replace`, which unassigns its previous type and removes what that
   * type propagated (#480). `mapRepresentations` maps the type's
   * `RepresentationMaps` onto each object (through `mappingTarget` or an
   * identity operator); `materialUsage` derives a layer or profile set
   * usage from the type's set (`layerSetDirection` `"AXIS1"`..`"AXIS3"`
   * overrides the class default); `predefinedType` reconciles
   * `PredefinedType`.
   */
  | {
      op: "assignType";
      typeObject: bigint;
      objects: bigint[];
      ownerHistory?: bigint;
      mapRepresentations?: boolean;
      mappingTarget?: bigint;
      materialUsage?: boolean;
      layerSetDirection?: "AXIS1" | "AXIS2" | "AXIS3";
      predefinedType?: "ignore" | "validate" | "sync";
      replace?: boolean;
    }
  /** `IfcRelContainedInSpatialStructure`; `replace` moves an element out of its previous container. */
  | { op: "contain"; structure: bigint; elements: bigint[]; ownerHistory?: bigint; replace?: boolean }
  /** `IfcRelAggregates`; `replace` moves a part out of its previous whole. */
  | { op: "aggregate"; parent: bigint; parts: bigint[]; ownerHistory?: bigint; replace?: boolean }
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
    }
  /** An `IfcShapeRepresentation` of `items` in `context`, unassigned; `representationType` checked against the items (`CorrectItemsForType`) or inferred, `identifier` defaulting to the context's `ContextIdentifier`. */
  | {
      op: "shapeRepresentation";
      context: bigint;
      items: bigint[];
      identifier?: string;
      representationType?: string;
    }
  /** Assign a shape representation to a product (its `IfcProductDefinitionShape`) or a type (a new `IfcRepresentationMap`). */
  | { op: "assignRepresentation"; representation: bigint; to: bigint }
  /** Take a shape representation off a product or type; it stays, unassigned. */
  | { op: "unassignRepresentation"; representation: bigint; from: bigint }
  /** Copy a shape representation and its items, sharing its context, styles and mapped type geometry. */
  | { op: "copyRepresentation"; representation: bigint }
  /** Remove a shape representation with what only it uses. */
  | { op: "removeRepresentation"; representation: bigint }
  /** Copy a product or type with what it solely owns (see `IfcModel.copyProduct`); produces the copy. */
  | ({ op: "copyProduct"; product: bigint } & Omit<CopyProductOptions, "dryRun">)
  /** Deep-copy an entity and what it references: classes in `copy` copied, in `share` (and entities a UNIQUE clause binds) shared; other `IfcRoot`s shared unless `copyRoots`, definitions unless `copyDefinitions`. Produces the copy. */
  | { op: "copyEntity"; entity: bigint; copy?: string[]; share?: string[]; copyRoots?: boolean; copyDefinitions?: boolean };

/** What a committed `IfcModel.author` batch did. */
export interface AuthoringResult {
  /** Per operation, the id of the entity it produced; `undefined` for a removal. */
  ids: (bigint | undefined)[];
  created: bigint[];
  removed: bigint[];
}

/** Options of `IfcModel.removeProduct`; every flag defaults to `false`. */
export interface RemoveProductOptions {
  /** Re-base placements relative to a removed placement onto its parent, composing the transforms, instead of refusing. */
  rebasePlacements?: boolean;
  /** Remove a type object's typing relationship, leaving its occurrences untyped, instead of refusing. */
  untypeOccurrences?: boolean;
  /** Report only; the model is unchanged. */
  dryRun?: boolean;
}

/** Options of `IfcModel.removeRepresentation`. */
export interface RemoveRepresentationOptions {
  /** Report only; the model is unchanged. */
  dryRun?: boolean;
}

/** Options of `IfcModel.purge`. */
export interface PurgeOptions {
  /** Entity types to purge, subtypes included; empty or absent for every resource entity no `IfcRoot` reaches. */
  classes?: string[];
  /** Also purge unreferenced `IfcRoot`s of `classes`. */
  includeRoots?: boolean;
  /** Also purge an unreferenced owner history with its actors. */
  includeOwnerHistory?: boolean;
  /** Report only; the model is unchanged. */
  dryRun?: boolean;
}

/** An entity a removal or purge removed. */
export interface RemovedEntity {
  id: bigint;
  /** Its type, upper case. */
  typeName: string;
}

/** An entity a removal reached and kept. */
export interface KeptEntity {
  id: bigint;
  typeName: string;
  /** `shared` (`users` still need it), `related-object`, `definition` (only a purge removes it) or `protected` (a purge skipped it). */
  reason: "shared" | "related-object" | "definition" | "protected";
  users: bigint[];
}

/** What `IfcModel.removeProduct`, `IfcModel.removeRepresentation` or `IfcModel.purge` did, or, for a dry run, would do. */
export interface RemovalReport {
  removed: RemovedEntity[];
  kept: KeptEntity[];
  /** Relationships and layer assignments that lost a removed member and stay. */
  detached: bigint[];
  /** Placements re-based onto a kept ancestor. */
  rebased: bigint[];
  /** `false` for a dry run. */
  applied: boolean;
}

/** Options of `IfcModel.copyProduct` and the `copyProduct` operation; the flags default to `true`, `dryRun` to `false`. */
export interface CopyProductOptions {
  /** Move the copy by this vector, in its parent placement's frame. */
  offset?: [number, number, number];
  /** Place the copy by this `IfcObjectPlacement` instead. */
  placement?: bigint;
  /** Copy the openings voiding the product. */
  openings?: boolean;
  /** Copy the doors and windows filling a copied opening. */
  fillings?: boolean;
  /** Contain the copy in the original's spatial structure element. */
  container?: boolean;
  /** Make the copy a part of the original's whole. */
  aggregate?: boolean;
  /** Report only; the model is unchanged. */
  dryRun?: boolean;
}

/** Options of `IfcModel.append`. */
export interface AppendOptions {
  /** `keep` (the default), `merge`, `regenerate` or `regenerate-colliding`. */
  globalIds?: "keep" | "merge" | "regenerate" | "regenerate-colliding";
  /** The resources reused when this model holds an equal one; all when absent, none for `[]`. */
  reuse?: ("units" | "contexts" | "ownerHistory" | "materials" | "styles")[];
  /** Report only; the model is unchanged. */
  dryRun?: boolean;
}

/** An original and its copy (or, in `CopyReport.reused`, the entity used for it). */
export interface CopiedEntity {
  original: bigint;
  copy: bigint;
  typeName: string;
}

/** What `IfcModel.copyProduct` or `IfcModel.append` did, or, for a dry run, would do. */
export interface CopyReport {
  /** The product's copy; `undefined` for an append. */
  copy?: bigint;
  /** Every original with its copy, in creation order. */
  copied: CopiedEntity[];
  /** Append: source entities this model already holds, with the one used. */
  reused: CopiedEntity[];
  /** Existing relationships and layers the copies joined. */
  joined: bigint[];
  /** Relationships of an original the copies do not get. */
  skipped: bigint[];
  /** Append: copies given a new `GlobalId`. */
  regenerated: bigint[];
  /** `false` for a dry run. */
  applied: boolean;
/** Options of `IfcModel.exportTable`. */
export interface TableOptions {
  /** `"csv"` (default), `"tsv"` or `"xlsx"` (`feature-disabled` in the npm package). */
  format?: "csv" | "tsv" | "xlsx";
  /** The key column: `"GlobalId"` (default) or `"id"`. */
  key?: "GlobalId" | "id";
  /** The cell text of `NULL`; empty by default. */
  null?: string;
}

/** Options of `IfcModel.importTable`. */
export interface TableImportOptions {
  /** `"csv"` (default), `"tsv"` or `"xlsx"`. */
  format?: "csv" | "tsv" | "xlsx";
  /** The key column; by default `"GlobalId"` when the table has one, else `"id"`. */
  key?: "GlobalId" | "id";
  /** The cell text of `NULL`; empty by default. */
  null?: string;
  /** Plan and report only; the model is unchanged. */
  dryRun?: boolean;
}

/** One value `IfcModel.importTable` changes. */
export interface TableChange {
  /** One-based; the header is row 1. */
  row: number;
  /** One-based. */
  column: number;
  /** The row's key as the table gives it. */
  key: string;
  /** The column's value path. */
  path: string;
  /** The row's entity. */
  entity: bigint;
  /** The entity written, which a path such as `type.Name` finds elsewhere. */
  targetEntity: bigint;
  /** The attribute name or `Set.Property` written. */
  target: string;
  /** The value now, as cell text. */
  before: string;
  /** The value then, as cell text. */
  after: string;
}

/** One refused table, row or cell of `IfcModel.importTable`. */
export interface TableRefusal {
  /** One-based; `undefined` for the whole table. */
  row: number | undefined;
  /** One-based; `undefined` for a whole row or the table. */
  column: number | undefined;
  /** `malformed-table`, `invalid-header`, `duplicate-column`, `no-key-column`, `ragged-row`, `empty-key`, `unknown-key`, `duplicate-key`, `ambiguous-key`, or the error code of a refused write (`type-mismatch`, `invalid-value`, ...). */
  code: string;
  message: string;
  /** The byte range of a header that does not parse. */
  span: [number, number] | undefined;
}

/** What `IfcModel.importTable` planned, and whether it wrote it. */
export interface TableImport {
  /** `false` for a dry run, a refused table, or one that changes nothing. */
  applied: boolean;
  changes: TableChange[];
  /** Any refusal means nothing was written. */
  refusals: TableRefusal[];
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
