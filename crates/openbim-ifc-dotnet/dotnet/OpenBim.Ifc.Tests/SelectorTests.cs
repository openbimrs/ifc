// Selectors and values by path (#464), over the IFC2X3, IFC4 and IFC4X3
// selector fixtures. The answers are pinned in the shared binding core;
// this checks how they cross into .NET.

using System.Collections.Generic;
using Xunit;

namespace OpenBim.Ifc.Tests;

public class SelectorTests
{
    public static IEnumerable<object[]> Releases() => new[]
    {
        new object[] { Fixtures.SelectorIfc2x3 },
        new object[] { Fixtures.SelectorIfc4 },
        new object[] { Fixtures.SelectorIfc4x3 },
    };

    [Theory]
    [MemberData(nameof(Releases))]
    public void SelectorsSelectTheSameIds(string file)
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(file));
        Assert.Equal("1.0", IfcModel.SelectorGrammarVersion);
        Assert.Equal(new[] { 17UL, 18UL, 28UL }, model.Select("IfcWall, type=WT-200 + IfcDoor"));
        Assert.Equal(new[] { 17UL }, model.Select("Pset_WallCommon.IsExternal=TRUE"));
        Assert.Equal(new[] { 19UL, 38UL, 40UL }, model.Select("location=\"Level 1\""));
        Assert.Equal(new[] { 17UL }, model.Select("IfcWall", new[] { 17UL, 28UL }));
    }

    [Fact]
    public void AQueryThatDoesNotParseNamesItsSpan()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.SelectorIfc4));
        var error = Assert.Throws<IfcException>(() => model.Select("IfcWall, colour=red"));
        Assert.Equal("invalid-selector", error.Code);
        Assert.Equal(IfcStatus.InvalidSelector, error.Status);
        Assert.Equal((9, 15), error.Span);
        var missing = Assert.Throws<IfcException>(() => model.GetValue(999, "Name"));
        Assert.Equal("missing-entity", missing.Code);
        Assert.Null(missing.Span);
    }

    [Theory]
    [MemberData(nameof(Releases))]
    public void ValuesAreReadAndWrittenByPath(string file)
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(file));
        Assert.Equal(new Value.Text("WT-200"), model.GetValue(17, "type.Name"));
        Assert.Equal(new Value.Ref(16), model.GetValue(17, "type"));
        var set = Assert.IsType<Value.List>(model.GetValue(17, "Pset_WallCommon"));
        var first = Assert.IsType<Value.List>(set.Items[0]);
        Assert.Equal(new Value.Text("IsExternal"), first.Items[0]);
        Assert.Equal(new Value.Bool(true), first.Items[1]);

        model.SetValue(17, "type.Name", "WT-300");
        Assert.Equal(new Value.Text("WT-300"), model.GetValue(18, "type.Name"));
        model.SetValue(19, "Custom_Notes.Note", new Value.Typed("IFCLABEL", new Value.Text("checked")));
        model.SetValue(19, "Custom_Notes.Note", "rechecked");
        Assert.Equal(new Value.Text("rechecked"), model.GetValue(19, "Custom_Notes.Note"));
        Assert.Equal("type-mismatch", Assert.Throws<IfcException>(() => model.SetValue(17, "Name", 3L)).Code);
        Assert.Equal("invalid-value", Assert.Throws<IfcException>(() => model.SetValue(17, "count", 3L)).Code);
    }
}
