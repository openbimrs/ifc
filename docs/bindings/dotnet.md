# C# and .NET

`OpenBim.Ifc` is the .NET binding of the IFC core
([`openbim-ifc-dotnet`](/reference/crates/openbim-ifc-dotnet)), published to
NuGet. It is C# over the versioned [C ABI](/bindings/c), and the package
carries the C ABI's native library for every platform it supports.

```bash
dotnet add package OpenBim.Ifc
```

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

Not bound yet: geometry, and checked multi-edit transactions over
arbitrary entities. Use the Rust crates for those.

## How it is tested

The package is tested as NuGet installs it: `check-dotnet.py` packs the
`.nupkg`, installs it from a local feed into a fresh project and runs the
C# suite there, the smoke test of the C binding in C#. The `.NET` workflow
does this on Linux, macOS, Windows (also on .NET Framework 4.8) and
Windows on Arm; every release does it on each of the six runtimes before
it publishes. On every pull request, the gate holds the C# declarations to
the C header and the C# records to the shared records, field by field.

## API

Generated from the `OpenBim.Ifc` C# source.

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
| `IReadOnlyList<PropertySet> PropertySets(ulong id)` | The property sets, quantity sets and predefined property sets of object `id`: its own first, then those its type object holds, an occurrence property overriding an inherited one of the same name. |
| `ResolvedUnit ResolveUnit(string measureType, ulong? unit = null)` | The effective unit of a `measureType` value (`IFCAREAMEASURE`): `unit` when given (a property's stated unit), otherwise the project default, resolved exactly to SI. |
| `IReadOnlyList<ulong?> SetProperties(IEnumerable<PropertyEdit> edits)` | Write and remove property and quantity values as one checked transaction: every edit, in order, or none and the model unchanged. |
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
