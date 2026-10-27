// Undo and redo (#465), mirroring the shared binding core.

namespace OpenBim.Ifc;

/// <summary>How much history <see cref="IfcModel.History"/> reports.</summary>
/// <param name="Limit">The bound on undo entries; 0 while history is off.</param>
/// <param name="Undo">Entries <see cref="IfcModel.Undo"/> can step back through.</param>
/// <param name="Redo">Entries <see cref="IfcModel.Redo"/> can step forward through.</param>
public sealed record HistoryState(
    int Limit,
    int Undo,
    int Redo);

/// <summary>What <see cref="IfcModel.Undo"/> or <see cref="IfcModel.Redo"/> restored.</summary>
/// <param name="Created">Entities absent before, present after.</param>
/// <param name="Removed">Entities present before, absent after.</param>
/// <param name="Modified">Entities present both times whose content changed.</param>
/// <param name="Header">Whether the file header changed.</param>
public sealed record HistoryChange(
    EquatableList<ulong> Created,
    EquatableList<ulong> Removed,
    EquatableList<ulong> Modified,
    bool Header);
