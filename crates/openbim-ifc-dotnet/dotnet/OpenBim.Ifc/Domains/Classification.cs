// Classification (#123), mirroring the shared binding core.

namespace OpenBim.Ifc;

/// <summary>One classification of an object: its own, or its type object's.</summary>
/// <param name="Relationship">The <c>IfcRelAssociatesClassification</c>.</param>
/// <param name="GlobalId">The relationship's <c>GlobalId</c>.</param>
/// <param name="Source"><c>occurrence</c> or <c>type</c>.</param>
/// <param name="TypeObject">For <c>type</c>, the type object.</param>
/// <param name="Target">The classification reference, system or notation.</param>
/// <param name="Kind"><c>reference</c>, <c>system</c> or <c>notation</c>.</param>
/// <param name="Identification">The reference's identification, e.g. <c>21.22</c>.</param>
/// <param name="Name">Its name.</param>
/// <param name="Location">Its location, e.g. a URL.</param>
/// <param name="Notation">A notation's facets.</param>
/// <param name="Parents">The references above it, nearest first.</param>
/// <param name="System">The classification system it belongs to.</param>
public sealed record Classification(
    ulong Relationship,
    string? GlobalId,
    string Source,
    ulong? TypeObject,
    ulong Target,
    string Kind,
    string? Identification,
    string? Name,
    string? Location,
    EquatableList<string> Notation,
    EquatableList<ulong> Parents,
    ClassificationSystem? System);

/// <summary>An <c>IfcClassification</c>.</summary>
/// <param name="Id">Its entity id.</param>
/// <param name="Name">Its <c>Name</c>, e.g. <c>Uniclass</c>.</param>
/// <param name="Source">Its <c>Source</c>.</param>
/// <param name="Edition">Its <c>Edition</c>.</param>
public sealed record ClassificationSystem(
    ulong Id,
    string Name,
    string? Source,
    string? Edition);
