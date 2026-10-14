namespace OpenBim.Ifc;

/// <summary>One product's curves as polylines (#458), from <see cref="IfcModel.ProductCurves"/>.</summary>
/// <remarks>The positions are a copy the caller owns: every polyline's vertices in turn, each <see cref="ProductPolyline"/> naming its slice, ready to draw as line strips.</remarks>
public sealed class CurvedProduct
{
    internal CurvedProduct(ProductCurves curves, float[] positions)
    {
        Curves = curves;
        Positions = positions;
    }

    /// <summary>The product, its placement, its polylines and its refusal, if any.</summary>
    public ProductCurves Curves { get; }

    /// <summary><c>x, y, z</c> per vertex, metres, relative to <see cref="ProductCurves.Transform"/>.</summary>
    public float[] Positions { get; }
}
