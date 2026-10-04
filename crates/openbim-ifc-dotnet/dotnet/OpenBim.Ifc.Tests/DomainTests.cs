// The domain views (#123): records cross as tapes and decode into C#
// records with the shared codes, as smoke.c's `domains()` checks them.

using System.Linq;
using Xunit;

namespace OpenBim.Ifc.Tests;

public class DomainTests
{
    [Fact]
    public void DomainViews()
    {
        var data = Fixtures.Bytes(Fixtures.Domain);
        // docs:snippet dotnet-domain-views
        using var model = IfcModel.Parse(data);
        var wall = model.IdsOfType("IfcWall").Single();

        // Property sets: the wall's own first, then its type's; values typed.
        foreach (var set in model.PropertySets(wall))
        {
            foreach (var property in set.Properties)
            {
                System.Console.WriteLine($"{set.Name}.{property.Name} = {property.Value}");
                // Pset_WallCommon.IsExternal = IFCBOOLEAN(.T.)
            }
        }

        var material = model.Material(wall); // MaterialAssignment { Kind = "layer-set", Layers = [...] }
        var tree = model.SpatialTree(); // SpatialTree { Nodes = [...] }
        var classes = model.Classifications(wall); // [Classification { Identification = ... }]
        // docs:end

        Assert.Equal("layer-set", material!.Kind);
        Assert.Empty(tree.Nodes);
        Assert.Empty(classes);
    }

    [Fact]
    public void PropertySetsAreInheritedFromTheType()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.Domain));
        var set = Assert.Single(model.PropertySets(3));
        Assert.Equal(30UL, set.Id);
        Assert.Equal("3ZvctVUKr0kugbFTf53O9L", set.GlobalId);
        Assert.Equal("Pset_WallCommon", set.Name);
        Assert.Equal("IFCPROPERTYSET", set.TypeName);
        Assert.Equal("type", set.Source);
        Assert.Equal(2UL, set.SourceId);

        var property = Assert.Single(set.Properties);
        Assert.Equal(31UL, property.Id);
        Assert.Equal("IsExternal", property.Name);
        Assert.Equal("value", property.Kind);
        Assert.Equal(new Value.Typed("IFCBOOLEAN", new Value.Bool(true)), property.Value);
        Assert.Null(property.Enumeration);
        Assert.Empty(property.Members);

        // Equal snapshots compare equal, lists included.
        Assert.Equal(set, model.PropertySets(3).Single());
    }

    [Fact]
    public void TheMaterialIsTheTypesLayerSet()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.Domain));
        var material = model.Material(3)!;
        Assert.Equal(26UL, material.Relationship);
        Assert.Equal("type", material.Source);
        Assert.Equal(2UL, material.TypeObject);
        Assert.Equal(24UL, material.Set);
        Assert.Equal("WT-200", material.Name);
        var layer = Assert.Single(material.Layers);
        Assert.Equal(0.2, layer.Thickness, 12);
        Assert.Equal("Concrete", layer.Material!.Name);
        Assert.Equal(new Value.Unknown(), layer.IsVentilated);
        Assert.Null(model.Material(20));
    }

    [Fact]
    public void TheOtherViewsAnswerOnAnEmptyModel()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.Domain));
        var tree = model.SpatialTree();
        Assert.Equal("IFC4_ADD2_TC1", tree.Release);
        var systems = model.Systems();
        Assert.Empty(systems.Systems);
        Assert.Empty(systems.Anomalies);
        var cost = model.Cost();
        Assert.Empty(cost.Items);
        Assert.Empty(model.Georeferencing());
        Assert.Empty(model.Classifications(3));
    }

    [Fact]
    public void TheUnitOfAMeasureResolvesToSi()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.Domain));
        var error = Assert.Throws<IfcException>(() => model.ResolveUnit("IFCLABEL"));
        Assert.Equal("invalid-value", error.Code);
        var ratio = model.ResolveUnit("IFCRATIOMEASURE");
        Assert.Null(ratio.Unit);
        Assert.Equal(7, ratio.Dimensions.Count);
        Assert.All(ratio.Dimensions, exponent => Assert.Equal(0L, exponent));
        Assert.Equal(1.0, ratio.Scale);
    }

    [Fact]
    public void ViewsRefuseWithTheSharedCodes()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.Ifc2x3));
        var error = Assert.Throws<IfcException>(() => model.Georeferencing());
        Assert.Equal("unsupported-schema", error.Code);
        Assert.Equal(IfcStatus.UnsupportedSchema, error.Status);
        Assert.Equal("missing-entity", Assert.Throws<IfcException>(() => model.PropertySets(99)).Code);

        using var domain = IfcModel.Parse(Fixtures.Bytes(Fixtures.Domain));
        Assert.Equal("wrong-entity-type", Assert.Throws<IfcException>(() => domain.PropertySets(20)).Code);
    }
}
