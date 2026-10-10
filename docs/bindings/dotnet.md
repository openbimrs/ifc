# C# and .NET

`OpenBim.Ifc` is the .NET binding of the IFC core
([`openbim-ifc-dotnet`](/reference/crates/openbim-ifc-dotnet)), published to
NuGet. It is C# over the versioned [C ABI](/bindings/c), and the package
carries the C ABI's native library for every platform it supports.

```bash
dotnet add package OpenBim.Ifc
```

Task-sized recipes are in the [.NET cookbook](/cookbook/dotnet).

The binding exposes the record model over STEP -- parse, read and edit
attributes, and write -- plus lenient reads, the file header, validation,
ifcXML, the reachability lint, the domain views and writing property sets
(see [Beyond the record model](#beyond-the-record-model)).

## Platforms

| Target framework | Hosts |
| --- | --- |
| `net8.0` | .NET 8 and later |
| `netstandard2.0` | .NET Framework 4.6.2 and later, such as Revit 2024 and older, Navisworks and Tekla; Mono; older .NET |

The package holds the native library for `win-x64`, `win-arm64`,
`linux-x64`, `linux-arm64`, `osx-x64` and `osx-arm64` under
`runtimes/<rid>/native/`. A .NET application picks its runtime's copy
itself. A .NET Framework project, such as an AnyCPU add-in, names no
runtime identifier, so the package's build targets copy the Windows
libraries into its output, and the assembly loads the one for its process
from next to itself. A plug-in a .NET 8 host loads by path finds it the
same way. Every library is 64-bit.

## Read, edit and write

<!-- SNIPPET:dotnet-read-edit-write -->

```csharp
using var model = IfcModel.Parse(data); // or IfcModel.Open("model.ifc")
var schema = model.Schema; // "IFC4"

foreach (var wall in model.IdsOfType("IfcWall"))
{
    var name = (Value.Text)model.Attribute(wall, 2); // Text { Value = Wall }
    model.SetAttribute(wall, 2, new Value.Text(name.Value + " (checked)"));
}

byte[] written = model.Write(); // STEP, ready to save
```

<!-- /SNIPPET -->

`IfcModel` owns a native model through a `SafeHandle`: dispose it, or the
finalizer releases it. Calls on one model are serialised by the library,
so a model may be shared between threads; calls on different models run in
parallel.

Attribute values are a closed record hierarchy, one case per STEP form, so
nothing is lost in a round trip: `Value.Null` is `$`, `Value.Derived` is
`*`, `Value.Unknown` is `.U.` and is never a `Value.Bool`, `Value.Integer`
(64-bit) is not `Value.Real`, and `Value.Typed` keeps a wrapper such as
`IFCLENGTHMEASURE(2.5)` apart from its payload. Values compare by value,
lists included, and print in STEP form. No host value converts
implicitly: `3` could be an integer or a real, and `"x"` a text or an
enumeration.

### Attributes by name

<!-- SNIPPET:dotnet-by-name -->

```csharp
using var model = IfcModel.Parse(data);
var wall = model.IdsOfType("IfcWall").Single();

// Slots as the declared release (here IFC4) defines them, inherited first.
foreach (var attribute in model.AttributeNames(wall))
{
    System.Console.WriteLine($"{attribute.Index} {attribute.Name}: {attribute.TypeName}");
    // 0 GlobalId: IfcGloballyUniqueId, 1 OwnerHistory: IfcOwnerHistory, 2 Name: IfcLabel, ...
}

var name = model.AttributeByName(wall, "name"); // any case: 'Wall'
model.SetAttributeByName(wall, "Name", new Value.Text("Renamed"));
```

<!-- /SNIPPET -->

`AttributeNames(id)` lists an entity's explicit attributes as
`AttributeInfo` records, in slot order with inherited ones first, as the
release its header declares defines them: `IfcTask.Status` is slot 6 in
an IFC2X3 file and slot 7 in an IFC4 one. `AttributeByName` and
`SetAttributeByName` match a name case-insensitively; the positional
calls stay raw slot access. An unknown name is refused with
`unknown-attribute`, a write to a slot the entity's type derives (written
`*`) with `derived-attribute`, and a refused write changes nothing.

`SetAttributeByNamePlain(id, name, value)` (#342) takes a plain .NET
value and coerces it against the attribute's declared type: a `string`
becomes a label (written bare) or the enumeration item it names in any
case; an integer an `INTEGER`, or a `REAL` where one is declared; a
`double` a `REAL`; a `bool` a `BOOLEAN` or `LOGICAL`; a sequence an
aggregate, element by element; an `EntityHandle` a reference, once the
entity exists and is of an accepted type; `null` `$`. In a SELECT, the one
member that takes the value is written as its typed parameter. A `Value`,
nested in a sequence too, is written exactly. A value that does not fit
throws `type-mismatch` (`IfcStatus.TypeMismatch`), one several SELECT
members take `ambiguous-value` (`IfcStatus.AmbiguousValue`), naming them.

<!-- SNIPPET:dotnet-plain -->

```csharp
using var model = IfcModel.Parse(data);
model.SetAttributeByNamePlain(1, "Name", "Renamed"); // IfcLabel: 'Renamed'
model.SetAttributeByNamePlain(1, "PredefinedType", "shear"); // IfcWallTypeEnum: .SHEAR.
// A Value is written exactly; an ambiguous SELECT member is refused.
model.SetAttributeByNamePlain(2, "NominalValue", new Value.Typed("IFCLABEL", new Value.Text("x")));
```

<!-- /SNIPPET -->

## Errors

<!-- SNIPPET:dotnet-errors -->

```csharp
using var model = IfcModel.Parse(data);
try
{
    model.Remove(99);
}
catch (IfcException error) when (error.Code == "missing-entity")
{
    // error.Status == IfcStatus.MissingEntity; error.Message names #99
}
```

<!-- /SNIPPET -->

Every refusal throws `IfcException`. Its `Code` is the stable code every
binding shares (`parse`, `missing-entity`, `invalid-value`,
`unsupported-schema`, ...), never renamed or reused, and its `Status` is
the C ABI's status. A refused call leaves the model unchanged. A disposed
model throws `ObjectDisposedException`.

## Beyond the record model

<!-- SNIPPET:dotnet-beyond-records -->

```csharp
// A damaged export: skip what cannot be read, and say what was skipped.
using var model = IfcModel.Parse(data, ParseOptions.Lenient);
var skipped = model.Diagnostics; // one message per recovery

var header = model.Header; // Header { Name = ..., Author = [...], Schema = [...] }
model.Header = header with { Author = new EquatableList<string>(new[] { "Reviewer" }) };

var report = model.Validate(); // ValidationReport { Conformant = ..., Findings = [...] }
var errors = report.Findings.Where(finding => finding.Severity == "error").ToList();

var xml = model.WriteIfcXml(); // lossless ifcXML; or xsdProfile: "IFC4"
using var fromXml = IfcModel.ParseIfcXml(xml);
```

<!-- /SNIPPET -->

- **Lenient reads.** `IfcModel.Parse` and `IfcModel.Open` take a
  `ParseOptions` record: `OnMalformed.Skip`, `CheckReferences`,
  `AcceptRealWithoutPoint`, or the `ParseOptions.Lenient` preset. Every
  recovery is listed in `Diagnostics`.
- **Header.** `model.Header` is a `Header` record with every
  `FILE_DESCRIPTION`, `FILE_NAME` and `FILE_SCHEMA` field; assign a changed
  copy (`with`) to replace it.
- **Validation.** `Validate(maxFindings)` checks the model against the
  schema its header declares and returns a `ValidationReport`: counts by
  severity, `Conformant`, `Truncated`, and `ValidationFinding`s sorted by
  severity, rule, entity and slot.
- **ifcXML.** `WriteIfcXml()` and `IfcModel.ParseIfcXml(data)` use this
  library's lossless layout; `xsdProfile: "IFC4"` or `"IFC4X3_ADD2"`
  selects the buildingSMART XSD layout, which refuses with `write` what it
  cannot carry exactly.
- **Reachability.** `UnreachableProducts()` lists products no viewer will
  draw as `UnreachableProduct`s with a stable `Reason`.

### Domain views

<!-- SNIPPET:dotnet-domain-views -->

```csharp
using var model = IfcModel.Parse(data);
var wall = model.IdsOfType("IfcWall").Single();

// Property sets: the wall's own first, then its type's; values typed.
foreach (var set in model.PropertySets(wall))
{
    foreach (var property in set.Properties)
    {
        System.Console.WriteLine($"{set.Name}.{property.Name} = {property.Value}");
        // Pset_WallCommon.IsExternal = IFCBOOLEAN(.T.)
    }
}

var material = model.Material(wall); // MaterialAssignment { Kind = "layer-set", Layers = [...] }
var tree = model.SpatialTree(); // SpatialTree { Nodes = [...] }
var classes = model.Classifications(wall); // [Classification { Identification = ... }]
```

<!-- /SNIPPET -->

<!-- SNIPPET:dotnet-property-sets-many -->

```csharp
var every = model.PropertySetsMany(); // every object definition, one pass
foreach (var answer in model.PropertySetsMany(new ulong[] { 3, 20 }))
{
    // Refusal: the code and message PropertySets(answer.Object) would throw.
    System.Console.WriteLine($"#{answer.Object}: {answer.Refusal?.Code ?? answer.Sets.Count + " set(s)"}");
}
```

<!-- /SNIPPET -->

The domain views of the Rust facade cross the C ABI as value tapes and
arrive as C# records, keyed by entity id with the `GlobalId` where the
entity has one; lists are `EquatableList<T>`, and IFC values the cases of
`Value`, typed with their declared type. A view reads the model as it is
at the call.

- **Property sets.** `PropertySets(id)` returns `PropertySet`s: the
  object's own, then those its type object holds, an occurrence property
  overriding an inherited one. `ResolveUnit(measureType, unit)` resolves a
  property's unit, or the project default, exactly to SI. Read against
  IFC2X3, IFC4 or IFC4X3.
  `PropertySetsMany(ids)` (#358) answers for many objects, or with null
  every object definition, in one pass through one property index: linear
  in the model where a loop of `PropertySets` is quadratic. Each
  `ObjectPropertySets` holds exactly what `PropertySets` returns for its
  object, or the `PropertyRefusal` (code, message) it throws.
- **Spatial tree.** `SpatialTree()` returns a `SpatialTree` of
  `SpatialNode`s with parents, children, contained and referenced elements.
- **Classification.** `Classifications(id)` returns the object's own and
  its type's `Classification`s with their `ClassificationSystem`.
- **Material.** `Material(id)` returns the one `MaterialAssignment` that
  applies, its own or its type's, or null.
- **Systems.** `Systems()` returns a `SystemsView`: every `IfcSystem`
  with members and served structures, and the `SystemAnomaly`s.
- **Cost.** `Cost()` returns the `CostSchedule`s and `CostItem`s with
  their `CostValue` trees.
- **Georeferencing.** `Georeferencing()` returns a `MapConversion` per
  coordinate operation (IFC4, IFC4X3), resolved with the project length
  unit.

Two records are named differently from the other bindings: the core's
`System` is `IfcSystem`, since a type named `System` would hide the
`System` namespace, and `Systems` is `SystemsView`, since a record cannot
have a member named like itself. Refusals carry the shared codes:
`unsupported-schema`, `invalid-model`, `missing-reference`,
`budget-exceeded`, `unsupported` and `wrong-entity-type`.

#### Writing property sets

<!-- SNIPPET:dotnet-domain-write -->

```csharp
// Wall #3 inherits IsExternal from its type: the write overrides it on
// the wall and never changes the type's shared set.
var holders = model.SetProperties(new[]
{
    new PropertyEdit(3, "Pset_WallCommon", "IsExternal", new Value.Typed("IFCBOOLEAN", new Value.Bool(false))),
    new PropertyEdit(3, "Custom", "Note", new Value.Typed("IFCLABEL", new Value.Text("checked"))),
});
// holders: per edit, the entity now holding the value

var own = model.PropertySets(3).Single(set => set.Source == "occurrence" && set.Name == "Pset_WallCommon");
```

<!-- /SNIPPET -->

`SetProperties(edits)` writes and removes property and quantity values as
one checked transaction: every `PropertyEdit`, in order, or, when any is
refused, none, and the model is unchanged. An edit addresses a property
the way `PropertySets` reports it, by object, set name and property name,
and its `Value` is that property's `Value`. `PropertyEdit.Removal(object,
set, name)` removes one; `SetProperty` and `RemoveProperty` are the
one-edit forms. The result holds, per edit, the entity now holding the
value. Values are checked against the declared release and, for a
`Pset_`/`Qto_` set, the release's PSD/QTO catalog, which the native
library embeds; the refusals are those of the other bindings
(`invalid-value`, `template-violation`, `missing-property`, `unsupported`,
`wrong-entity-type`).

Not bound yet: checked multi-edit transactions over arbitrary entities.
Use the Rust crates for those.

## Geometry

Geometry crosses at three levels ([ADR 0021](/adr/0021-bindings-carry-placements-and-opt-in-meshes)):
placements, the exact neutral representation and meshes.
`ProductPlacements(ids)` returns, for each product with a shape (or for
the ids given), its world placement and the Body representation a viewer
draws.

<!-- SNIPPET:dotnet-geometry -->

```csharp
using var model = IfcModel.Parse(data);
foreach (var product in model.ProductPlacements())
{
    if (product.Refusal is { } refusal)
    {
        System.Console.WriteLine($"#{product.Id}: {refusal.Code} {refusal.Message}");
        continue;
    }
    // A column-major 4x4 in metres; the translation is its last column.
    var t = product.Transform!;
    System.Console.WriteLine($"{product.TypeName} at {t[12]} {t[13]} {t[14]}, Body {product.Representation?.RepresentationType}");
}
```

<!-- /SNIPPET -->

`ProductPlacement.Transform` is a 4x4 column-major matrix in metres;
`Representation` is a `SelectedRepresentation`, or null for a product
with an axis or footprint only. A product that cannot be placed is a
record with a `GeometryRefusal` (`Code` `unsupported`, `invalid-model`,
`missing-reference` or `budget-exceeded`), not an exception.
`ProductMeshes(ids)` returns a `MeshedProduct` per product: its
`ProductMesh` record and `float[] Positions` (relative to its transform)
and `uint[] Indices`. It needs a native library built with the C ABI's
`mesh` feature; the packaged one throws `IfcException` with code
`feature-disabled`.

`ProductGeometry(ids, encoding)` returns each product's Body as Axiolid's
neutral geometry graph ([#367](https://github.com/openbimrs/ifc/issues/367)),
exact, for your own kernel: a `ProductGraph` per product, its
`ProductGeometry` record (transform, encoding, payload size, refusal) and
`byte[] Payload`, Axiolid's versioned wire format as UTF-8 JSON
(`GeometryEncoding.Json`, also as `string Json`) or CBOR
(`GeometryEncoding.Cbor`). It needs a native library built with the C
ABI's `graph` feature; the packaged one throws `IfcException` with code
`feature-disabled`.

<!-- SNIPPET:dotnet-geometry-graph -->

```csharp
using var model = IfcModel.Parse(data);
foreach (var product in model.ProductGeometry())
{
    if (product.Geometry.Refusal is { } refusal)
    {
        System.Console.WriteLine($"#{product.Geometry.Id}: {refusal.Code}");
        continue;
    }
    // {"format":"axiolid-geometry-graph","version":"1.1","graph":{...}},
    // exact, in world metres: hand it to your own kernel's reader.
    System.Console.WriteLine($"{product.Geometry.TypeName}: {product.Json}");
}
```

<!-- /SNIPPET -->

The envelope is `{"format":"axiolid-geometry-graph","version":"1.1",
"graph":{"nodes":[...],"roots":[...]}}`, in world coordinates, metres,
with the record's transform already applied. Its `version` is the lowest the content needs: `1.0`, or `1.1` when a
station carries a seam-snapping window
([#423](https://github.com/openbimrs/ifc/issues/423)); `axiolid-model`
0.3.9 labels every payload `1.1` (axiolid/kernel#297), and a reader on
`axiolid-model` 0.3.8 or older refuses `1.1`. `ProductGraph.Format`
and `ProductGraph.FormatVersion` name the format and the newest version
the package writes, `"1.1"`. A major version of the wire format
would be a breaking release of the package (ADR 0021).

## Creating entities

<!-- SNIPPET:dotnet-authoring -->

```csharp
// IfcModel.Handle(i): the entity operation i of the batch produces.
var ids = model.Author(new[]
{
    AuthorOp.Project(new Dictionary<string, Value> { ["Name"] = new Value.Text("Demo") }), // 0
    AuthorOp.Placement(), // 1: at the origin
    AuthorOp.Spatial("IfcSite", IfcModel.Handle(0), placement: IfcModel.Handle(1)), // 2
    AuthorOp.Spatial("IfcBuilding", IfcModel.Handle(2)), // 3
    AuthorOp.Placement(relativeTo: IfcModel.Handle(1)), // 4
    AuthorOp.Spatial("IfcBuildingStorey", IfcModel.Handle(3), placement: IfcModel.Handle(4)), // 5
    AuthorOp.TypeObject("IfcWallType", new Dictionary<string, Value> { ["PredefinedType"] = new Value.Enum("STANDARD") }), // 6
    AuthorOp.Placement(relativeTo: IfcModel.Handle(4), location: (1, 2, 0)), // 7
    AuthorOp.Product(
        "IfcWall",
        new Dictionary<string, Value> { ["Name"] = new Value.Text("Wall") },
        container: IfcModel.Handle(5), // IfcRelContainedInSpatialStructure
        placement: IfcModel.Handle(7),
        typeObject: IfcModel.Handle(6)), // IfcRelDefinesByType
});
var wall = ids[8]!.Value; // every IfcRoot got a GlobalId
```

<!-- /SNIPPET -->

`model.Author(ops)`, with `AuthorOp` built by its factory methods, creates
and edits entities as one checked transaction against the release the header
declares: every operation, in order, or, when any is refused, none, and the
model is unchanged. An operation names the entity an earlier operation of
the same batch produced by `IfcModel.Handle(index)`, anywhere an id goes,
attribute values included, and the result holds per operation the id its
entity received. `CreateEntity(type, attributes)` and
`RemoveWithRelationships(id)` are the one-operation forms. A model built
from nothing needs a header naming its release first (assign `Header`).

| Operation | What it writes |
| --- | --- |
| `Create` | one entity by `type` and named `attributes` |
| `Edit` | named `attributes` of `entity`; the whole entity is checked again |
| `Remove` | removes `entity` and takes it out of every relationship; a relationship left without an end goes too |
| `Project` | the model's one `IfcProject` |
| `Spatial` | a spatial element of `type` and its `IfcRelAggregates` under `parent` |
| `Product` | a product, its `IfcRelContainedInSpatialStructure` in `container` and its `IfcRelDefinesByType` by `typeObject` |
| `TypeObject` | a type object (`IfcWallType`, ...) |
| `AssignType`, `Contain`, `Aggregate` | one relationship; an object already related is refused |
| `Placement` | an `IfcLocalPlacement` over an `IfcAxis2Placement3D` at `location`, relative to `relativeTo`; `axis` and `refDirection` both or neither |
| `OwnerHistory` | an `IfcOwnerHistory` with its person, organization and application |

Every record is built by attribute name through `ifc-author` against the
declared release, and refused with the shared codes: a type the release does
not declare (`unsupported-schema`) or an abstract one (`wrong-entity-type`);
an unknown name (`unknown-attribute`); a value of the wrong type or form, or
an aggregate outside its declared bounds (`invalid-value`); a required
attribute left unset (`missing-attribute`); a derived one set
(`derived-attribute`); a reference to an entity that does not exist
(`missing-reference`) or of a type the attribute does not accept
(`wrong-entity-type`). An object is contained, aggregated and typed once and
a model holds one `IfcProject` (`invalid-model`); a removal an entity other
than a relationship still needs is `still-referenced`.

An `IfcRoot` created without a `GlobalId` gets a fresh one. `OwnerHistory`
is never invented: a builder writes the one it is given on every record it
creates, and IFC2X3, which requires it, refuses a record without
(`missing-attribute`).

## How it is tested

The package is tested as NuGet installs it: `check-dotnet.py` packs the
`.nupkg`, installs it from a local feed into a fresh project and runs the
C# suite there, the smoke test of the C binding in C#. The `.NET` workflow
does this on Linux, macOS, Windows (also on .NET Framework 4.8) and
Windows on Arm; every release does it on each of the six runtimes before
it publishes. On every pull request, the gate holds the C# declarations to
the C header and the C# records to the shared records, field by field.

## API

Generated from the `OpenBim.Ifc` C# source. The
[.NET API reference](/api/dotnet/index.html){target="_self"} documents every
public type of the package, the domain records, `AuthorOp`, `ParseOptions`
and `MeshedProduct` included, from its XML documentation comments.

<!-- API:DOTNET:BEGIN -->

| Member | Description |
| --- | --- |
| `new IfcModel()` | An empty model. |
| `static IfcModel Parse(byte[] data, ParseOptions? options = null)` | Parse a STEP (`.ifc`) file from its bytes; `options` relaxes the strict read. |
| `static IfcModel ParseIfcXml(byte[] data, string? xsdProfile = null)` | Parse an ifcXML document: this library's lossless layout, or with `xsdProfile` (`IFC4` or `IFC4X3_ADD2`) the buildingSMART XSD layout of that release. |
| `static IfcModel Open(string path, bool mapped = false, ParseOptions? options = null)` | Read a STEP file from disk, once, straight into the model; `io` when it cannot be read. |
| `byte[] Write()` | Serialize as STEP bytes. |
| `byte[] WriteIfcXml(string? xsdProfile = null)` | Serialize as ifcXML bytes, in the layout `ParseIfcXml` reads; an XSD-layout write refuses with `write` what the layout cannot carry. |
| `Header Header { get; set; }` | The STEP file header; assign a changed copy (`with`) to replace it. |
| `ValidationReport Validate(int? maxFindings = null)` | Validate against the schema the header declares; findings sorted by severity, rule, entity and slot. `maxFindings` caps the report (default 10,000). |
| `IReadOnlyList<UnreachableProduct> UnreachableProducts()` | Products no viewer will draw, with a stable reason, in id order. |
| `IReadOnlyList<ProductPlacement> ProductPlacements(IReadOnlyList<ulong>? ids = null)` | Each product's world placement (a column-major 4x4 in metres) and the Body representation a viewer draws, for `ids` or, when null, every product with a shape, in id order (#328). |
| `IReadOnlyList<ProductGraph> ProductGeometry(IReadOnlyList<ulong>? ids = null, GeometryEncoding encoding = GeometryEncoding.Json)` | Each product's Body as Axiolid's neutral geometry graph (#367), for `ids` or, when null, every product with a shape, in id order, encoded as `encoding` in Axiolid's wire format (1.0, or 1.1 when a station carries a seam-snapping window). |
| `IReadOnlyList<MeshedProduct> ProductMeshes(IReadOnlyList<ulong>? ids = null)` | Each product's Body as triangles from the reference backend, for `ids` or, when null, every product with a shape, in id order (#328). |
| `IReadOnlyList<PropertySet> PropertySets(ulong id)` | The property sets, quantity sets and predefined property sets of object `id`: its own first, then those its type object holds, an occurrence property overriding an inherited one of the same name. |
| `IReadOnlyList<ObjectPropertySets> PropertySetsMany(IReadOnlyList<ulong>? ids = null)` | `PropertySets` of each of `ids`, in that order, or, when null, of every object definition (`IfcObjectDefinition` and its subtypes) in file order, in one pass (#358). |
| `ResolvedUnit ResolveUnit(string measureType, ulong? unit = null)` | The effective unit of a `measureType` value (`IFCAREAMEASURE`): `unit` when given (a property's stated unit), otherwise the project default, resolved exactly to SI. |
| `IReadOnlyList<ulong?> SetProperties(IEnumerable<PropertyEdit> edits)` | Write and remove property and quantity values as one checked transaction: every edit, in order, or none and the model unchanged. |
| `const ulong HandleBase = 4611686018427387904UL { get; }` | The first id of the handle range (2^62): `HandleBase + i` names the entity operation `i` of an `Author` batch produced. |
| `static ulong Handle(int index) =>` | The handle of the entity operation `index` of an `Author` batch produces, usable wherever a later operation takes an id. |
| `IReadOnlyList<ulong?> Author(IEnumerable<AuthorOp> ops)` | Apply authoring operations as one checked transaction against the release the header declares: every operation, in order, or none and the model unchanged. |
| `ulong CreateEntity(string type, IEnumerable<KeyValuePair<string, Value>>? attributes = null)` | Create one entity of `type` from named attributes, checked against the declared release (`Author` with one `AuthorOp.Create`); returns its id. |
| `void RemoveWithRelationships(ulong id)` | Remove entity `id` with the relationships that reference it, leaving nothing dangling; refused with `still-referenced` while an entity other than a relationship needs it. |
| `ulong SetProperty(ulong obj, string set, string name, Value value, string? setType = null)` | Write one value (`SetProperties` with one edit); returns the id of the entity holding it. |
| `void RemoveProperty(ulong obj, string set, string name)` | Remove one property from `obj`'s own set (`SetProperties` with one edit). |
| `SpatialTree SpatialTree()` | The spatial containment tree: every container with its parent, sub-containers and contained elements. |
| `IReadOnlyList<Classification> Classifications(ulong id)` | The classifications of object `id`: its own, then its type's. |
| `MaterialAssignment? Material(ulong id)` | The material association of object `id`, its own or its type's, or null. |
| `SystemsView Systems()` | Every system with its members and served structures, and the memberships the reader could not honour. |
| `Cost Cost()` | Every cost schedule and cost item; values as authored, typed. |
| `IReadOnlyList<MapConversion> Georeferencing()` | Every coordinate operation resolved with the project length unit; empty when the model has none. |
| `int Count { get; }` | Number of entities. |
| `string? Schema { get; }` | The first `FILE_SCHEMA` token, e.g. `IFC4`, or null. |
| `IReadOnlyList<string> Diagnostics { get; }` | Non-fatal problems found while reading, such as the records a lenient read skipped. |
| `IReadOnlyList<ulong> Ids()` | Every entity id, in file order. |
| `IReadOnlyList<ulong> IdsOfType(string typeName)` | Ids of every entity of exactly `typeName` (case-insensitive); subtypes are not included. |
| `IReadOnlyList<ulong> IdsOfTypeIncludingSubtypes(string typeName)` | Ids of `typeName` or any subtype, per the file's schema; `unsupported-schema` when it is not bundled. |
| `string TypeOf(ulong id)` | The upper-case type name of entity `id`. |
| `IReadOnlyList<Value> Attributes(ulong id)` | Every attribute of entity `id`, in declaration order. |
| `Value Attribute(ulong id, int index)` | Attribute `index` of entity `id`; `Value.Null` past the end. |
| `Value SetAttribute(ulong id, int index, Value value)` | Set attribute `index` of entity `id`, padding a gap past the end with `$`; returns the old value. |
| `IReadOnlyList<AttributeInfo> AttributeNames(ulong id)` | Every explicit attribute of entity `id` in slot order, inherited first, as the release the header declares defines them; `INVERSE` attributes hold no slot and are not listed. |
| `Value AttributeByName(ulong id, string name)` | Attribute `name` of entity `id`, matched case-insensitively (`Name`) and resolved against the declared release; `Value.Null` when the record stops before its slot. |
| `Value SetAttributeByName(ulong id, string name, Value value)` | Set attribute `name` of entity `id`; returns the old value. A derived attribute is refused with `derived-attribute`, an unknown name with `unknown-attribute`, and a refused write changes nothing. |
| `Value SetAttributeByNamePlain(ulong id, string name, object? value)` | Set attribute `name` of entity `id` from a plain .NET value, coerced against the attribute's declared type in the declared release (#342); returns the old value. |
| `ulong Add(string typeName, IEnumerable<Value> attributes)` | Append an entity of `typeName`; returns its new id. |
| `void Remove(ulong id)` | Remove entity `id`, leaving references to it dangling. |
| `IReadOnlyList<DanglingReference> DanglingReferences()` | Every reference to an id the model does not contain. |
| `bool IsDisposed { get; }` | Whether `Dispose` has released the native model. |
| `void Dispose()` | Release the native model. Every later call throws `ObjectDisposedException`. |

Attribute values are the cases of the closed record `Value`:

| Value | Meaning |
| --- | --- |
| `Value.Null` | `$`: the attribute is not set. |
| `Value.Derived` | `*`: derived in a supertype; distinct from `$`. |
| `Value.Bool(bool Value)` | `.T.` or `.F.`. |
| `Value.Unknown` | `.U.`: the third logical state, never a `Bool`. |
| `Value.Integer(long Value)` | An integer literal (64-bit). |
| `Value.Real(double Value)` | A real literal; finite, or the library refuses it with `invalid-value`. |
| `Value.Text(string Value)` | A string literal, decoded. |
| `Value.Binary(string Value)` | A binary literal: its hexadecimal digits, as written in the file. |
| `Value.Enum(string Value)` | An enumeration value, e.g. `.ELEMENT.`. |
| `Value.Ref(ulong Id)` | A reference to an entity, `#42`. |
| `Value.List(EquatableList<Value> Items)` | An aggregate, `(1,2)`. |
| `Value.Typed(string Type, Value Value)` | A typed value, `IFCLENGTHMEASURE(2.5)`. |

<!-- API:DOTNET:END -->
