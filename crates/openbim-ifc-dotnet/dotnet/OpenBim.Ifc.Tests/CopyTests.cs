// CopyProduct, the copy operations of a batch, and Append (#479), over the
// cascade fixtures. The copies are pinned in the shared binding core; this
// checks how the reports and options cross into .NET.

using System.Collections.Generic;
using System.Linq;
using Xunit;

namespace OpenBim.Ifc.Tests;

public class CopyTests
{
    public static IEnumerable<object[]> Releases() => new[]
    {
        new object[] { Fixtures.CascadeIfc2x3 },
        new object[] { Fixtures.CascadeIfc4 },
        new object[] { Fixtures.CascadeIfc4x3 },
    };

    [Theory]
    [MemberData(nameof(Releases))]
    public void ACopyReportsAndADryRunChangesNothing(string file)
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(file));
        var before = model.Count;
        var dry = model.CopyProduct(20, dryRun: true);
        Assert.False(dry.Applied);
        Assert.Equal(before, model.Count);
        var done = model.CopyProduct(20);
        Assert.True(done.Applied);
        Assert.Equal(before + done.Copied.Count, model.Count);
        Assert.Equal(done.Copy, done.Copied.Single(c => c.Original == 20).Copy);
        Assert.Contains(67UL, done.Joined);
        Assert.Contains(done.Copied, c => c.Original == 100);
        Assert.Empty(model.DanglingReferences());
    }

    [Fact]
    public void OptionsAndRefusalsCross()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.CascadeIfc4));
        var bare = model.CopyProduct(20, offset: (0, 5, 0), openings: false, container: false);
        Assert.Contains(91UL, bare.Skipped);
        Assert.Contains(19UL, bare.Skipped);
        Assert.DoesNotContain(bare.Copied, c => c.Original == 81);
        Assert.Equal("invalid-value", Assert.Throws<IfcException>(() => model.CopyProduct(20, offset: (1, 0, 0), placement: 18)).Code);
        Assert.Equal("wrong-entity-type", Assert.Throws<IfcException>(() => model.CopyProduct(61)).Code);
    }

    [Fact]
    public void CopyOperationsRunInABatch()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.CascadeIfc4));
        var ids = model.Author(new[]
        {
            AuthorOp.CopyProduct(100, fillings: false),
            AuthorOp.CopyEntity(26, share: new[] { "IfcProfileDef" }),
        });
        var solid = ids[1]!.Value;
        Assert.Equal(new Value.Ref(27), model.AttributeByName(solid, "SweptArea"));
    }

    [Fact]
    public void AppendMergesRegeneratesAndRefuses()
    {
        using var source = IfcModel.Parse(Fixtures.Bytes(Fixtures.CascadeIfc4));
        using var target = IfcModel.Parse(Fixtures.Bytes(Fixtures.CascadeIfc4));
        Assert.Equal("invalid-value", Assert.Throws<IfcException>(() => target.Append(source, new[] { 66UL })).Code);
        var merged = target.Append(source, new[] { 66UL }, GlobalIdMode.Merge);
        Assert.Empty(merged.Copied);
        Assert.Equal(66UL, merged.Reused[0].Copy);
        Assert.Null(merged.Copy);
        var report = target.Append(source, new[] { 20UL }, GlobalIdMode.Regenerate);
        Assert.Contains(19UL, report.Skipped);
        Assert.NotEmpty(report.Regenerated);
        var bare = target.Append(source, new[] { 111UL }, GlobalIdMode.Regenerate, ReuseKinds.None);
        Assert.DoesNotContain(bare.Reused, r => r.Original == 11);
        using var old = IfcModel.Parse(Fixtures.Bytes(Fixtures.CascadeIfc2x3));
        Assert.Equal("unsupported-schema", Assert.Throws<IfcException>(() => old.Append(source, new[] { 111UL }, GlobalIdMode.Regenerate)).Code);
    }
}
