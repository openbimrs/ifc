// Unit conversion and a model's length unit (#469), mirroring the shared
// binding core.

using OpenBim.Ifc.Native;

namespace OpenBim.Ifc;

/// <summary>What <see cref="IfcModel.ConvertLengthUnit"/> did, or, for a dry run, would do.</summary>
/// <param name="From">The project length unit before, as named (<c>MILLI METRE</c>, <c>foot</c>).</param>
/// <param name="To">The project length unit after.</param>
/// <param name="LengthFactor">The factor lengths were multiplied by.</param>
/// <param name="AreaFactor">The factor areas were multiplied by, when the area unit followed.</param>
/// <param name="VolumeFactor">The factor volumes were multiplied by, when the volume unit followed.</param>
/// <param name="LengthUnit">The new length unit entity; null when nothing changes.</param>
/// <param name="AreaUnit">The new area unit entity.</param>
/// <param name="VolumeUnit">The new volume unit entity.</param>
/// <param name="Converted">Rescaled values per entity type and attribute.</param>
/// <param name="Kept">Length-like values left as they are, with the reason.</param>
/// <param name="EntitiesChanged">Entities with at least one rescaled value.</param>
/// <param name="ValuesConverted">Numbers rescaled.</param>
/// <param name="Created">Entities created, or for a dry run the ids the commit will create.</param>
/// <param name="Removed">Entities removed, or for a dry run planned for removal.</param>
/// <param name="Applied">False for a dry run.</param>
public sealed record LengthUnitReport(
    string From,
    string To,
    double LengthFactor,
    double? AreaFactor,
    double? VolumeFactor,
    ulong? LengthUnit,
    ulong? AreaUnit,
    ulong? VolumeUnit,
    EquatableList<ConvertedValues> Converted,
    EquatableList<KeptValues> Kept,
    long EntitiesChanged,
    long ValuesConverted,
    EquatableList<ulong> Created,
    EquatableList<ulong> Removed,
    bool Applied);

/// <summary>Values of one attribute of one entity type that a length-unit conversion rescaled.</summary>
/// <param name="EntityType">The entity type, upper case.</param>
/// <param name="Attribute">The attribute.</param>
/// <param name="Kind"><c>length</c>, <c>area</c>, <c>volume</c>, <c>inverse-length</c>, <c>inverse-area</c> or <c>map-scale</c>.</param>
/// <param name="Entities">Entities with a value rescaled here.</param>
/// <param name="Values">Numbers rescaled.</param>
public sealed record ConvertedValues(
    string EntityType,
    string Attribute,
    string Kind,
    long Entities,
    long Values);

/// <summary>Length-like values of one attribute that a length-unit conversion left as they are.</summary>
/// <param name="EntityType">The entity type, upper case.</param>
/// <param name="Attribute">The attribute.</param>
/// <param name="Reason"><c>explicit-unit</c> (the value states its own unit), <c>map-unit</c> (a map coordinate in the CRS's <c>MapUnit</c>), <c>unitless-parameter</c> or <c>unit-not-followed</c>.</param>
/// <param name="Values">Numbers kept.</param>
public sealed record KeptValues(
    string EntityType,
    string Attribute,
    string Reason,
    long Values);

/// <summary>How <see cref="Units.FormatLength"/> writes a length.</summary>
public enum LengthStyle
{
    /// <summary>Decimal, in a display unit, with a number of digits after the point.</summary>
    Decimal = 0,
    /// <summary>Feet and inches, <c>5' - 6 1/2"</c>, rounded to a fraction of an inch.</summary>
    FeetInches = 1,
    /// <summary>Inches, <c>66 1/2"</c>, rounded to a fraction of an inch.</summary>
    Inches = 2,
}

/// <summary>Conversion between units named as text (#469): an SI unit with its prefix written
/// before it (<c>METRE</c>, <c>MILLIMETRE</c>, <c>MILLI METRE</c>, <c>SQUARE_MILLIMETRE</c>;
/// <c>METER</c> spellings accepted), or a named conversion-based unit of IFC4 Table 673 with exact
/// factors (<c>inch</c>, <c>foot</c>, <c>yard</c>, <c>mile</c>, <c>square foot</c>,
/// <c>gallon US</c>, <c>pound</c>, <c>degree</c> ...) or <c>US survey foot</c>. Needs no model.</summary>
public static unsafe class Units
{
    /// <summary>Convert <paramref name="value"/> from unit <paramref name="from"/> to unit <paramref name="to"/>.</summary>
    /// <exception cref="IfcException"><c>type-mismatch</c> for units of different dimensions; <c>invalid-value</c> for text that names no unit; <c>unsupported</c> for a unit with an offset (degree Fahrenheit, #110).</exception>
    public static double Convert(double value, string from, string to)
    {
        var fromBytes = Calls.Encode(from);
        var toBytes = Calls.Encode(to);
        double converted = 0;
        IfcStatus status;
        fixed (byte* f = fromBytes)
        fixed (byte* t = toBytes)
        {
            status = NativeMethods.openbim_ifc_v0_1_convert_unit(value, f, (nuint)fromBytes.Length, t, (nuint)toBytes.Length, &converted);
        }
        if (status != IfcStatus.Ok)
        {
            throw Calls.Error(status, null);
        }
        return converted;
    }

    /// <summary>Write <paramref name="value"/>, a length in unit <paramref name="unit"/>, as text.</summary>
    /// <param name="value">The length.</param>
    /// <param name="unit">Its unit.</param>
    /// <param name="style">Decimal, feet and inches, or inches.</param>
    /// <param name="places">Decimal digits, or the denominator of the inch fraction (2, 4, 8, 16 ...).</param>
    /// <param name="displayUnit">For <see cref="LengthStyle.Decimal"/>, the unit to write in; null for <paramref name="unit"/>.</param>
    /// <param name="suppressZeroInches">Write a whole number of feet as <c>5'</c> rather than <c>5' - 0"</c>.</param>
    /// <exception cref="IfcException"><c>type-mismatch</c> for a unit that is not a length; <c>invalid-value</c> for a unit name or a zero denominator.</exception>
    public static string FormatLength(double value, string unit, LengthStyle style, uint places, string? displayUnit = null, bool suppressZeroInches = false)
    {
        var unitBytes = Calls.Encode(unit);
        var displayBytes = displayUnit is null ? null : Calls.Encode(displayUnit);
        uint flags = suppressZeroInches ? 1u : 0u;
        nuint required = 0;
        IfcStatus status;
        fixed (byte* u = unitBytes)
        fixed (byte* d = displayBytes)
        {
            status = NativeMethods.openbim_ifc_v0_1_format_length(value, u, (nuint)unitBytes.Length, (uint)style, places, d, (nuint)(displayBytes?.Length ?? 0), flags, null, 0, &required);
            if (status == IfcStatus.BufferTooSmall)
            {
                var buffer = new byte[(int)required];
                fixed (byte* b = buffer)
                {
                    status = NativeMethods.openbim_ifc_v0_1_format_length(value, u, (nuint)unitBytes.Length, (uint)style, places, d, (nuint)(displayBytes?.Length ?? 0), flags, b, required, &required);
                }
                if (status == IfcStatus.Ok)
                {
                    return Calls.Utf8.GetString(buffer, 0, buffer.Length - 1);
                }
            }
        }
        throw Calls.Error(status, null);
    }
}
