// Systems (#123), mirroring the shared binding core. Two records are
// renamed: the core's `System` is `IfcSystem`, since a type named `System`
// would hide the `System` namespace, and its `Systems` is `SystemsView`,
// since a record cannot have a member named like itself.

namespace OpenBim.Ifc;

/// <summary>Every system, and the memberships the reader could not honour.</summary>
/// <param name="Systems">The systems, in file order.</param>
/// <param name="Anomalies">Memberships the reader could not honour.</param>
public sealed record SystemsView(
    EquatableList<IfcSystem> Systems,
    EquatableList<SystemAnomaly> Anomalies);

/// <summary>An <c>IfcSystem</c> or subtype, with its members and served structures.</summary>
/// <param name="Id">Its entity id.</param>
/// <param name="GlobalId">Its <c>GlobalId</c>.</param>
/// <param name="TypeName">Its entity type, upper-case.</param>
/// <param name="Name">Its <c>Name</c>.</param>
/// <param name="LongName">Its <c>LongName</c>.</param>
/// <param name="PredefinedType">Its <c>PredefinedType</c>.</param>
/// <param name="Members">Its members.</param>
/// <param name="ServicedBuildings">The buildings it serves.</param>
/// <param name="ServicedFacilities">The facilities it serves (IFC4X3).</param>
public sealed record IfcSystem(
    ulong Id,
    string? GlobalId,
    string TypeName,
    string? Name,
    string? LongName,
    string? PredefinedType,
    EquatableList<ulong> Members,
    EquatableList<ulong> ServicedBuildings,
    EquatableList<ulong> ServicedFacilities);

/// <summary>A membership the reader could not honour.</summary>
/// <param name="Kind">What went wrong.</param>
/// <param name="Subject">The entity it concerns.</param>
/// <param name="Other">The other entity involved, if any.</param>
/// <param name="Message">A one-line explanation.</param>
public sealed record SystemAnomaly(
    string Kind,
    ulong Subject,
    ulong? Other,
    string Message);
