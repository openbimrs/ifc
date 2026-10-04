// Cost (#123), mirroring the shared binding core.

namespace OpenBim.Ifc;

/// <summary>Every cost schedule and cost item; values as authored, typed.</summary>
/// <param name="Schedules">The cost schedules.</param>
/// <param name="Items">The cost items.</param>
/// <param name="Anomalies">Assignments the reader could not honour.</param>
public sealed record Cost(
    EquatableList<CostSchedule> Schedules,
    EquatableList<CostItem> Items,
    EquatableList<CostAnomaly> Anomalies);

/// <summary>An <c>IfcCostSchedule</c>.</summary>
/// <param name="Id">Its entity id.</param>
/// <param name="GlobalId">Its <c>GlobalId</c>.</param>
/// <param name="Name">Its <c>Name</c>.</param>
/// <param name="Identification">Its <c>Identification</c>.</param>
/// <param name="Status">Its <c>Status</c>.</param>
/// <param name="PredefinedType">Its <c>PredefinedType</c>.</param>
/// <param name="Items">Its top-level cost items.</param>
public sealed record CostSchedule(
    ulong Id,
    string? GlobalId,
    string? Name,
    string? Identification,
    string? Status,
    string? PredefinedType,
    EquatableList<ulong> Items);

/// <summary>An <c>IfcCostItem</c>.</summary>
/// <param name="Id">Its entity id.</param>
/// <param name="GlobalId">Its <c>GlobalId</c>.</param>
/// <param name="Name">Its <c>Name</c>.</param>
/// <param name="Identification">Its <c>Identification</c>.</param>
/// <param name="Description">Its <c>Description</c>.</param>
/// <param name="PredefinedType">Its <c>PredefinedType</c>.</param>
/// <param name="Parent">The item it nests in.</param>
/// <param name="Children">The items nested in it.</param>
/// <param name="Values">Its <c>CostValues</c>.</param>
/// <param name="Quantities">Its <c>CostQuantities</c>.</param>
/// <param name="Objects">The objects assigned to it.</param>
public sealed record CostItem(
    ulong Id,
    string? GlobalId,
    string? Name,
    string? Identification,
    string? Description,
    string? PredefinedType,
    ulong? Parent,
    EquatableList<ulong> Children,
    EquatableList<CostValue> Values,
    EquatableList<ulong> Quantities,
    EquatableList<ulong> Objects);

/// <summary>An <c>IfcCostValue</c>, with its components.</summary>
/// <param name="Id">Its entity id.</param>
/// <param name="Name">Its <c>Name</c>.</param>
/// <param name="Description">Its <c>Description</c>.</param>
/// <param name="Category">Its <c>Category</c>.</param>
/// <param name="Condition">Its <c>Condition</c>.</param>
/// <param name="AppliedValue">Its <c>AppliedValue</c>, typed.</param>
/// <param name="Operator">Its <c>ArithmeticOperator</c>.</param>
/// <param name="UnitBasis">Its <c>UnitBasis</c>.</param>
/// <param name="Components">Its <c>Components</c>.</param>
public sealed record CostValue(
    ulong Id,
    string? Name,
    string? Description,
    string? Category,
    string? Condition,
    Value AppliedValue,
    string? Operator,
    UnitBasis? UnitBasis,
    EquatableList<CostValue> Components);

/// <summary>An <c>IfcMeasureWithUnit</c> a cost value is per.</summary>
/// <param name="Id">Its entity id.</param>
/// <param name="Value">Its <c>ValueComponent</c>, typed.</param>
/// <param name="Unit">Its <c>UnitComponent</c>.</param>
public sealed record UnitBasis(
    ulong Id,
    Value Value,
    ulong? Unit);

/// <summary>A cost item claimed by two relationships.</summary>
/// <param name="Item">The cost item.</param>
/// <param name="Kept">The relationship kept.</param>
/// <param name="Rejected">The relationship rejected.</param>
/// <param name="Relation">The relationship that raised it.</param>
public sealed record CostAnomaly(
    ulong Item,
    ulong Kept,
    ulong Rejected,
    ulong Relation);
