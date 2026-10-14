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

    /// <summary>The wall with an <c>Axis</c> beside its Body: an <c>IfcPolyline</c> from (-2000, 0) to (2000, 0) mm (#458).</summary>
    private static readonly string AxisWall = Wall
        .Replace("#16=IFCPRODUCTDEFINITIONSHAPE($,$,(#15));", "#16=IFCPRODUCTDEFINITIONSHAPE($,$,(#18,#15));")
        .Replace(
            "ENDSEC;\nEND-ISO-10303-21;\n",
            "#18=IFCSHAPEREPRESENTATION(#5,'Axis','Curve2D',(#19));\n"
            + "#19=IFCPOLYLINE((#20,#21));\n"
            + "#20=IFCCARTESIANPOINT((-2000.,0.));\n"
            + "#21=IFCCARTESIANPOINT((2000.,0.));\n"
            + "ENDSEC;\nEND-ISO-10303-21;\n");

    [Fact]
    public void CurvesAreOptInAndOtherwisePolylines()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(AxisWall));
        try
        {
            var printed = new System.Collections.Generic.List<string>();
            // docs:snippet dotnet-geometry-curves
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
            // docs:end
            Assert.Equal(new[] { "#17: 2 vertices, bound 0" }, printed);
            var wall = Assert.Single(model.ProductCurves(null, "Axis"));
            Assert.Null(wall.Curves.Refusal);
            var axis = Assert.Single(wall.Curves.Polylines);
            Assert.Equal(new ProductPolyline(19, 0, 2, false, 0.0, 0.001), axis);
            Assert.Equal(wall.Curves.VertexCount * 3, (long)wall.Positions.Length);
            Assert.Equal(-2.0, (double)wall.Positions[0], 5);
            Assert.Equal(2.0, (double)wall.Positions[3], 5);
            // Plan, the default, picks the same Axis; the Body has no curves.
            Assert.Equal(wall.Curves, Assert.Single(model.ProductCurves()).Curves);
            Assert.Empty(Assert.Single(model.ProductCurves(null, "Body")).Curves.Polylines);
            // The Body mesh is a solid; an Axis has nothing to mesh.
            Assert.Equal("solid", Assert.Single(model.ProductMeshes()).Mesh.Closure);
            Assert.Empty(Assert.Single(model.ProductMeshes(null, "Axis")).Indices);
            var error = Assert.Throws<IfcException>(() => model.ProductCurves(null, "Centreline"));
            Assert.Equal("invalid-value", error.Code);
        }
        catch (IfcException error) when (error.Code == "feature-disabled")
        {
            // The packaged library leaves meshes and curves out.
            Assert.Equal(IfcStatus.FeatureDisabled, error.Status);
        }
    }

    /// <summary>The wall, made of a material whose styled representation
    /// paints it red, a quarter transparent (#449).</summary>
    private static readonly string RedWall = Wall.Replace(
        "ENDSEC;\nEND-ISO-10303-21;\n",
        "#18=IFCMATERIAL('Red',$,$);\n"
        + "#19=IFCCOLOURRGB($,0.8,0.1,0.1);\n"
        + "#20=IFCSURFACESTYLERENDERING(#19,0.25,$,$,$,$,$,$,.NOTDEFINED.);\n"
        + "#21=IFCSURFACESTYLE('Red',.BOTH.,(#20));\n"
        + "#22=IFCSTYLEDITEM($,(#21),$);\n"
        + "#23=IFCSTYLEDREPRESENTATION(#5,'Style','Material',(#22));\n"
        + "#24=IFCMATERIALDEFINITIONREPRESENTATION($,$,(#23),#18);\n"
        + "#25=IFCRELASSOCIATESMATERIAL('2YvctVUKr0kugbFTf53O9L',$,$,$,(#17),#18);\n"
        + "ENDSEC;\nEND-ISO-10303-21;\n");

    [Fact]
    public void MeshesCarryTheirMaterialColour()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(RedWall));
        try
        {
            var mesh = Assert.Single(model.ProductMeshes()).Mesh;
            Assert.Null(mesh.ColourRefusal);
            var colour = Assert.IsType<SurfaceColour>(mesh.Colour);
            Assert.Equal((0.8, 0.1, 0.1), (colour.Red, colour.Green, colour.Blue));
            Assert.Equal(0.25, colour.Transparency);
            Assert.Equal("material", colour.Source);
            Assert.Equal(18UL, colour.SourceEntity);
            Assert.Equal(21UL, colour.SurfaceStyle);
        }
        catch (IfcException error)
        {
            // The packaged library leaves meshes out.
            Assert.Equal("feature-disabled", error.Code);
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
                // {"format":"axiolid-geometry-graph","version":"1.0","graph":{...}},
                // exact, in world metres: hand it to your own kernel's reader.
                System.Console.WriteLine($"{product.Geometry.TypeName}: {product.Json}");
            }
            // docs:end

            var wall = Assert.Single(model.ProductGeometry());
            Assert.Null(wall.Geometry.Refusal);
            Assert.Equal("json", wall.Geometry.Encoding);
            Assert.Equal(wall.Geometry.PayloadSize, (long)wall.Payload.Length);
            // Axiolid's writer orders the envelope: format, version, graph. The version is the
            // lowest the content needs: no station carries a seam window, so 1.0
            // (axiolid/kernel#297).
            Assert.StartsWith(
                "{\"format\":\"" + ProductGraph.Format + "\",\"version\":\"1.0\",\"graph\":{\"nodes\":[",
                wall.Json!);
            Assert.Equal("1.3", ProductGraph.FormatVersion);
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
