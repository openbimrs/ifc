// Shape representations (#481): assemble, assign, copy, unassign and remove
// over the IFC2X3, IFC4 and IFC4X3 cascade fixtures. The operations are
// pinned in openbim-ifc; this checks how they cross into .NET.

using System.Collections.Generic;
using System.Linq;
using Xunit;

namespace OpenBim.Ifc.Tests;

public class RepresentationTests
{
    public static IEnumerable<object[]> Releases() => new[]
    {
        new object[] { Fixtures.CascadeIfc2x3 },
        new object[] { Fixtures.CascadeIfc4 },
        new object[] { Fixtures.CascadeIfc4x3 },
    };

    private static ulong H(int index) => IfcModel.Handle(index);

    private static Dictionary<string, Value> Attributes(params (string Name, Value Value)[] pairs) =>
        pairs.ToDictionary(pair => pair.Name, pair => pair.Value);

    /// <summary>Ops 0..3: a 1.2 x 0.1 x 1.0 extrusion (op 3).</summary>
    private static List<AuthorOp> Extrusion() => new()
    {
        AuthorOp.Create("IfcCartesianPoint", Attributes(("Coordinates", new Value.List(new Value.Real(0.6), new Value.Real(0.05))))),
        AuthorOp.Create("IfcAxis2Placement2D", Attributes(("Location", new Value.Ref(H(0))))),
        AuthorOp.Create("IfcRectangleProfileDef", Attributes(
            ("ProfileType", new Value.Enum("AREA")),
            ("Position", new Value.Ref(H(1))),
            ("XDim", new Value.Real(1.2)),
            ("YDim", new Value.Real(0.1)))),
        AuthorOp.Create("IfcExtrudedAreaSolid", Attributes(
            ("SweptArea", new Value.Ref(H(2))),
            ("Position", new Value.Ref(9)),
            ("ExtrudedDirection", new Value.Ref(31)),
            ("Depth", new Value.Real(1.0)))),
    };

    [Theory]
    [MemberData(nameof(Releases))]
    public void AssembleAssignCopyUnassignAndRemove(string file)
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(file));
        var ops = Extrusion();
        ops.Add(AuthorOp.ShapeRepresentation(11, new[] { H(3) }));
        ops.Add(AuthorOp.AssignRepresentation(H(4), 130));
        var ids = model.Author(ops);
        Assert.Equal(new Value.Text("SweptSolid"), model.AttributeByName(ids[4]!.Value, "RepresentationType"));
        Assert.Equal(new Value.Ref(ids[5]!.Value), model.AttributeByName(130, "Representation"));

        var swap = model.Author(new[]
        {
            AuthorOp.CopyRepresentation(25),
            AuthorOp.UnassignRepresentation(25, 20),
            AuthorOp.RemoveRepresentation(25),
            AuthorOp.AssignRepresentation(H(0), 20),
        });
        Assert.Null(swap[2]);
        Assert.Equal("missing-entity", Assert.Throws<IfcException>(() => model.TypeOf(25)).Code);
        var shape = (Value.Ref)model.AttributeByName(20, "Representation");
        Assert.Equal(new Value.List(new Value.Ref(swap[0]!.Value)), model.AttributeByName(shape.Id, "Representations"));

        var before = model.Count;
        var dry = model.RemoveRepresentation(105, dryRun: true);
        Assert.False(dry.Applied);
        Assert.Equal(new[] { 104UL, 105UL, 106UL, 107UL, 108UL }, dry.Removed.Select(e => e.Id));
        Assert.Equal(before, model.Count);
        Assert.True(model.RemoveRepresentation(105).Applied);
        Assert.Equal(before - 5, model.Count);
        Assert.Empty(model.DanglingReferences());
    }

    [Fact]
    public void RefusalsHaveTheirCodes()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.CascadeIfc4));
        var ops = Extrusion();
        ops.Add(AuthorOp.ShapeRepresentation(11, new[] { H(3) }, representationType: "Brep"));
        Assert.Equal("invalid-value", Assert.Throws<IfcException>(() => model.Author(ops)).Code);
        Assert.Equal("invalid-model", Assert.Throws<IfcException>(() => model.Author(new[] { AuthorOp.AssignRepresentation(25, 130) })).Code);
        Assert.Equal("invalid-model", Assert.Throws<IfcException>(() => model.Author(new[] { AuthorOp.UnassignRepresentation(45, 20) })).Code);
        Assert.Equal("still-referenced", Assert.Throws<IfcException>(() => model.RemoveRepresentation(114)).Code);
        Assert.Equal("wrong-entity-type", Assert.Throws<IfcException>(() => model.RemoveRepresentation(26)).Code);
    }
}
