# C# and .NET cookbook

Task-sized recipes for [`OpenBim.Ifc`](/bindings/dotnet), the .NET
binding of the IFC core. Each recipe is a region of
`crates/openbim-ifc-dotnet/dotnet/OpenBim.Ifc.Tests/CookbookTests.cs`,
which runs against the packed NuGet package on Linux, macOS and Windows
and checks what the recipe prints, so the code below is the code that ran.
The comments show its output for the two files the test carries inline: a
trimmed copy of `test/fixtures/synthetic-properties/synthetic_properties.ifc`
(two walls on one storey, with their property sets) and one placed,
extruded wall.

For the full surface, see the [binding page](/bindings/dotnet) and the
[.NET API reference](/api/dotnet/index.html){target="_self"}.

## Set up

```bash
dotnet add package OpenBim.Ifc
```

The recipes use `using OpenBim.Ifc;`, `using System.Linq;`, `using System.IO;`
and `using System.Collections.Generic;`. `IfcModel` owns a native model:
dispose it, here with `using var`.

## Open a file

<!-- SNIPPET:cookbook-dotnet-open -->

```csharp
using var model = IfcModel.Open(path); // read once, straight from disk
Console.WriteLine($"{model.Schema}, {model.Count} entities"); // IFC4, 32 entities
Console.WriteLine($"{model.Header.OriginatingSystem}, {model.Header.TimeStamp}");

using var same = IfcModel.Parse(File.ReadAllBytes(path)); // or from bytes you hold
```

<!-- /SNIPPET -->

A file that does not parse throws `IfcException` with `Code` `parse`;
`IfcModel.Parse(data, ParseOptions.Lenient)` reads a damaged export and
lists what it skipped in `model.Diagnostics`.

## Read property sets

One object's property sets, its own first, then those it inherits from
its type:

<!-- SNIPPET:cookbook-dotnet-property-sets -->

```csharp
var wall = model.IdsOfType("IfcWall").First(id => model.AttributeByName(id, "Name") == new Value.Text("Wall A"));
foreach (var set in model.PropertySets(wall))
{
    // Source: "occurrence" (the wall's own) or "type" (inherited from its type)
    foreach (var property in set.Properties)
    {
        Console.WriteLine($"{set.Name}.{property.Name} = {property.Value} ({set.Source})");
    }
}
// Pset_WallCommon.IsExternal = IFCBOOLEAN(.F.) (occurrence)
// Qto_WallBaseQuantities.Width = IFCLENGTHMEASURE(200) (occurrence)
// Pset_WallCommon.FireRating = IFCLABEL('F30') (type)
```

<!-- /SNIPPET -->

Many objects in one call, here every wall:

<!-- SNIPPET:cookbook-dotnet-property-sets-many -->

```csharp
// One pass for every wall: the property index is built once, so this stays
// linear in the model where a loop of PropertySets is quadratic.
var rows = new Dictionary<ulong, Dictionary<string, Value>>();
foreach (var answer in model.PropertySetsMany(model.IdsOfType("IfcWall")))
{
    if (answer.Refusal is { } refusal)
    {
        Console.Error.WriteLine($"#{answer.Object}: {refusal.Code} {refusal.Message}");
        continue;
    }
    var row = rows[answer.Object] = new Dictionary<string, Value>();
    foreach (var set in answer.Sets)
    {
        foreach (var property in set.Properties)
        {
            var key = $"{set.Name}.{property.Name}";
            if (!row.ContainsKey(key)) row[key] = property.Value; // the occurrence's value wins
        }
    }
}
```

<!-- /SNIPPET -->

`PropertySetsMany()` with no ids answers for every object definition in
the file.

## List storeys and their elements

<!-- SNIPPET:cookbook-dotnet-storeys -->

```csharp
var tree = model.SpatialTree();
foreach (var storey in tree.Nodes.Where(node => node.Kind == "storey"))
{
    Console.WriteLine($"{storey.Name} (#{storey.Id})");
    foreach (var id in storey.Elements)
    {
        Console.WriteLine($"  {model.TypeOf(id)} #{id} {model.AttributeByName(id, "Name")}");
    }
}
// Level 0 (#25)
//   IFCWALL #30 'Wall A'
//   IFCWALL #31 'Wall B'
```

<!-- /SNIPPET -->

Each node also has its `Parent` and `Children`, so the same tree gives the
project, site and building above the storeys and the spaces below them;
`tree.Orphans` lists elements no container holds.

## Validate

<!-- SNIPPET:cookbook-dotnet-validate -->

```csharp
var report = model.Validate(); // against the schema the header declares
if (!report.Conformant)
{
    Console.WriteLine($"{report.Errors} error(s), {report.Warnings} warning(s)");
}
foreach (var finding in report.Findings)
{
    // Severity: "error", "evaluation-error", "warning" or "unsupported"
    var at = finding.Entity is { } id ? $"#{id}" : "file";
    Console.WriteLine($"{finding.Severity} {finding.Rule} {at} {finding.AttributeName}: {finding.Message}");
}
```

<!-- /SNIPPET -->

The recipe ran on the file with Wall A's `PredefinedType` set to an item
`IfcWallTypeEnum` does not have. `Validate(maxFindings)` caps the report;
`report.Truncated` says when the cap was reached.

## Convert to and from ifcXML

<!-- SNIPPET:cookbook-dotnet-ifcxml -->

```csharp
byte[] xml = model.WriteIfcXml(); // this library's lossless layout, UTF-8
using var back = IfcModel.ParseIfcXml(xml);
byte[] step = back.Write(); // the same entities, as STEP again

// The buildingSMART XSD layout of a release, for tools that read it.
byte[] xsd = model.WriteIfcXml(xsdProfile: "IFC4");
using var fromXsd = IfcModel.ParseIfcXml(xsd, xsdProfile: "IFC4");
```

<!-- /SNIPPET -->

The lossless layout carries everything the STEP file does. An XSD layout
refuses, with `Code` `write`, what it cannot carry exactly.

## Edit a property set and attributes

<!-- SNIPPET:cookbook-dotnet-edit -->

```csharp
var wall = model.IdsOfType("IfcWall").First(id => model.AttributeByName(id, "Name") == new Value.Text("Wall B"));

// Property values are exact IFC values; Pset_ and Qto_ sets are checked
// against the release's PSD/QTO catalog, which the native library embeds.
// One checked transaction: every edit, or none and the model unchanged.
model.SetProperties(new[]
{
    // Wall B inherits FireRating from its type: this overrides it on the wall.
    new PropertyEdit(wall, "Pset_WallCommon", "FireRating", new Value.Typed("IFCLABEL", new Value.Text("F90"))),
    // A set the wall lacks is created with its relationship.
    new PropertyEdit(wall, "Checks", "Reviewer", new Value.Typed("IFCLABEL", new Value.Text("QA"))),
});

// Attributes take plain values, coerced against their declared type.
model.SetAttributeByNamePlain(wall, "Name", "Wall B (checked)"); // IfcLabel
model.SetAttributeByNamePlain(wall, "PredefinedType", "solidwall"); // IfcWallTypeEnum: .SOLIDWALL.

File.WriteAllBytes(path, model.Write());
```

<!-- /SNIPPET -->

Property values are exact `Value`s, typed as `PropertySets` reports them;
`SetAttributeByNamePlain` takes plain .NET values (`string`, integers,
`double`, `bool`, sequences, `EntityHandle`, `null`) coerced against the
attribute's declared type. A refused edit throws `IfcException` and
changes nothing: `invalid-value`, `template-violation` (the PSD/QTO catalog
disagrees), `type-mismatch` (a plain value that does not fit).

## Read placements, meshes and exact geometry

<!-- SNIPPET:cookbook-dotnet-placements -->

```csharp
foreach (var product in model.ProductPlacements())
{
    if (product.Refusal is { } refusal) // per product, never thrown
    {
        Console.WriteLine($"#{product.Id}: {refusal.Code}");
        continue;
    }
    // A column-major 4x4 in metres; the translation is its last column.
    var t = product.Transform!;
    Console.WriteLine($"{product.TypeName} at {t[12]} {t[13]} {t[14]}, Body {product.Representation?.RepresentationType}");
}
// IFCWALL at 1 2 0, Body SweptSolid
```

<!-- /SNIPPET -->

Meshes need a native library built with the C ABI's `mesh` feature; the
packaged one leaves the geometry kernel out:

<!-- SNIPPET:cookbook-dotnet-meshes -->

```csharp
// A native library built with the C ABI's `mesh` feature; the packaged
// one throws IfcException with Code "feature-disabled".
foreach (var meshed in model.ProductMeshes())
{
    if (meshed.Mesh.Refusal is { } || meshed.Indices.Length == 0) continue; // refused, or no Body
    var m = meshed.Mesh.Transform!; // column-major, metres; positions are relative to it
    var p = meshed.Positions; // x y z per vertex; Indices: three per triangle
    var world = new double[p.Length];
    for (var i = 0; i < p.Length; i += 3)
    {
        for (var axis = 0; axis < 3; axis++)
        {
            world[i + axis] = m[axis] * p[i] + m[4 + axis] * p[i + 1] + m[8 + axis] * p[i + 2] + m[12 + axis];
        }
    }
    Console.WriteLine($"{meshed.Mesh.TypeName}: {meshed.Indices.Length / 3} triangles, x from {world.Where((_, i) => i % 3 == 0).Min()}");
}
```

<!-- /SNIPPET -->

Positions are `float`s relative to each product's `double` transform, so a
site kilometres from the origin keeps its millimetres.

The exact representation instead of triangles
([#367](https://github.com/openbimrs/ifc/issues/367)): each Body as
Axiolid's neutral geometry graph in its wire format 1.0, for your own
kernel. It needs a native library built with the C ABI's `graph`
feature:

<!-- SNIPPET:cookbook-dotnet-graphs -->

```csharp
// A native library built with the C ABI's `graph` feature; the packaged
// one throws IfcException with Code "feature-disabled".
foreach (var product in model.ProductGeometry()) // or GeometryEncoding.Cbor
{
    if (product.Geometry.Refusal is { } || product.Payload.Length == 0) continue; // refused, or no Body
    // Axiolid's wire format 1.0, exact, in world metres: UTF-8 JSON for
    // your JSON reader (System.Text.Json, Newtonsoft) or your kernel.
    byte[] payload = product.Payload;
    Console.WriteLine($"{product.Geometry.TypeName}: {payload.Length} bytes, {product.Json!.Substring(0, 52)}");
}
```

<!-- /SNIPPET -->

`ProductGraph.Payload` is the envelope `{"format":"axiolid-geometry-graph",
"version":"1.0","graph":{...}}`, exact, in world coordinates, metres;
`GeometryEncoding.Cbor` gives it as CBOR. The
[binding page](/bindings/dotnet#geometry) describes it.

## Create a model

<!-- SNIPPET:cookbook-dotnet-create -->

```csharp
using var model = new IfcModel();
model.Header = model.Header with
{
    Name = "new.ifc",
    TimeStamp = DateTime.Now.ToString("s"), // 2026-10-08T12:00:00
    OriginatingSystem = "cookbook",
    Schema = new EquatableList<string>(new[] { "IFC4" }), // the release every operation is checked against
};
static Dictionary<string, Value> Named(string name) => new() { ["Name"] = new Value.Text(name) };
// IfcModel.Handle(i): the entity operation i of this batch creates.
var ids = model.Author(new[]
{
    AuthorOp.Project(Named("Demo")), // 0
    AuthorOp.Placement(), // 1
    AuthorOp.Spatial("IfcSite", IfcModel.Handle(0), placement: IfcModel.Handle(1)), // 2
    AuthorOp.Spatial("IfcBuilding", IfcModel.Handle(2)), // 3
    AuthorOp.Spatial("IfcBuildingStorey", IfcModel.Handle(3), Named("Level 0")), // 4
    AuthorOp.Placement(relativeTo: IfcModel.Handle(1), location: (4, 0, 0)), // 5
    AuthorOp.Product("IfcWall", Named("Wall"), container: IfcModel.Handle(4), placement: IfcModel.Handle(5)), // 6
});
var wall = ids[6]!.Value; // every IfcRoot got a GlobalId
byte[] data = model.Write();
```

<!-- /SNIPPET -->

Every operation is checked against the release the header declares, and a
batch is one transaction. The [binding page](/bindings/dotnet#creating-entities)
lists every operation.
