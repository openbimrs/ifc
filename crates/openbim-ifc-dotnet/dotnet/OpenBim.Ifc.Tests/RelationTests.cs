// Entities by GlobalId, new GlobalIds, and an object's type, a type's
// occurrences, openings and fillings (#463), over the IFC2X3, IFC4 and
// IFC4X3 relation fixtures. The answers are pinned in the shared binding
// core; this checks how they cross into .NET.

using System.Collections.Generic;
using System.Linq;
using System.Text.RegularExpressions;
using Xunit;

namespace OpenBim.Ifc.Tests;

public class RelationTests
{
    public static IEnumerable<object[]> Releases() => new[]
    {
        new object[] { Fixtures.RelationsIfc2x3 },
        new object[] { Fixtures.RelationsIfc4 },
        new object[] { Fixtures.RelationsIfc4x3 },
    };

    private static string GlobalIdOf(IfcModel model, ulong id) => ((Value.Text)model.Attribute(id, 0)).Value;

    [Theory]
    [MemberData(nameof(Releases))]
    public void AnEntityIsFoundByItsGlobalId(string file)
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(file));
        Assert.Equal(17UL, model.ByGuid(GlobalIdOf(model, 17)));
        Assert.Null(model.ByGuid("3MtNd0Uaz8Ix2OPtRq_3Kk"));
        Assert.Empty(model.DuplicateGlobalIds());
        var error = Assert.Throws<IfcException>(() => model.ByGuid("not a GlobalId"));
        Assert.Equal("invalid-value", error.Code);
    }

    [Fact]
    public void ACopiedGlobalIdIsRefusedAndListed()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.RelationsIfc4));
        var guid = GlobalIdOf(model, 17);
        Assert.Equal(17UL, model.ByGuid(guid));
        var copy = model.Add(model.TypeOf(17), model.Attributes(17));
        var error = Assert.Throws<IfcException>(() => model.ByGuid(guid));
        Assert.Equal("invalid-model", error.Code);
        var duplicate = Assert.Single(model.DuplicateGlobalIds());
        Assert.Equal(guid, duplicate.GlobalId);
        Assert.Equal(new[] { 17UL, copy }, duplicate.Ids);
#pragma warning disable CS0618 // Remove is deprecated (#478); this is its own test.
        model.Remove(copy);
#pragma warning restore CS0618
        Assert.Equal(17UL, model.ByGuid(guid));
    }

    [Theory]
    [MemberData(nameof(Releases))]
    public void TypesOpeningsAndFillingsFollowTheRelationships(string file)
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(file));
        Assert.Equal(16UL, model.TypeObject(17));
        Assert.Null(model.TypeObject(19));
        Assert.Equal(new[] { 17UL, 18UL }, model.OccurrencesOf(16));
        Assert.Equal(27UL, model.TypeObject(28));
        Assert.Equal(new[] { 21UL, 23UL }, model.OpeningsOf(17));
        Assert.Equal(new[] { 28UL }, model.FillingsOf(21));
        Assert.Empty(model.FillingsOf(23));
        Assert.Equal(18UL, model.VoidedElementOf(22));
    }

    [Fact]
    public void RefusalsCarryTheSharedCodes()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.RelationsIfc2x3));
        Assert.Equal("missing-entity", Assert.Throws<IfcException>(() => model.TypeObject(999)).Code);
        Assert.Equal("wrong-entity-type", Assert.Throws<IfcException>(() => model.TypeObject(8)).Code);
        Assert.Equal("wrong-entity-type", Assert.Throws<IfcException>(() => model.OccurrencesOf(17)).Code);
    }

    [Fact]
    public void NewGlobalIdsAreValidAndUniqueAndNameBasedOnesReproducible()
    {
        var drawn = new HashSet<string>(Enumerable.Range(0, 1000).Select(_ => IfcModel.NewGuid()));
        Assert.Equal(1000, drawn.Count);
        Assert.All(drawn, guid => Assert.Matches(new Regex("^[0-3][0-9A-Za-z_$]{21}$"), guid));
        var named = IfcModel.NameBasedGuid("my-exporter", "wall", "42");
        Assert.Equal(named, IfcModel.NameBasedGuid("my-exporter", "wall", "42"));
        Assert.NotEqual(named, IfcModel.NameBasedGuid("my-exporter", "wall", "43"));
        // Pinned in the facade: every binding derives the same GlobalId.
        Assert.Equal("347eFU5Y69bPY9V9tumQuV", IfcModel.NameBasedGuid("openbim-ifc", "example"));
        Assert.Equal(22, IfcModel.NameBasedGuid().Length);
    }

    private static ulong[] Refs(Value value) =>
        ((Value.List)value).Items.Select(item => ((Value.Ref)item).Id).ToArray();

    [Theory]
    [MemberData(nameof(Releases))]
    public void InverseAttributesReadThroughTheDeclaredRelease(string file)
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(file));
        var ifc2x3 = file == Fixtures.RelationsIfc2x3;
        if (ifc2x3)
        {
            Assert.Equal(new[] { 20UL, 36UL }, Refs(model.AttributeByName(17, "IsDefinedBy")));
            Assert.Equal("unknown-attribute", Assert.Throws<IfcException>(() => model.AttributeByName(17, "IsTypedBy")).Code);
        }
        else
        {
            Assert.Equal(new[] { 36UL }, Refs(model.AttributeByName(17, "isdefinedby")));
            Assert.Equal(new[] { 20UL }, Refs(model.AttributeByName(17, "IsTypedBy")));
        }
        Assert.Equal(new[] { 33UL }, Refs(model.AttributeByName(17, "ContainedInStructure")));
        Assert.Empty(Refs(model.AttributeByName(11, "Decomposes")));
        Assert.Equal(new Value.Ref(24), model.AttributeByName(21, "VoidsElements"));
        var refused = Assert.Throws<IfcException>(() => model.SetAttributeByName(17, "HasOpenings", new Value.Null()));
        Assert.Equal("inverse-attribute", refused.Code);
        Assert.Equal(IfcStatus.InverseAttribute, refused.Status);

        Assert.Equal(new[] { new InverseReference(24, 5), new InverseReference(30, 4) }, model.InverseReferences(21));
        Assert.Equal(5, model.TotalInverses(17));
        Assert.Equal("missing-entity", Assert.Throws<IfcException>(() => model.TotalInverses(999)).Code);
        var voids = model.InverseAttributeNames(21).Single(info => info.Name == "VoidsElements");
        Assert.Equal(new InverseAttributeInfo("VoidsElements", "IfcRelVoidsElement", "RelatedOpeningElement", false, 1, 1, "IfcFeatureElementSubtraction"), voids);
        var defined = model.InverseAttributeNames(17).Single(info => info.Name == "IsDefinedBy");
        Assert.Equal(ifc2x3 ? "IfcRelDefines" : "IfcRelDefinesByProperties", defined.Entity);
        Assert.Null(defined.Upper);
    }
}
