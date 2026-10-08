// Geometry (#328, ADR 0021), mirroring the shared binding core: placements
// with the selected Body, and the metadata of a mesh, whose arrays cross
// separately (MeshedProduct).

namespace OpenBim.Ifc;

/// <summary>Why one product has no placement, representation or mesh.</summary>
/// <param name="Code"><c>unsupported</c>, <c>invalid-model</c>, <c>missing-reference</c> or <c>budget-exceeded</c>.</param>
/// <param name="Entity">The entity at fault, when the refusal names one.</param>
/// <param name="Message">A one-line explanation.</param>
public sealed record GeometryRefusal(
    string Code,
    ulong? Entity,
    string Message);

/// <summary>The <c>IfcShapeRepresentation</c> selected as a product's Body, and its context.</summary>
/// <param name="Id">The representation's entity id.</param>
/// <param name="Identifier"><c>RepresentationIdentifier</c>, e.g. <c>Body</c>.</param>
/// <param name="RepresentationType"><c>RepresentationType</c>, e.g. <c>SweptSolid</c>.</param>
/// <param name="Context"><c>ContextOfItems</c>.</param>
/// <param name="ContextType">The context's <c>ContextType</c>, e.g. <c>Model</c>.</param>
/// <param name="ContextIdentifier">The context's <c>ContextIdentifier</c>, e.g. <c>Body</c>.</param>
/// <param name="TargetView">A sub-context's <c>TargetView</c>, e.g. <c>MODEL_VIEW</c>.</param>
public sealed record SelectedRepresentation(
    ulong Id,
    string? Identifier,
    string? RepresentationType,
    ulong? Context,
    string? ContextType,
    string? ContextIdentifier,
    string? TargetView);

/// <summary>One product's world placement and selected Body representation.</summary>
/// <param name="Id">The product's entity id.</param>
/// <param name="GlobalId">Its <c>GlobalId</c>.</param>
/// <param name="TypeName">Its entity type, upper-case.</param>
/// <param name="Transform">The world placement, a column-major 4x4 in metres; null when refused.</param>
/// <param name="Representation">The Body; null for a product with no solid representation (an axis only).</param>
/// <param name="Refusal">Why the placement or the selection was refused.</param>
public sealed record ProductPlacement(
    ulong Id,
    string? GlobalId,
    string TypeName,
    EquatableList<double>? Transform,
    SelectedRepresentation? Representation,
    GeometryRefusal? Refusal);

/// <summary>One product's mesh, without its arrays (<see cref="MeshedProduct"/> carries them).</summary>
/// <param name="Id">The product's entity id.</param>
/// <param name="GlobalId">Its <c>GlobalId</c>.</param>
/// <param name="TypeName">Its entity type, upper-case.</param>
/// <param name="Transform">The world placement, a column-major 4x4 in metres: a vertex's world position is this matrix applied to it. Null when refused.</param>
/// <param name="VertexCount">Vertices: a third of the positions.</param>
/// <param name="TriangleCount">Triangles: a third of the indices.</param>
/// <param name="Refusal">Why there is no mesh. No vertices and no refusal: a product with no Body representation.</param>
public sealed record ProductMesh(
    ulong Id,
    string? GlobalId,
    string TypeName,
    EquatableList<double>? Transform,
    long VertexCount,
    long TriangleCount,
    GeometryRefusal? Refusal);
