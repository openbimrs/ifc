// Attributes by name (#326), mirroring the shared binding core.

namespace OpenBim.Ifc;

/// <summary>One explicit attribute of an entity, as the release its file declares defines it.</summary>
/// <param name="Name">The declared name, in the schema's spelling, e.g. <c>Name</c>.</param>
/// <param name="Index">The zero-based slot in the STEP record: the index of <see cref="IfcModel.Attribute"/>.</param>
/// <param name="TypeName">The declared type, or an aggregate's innermost element type.</param>
/// <param name="Optional">Whether <c>$</c> is a valid value.</param>
/// <param name="Aggregate">Whether the type is a <c>LIST</c>, <c>SET</c>, <c>BAG</c> or <c>ARRAY</c>.</param>
/// <param name="Derived">Whether the slot is derived for this entity: written <c>*</c>, not writable.</param>
/// <param name="DeclaredBy">The entity whose declaration introduces the attribute.</param>
public sealed record AttributeInfo(
    string Name,
    int Index,
    string TypeName,
    bool Optional,
    bool Aggregate,
    bool Derived,
    string DeclaredBy);
