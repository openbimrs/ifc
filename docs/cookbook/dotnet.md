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

Curves have no triangles: an `Axis`, a `FootPrint` or an annotation's
curves come as polylines from the same native library
([#458](https://github.com/openbimrs/ifc/issues/458)), a `CurvedProduct`
per product with one `float[]` of positions and a `ProductPolyline` per
curve naming its slice and the certified bound on how far the exact curve
lies from it:

<!-- SNIPPET:dotnet-geometry-curves -->

```csharp
foreach (var product in model.ProductCurves(null, "Axis"))
{
    foreach (var line in product.Curves.Polylines)
    {
        // Positions: x y z per vertex, metres, relative to
        // product.Curves.Transform; this curve is a slice of them.
        var xyz = product.Positions.Skip((int)line.FirstVertex * 3).Take((int)line.VertexCount * 3).ToArray();
        printed.Add($"#{product.Curves.Id}: {xyz.Length / 3} vertices, bound {line.ChordBound}");
    }
}
```

<!-- /SNIPPET -->

Trimmed, composite, offset and intrinsic curves compile
([#506](https://github.com/openbimrs/ifc/issues/506)). A curve the
reference compiler has no certified polyline for (an implicit curve, an
offset of an offset) is refused with code `unsupported`, its message the
compiler's reason.

The exact representation instead of triangles
([#367](https://github.com/openbimrs/ifc/issues/367)): each Body as
Axiolid's neutral geometry graph in its wire format, for your own
kernel. It needs a native library built with the C ABI's `graph`
feature:

<!-- SNIPPET:cookbook-dotnet-graphs -->

```csharp
// A native library built with the C ABI's `graph` feature; the packaged
// one throws IfcException with Code "feature-disabled".
foreach (var product in model.ProductGeometry()) // or GeometryEncoding.Cbor
{
    if (product.Geometry.Refusal is { } || product.Payload.Length == 0) continue; // refused, or no Body
    // Axiolid's wire format, exact, in world metres: UTF-8 JSON for
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

## Find entities by GlobalId, types and openings

An entity by its `GlobalId`, an occurrence's type object and the type's
other occurrences, and a wall's openings with what fills them, read from
`test/fixtures/synthetic-relations/relations_ifc4.ifc`:

<!-- SNIPPET:cookbook-dotnet-global-ids -->

```csharp
string? Name(ulong id) => (model.AttributeByName(id, "Name") as Value.Text)?.Value;
// The first lookup indexes every GlobalId once; the rest reuse it.
ulong wall = model.ByGuid("0LwOtle8vEaxZPGHkxQdYT") ?? throw new KeyNotFoundException();

ulong wallType = model.TypeObject(wall)!.Value; // IfcRelDefinesByType
Print($"{Name(wallType)}: {string.Join(", ", model.OccurrencesOf(wallType))}"); // WT-200: 17, 18
foreach (var opening in model.OpeningsOf(wall)) // IfcRelVoidsElement
{
    Print($"{Name(opening)}: [{string.Join(", ", model.FillingsOf(opening).Select(Name))}]");
}
// Door opening: [Door 1]
// Recess: []

var duplicates = model.DuplicateGlobalIds(); // empty; a copied GlobalId is listed here
string fresh = IfcModel.NewGuid(); // random, version 4
string named = IfcModel.NameBasedGuid("my-exporter", "wall", "42"); // same every run
```

<!-- /SNIPPET -->

A `GlobalId` that several entities carry throws `IfcException` with
`Code` `invalid-model` naming every holder; `DuplicateGlobalIds()` lists
them all. `IfcModel.NewGuid()` draws an RFC 9562 version-4 UUID from the
operating system; `IfcModel.NameBasedGuid(parts)` is version 8, the same
for the same parts in every binding. Both are IFC's 22-character form.

## Select entities and read values by path

A [selector](/guide/selectors) picks entities by class, attribute,
property, type, material, classification, location and group; a value path
reads (and writes) one value of an entity. Read from
`test/fixtures/synthetic-relations/selector_ifc4.ifc`:

<!-- SNIPPET:cookbook-dotnet-selectors -->

```csharp
var walls = model.Select("IfcWall, Pset_WallCommon.FireRating=F90"); // 17, 18
Print(string.Join(", ", model.Select("IfcDoor + location=\"Level 1\""))); // 19, 28, 38, 40

Print(((Value.Text)model.GetValue(walls[0], "type.Name")).Value); // WT-200
// A property set is a list of [name, value] pairs.
var pset = (Value.List)model.GetValue(walls[0], "Pset_WallCommon");
Print(string.Join(", ", pset.Items.Select(pair => ((Value.Text)((Value.List)pair).Items[0]).Value)));
// IsExternal, FireRating, AcousticRating

model.SetValue(walls[0], "type.Name", "WT-300"); // one checked transaction
var error = Assert.Throws<IfcException>(() => model.Select("IfcWall, colour=red"));
Print($"{error.Code} {error.Span}"); // invalid-selector (9, 15)
```

<!-- /SNIPPET -->

`Select(query, baseIds)` restricts a selector to given ids. `GetValue`
returns a `Value`: several values as a `Value.List`, a property set as a
`Value.List` of two-item lists, each the member's name and its value.
`SetValue` takes a plain .NET value or a `Value`. A selector that does not
parse throws `IfcException` with `Code` `invalid-selector` and `Span`, the
byte range it refuses in the UTF-8 query.

## Remove a product and purge

A door with what it alone owns, a wall whose openings place their fillings,
then what nothing uses any more, from
`test/fixtures/synthetic-relations/cascade_ifc4.ifc`:

<!-- SNIPPET:cookbook-dotnet-remove-product -->

```csharp
// Door #100 with what it alone owns: placement, representation, its
// filling and type relationships. A dry run changes nothing.
RemovalReport report = model.RemoveProduct(100, dryRun: true);
Print($"{report.Removed.Count} removed"); // 11 removed
foreach (var kept in report.Kept.Where(kept => kept.Reason == "shared"))
{
    Print($"#{kept.Id} {kept.TypeName} used by {string.Join(", ", kept.Users)}");
}
// #82 IFCLOCALPLACEMENT used by 81       the opening's placement
// #112 IFCREPRESENTATIONMAP used by 111  the door type's geometry

model.RemoveProduct(100); // applied: one checked transaction
// A wall whose openings place their fillings: re-base those
// placements onto the storey so nothing moves, or it is refused.
model.RemoveProduct(20, rebasePlacements: true);

// Then purge what nothing uses any more: no classes means every
// resource entity no IfcRoot reaches.
RemovalReport purged = model.Purge(dryRun: true);
```

<!-- /SNIPPET -->

`rebasePlacements: true` re-bases placements relative to a removed one onto
its parent, composing the transforms; `untypeOccurrences: true` lets a type
object with occurrences go. Without them those removals throw
`still-referenced`. `Purge(new[] { "IfcPropertySet" }, includeRoots: true)`
purges unreferenced entities of the named classes only. The
[binding page](/bindings/dotnet#removing-products-and-purging) lists what
the cascade removes and keeps.

## Assign a type and propagate it

Door 1 typed again by its door type, with the type's representation map
mapped onto it, and Wall B moved into the building, from
`test/fixtures/synthetic-relations/cascade_ifc4.ifc`:

<!-- SNIPPET:cookbook-dotnet-assign-type -->

```csharp
// Door 1 (#100) typed again by D-90 (#111) with replace: what the old
// assignment mapped is taken back, the type's representation map is
// mapped afresh, and PredefinedType is left to the type.
model.Author(new[]
{
    AuthorOp.AssignType(111, new ulong[] { 100 }, mapRepresentations: true, predefinedType: PredefinedTypeMode.Sync, replace: true),
});
// A new door, typed the same way in the batch that creates it.
var ids = model.Author(new[]
{
    AuthorOp.Product("IfcDoor", new Dictionary<string, Value> { ["Name"] = new Value.Text("Door 2") }, container: 14),
    AuthorOp.AssignType(111, new[] { IfcModel.Handle(0) }, mapRepresentations: true),
});
// Wall B (#40) moves into the building (#13); without replace the
// second containment is refused with invalid-model.
model.Author(new[] { AuthorOp.Contain(13, new ulong[] { 40 }, replace: true) });
```

<!-- /SNIPPET -->

The type assignment takes `mapRepresentations`, `mappingTarget`, `materialUsage`, `layerSetDirection`, `predefinedType` and `replace`. Every option is off by default: the
operation then writes only the `IfcRelDefinesByType` and refuses an object
that already has a type (`invalid-model`). With `replace: true` the previous
type is unassigned and the representations and material usage it
propagated are removed with what only they use; `AuthorOp.Contain` and `AuthorOp.Aggregate` take the
same flag. `materialUsage` derives an `IfcMaterialLayerSetUsage` (walls
`AXIS2`, slabs and plates `AXIS3`, `POSITIVE`, offset 0) or an
`IfcMaterialProfileSetUsage` from the type's set; `PredefinedTypeMode.Validate` refuses an
object whose own `PredefinedType` contradicts the type's. The
[binding page](/bindings/dotnet#assigning-types) lists the rules.

## Assemble, assign, copy and remove representations

A body for a window from new items, a wall's body swapped for a copy of
another's, and the dry run of removing a door's body, on the same fixture:

<!-- SNIPPET:cookbook-dotnet-representation -->

```csharp
static ulong H(int index) => IfcModel.Handle(index); // the entity operation i of the batch produces
var ids = model.Author(new[]
{
    // The items first: a profile swept 1 m up, through the generic create.
    AuthorOp.Create("IfcCartesianPoint", new Dictionary<string, Value> { ["Coordinates"] = new Value.List(new Value.Real(0.6), new Value.Real(0.05)) }),
    AuthorOp.Create("IfcAxis2Placement2D", new Dictionary<string, Value> { ["Location"] = new Value.Ref(H(0)) }),
    AuthorOp.Create("IfcRectangleProfileDef", new Dictionary<string, Value>
    {
        ["ProfileType"] = new Value.Enum("AREA"),
        ["Position"] = new Value.Ref(H(1)),
        ["XDim"] = new Value.Real(1.2),
        ["YDim"] = new Value.Real(0.1),
    }),
    AuthorOp.Create("IfcExtrudedAreaSolid", new Dictionary<string, Value>
    {
        ["SweptArea"] = new Value.Ref(H(2)),
        ["Position"] = new Value.Ref(9),
        ["ExtrudedDirection"] = new Value.Ref(31),
        ["Depth"] = new Value.Real(1.0),
    }),
    // Context #11 is the 'Body' sub-context: the identifier is 'Body',
    // and the type is inferred from the item.
    AuthorOp.ShapeRepresentation(11, new[] { H(3) }), // 4
    AuthorOp.AssignRepresentation(H(4), 130), // 5: window #130's shape
    // Wall #20's body swapped for a copy of wall #40's.
    AuthorOp.CopyRepresentation(45), // 6
    AuthorOp.UnassignRepresentation(25, 20),
    AuthorOp.RemoveRepresentation(25),
    AuthorOp.AssignRepresentation(H(6), 20),
});
var type = model.AttributeByName(ids[4]!.Value, "RepresentationType"); // SweptSolid
// One alone, with a report of what went and what stayed:
RemovalReport report = model.RemoveRepresentation(105, dryRun: true); // the door's body
var removed = report.Removed.Select(entity => entity.Id); // 104, 105, 106, 107, 108
```

<!-- /SNIPPET -->

`AuthorOp.ShapeRepresentation` checks a given `representationType` against
the items by the release's `CorrectItemsForType` rule, and refuses a
mismatch with `invalid-value`. A representation belongs to one product shape, map or shape
aspect, so assigning one already used is refused (`invalid-model`); on a
type it becomes a new `IfcRepresentationMap`. A copy shares its context,
styles and the type geometry a mapped item maps, and copies everything
else. The [binding page](/bindings/dotnet#shape-representations) has the
rules.

## Copy a product and append from another model

Wall A copied with its openings, door and window, then a wall type taken
from another file, from `test/fixtures/synthetic-relations/cascade_ifc4.ifc`:

<!-- SNIPPET:cookbook-dotnet-copy -->

```csharp
// Wall A (#20) with what it alone owns -- placement, body, its own
// property set, material usage, openings with their door and window
// -- two metres along. Its type, shared property set,
// classification, storey and layer are shared: the copy joins them.
CopyReport report = model.CopyProduct(20, offset: (0, 2, 0));
Print($"#{report.Copy} copies {report.Copied.Count}, joins {string.Join(", ", report.Joined)}");
// #190 copies 55, joins 19, 62, 67, 79, 80, 120, 136

// From another model: the wall type with its property set. Equal
// units, contexts, owner history, materials and styles are reused;
// this model holds the type's GlobalIds already, so new ones.
CopyReport appended = model.Append(library, new ulong[] { 66 }, GlobalIdMode.Regenerate);
Print(string.Join(", ", appended.Copied.Select(entity => entity.TypeName)));
// IFCPROPERTYSINGLEVALUE, IFCPROPERTYSET, IFCWALLTYPE
```

<!-- /SNIPPET -->

`CopyProduct` copies what `RemoveProduct` would remove with the product
and shares what it would keep; `openings: false`, `fillings: false`,
`container: false` and `aggregate: false` leave those out, and
`placement:` puts the copy on a placement of your own. `Append` keeps
`GlobalId`s with `GlobalIdMode.Keep` and refuses one the model holds;
`GlobalIdMode.Merge` treats a `GlobalId` the model holds (on the same type)
as the same object. It refuses models whose length or angle units differ
(`unsupported`), and entities the target release cannot hold
(`unsupported-schema`). The
[binding page](/bindings/dotnet#copying-products-and-appending) lists what is copied, shared and skipped.

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
