// Removing a product with what it owns, and purging (#478), mirroring the
// shared binding core.

namespace OpenBim.Ifc;

/// <summary>What <see cref="IfcModel.RemoveProduct"/> or <see cref="IfcModel.Purge"/> did, or, for a dry run, would do. Every list is in id order.</summary>
/// <param name="Removed">Every entity removed.</param>
/// <param name="Kept">Entities reached and kept, with the reason.</param>
/// <param name="Detached">Relationships and layer assignments that lost a removed member and stay.</param>
/// <param name="Rebased">Placements re-based onto a kept ancestor.</param>
/// <param name="Applied">False for a dry run.</param>
public sealed record RemovalReport(
    EquatableList<RemovedEntity> Removed,
    EquatableList<KeptEntity> Kept,
    EquatableList<ulong> Detached,
    EquatableList<ulong> Rebased,
    bool Applied);

/// <summary>An entity a removal or purge removed.</summary>
/// <param name="Id">Its id.</param>
/// <param name="TypeName">Its type, upper case.</param>
public sealed record RemovedEntity(
    ulong Id,
    string TypeName);

/// <summary>An entity a removal reached and kept.</summary>
/// <param name="Id">Its id.</param>
/// <param name="TypeName">Its type, upper case.</param>
/// <param name="Reason"><c>shared</c> (<paramref name="Users"/> still need it), <c>related-object</c> (a type, container or filling), <c>definition</c> (a material, classification reference, unit, context or owner history, which only a purge removes) or <c>protected</c> (a purge skipped it).</param>
/// <param name="Users">For <c>shared</c>, the entities that stay and use it.</param>
public sealed record KeptEntity(
    ulong Id,
    string TypeName,
    string Reason,
    EquatableList<ulong> Users);
