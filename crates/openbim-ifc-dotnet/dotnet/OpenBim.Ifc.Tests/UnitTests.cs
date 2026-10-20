// Unit conversion and ConvertLengthUnit (#469). The conversion is pinned in
// the shared binding core; this checks how it crosses into .NET.

using System.Linq;
using Xunit;

namespace OpenBim.Ifc.Tests;

public class UnitTests
{
    [Fact]
    public void NamedUnitsConvertAndFormat()
    {
        Assert.Equal(304.8, Units.Convert(1.0, "foot", "MILLIMETRE"));
        Assert.Equal(1.5, Units.Convert(1500.0, "MILLI METRE", "METRE"));
        Assert.Equal("type-mismatch", Assert.Throws<IfcException>(() => Units.Convert(1.0, "METRE", "SQUARE_METRE")).Code);
        Assert.Equal("invalid-value", Assert.Throws<IfcException>(() => Units.Convert(1.0, "furlong", "METRE")).Code);
        Assert.Equal("unsupported", Assert.Throws<IfcException>(() => Units.Convert(1.0, "degree Fahrenheit", "KELVIN")).Code);
        Assert.Equal("1' - 5 1/2\"", Units.FormatLength(444.5, "MILLIMETRE", LengthStyle.FeetInches, 8, suppressZeroInches: true));
        Assert.Equal("1.25", Units.FormatLength(1250.0, "MILLIMETRE", LengthStyle.Decimal, 2, "METRE"));
        Assert.Equal("66 1/2\"", Units.FormatLength(66.5, "inch", LengthStyle.Inches, 2));
    }

    [Fact]
    public void AModelConvertsItsLengthUnit()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.CascadeIfc4));
        Assert.Equal(2.0, model.ConvertUnitValue(2.0, 6, 6));
        var count = model.Count;

        var dry = model.ConvertLengthUnit("MILLIMETRE", dryRun: true);
        Assert.False(dry.Applied);
        Assert.Equal(count, model.Count);
        Assert.Equal(new Value.Real(3.0), model.Attribute(26, 3));

        var done = model.ConvertLengthUnit("MILLIMETRE");
        Assert.True(done.Applied);
        Assert.Equal("METRE", done.From);
        Assert.Equal("MILLI METRE", done.To);
        Assert.Equal(1000.0, done.LengthFactor);
        Assert.Equal(dry.ValuesConverted, done.ValuesConverted);
        Assert.Contains(done.Converted, c => c.EntityType == "IFCCARTESIANPOINT" && c.Kind == "length");
        Assert.Equal(new Value.Real(3000.0), model.Attribute(26, 3));
        Assert.Empty(model.DanglingReferences());

        Assert.Equal("invalid-value", Assert.Throws<IfcException>(() => model.ConvertLengthUnit("SQUARE_METRE")).Code);
    }
}
