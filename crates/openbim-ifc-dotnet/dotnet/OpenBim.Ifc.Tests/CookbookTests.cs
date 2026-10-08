// The .NET cookbook (docs/cookbook/dotnet.md, #331): every recipe on that
// page is a `docs:snippet` region below, run against the packed OpenBim.Ifc
// package by scripts/check-dotnet.py and checked against the file it reads.
// The suite is copied to a fresh project before it runs, so the files are
// inline: a trimmed copy of test/fixtures/synthetic-properties and one
// placed, extruded wall.

using System;
using System.Collections.Generic;
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
        + "#5=IFCOWNERHISTORY(#3,#4,$,.ADDED.,$,$,$,0);\n"
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
