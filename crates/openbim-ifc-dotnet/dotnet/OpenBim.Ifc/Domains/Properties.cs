// Property sets, quantities and units (#123). Each record mirrors the
// shared binding core's, field for field and in the same order.

namespace OpenBim.Ifc;

/// <summary>
/// A property set, quantity set or predefined property set applying to an
/// object. <see cref="Source"/> is <c>occurrence</c> (stated on the object) or
/// <c>type</c> (held by the type object <see cref="SourceId"/>). An inherited
/// property the occurrence overrides is left out.
/// </summary>
/// <param name="Id">The set's entity id.</param>
/// <param name="GlobalId">The set's <c>GlobalId</c>.</param>
/// <param name="Name">The set's <c>Name</c>, or the entity name of a predefined set that states none.</param>
/// <param name="TypeName">The set's entity type, upper-case: <c>IFCPROPERTYSET</c>, <c>IFCELEMENTQUANTITY</c>, ...</param>
/// <param name="Source"><c>occurrence</c> or <c>type</c>.</param>
/// <param name="SourceId">For <c>type</c>, the type object's id.</param>
/// <param name="Properties">The set's properties and quantities, in file order.</param>
public sealed record PropertySet(
    ulong Id,
    string? GlobalId,
    string Name,
    string TypeName,
    string Source,
    ulong? SourceId,
    EquatableList<Property> Properties);

/// <summary>
/// One object's answer from <see cref="IfcModel.PropertySetsMany"/> (#358):
/// exactly what <see cref="IfcModel.PropertySets"/> returns for it, or the
/// refusal it throws.
/// </summary>
/// <param name="Object">The object's entity id.</param>
/// <param name="Sets">Its sets, as <see cref="IfcModel.PropertySets"/> returns them; empty when refused.</param>
/// <param name="Refusal">The code and message <see cref="IfcModel.PropertySets"/> throws for it.</param>
public sealed record ObjectPropertySets(
    ulong Object,
    EquatableList<PropertySet> Sets,
    PropertyRefusal? Refusal);

/// <summary>Why one object's property sets were refused.</summary>
/// <param name="Code">The shared error code, as <see cref="IfcException.Code"/>.</param>
/// <param name="Message">The error's message.</param>
public sealed record PropertyRefusal(string Code, string Message);

/// <summary>
/// One property, quantity or predefined-set attribute. <see cref="Value"/>
/// keeps the declared type, e.g. <c>IFCLENGTHMEASURE(0.2)</c>, and is
/// exactly what a <see cref="PropertyEdit"/> writes back.
/// </summary>
/// <param name="Id">The property or quantity entity; for a predefined set's attribute, the set's id.</param>
/// <param name="Name"><c>Name</c>, or the attribute name of a predefined set.</param>
/// <param name="TypeName">The entity type, upper-case: <c>IFCPROPERTYSINGLEVALUE</c>, <c>IFCQUANTITYLENGTH</c>, ...</param>
/// <param name="Kind"><c>value</c>, <c>enumerated</c>, <c>list</c>, <c>bounded</c>, <c>table</c>, <c>reference</c> or <c>complex</c>.</param>
/// <param name="ValueType">The declared IFC value type (<c>IFCLENGTHMEASURE</c>), when there is one.</param>
/// <param name="Unit">The unit entity the property states; null when the project default applies or no unit does.</param>
/// <param name="Value">The value, typed; a <c>LIST</c> for <c>enumerated</c> and <c>list</c>, a <c>REF</c> for <c>reference</c>.</param>
/// <param name="Enumeration">For <c>enumerated</c>: the enumeration the values come from.</param>
/// <param name="Bounds">For <c>bounded</c>: the bounds and set point.</param>
/// <param name="Table">For <c>table</c>: the rows and units.</param>
/// <param name="Usage">For <c>reference</c> and <c>complex</c>: <c>UsageName</c> or <c>Usage</c>.</param>
/// <param name="Discrimination">For a <c>complex</c> quantity: <c>Discrimination</c>.</param>
/// <param name="Quality">For a <c>complex</c> quantity: <c>Quality</c>.</param>
/// <param name="Members">For <c>complex</c>: its members, each resolved as a property.</param>
public sealed record Property(
    ulong Id,
    string Name,
    string TypeName,
    string Kind,
    string? ValueType,
    ulong? Unit,
    Value Value,
    PropertyEnumeration? Enumeration,
    PropertyBounds? Bounds,
    PropertyTable? Table,
    string? Usage,
    string? Discrimination,
    string? Quality,
    EquatableList<Property> Members);

/// <summary>An <c>IfcPropertyEnumeration</c>: the values an enumerated property permits.</summary>
/// <param name="Id">Its entity id.</param>
/// <param name="Name">Its <c>Name</c>.</param>
/// <param name="Values">The permitted values, typed.</param>
public sealed record PropertyEnumeration(
    ulong Id,
    string Name,
    EquatableList<Value> Values);

/// <summary>An <c>IfcPropertyBoundedValue</c>'s values; <see cref="Value.Null"/> when unstated.</summary>
/// <param name="Lower"><c>LowerBoundValue</c>.</param>
/// <param name="Upper"><c>UpperBoundValue</c>.</param>
/// <param name="SetPoint"><c>SetPointValue</c> (IFC4, IFC4X3).</param>
public sealed record PropertyBounds(
    Value Lower,
    Value Upper,
    Value SetPoint);

/// <summary>One row of a <see cref="PropertyTable"/>.</summary>
/// <param name="Defining">The defining value, typed.</param>
/// <param name="Defined">The defined value, typed.</param>
public sealed record PropertyTableRow(
    Value Defining,
    Value Defined);

/// <summary>An <c>IfcPropertyTableValue</c>.</summary>
/// <param name="Rows">The rows, in file order.</param>
/// <param name="Expression"><c>Expression</c>.</param>
/// <param name="DefiningUnit"><c>DefiningUnit</c>.</param>
/// <param name="DefinedUnit"><c>DefinedUnit</c>.</param>
/// <param name="Interpolation"><c>CurveInterpolation</c> (IFC4, IFC4X3), e.g. <c>LINEAR</c>.</param>
public sealed record PropertyTable(
    EquatableList<PropertyTableRow> Rows,
    string? Expression,
    ulong? DefiningUnit,
    ulong? DefinedUnit,
    string? Interpolation);

/// <summary>A measure's effective unit, resolved to SI: <c>si = value * Scale + Offset</c>.</summary>
/// <param name="Unit">The unit entity that applies; null for a dimensionless measure.</param>
/// <param name="FromProject">Whether <see cref="Unit"/> is the project default rather than the stated one.</param>
/// <param name="Dimensions">The SI exponents <c>(L, M, T, I, Θ, N, J)</c>.</param>
/// <param name="Scale">Multiplier into the SI base unit.</param>
/// <param name="Offset">Added after scaling; non-zero only for degrees Celsius.</param>
public sealed record ResolvedUnit(
    ulong? Unit,
    bool FromProject,
    EquatableList<long> Dimensions,
    double Scale,
    double Offset);

/// <summary>
/// One edit for <see cref="IfcModel.SetProperties"/>: write <see cref="Value"/>
/// to property <see cref="Name"/> of set <see cref="Set"/> on
/// <see cref="Object"/>, or, built with <see cref="Removal"/>, remove it.
/// </summary>
/// <remarks>
/// <see cref="Value"/> is the read side's <see cref="Property.Value"/>: a
/// <see cref="Value.Typed"/> IFC value (or <see cref="Value.Null"/>), a
/// <see cref="Value.List"/> of them for an enumerated or list value, a typed
/// measure for a quantity. <see cref="SetType"/> (<c>IfcPropertySet</c> or
/// <c>IfcElementQuantity</c>) names the entity of a set the edit creates that
/// neither the type object nor the catalog describes.
/// </remarks>
/// <param name="Object">The object (occurrence or type) whose set the edit addresses.</param>
/// <param name="Set">The set name, e.g. <c>Pset_WallCommon</c>.</param>
/// <param name="Name">The property name, e.g. <c>IsExternal</c>.</param>
/// <param name="Value">The value to write; unused by a removal.</param>
/// <param name="SetType">The entity of a new set, when nothing else names it.</param>
/// <param name="Remove">Remove the property instead of writing it.</param>
public sealed record PropertyEdit(
    ulong Object,
    string Set,
    string Name,
    Value? Value = null,
    string? SetType = null,
    bool Remove = false)
{
    /// <summary>An edit removing property <paramref name="name"/> from <paramref name="obj"/>'s own set.</summary>
    public static PropertyEdit Removal(ulong obj, string set, string name) => new(obj, set, name, Remove: true);
}
