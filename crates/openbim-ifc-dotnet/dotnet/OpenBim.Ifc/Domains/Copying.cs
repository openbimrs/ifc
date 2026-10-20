// Copying a product with what it owns, and appending from another model
// (#479), mirroring the shared binding core.

using System;

namespace OpenBim.Ifc;

/// <summary>What <see cref="IfcModel.CopyProduct"/> or <see cref="IfcModel.Append"/> did, or, for a dry run, would do.</summary>
/// <param name="Copy">The product's copy; null for an append.</param>
/// <param name="Copied">Every original with its copy, in creation order.</param>
/// <param name="Reused">Append: source entities the target already holds, with the one used.</param>
/// <param name="Joined">Existing relationships and layers the copies joined.</param>
/// <param name="Skipped">Relationships of an original the copies do not get.</param>
/// <param name="Regenerated">Append: copies given a new <c>GlobalId</c>.</param>
/// <param name="Applied">False for a dry run.</param>
public sealed record CopyReport(
    ulong? Copy,
    EquatableList<CopiedEntity> Copied,
    EquatableList<CopiedEntity> Reused,
    EquatableList<ulong> Joined,
    EquatableList<ulong> Skipped,
    EquatableList<ulong> Regenerated,
    bool Applied);

/// <summary>An original and its copy (or, in <see cref="CopyReport.Reused"/>, the entity used for it).</summary>
/// <param name="Original">The original's id: in this model for a copy, in the source for an append.</param>
/// <param name="Copy">The copy's id.</param>
/// <param name="TypeName">Its type, upper case.</param>
public sealed record CopiedEntity(
    ulong Original,
    ulong Copy,
    string TypeName);

/// <summary>How <see cref="IfcModel.Append"/> treats the <c>GlobalId</c>s it copies.</summary>
public enum GlobalIdMode
{
    /// <summary>Keep every <c>GlobalId</c>; one the target holds is refused with <c>invalid-value</c>.</summary>
    Keep = 0,

    /// <summary>Give every copy a new <c>GlobalId</c>.</summary>
    Regenerate = 1,

    /// <summary>Give a new <c>GlobalId</c> only where the target holds the original's.</summary>
    RegenerateColliding = 2,

    /// <summary>An <c>IfcRoot</c> the target holds by <c>GlobalId</c> and type is that entity, reused, and a relationship the target holds is joined.</summary>
    Merge = 3,
}

/// <summary>The resources <see cref="IfcModel.Append"/> maps to an equal one the target holds instead of copying.</summary>
[Flags]
public enum ReuseKinds
{
    /// <summary>Copy every resource.</summary>
    None = 0,

    /// <summary>Units.</summary>
    Units = 1,

    /// <summary>Representation contexts.</summary>
    Contexts = 2,

    /// <summary>The owner history and its actors.</summary>
    OwnerHistory = 4,

    /// <summary>Material definitions.</summary>
    Materials = 8,

    /// <summary>Presentation styles.</summary>
    Styles = 16,

    /// <summary>Every kind: the default.</summary>
    All = Units | Contexts | OwnerHistory | Materials | Styles,
}
