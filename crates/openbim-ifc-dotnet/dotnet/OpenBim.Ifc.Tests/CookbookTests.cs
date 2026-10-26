// The .NET cookbook (docs/cookbook/dotnet.md, #331): every recipe on that
// page is a `docs:snippet` region below, run against the packed OpenBim.Ifc
// package by scripts/check-dotnet.py and checked against the file it reads.
// The suite is copied to a fresh project before it runs, so the files are
// inline: a trimmed copy of test/fixtures/synthetic-properties and one
// placed, extruded wall.

using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using Xunit;

namespace OpenBim.Ifc.Tests;

public class CookbookTests
{
    /// <summary>Two walls on one storey; Wall A states IsExternal and a width, both inherit FireRating from their type.</summary>
    private const string Properties =
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');\n"
        + "FILE_NAME('properties.ifc','2026-10-08T00:00:00',(''),(''),'','cookbook','');\n"
        + "FILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n"
        + "#1=IFCPERSON($,'Generator',$,$,$,$,$,$);\n"
        + "#2=IFCORGANIZATION($,'openbim.rs',$,$,$);\n"
        + "#3=IFCPERSONANDORGANIZATION(#1,#2,$);\n"
        + "#4=IFCAPPLICATION(#2,'1','cookbook','cb');\n"
        + "#5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,0);\n"
        + "#6=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n"
        + "#7=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);\n"
        + "#18=IFCUNITASSIGNMENT((#6));\n"
        + "#19=IFCCARTESIANPOINT((0.,0.,0.));\n"
        + "#20=IFCAXIS2PLACEMENT3D(#19,$,$);\n"
        + "#21=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#20,$);\n"
        + "#22=IFCPROJECT('0WtIqeFgfBFQdcY1sx7FNt',#5,'Properties',$,$,$,$,(#21),#18);\n"
        + "#23=IFCSITE('0HFj6_unzBE92O5Oy4DtBD',#5,'Site',$,$,$,$,$,$,$,$,$,$,$);\n"
        + "#24=IFCBUILDING('2lAse9OiLA8fN8_11KuxQ_',#5,'Building',$,$,$,$,$,$,$,$,$);\n"
        + "#25=IFCBUILDINGSTOREY('1B3Fknknr1IB15rEwzoEkq',#5,'Level 0',$,$,$,$,$,$,$);\n"
        + "#26=IFCRELAGGREGATES('3SWTIOzQn9Wuyh2$ztj4fM',#5,$,$,#22,(#23));\n"
        + "#27=IFCRELAGGREGATES('3VFem8W1v7Deoe8oRjxlMW',#5,$,$,#23,(#24));\n"
        + "#28=IFCRELAGGREGATES('0YDPjXuOb1x9oIKcZlnv4w',#5,$,$,#24,(#25));\n"
        + "#29=IFCWALLTYPE('0dGA15OQf9_P5UdNj5qcto',#5,'WT-200',$,$,(#36),$,$,$,.SOLIDWALL.);\n"
        + "#30=IFCWALL('31t64Uzj98DhdKplMxBpF5',#5,'Wall A',$,$,$,$,$,$);\n"
        + "#31=IFCWALL('2CazjTQP11iu6o3c47EEd8',#5,'Wall B',$,$,$,$,$,$);\n"
        + "#32=IFCRELCONTAINEDINSPATIALSTRUCTURE('1jbZpUGIXFjB7a6sZlrkkE',#5,$,$,(#30,#31),#25);\n"
        + "#33=IFCRELDEFINESBYTYPE('0xtMgJ4Tb6LQGWVe9NZXe6',#5,$,$,(#30,#31),#29);\n"
        + "#34=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$);\n"
        + "#35=IFCPROPERTYSINGLEVALUE('FireRating',$,IFCLABEL('F30'),$);\n"
        + "#36=IFCPROPERTYSET('0xkBHWpUvCCfDN3f1g8yKE',#5,'Pset_WallCommon',$,(#34,#35));\n"
        + "#37=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.F.),$);\n"
        + "#38=IFCPROPERTYSET('0Mz9fPfwfBYBYTYsJjiw_L',#5,'Pset_WallCommon',$,(#37));\n"
        + "#39=IFCRELDEFINESBYPROPERTIES('2DQfRF4Sn31h1Cc0o8Xge_',#5,$,$,(#30),#38);\n"
        + "#53=IFCQUANTITYLENGTH('Width',$,#7,200.,$);\n"
        + "#62=IFCELEMENTQUANTITY('2$RZu8sU1C0gtMu9KP7Dc0',#5,'Qto_WallBaseQuantities',$,'BaseQuantities',(#53));\n"
        + "#63=IFCRELDEFINESBYPROPERTIES('1CW18VpAX48Oa2oV3nDZTH',#5,$,$,(#30),#62);\n"
        + "ENDSEC;\nEND-ISO-10303-21;\n";

    /// <summary>One wall in a millimetre file, 1 m east and 2 m north, its Body a 4 m x 0.2 m x 3 m extrusion.</summary>
    private const string Geometry =
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n"
        + "FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n"
        + "#1=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);\n"
        + "#2=IFCUNITASSIGNMENT((#1));\n"
        + "#3=IFCCARTESIANPOINT((0.,0.,0.));\n"
        + "#4=IFCAXIS2PLACEMENT3D(#3,$,$);\n"
        + "#5=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#4,$);\n"
        + "#6=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,(#5),#2);\n"
        + "#7=IFCCARTESIANPOINT((1000.,2000.,0.));\n"
        + "#8=IFCAXIS2PLACEMENT3D(#7,$,$);\n"
        + "#9=IFCLOCALPLACEMENT($,#8);\n"
        + "#10=IFCAXIS2PLACEMENT2D(#11,$);\n"
        + "#11=IFCCARTESIANPOINT((0.,0.));\n"
        + "#12=IFCRECTANGLEPROFILEDEF(.AREA.,$,#10,4000.,200.);\n"
        + "#13=IFCDIRECTION((0.,0.,1.));\n"
        + "#14=IFCEXTRUDEDAREASOLID(#12,#4,#13,3000.);\n"
        + "#15=IFCSHAPEREPRESENTATION(#5,'Body','SweptSolid',(#14));\n"
        + "#16=IFCPRODUCTDEFINITIONSHAPE($,$,(#15));\n"
        + "#17=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,#9,#16,$,.STANDARD.);\n"
        + "ENDSEC;\nEND-ISO-10303-21;\n";

    /// <summary>Run <paramref name="recipe"/> and return what it printed.</summary>
    private static string Printed(Action recipe)
    {
        var previous = Console.Out;
        using var captured = new StringWriter { NewLine = "\n" };
        Console.SetOut(captured);
        try
        {
            recipe();
        }
        finally
        {
            Console.SetOut(previous);
        }
        return captured.ToString();
    }

    /// <summary>A scratch file holding <paramref name="text"/>, deleted with the returned handle.</summary>
    private sealed class TempFile : IDisposable
    {
        public TempFile(string text)
        {
            Path = System.IO.Path.Combine(System.IO.Path.GetTempPath(), Guid.NewGuid().ToString("N") + ".ifc");
            File.WriteAllBytes(Path, Fixtures.Bytes(text));
        }

        public string Path { get; }

        public void Dispose() => File.Delete(Path);
    }

    [Fact]
    public void Open()
    {
        using var file = new TempFile(Properties);
        var path = file.Path;
        var printed = Printed(() =>
        {
            // docs:snippet cookbook-dotnet-open
            using var model = IfcModel.Open(path); // read once, straight from disk
            Console.WriteLine($"{model.Schema}, {model.Count} entities"); // IFC4, 32 entities
            Console.WriteLine($"{model.Header.OriginatingSystem}, {model.Header.TimeStamp}");

            using var same = IfcModel.Parse(File.ReadAllBytes(path)); // or from bytes you hold
            // docs:end
            Assert.Equal(model.Count, same.Count);
        });
        Assert.StartsWith("IFC4, 32 entities\ncookbook, 2026-10-08T00:00:00\n", printed);
    }

    [Fact]
    public void PropertySetsOfOneObject()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Properties));
        var printed = Printed(() =>
        {
            // docs:snippet cookbook-dotnet-property-sets
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
            // docs:end
        });
        var lines = printed.Split('\n');
        Assert.Equal("Pset_WallCommon.IsExternal = IFCBOOLEAN(.F.) (occurrence)", lines[0]);
        Assert.StartsWith("Qto_WallBaseQuantities.Width = ", lines[1]);
        Assert.Contains("Pset_WallCommon.FireRating = IFCLABEL('F30') (type)", lines);
    }

    [Fact]
    public void PropertySetsOfManyObjects()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Properties));
        // docs:snippet cookbook-dotnet-property-sets-many
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
        // docs:end
        Assert.Equal(new ulong[] { 30, 31 }, rows.Keys.OrderBy(id => id));
        Assert.Equal(new Value.Typed("IFCBOOLEAN", new Value.Bool(false)), rows[30]["Pset_WallCommon.IsExternal"]);
        Assert.Equal(new Value.Typed("IFCBOOLEAN", new Value.Bool(true)), rows[31]["Pset_WallCommon.IsExternal"]);
        Assert.Equal(new Value.Typed("IFCLABEL", new Value.Text("F30")), rows[31]["Pset_WallCommon.FireRating"]);
    }

    [Fact]
    public void StoreysAndTheirElements()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Properties));
        var printed = Printed(() =>
        {
            // docs:snippet cookbook-dotnet-storeys
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
            // docs:end
        });
        Assert.Equal("Level 0 (#25)\n  IFCWALL #30 'Wall A'\n  IFCWALL #31 'Wall B'\n", printed);
    }

    [Fact]
    public void Validate()
    {
        var broken = Properties.Replace("'Wall A',$,$,$,$,$,$)", "'Wall A',$,$,$,$,$,.NOTANENUM.)");
        using var model = IfcModel.Parse(Fixtures.Bytes(broken));
        var printed = Printed(() =>
        {
            // docs:snippet cookbook-dotnet-validate
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
            // docs:end
            Assert.False(report.Conformant);
        });
        Assert.Contains(printed.Split('\n'), line => line.StartsWith("error ") && line.Contains(" #30 "));
        using var clean = IfcModel.Parse(Fixtures.Bytes(Properties));
        Assert.Equal(0, clean.Validate().Errors);
    }

    [Fact]
    public void IfcXml()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Geometry));
        // docs:snippet cookbook-dotnet-ifcxml
        byte[] xml = model.WriteIfcXml(); // this library's lossless layout, UTF-8
        using var back = IfcModel.ParseIfcXml(xml);
        byte[] step = back.Write(); // the same entities, as STEP again

        // The buildingSMART XSD layout of a release, for tools that read it.
        byte[] xsd = model.WriteIfcXml(xsdProfile: "IFC4");
        using var fromXsd = IfcModel.ParseIfcXml(xsd, xsdProfile: "IFC4");
        // docs:end
        Assert.Equal(model.Ids(), back.Ids());
        using var again = IfcModel.Parse(step);
        Assert.Equal(model.Count, again.Count);
        Assert.Single(fromXsd.IdsOfType("IfcWall"));
    }

    [Fact]
    public void Edit()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Properties));
        var path = Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N") + ".ifc");
        try
        {
            // docs:snippet cookbook-dotnet-edit
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
            // docs:end
            using var again = IfcModel.Open(path);
            Assert.Equal(new Value.Text("Wall B (checked)"), again.AttributeByName(31, "Name"));
            Assert.Equal(new Value.Enum("SOLIDWALL"), again.AttributeByName(31, "PredefinedType"));
            var own = again.PropertySets(31).Where(set => set.Source == "occurrence").ToList();
            Assert.Equal(
                new Value.Typed("IFCLABEL", new Value.Text("F90")),
                own.Single(set => set.Name == "Pset_WallCommon").Properties.Single(p => p.Name == "FireRating").Value);
            Assert.Equal(
                new Value.Typed("IFCLABEL", new Value.Text("QA")),
                own.Single(set => set.Name == "Checks").Properties.Single().Value);
            var typeSet = again.PropertySets(29).Single(set => set.Name == "Pset_WallCommon");
            Assert.Equal(
                new Value.Typed("IFCLABEL", new Value.Text("F30")),
                typeSet.Properties.Single(p => p.Name == "FireRating").Value);
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Fact]
    public void PlacementsAndMeshes()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Geometry));
        var printed = Printed(() =>
        {
            // docs:snippet cookbook-dotnet-placements
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
            // docs:end
        });
        Assert.Equal("IFCWALL at 1 2 0, Body SweptSolid\n", printed);

        try
        {
            // docs:snippet cookbook-dotnet-meshes
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
            // docs:end
        }
        catch (IfcException error) when (error.Code == "feature-disabled")
        {
            Assert.Equal(IfcStatus.FeatureDisabled, error.Status);
        }
    }

    [Fact]
    public void ExactGeometryGraphs()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Geometry));
        try
        {
            // docs:snippet cookbook-dotnet-graphs
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
            // docs:end
        }
        catch (IfcException error) when (error.Code == "feature-disabled")
        {
            Assert.Equal(IfcStatus.FeatureDisabled, error.Status);
        }
    }

    [Fact]
    public void SelectAndValuesByPath()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.SelectorIfc4));
        var printed = new List<string>();
        void Print(string line) => printed.Add(line);
        // docs:snippet cookbook-dotnet-selectors
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
        // docs:end
        Assert.Equal(new[] { 17UL, 18UL }, walls);
        Assert.Equal(new[] { "19, 28, 38, 40", "WT-200", "IsExternal, FireRating, AcousticRating", "invalid-selector (9, 15)" }, printed);
        Assert.Equal(new Value.Text("WT-300"), model.GetValue(18, "type.Name"));
    }

    [Fact]
    public void ExportATableAndImportItBack()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.SelectorIfc4));
        var printed = new List<string>();
        void Print(string line) => printed.Add(line);
        // docs:snippet cookbook-dotnet-tables
        byte[] csv = model.ExportTable("IfcWall", new[] { "Name", "type.Name" }, key: "id"); // or "xlsx"
        string text = System.Text.Encoding.UTF8.GetString(csv);
        Print(text.Split(new[] { "\r\n" }, StringSplitOptions.None)[1]); // 17,Wall A,WT-200

        // Edit the table, then plan the import: a dry run lists every change.
        var edited = System.Text.Encoding.UTF8.GetBytes(text.Replace("17,Wall A,", "17,Wall A1,"));
        foreach (var change in model.ImportTable(edited, dryRun: true).Changes)
        {
            Print($"row {change.Row} {change.Path}: {change.Before} -> {change.After}"); // row 2 Name: Wall A -> Wall A1
        }
        Print(model.ImportTable(edited).Applied.ToString()); // True: one checked transaction

        // A refused cell refuses the whole table, by row and column.
        var refused = model.ImportTable(System.Text.Encoding.UTF8.GetBytes("id,Name\n99,Ghost\n"));
        Print(string.Join(", ", refused.Refusals.Select(r => $"{r.Row}:{r.Column} {r.Code}"))); // 2:1 unknown-key
        // docs:end
        Assert.Equal(new[] { "17,Wall A,WT-200", "row 2 Name: Wall A -> Wall A1", "True", "2:1 unknown-key" }, printed);
        Assert.Equal(new Value.Text("Wall A1"), model.GetValue(17, "Name"));
    }

    [Fact]
    public void GlobalIdsTypesAndOpenings()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.RelationsIfc4));
        var printed = new List<string>();
        void Print(string line) => printed.Add(line);
        // docs:snippet cookbook-dotnet-global-ids
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
        // docs:end
        Assert.Equal(new[] { "WT-200: 17, 18", "Door opening: [Door 1]", "Recess: []" }, printed);
        Assert.Empty(duplicates);
        Assert.Equal(22, fresh.Length);
        Assert.Equal(named, IfcModel.NameBasedGuid("my-exporter", "wall", "42"));
    }

    [Fact]
    public void RemoveAProductAndPurge()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.CascadeIfc4));
        var printed = new List<string>();
        void Print(string line) => printed.Add(line);
        // docs:snippet cookbook-dotnet-remove-product
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
        // docs:end
        Assert.Equal(new[] { "11 removed", "#82 IFCLOCALPLACEMENT used by 81", "#112 IFCREPRESENTATIONMAP used by 111" }, printed);
        Assert.NotEmpty(purged.Removed);
        Assert.Empty(model.DanglingReferences());
    }

    [Fact]
    public void CopyAProductAndAppendFromAnotherModel()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.CascadeIfc4));
        using var library = IfcModel.Parse(Fixtures.Bytes(Fixtures.CascadeIfc4));
        var printed = new List<string>();
        void Print(string line) => printed.Add(line);
        // docs:snippet cookbook-dotnet-copy
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
        // docs:end
        Assert.Equal(new[]
        {
            "#190 copies 55, joins 19, 62, 67, 79, 80, 120, 136",
            "IFCPROPERTYSINGLEVALUE, IFCPROPERTYSET, IFCWALLTYPE",
        }, printed);
        Assert.Equal(new[] { 67UL }, appended.Skipped);
        Assert.Empty(model.DanglingReferences());
    }

    [Fact]
    public void ConvertUnitsAndTheLengthUnit()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.CascadeIfc4));
        var printed = new List<string>();
        void Print(string line) => printed.Add(line);
        // docs:snippet cookbook-dotnet-length-unit
        // One value between named units, and a length in feet and inches.
        double feet = Units.Convert(3000.0, "MILLIMETRE", "foot");
        string text = Units.FormatLength(3000.0, "MILLIMETRE", LengthStyle.FeetInches, 16, suppressZeroInches: true);
        Print($"3000 mm = {feet.ToString("F4", CultureInfo.InvariantCulture)} ft = {text}"); // 3000 mm = 9.8425 ft = 9' - 10 1/8"

        // The whole model in millimetres: every length, and areas and volumes
        // with it, as one checked transaction. dryRun: true only reports.
        LengthUnitReport report = model.ConvertLengthUnit("MILLIMETRE");
        Print($"{report.From} to {report.To}: lengths x{report.LengthFactor}"); // METRE to MILLI METRE: lengths x1000
        foreach (var kept in report.Kept)
        {
            Print($"{kept.EntityType}.{kept.Attribute} kept: {kept.Reason}");
        }
        // docs:end
        Assert.Equal("3000 mm = 9.8425 ft = 9' - 10 1/8\"", printed[0]);
        Assert.Equal("METRE to MILLI METRE: lengths x1000", printed[1]);
    }

    [Fact]
    public void AssignATypeWithPropagation()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.CascadeIfc4));
        // docs:snippet cookbook-dotnet-assign-type
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
        // docs:end
        var shape = ((Value.Ref)model.AttributeByName(ids[0]!.Value, "Representation")).Id;
        var body = ((Value.Ref)((Value.List)model.AttributeByName(shape, "Representations")).Items[0]).Id;
        Assert.Equal(new Value.Text("MappedRepresentation"), model.AttributeByName(body, "RepresentationType"));
        Assert.Empty(model.DanglingReferences());
        Assert.Equal(0L, model.Validate().Errors);
    }

    [Fact]
    public void AssembleAssignCopyAndRemoveRepresentations()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.CascadeIfc4));
        // docs:snippet cookbook-dotnet-representation
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
        // docs:end
        Assert.Equal(new Value.Text("SweptSolid"), type);
        Assert.Equal(new[] { 104UL, 105UL, 106UL, 107UL, 108UL }, removed);
        Assert.Empty(model.DanglingReferences());
    }

    [Fact]
    public void CreateAModelFromNothing()
    {
        // docs:snippet cookbook-dotnet-create
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
        // docs:end
        using var again = IfcModel.Parse(data);
        Assert.Equal("IFCWALL", again.TypeOf(wall));
        var storey = again.SpatialTree().Nodes.Single(node => node.Kind == "storey");
        Assert.Equal("Level 0", storey.Name);
        Assert.Equal(new[] { wall }, storey.Elements);
        Assert.Equal(0, again.Validate().Errors);
    }
}
