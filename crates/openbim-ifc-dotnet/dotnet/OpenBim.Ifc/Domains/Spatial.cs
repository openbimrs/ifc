// The spatial containment tree (#123), mirroring the shared binding core.

namespace OpenBim.Ifc;

/// <summary>The spatial containment tree: every container with its parent,
/// sub-containers and contained elements.</summary>
/// <param name="Release">The IFC release the tree was read with.</param>
/// <param name="Roots">The top containers, usually the project.</param>
/// <param name="Nodes">Every container.</param>
/// <param name="Orphans">Containers no decomposition reaches from a root.</param>
/// <param name="Dangling">Relationships naming an entity the file lacks.</param>
/// <param name="Anomalies">Relationships the reader could not honour, and which one it kept.</param>
public sealed record SpatialTree(
    string? Release,
    EquatableList<ulong> Roots,
    EquatableList<SpatialNode> Nodes,
    EquatableList<ulong> Orphans,
    EquatableList<SpatialDanglingReference> Dangling,
    EquatableList<SpatialAnomaly> Anomalies);

/// <summary>A spatial container.</summary>
/// <param name="Id">Its entity id.</param>
/// <param name="GlobalId">Its <c>GlobalId</c>.</param>
/// <param name="Name">Its <c>Name</c>.</param>
/// <param name="TypeName">Its entity type, upper-case.</param>
/// <param name="Kind"><c>project</c>, <c>site</c>, <c>building</c>, <c>storey</c>, <c>space</c> or <c>other</c>.</param>
/// <param name="Parent">The container it decomposes.</param>
/// <param name="Children">Its sub-containers.</param>
/// <param name="Elements">Elements it contains.</param>
/// <param name="Referenced">Elements it only references.</param>
public sealed record SpatialNode(
    ulong Id,
    string? GlobalId,
    string? Name,
    string TypeName,
    string Kind,
    ulong? Parent,
    EquatableList<ulong> Children,
    EquatableList<ulong> Elements,
    EquatableList<ulong> Referenced);

/// <summary>A relationship naming an entity the file lacks.</summary>
/// <param name="Relation">The relationship.</param>
/// <param name="Target">The missing id it names.</param>
public sealed record SpatialDanglingReference(
    ulong Relation,
    ulong Target);

/// <summary>A relationship the reader could not honour.</summary>
/// <param name="Kind">What went wrong, e.g. a second parent.</param>
/// <param name="Relation">The relationship.</param>
/// <param name="Subject">The entity it concerns.</param>
/// <param name="Kept">The relationship kept instead, if any.</param>
public sealed record SpatialAnomaly(
    string Kind,
    ulong Relation,
    ulong Subject,
    ulong? Kept);
