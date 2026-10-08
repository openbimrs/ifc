namespace OpenBim.Ifc;

/// <summary>One product's Body as triangles (#328), from <see cref="IfcModel.ProductMeshes"/>.</summary>
/// <remarks>The arrays are copies the caller owns, ready for a vertex and an index buffer.</remarks>
public sealed class MeshedProduct
{
    internal MeshedProduct(ProductMesh mesh, float[] positions, uint[] indices)
    {
        Mesh = mesh;
        Positions = positions;
        Indices = indices;
    }

    /// <summary>The product, its placement and its refusal, if any.</summary>
    public ProductMesh Mesh { get; }

    /// <summary><c>x, y, z</c> per vertex, metres, relative to <see cref="ProductMesh.Transform"/>.</summary>
    public float[] Positions { get; }

    /// <summary>Three vertex indices per triangle.</summary>
    public uint[] Indices { get; }
}
