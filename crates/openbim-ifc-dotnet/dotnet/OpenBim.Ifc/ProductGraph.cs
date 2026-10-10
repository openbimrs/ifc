namespace OpenBim.Ifc;

/// <summary>How <see cref="IfcModel.ProductGeometry"/> encodes each graph (#367).</summary>
public enum GeometryEncoding
{
    /// <summary>JSON text (RFC 8259), UTF-8: <see cref="ProductGraph.Json"/>.</summary>
    Json = 0,

    /// <summary>CBOR bytes (RFC 8949): smaller, and faster to decode.</summary>
    Cbor = 1,
}

/// <summary>One product's Body as Axiolid's neutral geometry graph (#367), from <see cref="IfcModel.ProductGeometry"/>.</summary>
/// <remarks>
/// The payload is Axiolid's versioned wire format 1.0:
/// <c>{"format":"axiolid-geometry-graph","version":"1.0","graph":{"nodes":[...],"roots":[...]}}</c>,
/// exact, in world coordinates, metres. The bytes are a copy the caller owns.
/// </remarks>
public sealed class ProductGraph
{
    /// <summary>The envelope's <c>format</c>.</summary>
    public const string Format = "axiolid-geometry-graph";

    /// <summary>The wire format version this build writes, the envelope's <c>version</c>.</summary>
    public const string FormatVersion = "1.0";

    internal ProductGraph(ProductGeometry geometry, byte[] payload)
    {
        Geometry = geometry;
        Payload = payload;
    }

    /// <summary>The product, its placement and its refusal, if any.</summary>
    public ProductGeometry Geometry { get; }

    /// <summary>The wire payload: UTF-8 JSON or CBOR bytes; empty without a graph.</summary>
    public byte[] Payload { get; }

    /// <summary>The payload as text for <see cref="GeometryEncoding.Json"/>; null for CBOR or without a graph.</summary>
    public string? Json =>
        Geometry.Encoding == "json" && Payload.Length > 0 ? System.Text.Encoding.UTF8.GetString(Payload) : null;
}
