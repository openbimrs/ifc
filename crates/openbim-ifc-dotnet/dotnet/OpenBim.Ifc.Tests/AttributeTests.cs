// Attributes by name (#326): resolved against the declared release, with
// the shared codes for an unknown or a derived attribute.

using System.Linq;
using Xunit;

namespace OpenBim.Ifc.Tests;

public class AttributeTests
{
    [Fact]
    public void ByName()
    {
        var data = Fixtures.Bytes(Fixtures.File);
        // docs:snippet dotnet-by-name
        using var model = IfcModel.Parse(data);
        var wall = model.IdsOfType("IfcWall").Single();

        // Slots as the declared release (here IFC4) defines them, inherited first.
        foreach (var attribute in model.AttributeNames(wall))
        {
            System.Console.WriteLine($"{attribute.Index} {attribute.Name}: {attribute.TypeName}");
            // 0 GlobalId: IfcGloballyUniqueId, 1 OwnerHistory: IfcOwnerHistory, 2 Name: IfcLabel, ...
        }

        var name = model.AttributeByName(wall, "name"); // any case: 'Wall'
        model.SetAttributeByName(wall, "Name", new Value.Text("Renamed"));
        // docs:end

        Assert.Equal(new Value.Text("Wall"), name);
        Assert.Equal(new Value.Text("Renamed"), model.Attribute(wall, 2));
    }

    [Fact]
    public void TheSlotsAreTheDeclaredReleases()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.File));
        var names = model.AttributeNames(1);
        Assert.Equal(9, names.Count);
        Assert.Equal(Enumerable.Range(0, 9), names.Select(attribute => attribute.Index));
        var globalId = names[0];
        Assert.Equal("GlobalId", globalId.Name);
        Assert.False(globalId.Optional);
        Assert.Equal("IfcRoot", globalId.DeclaredBy);
        Assert.Equal("PredefinedType", names[8].Name);
        Assert.Equal(new Value.Enum("STANDARD"), model.AttributeByName(1, "PREDEFINEDTYPE"));
        Assert.Equal(model.Attribute(1, 3), model.AttributeByName(1, "Description"));
    }

    [Fact]
    public void UnknownAndRefusedNamesCarryTheSharedCodes()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.File));
        var unknown = Assert.Throws<IfcException>(() => model.AttributeByName(1, "Height"));
        Assert.Equal("unknown-attribute", unknown.Code);
        Assert.Equal(IfcStatus.UnknownAttribute, unknown.Status);
        Assert.Equal("unknown-attribute",
            Assert.Throws<IfcException>(() => model.SetAttributeByName(1, "Height", new Value.Real(3.0))).Code);
        Assert.Equal("missing-entity", Assert.Throws<IfcException>(() => model.AttributeNames(99)).Code);

        // IfcSIUnit derives Dimensions: it reads as stored and refuses a write.
        var unit = model.Add("IfcSIUnit", new Value[] { new Value.Derived(), new Value.Enum("LENGTHUNIT"), new Value.Null(), new Value.Enum("METRE") });
        Assert.True(model.AttributeNames(unit).Single(attribute => attribute.Name == "Dimensions").Derived);
        Assert.IsType<Value.Derived>(model.AttributeByName(unit, "Dimensions"));
        var before = model.Write();
        var derived = Assert.Throws<IfcException>(() => model.SetAttributeByName(unit, "Dimensions", new Value.Null()));
        Assert.Equal("derived-attribute", derived.Code);
        Assert.Equal(IfcStatus.DerivedAttribute, derived.Status);
        Assert.Equal(before, model.Write());
    }
}
