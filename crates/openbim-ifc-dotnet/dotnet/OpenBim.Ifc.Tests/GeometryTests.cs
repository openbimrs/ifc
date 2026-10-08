// Geometry (#328): placements in every build, meshes when the native
// library carries the `mesh` feature, as smoke.c's `geometry()` checks them.

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
}
