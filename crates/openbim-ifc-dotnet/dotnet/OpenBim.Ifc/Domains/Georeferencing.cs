// Georeferencing (#123), mirroring the shared binding core.

namespace OpenBim.Ifc;

/// <summary>One coordinate operation, resolved with the project length unit.</summary>
/// <param name="Operation">The coordinate-operation entity.</param>
/// <param name="Kind"><c>map-conversion</c>, <c>map-conversion-scaled</c> or <c>rigid-operation</c>.</param>
/// <param name="Source"><c>SourceCRS</c>: a representation context or a CRS.</param>
/// <param name="SourceKind"><c>context</c> or <c>crs</c>.</param>
/// <param name="TargetCrs">The target <c>IfcProjectedCRS</c>.</param>
/// <param name="Eastings"><c>Eastings</c> as authored, in the map unit.</param>
/// <param name="Northings"><c>Northings</c> as authored, in the map unit.</param>
/// <param name="OrthogonalHeight"><c>OrthogonalHeight</c> as authored, in the map unit.</param>
/// <param name="XAxis"><c>(XAxisAbscissa, XAxisOrdinate)</c>, normalised.</param>
/// <param name="Scale"><c>Scale</c> as declared (1 when unset or for a rigid operation).</param>
/// <param name="Factors">IFC4X3 <c>(FactorX, FactorY, FactorZ)</c> of a scaled conversion.</param>
/// <param name="ProjectUnit">The project length unit the operation was resolved with.</param>
/// <param name="MapUnit">The map unit: the CRS's <c>MapUnit</c>, or the project unit when unset.</param>
/// <param name="MapUnitDeclared">Whether the CRS states <c>MapUnit</c> itself.</param>
/// <param name="Linear">Project metres to map metres as three columns, the images of the X, Y and Z axes.</param>
/// <param name="Translation">Where the project origin lands, in map metres.</param>
public sealed record MapConversion(
    ulong Operation,
    string Kind,
    ulong Source,
    string SourceKind,
    ProjectedCrs TargetCrs,
    double Eastings,
    double Northings,
    double OrthogonalHeight,
    EquatableList<double> XAxis,
    double Scale,
    EquatableList<double>? Factors,
    LengthUnit ProjectUnit,
    LengthUnit MapUnit,
    bool MapUnitDeclared,
    EquatableList<EquatableList<double>> Linear,
    EquatableList<double> Translation);

/// <summary>An <c>IfcProjectedCRS</c>.</summary>
/// <param name="Id">Its entity id.</param>
/// <param name="Name"><c>Name</c>, e.g. <c>EPSG:25832</c>.</param>
/// <param name="Description"><c>Description</c>.</param>
/// <param name="GeodeticDatum"><c>GeodeticDatum</c>.</param>
/// <param name="VerticalDatum"><c>VerticalDatum</c>.</param>
/// <param name="MapProjection"><c>MapProjection</c>.</param>
/// <param name="MapZone"><c>MapZone</c>.</param>
/// <param name="WellKnownText">IFC4X3 <c>WellKnownText</c>, when it defines the CRS.</param>
public sealed record ProjectedCrs(
    ulong Id,
    string? Name,
    string? Description,
    string? GeodeticDatum,
    string? VerticalDatum,
    string? MapProjection,
    string? MapZone,
    string? WellKnownText);

/// <summary>A length unit reduced to metres.</summary>
/// <param name="Name">The unit's name, e.g. <c>MILLI METRE</c>.</param>
/// <param name="MetresPerUnit">Metres per unit.</param>
public sealed record LengthUnit(
    string Name,
    double MetresPerUnit);
