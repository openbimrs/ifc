// Geometry (#328, #367, ADR 0021), mirroring the shared binding core:
// placements with the selected Body, the metadata of a graph, whose wire
// payload crosses separately (ProductGraph), and of a mesh, whose arrays
// cross separately (MeshedProduct).

namespace OpenBim.Ifc;

/// <summary>Why one product has no placement, representation, graph or mesh.</summary>
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

/// <summary>One product's geometry graph, without its payload (<see cref="ProductGraph"/> carries it).</summary>
/// <param name="Id">The product's entity id.</param>
/// <param name="GlobalId">Its <c>GlobalId</c>.</param>
/// <param name="TypeName">Its entity type, upper-case.</param>
/// <param name="Transform">The world placement, a column-major 4x4 in metres. The graph already has it applied: never apply it again. Null when refused.</param>
/// <param name="Encoding"><c>json</c> or <c>cbor</c>.</param>
/// <param name="PayloadSize">Bytes of the wire payload; 0 without one.</param>
/// <param name="Refusal">Why there is no graph. No payload and no refusal: a product with no Body representation.</param>
public sealed record ProductGeometry(
    ulong Id,
    string? GlobalId,
    string TypeName,
    EquatableList<double>? Transform,
    string Encoding,
    long PayloadSize,
    GeometryRefusal? Refusal);

/// <summary>The surface colour a mesh is painted with (#449): its representation item's <c>IfcStyledItem</c>, else its styled presentation layer, else its material's <c>IfcMaterialDefinitionRepresentation</c>.</summary>
/// <param name="Red">Red, in [0, 1].</param>
/// <param name="Green">Green, in [0, 1].</param>
/// <param name="Blue">Blue, in [0, 1].</param>
/// <param name="Transparency">In [0, 1], 0 opaque; null when the file states none, which the standard reads as opaque.</param>
/// <param name="Source">Which assignment supplied it, in that precedence: <c>item</c>, <c>layer</c> or <c>material</c>.</param>
/// <param name="SourceEntity">The <c>IfcStyledItem</c>, the layer assignment or the <c>IfcMaterial</c>.</param>
/// <param name="SurfaceStyle">The <c>IfcSurfaceStyle</c> the colour is read from.</param>
public sealed record SurfaceColour(
    double Red,
    double Green,
    double Blue,
    double? Transparency,
    string Source,
    ulong SourceEntity,
    ulong SurfaceStyle);

/// <summary>One product's mesh, without its arrays (<see cref="MeshedProduct"/> carries them).</summary>
/// <param name="Id">The product's entity id.</param>
/// <param name="GlobalId">Its <c>GlobalId</c>.</param>
/// <param name="TypeName">Its entity type, upper-case.</param>
/// <param name="Transform">The world placement, a column-major 4x4 in metres: a vertex's world position is this matrix applied to it. Null when refused.</param>
/// <param name="VertexCount">Vertices: a third of the positions.</param>
/// <param name="TriangleCount">Triangles: a third of the indices.</param>
/// <param name="Refusal">Why there is no mesh. No vertices and no refusal: a product with no Body representation.</param>
/// <param name="Colour">The surface colour the Body's items agree on; null when nothing colours the product or its colour is refused.</param>
/// <param name="ColourRefusal">Why the colour could not be resolved: <c>ambiguous</c> (two styled items or two colours at one level, a layer or profile set of several materials, a constituent set no shape aspect resolves, or Body items of different colours), or a shared geometry code. Never refuses the mesh itself.</param>
public sealed record ProductMesh(
    ulong Id,
    string? GlobalId,
    string TypeName,
    EquatableList<double>? Transform,
    long VertexCount,
    long TriangleCount,
    GeometryRefusal? Refusal,
    SurfaceColour? Colour,
    GeometryRefusal? ColourRefusal);
