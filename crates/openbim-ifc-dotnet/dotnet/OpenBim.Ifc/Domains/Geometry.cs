// Geometry (#328, #367, ADR 0021), mirroring the shared binding core:
// placements with the selected Body, the metadata of a graph, whose wire
// payload crosses separately (ProductGraph), of a mesh, whose arrays
// cross separately (MeshedProduct), and of a product's polylines (#458),
// whose positions cross separately (CurvedProduct).

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
/// <param name="Closure"><c>solid</c> when the mesh bounds a volume the file declared, <c>surface</c> for a surface (an area, no volume), <c>unknown</c> for an empty or refused mesh (#458).</param>
public sealed record ProductMesh(
    ulong Id,
    string? GlobalId,
    string TypeName,
    EquatableList<double>? Transform,
    long VertexCount,
    long TriangleCount,
    GeometryRefusal? Refusal,
    SurfaceColour? Colour,
    GeometryRefusal? ColourRefusal,
    string Closure);

/// <summary>One curve of a <see cref="ProductCurves"/>, as a slice of its positions (#458).</summary>
/// <param name="Curve">The IFC curve it was compiled from, when the lowering names one.</param>
/// <param name="FirstVertex">Its first vertex in the product's positions.</param>
/// <param name="VertexCount">How many vertices it has.</param>
/// <param name="Closed">Whether it is closed: its last vertex repeats its first.</param>
/// <param name="ChordBound">The certified bound, metres, on how far the exact curve lies from the polyline: 0 for a polyline the file authored.</param>
/// <param name="ChordRequested">The chord budget, metres: one millimetre.</param>
public sealed record ProductPolyline(
    ulong? Curve,
    long FirstVertex,
    long VertexCount,
    bool Closed,
    double ChordBound,
    double ChordRequested);

/// <summary>One product's curves, without their positions (<see cref="CurvedProduct"/> carries them).</summary>
/// <param name="Id">The product's entity id.</param>
/// <param name="GlobalId">Its <c>GlobalId</c>.</param>
/// <param name="TypeName">Its entity type, upper-case.</param>
/// <param name="Transform">The world placement, a column-major 4x4 in metres: a vertex's world position is this matrix applied to it. Null when refused.</param>
/// <param name="VertexCount">Vertices of every polyline together: a third of the positions.</param>
/// <param name="Polylines">The curves in authored order. Empty with no refusal: a product without the representation, or with no curves.</param>
/// <param name="Refusal">Why there are no polylines: a trimmed, composite, offset or intrinsic curve is <c>unsupported</c> until the reference compiler polylines it (#506).</param>
public sealed record ProductCurves(
    ulong Id,
    string? GlobalId,
    string TypeName,
    EquatableList<double>? Transform,
    long VertexCount,
    EquatableList<ProductPolyline> Polylines,
    GeometryRefusal? Refusal);
