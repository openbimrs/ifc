// Tables of values by path (#474), mirroring the shared binding core.

namespace OpenBim.Ifc;

/// <summary>What <see cref="IfcModel.ImportTable"/> planned, and whether it wrote it.</summary>
/// <param name="Applied">False for a dry run, a refused table, or one that changes nothing.</param>
/// <param name="Changes">Every value that changes, by row and column.</param>
/// <param name="Refusals">Every refusal; any refusal means nothing was written.</param>
public sealed record TableImport(
    bool Applied,
    EquatableList<TableChange> Changes,
    EquatableList<TableRefusal> Refusals);

/// <summary>One value <see cref="IfcModel.ImportTable"/> changes.</summary>
/// <param name="Row">One-based; the header is row 1.</param>
/// <param name="Column">One-based.</param>
/// <param name="Key">The row's key as the table gives it.</param>
/// <param name="Path">The column's value path.</param>
/// <param name="Entity">The row's entity.</param>
/// <param name="TargetEntity">The entity written, which a path such as <c>type.Name</c> finds elsewhere.</param>
/// <param name="Target">The attribute name or <c>Set.Property</c> written.</param>
/// <param name="Before">The value now, as cell text.</param>
/// <param name="After">The value then, as cell text.</param>
public sealed record TableChange(
    int Row,
    int Column,
    string Key,
    string Path,
    ulong Entity,
    ulong TargetEntity,
    string Target,
    string Before,
    string After);

/// <summary>One refused table, row or cell of <see cref="IfcModel.ImportTable"/>.</summary>
/// <param name="Row">One-based; null for the whole table.</param>
/// <param name="Column">One-based; null for a whole row or the table.</param>
/// <param name="Code"><c>malformed-table</c>, <c>invalid-header</c>, <c>duplicate-column</c>, <c>no-key-column</c>, <c>ragged-row</c>, <c>empty-key</c>, <c>unknown-key</c>, <c>duplicate-key</c>, <c>ambiguous-key</c>, or the error code of a refused write (<c>type-mismatch</c>, <c>invalid-value</c>, ...).</param>
/// <param name="Message">Why, in words.</param>
/// <param name="Span">The start and end byte of a header that does not parse, or null.</param>
public sealed record TableRefusal(
    int? Row,
    int? Column,
    string Code,
    string Message,
    EquatableList<int>? Span);
