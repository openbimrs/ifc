// A reference in a plain-value write (#342).

namespace OpenBim.Ifc;

/// <summary>
/// Entity <c>#Id</c> of the same model, as a plain value for
/// <see cref="IfcModel.SetAttributeByNamePlain"/>: written as a reference
/// once the entity is found and its type is one the attribute accepts.
/// <see cref="Value.Ref"/> is written exactly, unchecked.
/// </summary>
/// <param name="Id">The entity's id.</param>
public sealed record EntityHandle(ulong Id);
