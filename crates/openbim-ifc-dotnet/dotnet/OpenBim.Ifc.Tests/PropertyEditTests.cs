// Writing property sets (#123): one checked transaction with the shared
// codes, as smoke.c's `property_edits()` checks it.

using System.Linq;
using Xunit;

namespace OpenBim.Ifc.Tests;

public class PropertyEditTests
{
    [Fact]
    public void AnInheritedValueIsOverriddenOnTheOccurrence()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.Domain));
        // docs:snippet dotnet-domain-write
        // Wall #3 inherits IsExternal from its type: the write overrides it on
        // the wall and never changes the type's shared set.
        var holders = model.SetProperties(new[]
        {
            new PropertyEdit(3, "Pset_WallCommon", "IsExternal", new Value.Typed("IFCBOOLEAN", new Value.Bool(false))),
            new PropertyEdit(3, "Custom", "Note", new Value.Typed("IFCLABEL", new Value.Text("checked"))),
        });
        // holders: per edit, the entity now holding the value

        var own = model.PropertySets(3).Single(set => set.Source == "occurrence" && set.Name == "Pset_WallCommon");
        // docs:end

        Assert.Equal(2, holders.Count);
        Assert.All(holders, id => Assert.NotNull(id));
        Assert.Equal(new Value.Typed("IFCBOOLEAN", new Value.Bool(false)), own.Properties.Single().Value);
        var inherited = model.PropertySets(2).Single();
        Assert.Equal(new Value.Typed("IFCBOOLEAN", new Value.Bool(true)), inherited.Properties.Single().Value);
    }

    [Fact]
    public void ARefusedBatchLeavesTheModelUnchanged()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.Domain));
        var before = model.Write();
        var error = Assert.Throws<IfcException>(() => model.SetProperties(new[]
        {
            new PropertyEdit(3, "Custom", "Note", new Value.Typed("IFCLABEL", new Value.Text("x"))),
            PropertyEdit.Removal(3, "Custom", "Missing"),
        }));
        Assert.Equal("missing-property", error.Code);
        Assert.Equal(IfcStatus.MissingProperty, error.Status);
        Assert.Equal(before, model.Write());
    }

    [Fact]
    public void TheCatalogChecksTheDataType()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.Domain));
        var error = Assert.Throws<IfcException>(() =>
            model.SetProperty(3, "Pset_WallCommon", "IsExternal", new Value.Typed("IFCREAL", new Value.Real(1.0))));
        Assert.Equal("template-violation", error.Code);
        Assert.Equal(IfcStatus.TemplateViolation, error.Status);
    }

    [Fact]
    public void OneEditFormsWriteAndRemove()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.Domain));
        var id = model.SetProperty(3, "Pset_WallCommon", "IsExternal", new Value.Typed("IFCBOOLEAN", new Value.Bool(false)));
        Assert.NotEqual(0UL, id);
        model.RemoveProperty(3, "Pset_WallCommon", "IsExternal");
        Assert.Equal(new Value.Typed("IFCBOOLEAN", new Value.Bool(true)), model.PropertySets(3).Single().Properties.Single().Value);
        Assert.Equal("missing-property",
            Assert.Throws<IfcException>(() => model.RemoveProperty(3, "Pset_WallCommon", "IsExternal")).Code);
    }

    [Fact]
    public void AnEditWithoutAValueIsRefusedByTheCore()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.Domain));
        var error = Assert.Throws<IfcException>(() => model.SetProperties(new[] { new PropertyEdit(3, "Custom", "Note") }));
        Assert.Equal("invalid-value", error.Code);
    }
}
