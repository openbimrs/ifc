// Tables of values by path (#474), over the IFC2X3, IFC4 and IFC4X3
// selector fixtures. The answers are pinned in the shared binding core;
// this checks how they cross into .NET.

using System.Collections.Generic;
using System.Linq;
using System.Text;
using Xunit;

namespace OpenBim.Ifc.Tests;

public class TableTests
{
    public static IEnumerable<object[]> Releases() => new[]
    {
        new object[] { Fixtures.SelectorIfc2x3 },
        new object[] { Fixtures.SelectorIfc4 },
        new object[] { Fixtures.SelectorIfc4x3 },
    };

    [Theory]
    [MemberData(nameof(Releases))]
    public void AnExportedTableImportsBackAsANoOp(string file)
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(file));
        var before = model.Write();
        foreach (var format in new[] { "csv", "tsv", "xlsx" })
        {
            var table = model.ExportTable("IfcWall", new[] { "Name", "type.Name" }, format);
            var result = model.ImportTable(table, format);
            Assert.False(result.Applied);
            Assert.Empty(result.Changes);
            Assert.Empty(result.Refusals);
        }
        Assert.Equal(before, model.Write());
    }

    [Theory]
    [MemberData(nameof(Releases))]
    public void EditsArePlannedThenApplied(string file)
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(file));
        var text = Encoding.UTF8.GetString(model.ExportTable("IfcWall", new[] { "Name", "type.Name" }, key: "id"));
        Assert.StartsWith("id,Name,type.Name\r\n17,Wall A,WT-200\r\n", text);
        var edited = Encoding.UTF8.GetBytes(text.Replace("17,Wall A,WT-200", "17,Wall A,WT-250"));
        var dry = model.ImportTable(edited, dryRun: true);
        Assert.False(dry.Applied);
        Assert.Equal(new TableChange(2, 3, "17", "type.Name", 17, 16, "Name", "WT-200", "WT-250"), dry.Changes.Single());
        Assert.True(model.ImportTable(edited).Applied);
        Assert.Equal(new[] { 16UL }, model.Select("IfcWallType, Name=WT-250"));
    }

    [Fact]
    public void RefusalsAreDataAndWriteNothing()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.SelectorIfc4));
        var before = model.Write();
        var result = model.ImportTable(Encoding.UTF8.GetBytes("id,Name,Name.\n17,X,\n99,Y,\n"));
        Assert.False(result.Applied);
        Assert.Equal(new[] { "invalid-header", "unknown-key" }, result.Refusals.Select(refusal => refusal.Code));
        Assert.Equal(1, result.Refusals[0].Row);
        Assert.Equal(3, result.Refusals[0].Column);
        Assert.Equal(new[] { 5, 5 }, result.Refusals[0].Span!.ToArray());
        Assert.Equal(1, result.Refusals[1].Column);
        Assert.Equal(before, model.Write());
        Assert.Equal("invalid-value", Assert.Throws<IfcException>(() => model.ExportTable("IfcWall", new string[0], "ods")).Code);
    }
}
