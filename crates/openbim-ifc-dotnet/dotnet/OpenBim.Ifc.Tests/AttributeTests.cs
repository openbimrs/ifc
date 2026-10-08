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

    [Fact]
    public void PlainValuesAreCoercedAgainstTheDeclaredType()
    {
        var data = Fixtures.Bytes(Fixtures.File);
        // docs:snippet dotnet-plain
        using var model = IfcModel.Parse(data);
        model.SetAttributeByNamePlain(1, "Name", "Renamed"); // IfcLabel: 'Renamed'
        model.SetAttributeByNamePlain(1, "PredefinedType", "shear"); // IfcWallTypeEnum: .SHEAR.
        // A Value is written exactly; an ambiguous SELECT member is refused.
        model.SetAttributeByNamePlain(2, "NominalValue", new Value.Typed("IFCLABEL", new Value.Text("x")));
        // docs:end

        Assert.Equal(new Value.Text("Renamed"), model.AttributeByName(1, "Name"));
        Assert.Equal(new Value.Enum("SHEAR"), model.AttributeByName(1, "PredefinedType"));
        Assert.Equal(new Value.Typed("IFCLABEL", new Value.Text("x")), model.AttributeByName(2, "NominalValue"));

        var point = model.Add("IfcCartesianPoint", new Value[] { new Value.List(new Value[] { new Value.Real(0) }) });
        model.SetAttributeByNamePlain(point, "Coordinates", new object[] { 1, 2.5, 3L });
        Assert.Equal(
            new Value.List(new Value[] { new Value.Real(1), new Value.Real(2.5), new Value.Real(3) }),
            model.AttributeByName(point, "Coordinates"));
        var style = model.Add("IfcCurveStyle", new Value[] { new Value.Text("c") });
        model.SetAttributeByNamePlain(style, "CurveWidth", "by layer");
        Assert.Equal(
            new Value.Typed("IFCDESCRIPTIVEMEASURE", new Value.Text("by layer")),
            model.AttributeByName(style, "CurveWidth"));
        model.SetAttributeByNamePlain(1, "Description", null);
        Assert.Equal(new Value.Null(), model.AttributeByName(1, "Description"));

        var before = model.Write();
        var mismatch = Assert.Throws<IfcException>(() => model.SetAttributeByNamePlain(1, "PredefinedType", "CURVED"));
        Assert.Equal("type-mismatch", mismatch.Code);
        Assert.Equal(IfcStatus.TypeMismatch, mismatch.Status);
        var ambiguous = Assert.Throws<IfcException>(() => model.SetAttributeByNamePlain(2, "NominalValue", "x"));
        Assert.Equal("ambiguous-value", ambiguous.Code);
        Assert.Equal(IfcStatus.AmbiguousValue, ambiguous.Status);
        Assert.Contains("IfcLabel", ambiguous.Message);
        Assert.Equal("type-mismatch",
            Assert.Throws<IfcException>(() => model.SetAttributeByNamePlain(1, "Name", 3)).Code);
        Assert.Equal("missing-reference",
            Assert.Throws<IfcException>(() => model.SetAttributeByNamePlain(1, "ObjectPlacement", new EntityHandle(99))).Code);
        Assert.Equal("type-mismatch",
            Assert.Throws<IfcException>(() => model.SetAttributeByNamePlain(1, "ObjectPlacement", new EntityHandle(point))).Code);
        Assert.Equal(before, model.Write());
    }
}
