// Geometry (#328, #367): placements in every build, graphs and meshes when
// the native library carries the `graph` and `mesh` features, as smoke.c's
// `geometry()` checks them.

using System.Linq;
using Xunit;

namespace OpenBim.Ifc.Tests;

public class GeometryTests
{
    /// <summary>One wall in a millimetre file, 1 m east and 2 m north, its
    /// Body a 4 m x 0.2 m x 3 m extrusion: smoke.c's GEOMETRY_TEXT.</summary>
    private const string Wall =
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

    [Fact]
    public void PlacementsAndTheSelectedBody()
    {
        var data = Fixtures.Bytes(Wall);
        // docs:snippet dotnet-geometry
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
        // docs:end

        var wall = Assert.Single(model.ProductPlacements());
        Assert.Equal(17UL, wall.Id);
        Assert.Equal("IFCWALL", wall.TypeName);
        Assert.Equal("1YvctVUKr0kugbFTf53O9L", wall.GlobalId);
        Assert.Equal(16, wall.Transform!.Count);
        Assert.Equal(1.0, wall.Transform[12], 9);
        Assert.Equal(2.0, wall.Transform[13], 9);
        Assert.Equal(new SelectedRepresentation(15, "Body", "SweptSolid", 5, "Model", null, null), wall.Representation);
        Assert.Null(wall.Refusal);
    }

    [Fact]
    public void AMissingProductIsATypedRefusal()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Wall));
        var missing = Assert.Single(model.ProductPlacements(new ulong[] { 99 }));
        Assert.Null(missing.Transform);
        Assert.Equal("missing-reference", missing.Refusal!.Code);
        Assert.Empty(model.ProductPlacements(System.Array.Empty<ulong>()));
    }

    [Fact]
    public void MeshesAreOptInAndOtherwiseArrays()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Wall));
        try
        {
            var mesh = Assert.Single(model.ProductMeshes());
            Assert.Null(mesh.Mesh.Refusal);
            Assert.Equal(mesh.Mesh.VertexCount * 3, (long)mesh.Positions.Length);
            Assert.Equal(mesh.Mesh.TriangleCount * 3, (long)mesh.Indices.Length);
            Assert.True(mesh.Indices.Length >= 36);
            Assert.Equal(2.0, (double)mesh.Positions.Where((_, i) => i % 3 == 0).Max(), 4);
        }
        catch (IfcException error)
        {
            // The packaged library leaves meshes out.
            Assert.Equal("feature-disabled", error.Code);
            Assert.Equal(IfcStatus.FeatureDisabled, error.Status);
        }
    }

    [Fact]
    public void GraphsAreOptInAndOtherwiseAxiolidsWireFormat()
    {
        var data = Fixtures.Bytes(Wall);
        try
        {
            // docs:snippet dotnet-geometry-graph
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
            // docs:end

            var wall = Assert.Single(model.ProductGeometry());
            Assert.Null(wall.Geometry.Refusal);
            Assert.Equal("json", wall.Geometry.Encoding);
            Assert.Equal(wall.Geometry.PayloadSize, (long)wall.Payload.Length);
            // Axiolid's writer orders the envelope: format, version, graph. The version is the
            // lowest the content needs, 1.0 or 1.1 (axiolid-model 0.3.9 labels every payload 1.1).
            var heads = new[] { "1.0", "1.1" }.Select(version =>
                "{\"format\":\"" + ProductGraph.Format + "\",\"version\":\"" + version + "\",\"graph\":{\"nodes\":[");
            Assert.Contains(heads, head => wall.Json!.StartsWith(head, System.StringComparison.Ordinal));
            Assert.Equal("1.1", ProductGraph.FormatVersion);
            Assert.Equal("axiolid-geometry-graph", ProductGraph.Format);
            // Three nodes (profile, extrusion, placed instance), the last the root.
            Assert.EndsWith("],\"roots\":[2]}}", wall.Json!);
            Assert.Equal(model.ProductPlacements()[0].Transform, wall.Geometry.Transform);

            var cbor = Assert.Single(model.ProductGeometry(null, GeometryEncoding.Cbor));
            Assert.Equal("cbor", cbor.Geometry.Encoding);
            Assert.Null(cbor.Json);
            // A CBOR map of three entries, the first the text "format".
            Assert.Equal(0xa3, cbor.Payload[0]);
            Assert.Equal("format", System.Text.Encoding.UTF8.GetString(cbor.Payload, 2, 6));
            Assert.True(cbor.Payload.Length < wall.Payload.Length);

            var missing = Assert.Single(model.ProductGeometry(new ulong[] { 99 }));
            Assert.Equal("missing-reference", missing.Geometry.Refusal!.Code);
            Assert.Empty(missing.Payload);
        }
        catch (IfcException error)
        {
            // The packaged library leaves graphs out.
            Assert.Equal("feature-disabled", error.Code);
            Assert.Equal(IfcStatus.FeatureDisabled, error.Status);
        }
    }
}
