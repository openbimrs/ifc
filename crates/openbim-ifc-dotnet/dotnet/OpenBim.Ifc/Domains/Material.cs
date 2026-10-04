// Materials (#123), mirroring the shared binding core.

namespace OpenBim.Ifc;

/// <summary>The material association of an object, its own or its type object's.</summary>
/// <param name="Relationship">The <c>IfcRelAssociatesMaterial</c>.</param>
/// <param name="GlobalId">The relationship's <c>GlobalId</c>.</param>
/// <param name="Source"><c>occurrence</c> or <c>type</c>.</param>
/// <param name="TypeObject">For <c>type</c>, the type object.</param>
/// <param name="Target">The associated material definition.</param>
/// <param name="TypeName">Its entity type, upper-case.</param>
/// <param name="Kind"><c>material</c>, <c>layer-set</c>, <c>profile-set</c>, <c>constituent-set</c>, <c>list</c>, ...</param>
/// <param name="Set">The layer, profile or constituent set, when there is one.</param>
/// <param name="Name">The set's name.</param>
/// <param name="Materials">The materials, for a single material or a list.</param>
/// <param name="Layers">The layers of a layer set.</param>
/// <param name="Profiles">The profiles of a profile set.</param>
/// <param name="Constituents">The constituents of a constituent set.</param>
/// <param name="Usage">The layer or profile set usage.</param>
public sealed record MaterialAssignment(
    ulong Relationship,
    string? GlobalId,
    string Source,
    ulong? TypeObject,
    ulong Target,
    string TypeName,
    string Kind,
    ulong? Set,
    string? Name,
    EquatableList<MaterialRef> Materials,
    EquatableList<MaterialLayer> Layers,
    EquatableList<MaterialProfile> Profiles,
    EquatableList<MaterialConstituent> Constituents,
    MaterialUsage? Usage);

/// <summary>An <c>IfcMaterial</c>.</summary>
/// <param name="Id">Its entity id.</param>
/// <param name="Name">Its <c>Name</c>.</param>
/// <param name="Category">Its <c>Category</c>.</param>
public sealed record MaterialRef(
    ulong Id,
    string Name,
    string? Category);

/// <summary>An <c>IfcMaterialLayer</c>.</summary>
/// <param name="Id">Its entity id.</param>
/// <param name="Material">Its material, if any.</param>
/// <param name="Thickness">Its thickness in metres.</param>
/// <param name="IsVentilated"><c>IsVentilated</c> as authored: a logical or <c>$</c>.</param>
/// <param name="Name">Its <c>Name</c>.</param>
/// <param name="Category">Its <c>Category</c>.</param>
/// <param name="Priority">Its <c>Priority</c>.</param>
public sealed record MaterialLayer(
    ulong Id,
    MaterialRef? Material,
    double Thickness,
    Value IsVentilated,
    string? Name,
    string? Category,
    long? Priority);

/// <summary>An <c>IfcMaterialProfile</c>.</summary>
/// <param name="Id">Its entity id.</param>
/// <param name="Material">Its material, if any.</param>
/// <param name="Profile">Its profile definition.</param>
/// <param name="Name">Its <c>Name</c>.</param>
/// <param name="Category">Its <c>Category</c>.</param>
/// <param name="Priority">Its <c>Priority</c>.</param>
public sealed record MaterialProfile(
    ulong Id,
    MaterialRef? Material,
    ulong Profile,
    string? Name,
    string? Category,
    long? Priority);

/// <summary>An <c>IfcMaterialConstituent</c>.</summary>
/// <param name="Id">Its entity id.</param>
/// <param name="Material">Its material.</param>
/// <param name="Name">Its <c>Name</c>.</param>
/// <param name="Category">Its <c>Category</c>.</param>
/// <param name="Fraction">Its <c>Fraction</c>.</param>
public sealed record MaterialConstituent(
    ulong Id,
    MaterialRef Material,
    string? Name,
    string? Category,
    double? Fraction);

/// <summary>An <c>IfcMaterialLayerSetUsage</c> or <c>IfcMaterialProfileSetUsage</c>.</summary>
/// <param name="LayerSetDirection">A layer set usage's <c>LayerSetDirection</c>.</param>
/// <param name="DirectionSense">A layer set usage's <c>DirectionSense</c>.</param>
/// <param name="OffsetFromReferenceLine">A layer set usage's offset, in metres.</param>
/// <param name="ReferenceExtent">A layer set usage's <c>ReferenceExtent</c>, in metres.</param>
/// <param name="CardinalPoint">A profile set usage's <c>CardinalPoint</c>.</param>
/// <param name="EndSet">A tapering usage's end profile set.</param>
/// <param name="CardinalEndPoint">A tapering usage's <c>CardinalEndPoint</c>.</param>
public sealed record MaterialUsage(
    string? LayerSetDirection,
    string? DirectionSense,
    double? OffsetFromReferenceLine,
    double? ReferenceExtent,
    long? CardinalPoint,
    ulong? EndSet,
    long? CardinalEndPoint);
