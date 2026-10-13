// RemoveProduct and Purge (#478), over the IFC2X3, IFC4 and IFC4X3 cascade
// fixtures. The cascade is pinned in the shared binding core; this checks
// how the reports cross into .NET.

using System.Collections.Generic;
using System.Linq;
using Xunit;

namespace OpenBim.Ifc.Tests;

public class RemovalTests
{
    public static IEnumerable<object[]> Releases() => new[]
    {
        new object[] { Fixtures.CascadeIfc2x3 },
        new object[] { Fixtures.CascadeIfc4 },
        new object[] { Fixtures.CascadeIfc4x3 },
    };

    [Theory]
    [MemberData(nameof(Releases))]
    public void ADryRunReportsAndTheRemovalApplies(string file)
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(file));
        var before = model.Count;
        var dry = model.RemoveProduct(100, dryRun: true);
        Assert.False(dry.Applied);
        Assert.Equal(before, model.Count);
        Assert.Equal(new RemovedEntity(100, "IFCDOOR"), dry.Removed[0]);
        var map = dry.Kept.Single(kept => kept.Id == 112);
        Assert.Equal("shared", map.Reason);
        Assert.Equal(new[] { 111UL }, map.Users);

        var done = model.RemoveProduct(100);
        Assert.True(done.Applied);
        Assert.Equal(dry.Removed, done.Removed);
        Assert.Equal(before - 11, model.Count);
        Assert.Empty(model.DanglingReferences());
    }

    [Fact]
    public void RefusalsHaveTheirCodesAndOptionsLiftThem()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.CascadeIfc4));
        Assert.Equal(IfcStatus.StillReferenced, Assert.Throws<IfcException>(() => model.RemoveProduct(20)).Status);
        Assert.Equal("still-referenced", Assert.Throws<IfcException>(() => model.RemoveProduct(66)).Code);
        Assert.Equal("wrong-entity-type", Assert.Throws<IfcException>(() => model.RemoveProduct(61)).Code);
        Assert.Equal(new[] { 101UL, 131UL }, model.RemoveProduct(20, rebasePlacements: true).Rebased);
        Assert.Equal(4, model.RemoveProduct(66, untypeOccurrences: true).Removed.Count);
    }

    [Theory]
    [MemberData(nameof(Releases))]
    public void APurgeByClassThenByDefault(string file)
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(file));
        var skipped = model.Purge(new[] { "IfcPropertySet" }, dryRun: true);
        Assert.Empty(skipped.Removed);
        Assert.Equal("protected", skipped.Kept[0].Reason);
        Assert.Equal(new[] { 173UL }, model.Purge(new[] { "IfcPropertySet" }, includeRoots: true).Removed.Select(e => e.Id));
        Assert.Equal(new[] { 170UL, 171UL, 172UL, 174UL }, model.Purge().Removed.Select(e => e.Id));
        Assert.Equal("unsupported-schema", Assert.Throws<IfcException>(() => model.Purge(new[] { "IfcNoSuchThing" })).Code);
    }
}
