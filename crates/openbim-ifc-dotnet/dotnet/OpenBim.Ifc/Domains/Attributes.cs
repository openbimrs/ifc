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

/// <summary>One <c>INVERSE</c> attribute of an entity (#462), as the release its file declares defines it: <c>Name : SET [Lower:Upper] OF Entity FOR ForAttribute</c>.</summary>
/// <param name="Name">The declared name, in the schema's spelling, e.g. <c>IsDefinedBy</c>.</param>
/// <param name="Entity">The referencing entity type before <c>FOR</c>; its subtypes count too.</param>
/// <param name="ForAttribute">The attribute of <paramref name="Entity"/> after <c>FOR</c>, which references this entity directly or as an aggregate member.</param>
/// <param name="Aggregate">Whether the inverse is a <c>SET</c> or <c>BAG</c>, read as a <see cref="Value.List"/>; otherwise a single <see cref="Value.Ref"/> or <see cref="Value.Null"/>.</param>
/// <param name="Lower">The declared lower bound; 1 for a single-valued inverse.</param>
/// <param name="Upper">The declared upper bound, null for <c>?</c>.</param>
/// <param name="DeclaredBy">The entity whose declaration introduces the inverse.</param>
public sealed record InverseAttributeInfo(
    string Name,
    string Entity,
    string ForAttribute,
    bool Aggregate,
    long Lower,
    long? Upper,
    string DeclaredBy);

/// <summary>One reference to an entity (#462): entity <paramref name="Id"/> holds it in attribute <paramref name="Index"/>, directly or inside an aggregate.</summary>
/// <param name="Id">The referencing entity.</param>
/// <param name="Index">The zero-based attribute index holding the reference.</param>
public sealed record InverseReference(
    ulong Id,
    int Index);
