using System;
using System.Collections.Generic;
using System.Linq;
using OpenBim.Ifc.Native;

namespace OpenBim.Ifc;

/// <summary>
/// An IFC model: entities keyed by their <c>#id</c>, in file order, held by
/// the native library until <see cref="Dispose"/>.
/// </summary>
/// <remarks>
/// A refused call throws <see cref="IfcException"/> with a stable
/// <see cref="IfcException.Code"/> and leaves the model unchanged. Calls on
/// one model are serialised by the library, so a model may be shared between
/// threads; calls on different models run in parallel. A parsed model
/// decodes each entity the first time it is read: parsing checks every record
/// but builds nothing, so opening a large file is fast.
/// </remarks>
public sealed unsafe class IfcModel : IDisposable
{
    private readonly ModelHandle handle;

    private IfcModel(ModelHandle handle)
    {
        this.handle = handle;
    }

    /// <summary>An empty model.</summary>
    public IfcModel()
    {
        ulong model = 0;
        var status = NativeMethods.openbim_ifc_v0_1_model_create(&model);
        if (status != IfcStatus.Ok)
        {
            throw Calls.Error(status, null);
        }
        handle = ModelHandle.Own(model);
    }

    /// <summary>Parse a STEP (<c>.ifc</c>) file from its bytes, or an ifcZIP archive holding STEP or ifcXML, recognized by its ZIP signature and read within a decompression budget; <paramref name="options"/> relaxes the strict read.</summary>
    public static IfcModel Parse(byte[] data, ParseOptions? options = null)
    {
        if (data is null)
        {
            throw new ArgumentNullException(nameof(data));
        }
        var flags = (options ?? ParseOptions.Strict).Flags;
        var error = new byte[1024];
        ulong model = 0;
        IfcStatus status;
        fixed (byte* bytes = data)
        fixed (byte* message = error)
        {
            status = NativeMethods.openbim_ifc_v0_1_model_parse_with_options(bytes, (nuint)data.Length, flags, &model, message, (nuint)error.Length);
        }
        return Loaded(status, model, error);
    }

    /// <summary>Parse an ifcXML document: this library's lossless layout, or with <paramref name="xsdProfile"/> (<c>IFC4</c> or <c>IFC4X3_ADD2</c>) the buildingSMART XSD layout of that release.</summary>
    public static IfcModel ParseIfcXml(byte[] data, string? xsdProfile = null)
    {
        if (data is null)
        {
            throw new ArgumentNullException(nameof(data));
        }
        var profile = xsdProfile is null ? null : Calls.Encode(xsdProfile);
        var error = new byte[1024];
        ulong model = 0;
        IfcStatus status;
        fixed (byte* bytes = data)
        fixed (byte* name = profile)
        fixed (byte* message = error)
        {
            status = NativeMethods.openbim_ifc_v0_1_model_parse_ifcxml(bytes, (nuint)data.Length, name, (nuint)(profile?.Length ?? 0), &model, message, (nuint)error.Length);
        }
        return Loaded(status, model, error);
    }

    /// <summary>Read a STEP file from disk, once, straight into the model; <c>io</c> when it cannot be read.</summary>
    /// <remarks>
    /// With <paramref name="mapped"/> the file is memory-mapped instead: nothing
    /// is copied, and the model keeps decoding from the file while it lives, so
    /// the file must not be modified or truncated until the model is disposed.
    /// </remarks>
    public static IfcModel Open(string path, bool mapped = false, ParseOptions? options = null)
    {
        var bytes = Calls.Encode(path);
        var flags = (options ?? ParseOptions.Strict).Flags;
        var error = new byte[1024];
        ulong model = 0;
        IfcStatus status;
        fixed (byte* name = bytes)
        fixed (byte* message = error)
        {
            status = mapped
                ? NativeMethods.openbim_ifc_v0_1_model_open_mapped_with_options(name, (nuint)bytes.Length, flags, &model, message, (nuint)error.Length)
                : NativeMethods.openbim_ifc_v0_1_model_open_with_options(name, (nuint)bytes.Length, flags, &model, message, (nuint)error.Length);
        }
        return Loaded(status, model, error);
    }

    private static IfcModel Loaded(IfcStatus status, ulong model, byte[] error)
    {
        if (status != IfcStatus.Ok)
        {
            throw Calls.Error(status, Calls.ErrorText(error));
        }
        return new IfcModel(ModelHandle.Own(model));
    }

    /// <summary>Serialize as STEP bytes.</summary>
    public byte[] Write()
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        return Bytes(m, (buffer, capacity, required) => NativeMethods.openbim_ifc_v0_1_model_write(m, buffer, capacity, required));
    }

    /// <summary>Serialize as ifcXML bytes, in the layout <see cref="ParseIfcXml"/> reads; an XSD-layout write refuses with <c>write</c> what the layout cannot carry.</summary>
    public byte[] WriteIfcXml(string? xsdProfile = null)
    {
        var profile = xsdProfile is null ? null : Calls.Encode(xsdProfile);
        using var lease = handle.Acquire();
        var m = lease.Model;
        return Bytes(m, (buffer, capacity, required) =>
        {
            fixed (byte* name = profile)
            {
                return NativeMethods.openbim_ifc_v0_1_model_write_ifcxml(m, name, (nuint)(profile?.Length ?? 0), buffer, capacity, required);
            }
        });
    }

    /// <summary>Serialize as ifcZIP bytes: one Deflate entry holding STEP (<c>model.ifc</c>), or with <paramref name="inner"/> <c>ifcxml</c> ifcXML (<c>model.ifcXML</c>) in the layout <paramref name="xsdProfile"/> names, as for <see cref="WriteIfcXml"/>; <see cref="Parse"/> reads it back. Refuses with <c>invalid-value</c> another <paramref name="inner"/>, or a profile with STEP inside.</summary>
    public byte[] WriteIfcZip(string inner = "step", string? xsdProfile = null)
    {
        if (inner is null)
        {
            throw new ArgumentNullException(nameof(inner));
        }
        var format = Calls.Encode(inner);
        var profile = xsdProfile is null ? null : Calls.Encode(xsdProfile);
        using var lease = handle.Acquire();
        var m = lease.Model;
        return Bytes(m, (buffer, capacity, required) =>
        {
            fixed (byte* text = format)
            fixed (byte* name = profile)
            {
                return NativeMethods.openbim_ifc_v0_1_model_write_ifczip(m, text, (nuint)format.Length, name, (nuint)(profile?.Length ?? 0), buffer, capacity, required);
            }
        });
    }

    /// <summary>The STEP file header; assign a changed copy (<c>with</c>) to replace it.</summary>
    public Header Header
    {
        get
        {
            using var lease = handle.Acquire();
            var m = lease.Model;
            return RecordDecoder.Decode<Header>(Tape(m, (n, nc, nr, s, sc, sr) => NativeMethods.openbim_ifc_v0_1_model_header(m, n, nc, nr, s, sc, sr)).One());
        }
        set
        {
            if (value is null)
            {
                throw new ArgumentNullException(nameof(value));
            }
            var tape = new TapeWriter();
            tape.WriteListHead(10);
            Texts(tape, value.Description);
            tape.WriteString(Kind.Text, value.ImplementationLevel);
            tape.WriteString(Kind.Text, value.Name);
            tape.WriteString(Kind.Text, value.TimeStamp);
            Texts(tape, value.Author);
            Texts(tape, value.Organization);
            tape.WriteString(Kind.Text, value.PreprocessorVersion);
            tape.WriteString(Kind.Text, value.OriginatingSystem);
            tape.WriteString(Kind.Text, value.Authorization);
            Texts(tape, value.Schema);
            var nodes = tape.Nodes;
            var strings = tape.Strings;
            using var lease = handle.Acquire();
            fixed (ValueNode* n = nodes)
            fixed (byte* s = strings)
            {
                Calls.Check(lease.Model, NativeMethods.openbim_ifc_v0_1_model_set_header(lease.Model, n, (nuint)nodes.Length, s, (nuint)strings.Length));
            }
        }
    }

    private static void Texts(TapeWriter tape, EquatableList<string> texts)
    {
        tape.WriteListHead(texts.Count);
        foreach (var text in texts)
        {
            tape.WriteString(Kind.Text, text);
        }
    }

    /// <summary>Validate against the schema the header declares; findings sorted by severity, rule, entity and slot. <paramref name="maxFindings"/> caps the report (default 10,000).</summary>
    public ValidationReport Validate(int? maxFindings = null)
    {
        if (maxFindings is <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(maxFindings), "a finding budget is positive");
        }
        var max = (nuint)(maxFindings ?? 0);
        var summary = new NativeValidationSummary[1];
        using var lease = handle.Acquire();
        var m = lease.Model;
        var findings = Tape(m, (n, nc, nr, s, sc, sr) =>
        {
            fixed (NativeValidationSummary* counts = summary)
            {
                return NativeMethods.openbim_ifc_v0_1_model_validate(m, max, counts, n, nc, nr, s, sc, sr);
            }
        }).One();
        var c = summary[0];
        return new ValidationReport(
            c.Conformant != 0,
            c.Truncated != 0,
            (long)c.Errors,
            (long)c.EvaluationErrors,
            (long)c.Warnings,
            (long)c.Unsupported,
            RecordDecoder.DecodeList<ValidationFinding>(findings));
    }

    /// <summary>Products no viewer will draw, with a stable reason, in id order.</summary>
    public IReadOnlyList<UnreachableProduct> UnreachableProducts()
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        return RecordDecoder.DecodeList<UnreachableProduct>(Tape(m, (n, nc, nr, s, sc, sr) =>
        {
            nuint count;
            return NativeMethods.openbim_ifc_v0_1_model_unreachable_products(m, &count, n, nc, nr, s, sc, sr);
        }).One());
    }

    /// <summary>Each product's world placement (a column-major 4x4 in metres) and the Body representation a viewer draws, for <paramref name="ids"/> or, when null, every product with a shape, in id order (#328).</summary>
    /// <remarks>A product that cannot be placed is a record with a typed <see cref="ProductPlacement.Refusal"/>, not an exception.</remarks>
    public IReadOnlyList<ProductPlacement> ProductPlacements(IReadOnlyList<ulong>? ids = null)
    {
        var selection = ids?.ToArray();
        if (selection is { Length: 0 })
        {
            return Array.Empty<ProductPlacement>();
        }
        using var lease = handle.Acquire();
        var m = lease.Model;
        return RecordDecoder.DecodeList<ProductPlacement>(Tape(m, (n, nc, nr, s, sc, sr) =>
        {
            nuint count;
            fixed (ulong* p = selection)
            {
                return NativeMethods.openbim_ifc_v0_1_model_product_placements(m, p, (nuint)(selection?.Length ?? 0), &count, n, nc, nr, s, sc, sr);
            }
        }).One());
    }

    /// <summary>Each product's Body as Axiolid's neutral geometry graph (#367), for <paramref name="ids"/> or, when null, every product with a shape, in id order, encoded as <paramref name="encoding"/> in Axiolid's wire format (1.0; 1.1 when a station carries a seam-snapping window, 1.2 when a run's sections do, 1.3 when an offset run continues to a basis end the kernel measures, 1.4 when a polygonal bounded half-space clips).</summary>
    /// <remarks>Exact (extrusions, sweeps, B-splines, unevaluated booleans), in world coordinates, metres, for a host's own kernel. A product that cannot be lowered has a typed <see cref="OpenBim.Ifc.ProductGeometry.Refusal"/>. Needs a native library built with the <c>graph</c> feature; the packaged one throws <see cref="IfcException"/> with code <c>feature-disabled</c>.</remarks>
    public IReadOnlyList<ProductGraph> ProductGeometry(IReadOnlyList<ulong>? ids = null, GeometryEncoding encoding = GeometryEncoding.Json)
    {
        var selection = ids?.ToArray();
        if (selection is { Length: 0 })
        {
            return Array.Empty<ProductGraph>();
        }
        ulong graphs;
        using (var lease = handle.Acquire())
        {
            var m = lease.Model;
            fixed (ulong* p = selection)
            {
                Calls.Check(m, NativeMethods.openbim_ifc_v0_1_model_product_geometry(m, p, (nuint)(selection?.Length ?? 0), (uint)encoding, &graphs));
            }
        }
        // A copy for the lambda: a captured local cannot have its address taken.
        var set = graphs;
        try
        {
            var status = Calls.Tape((n, nc, nr, s, sc, sr) =>
            {
                nuint count;
                return NativeMethods.openbim_ifc_v0_1_graphs_records(set, &count, n, nc, nr, s, sc, sr);
            }, out var tape);
            if (status != IfcStatus.Ok)
            {
                throw Calls.Error(status, null);
            }
            var records = RecordDecoder.DecodeList<ProductGeometry>(tape.One());
            var result = new ProductGraph[records.Count];
            for (var i = 0; i < records.Count; i++)
            {
                result[i] = new ProductGraph(records[i], GraphPayload(set, i));
            }
            return result;
        }
        finally
        {
            NativeMethods.openbim_ifc_v0_1_graphs_destroy(set);
        }
    }

    private static byte[] GraphPayload(ulong graphs, int index)
    {
        nuint need;
        var status = NativeMethods.openbim_ifc_v0_1_graphs_payload(graphs, (nuint)index, null, 0, &need);
        if (status != IfcStatus.Ok && status != IfcStatus.BufferTooSmall)
        {
            throw Calls.Error(status, null);
        }
        var buffer = new byte[checked((int)need)];
        fixed (byte* p = buffer)
        {
            status = NativeMethods.openbim_ifc_v0_1_graphs_payload(graphs, (nuint)index, p, (nuint)buffer.Length, &need);
        }
        return status == IfcStatus.Ok ? buffer : throw Calls.Error(status, null);
    }

    /// <summary>Each product's Body as triangles from the reference backend, for <paramref name="ids"/> or, when null, every product with a shape, in id order (#328).</summary>
    /// <remarks>A product that cannot be meshed has a typed <see cref="ProductMesh.Refusal"/>. Needs a native library built with the <c>mesh</c> feature; the packaged one throws <see cref="IfcException"/> with code <c>feature-disabled</c>.</remarks>
    public IReadOnlyList<MeshedProduct> ProductMeshes(IReadOnlyList<ulong>? ids = null) => ProductMeshes(ids, "Body");

    /// <summary>The surfaces and solids of each product's <paramref name="representation"/> as one mesh (#458): <c>Body</c> as <see cref="ProductMeshes(IReadOnlyList{ulong})"/>, or <c>Surface</c>, <c>Plan</c>, <c>Axis</c>, <c>FootPrint</c>, <c>Annotation</c>.</summary>
    /// <remarks>Curves are left to <see cref="ProductCurves(IReadOnlyList{ulong}, string)"/>; <see cref="ProductMesh.Closure"/> says <c>solid</c> or <c>surface</c>. A product without the representation, or with curves only, has an empty mesh and no refusal; only a Body mesh carries a colour. Throws <c>invalid-value</c> for another name, and <c>feature-disabled</c> as the Body call does.</remarks>
    public IReadOnlyList<MeshedProduct> ProductMeshes(IReadOnlyList<ulong>? ids, string representation)
    {
        var name = Calls.Encode(representation ?? throw new ArgumentNullException(nameof(representation)));
        var selection = ids?.ToArray();
        if (selection is { Length: 0 })
        {
            return Array.Empty<MeshedProduct>();
        }
        ulong meshes;
        using (var lease = handle.Acquire())
        {
            var m = lease.Model;
            fixed (ulong* p = selection)
            fixed (byte* r = name)
            {
                Calls.Check(m, NativeMethods.openbim_ifc_v0_1_model_product_meshes_for(m, p, (nuint)(selection?.Length ?? 0), r, (nuint)name.Length, &meshes));
            }
        }
        // A copy for the lambda: a captured local cannot have its address taken.
        var set = meshes;
        try
        {
            var status = Calls.Tape((n, nc, nr, s, sc, sr) =>
            {
                nuint count;
                return NativeMethods.openbim_ifc_v0_1_meshes_records(set, &count, n, nc, nr, s, sc, sr);
            }, out var tape);
            if (status != IfcStatus.Ok)
            {
                throw Calls.Error(status, null);
            }
            var records = RecordDecoder.DecodeList<ProductMesh>(tape.One());
            var result = new MeshedProduct[records.Count];
            for (var i = 0; i < records.Count; i++)
            {
                result[i] = new MeshedProduct(records[i], MeshPositions(set, i), MeshIndices(set, i));
            }
            return result;
        }
        finally
        {
            NativeMethods.openbim_ifc_v0_1_meshes_destroy(set);
        }
    }

    /// <summary>The curves of each product's <paramref name="representation"/> as polylines from the reference backend (#458), for <paramref name="ids"/> or, when null, every product with a shape, in id order.</summary>
    /// <remarks><paramref name="representation"/> is <c>Plan</c> by default (the best drawing curves: <c>Plan</c>, <c>Annotation</c>, <c>FootPrint</c>, then <c>Axis</c>, preferring a <c>PLAN_VIEW</c> context), or <c>Axis</c>, <c>FootPrint</c>, <c>Annotation</c>, <c>Body</c>, <c>Surface</c>; another throws <c>invalid-value</c>. A product whose curves cannot be compiled has a typed <see cref="OpenBim.Ifc.ProductCurves.Refusal"/>: a curve the reference compiler has no certified polyline for (an implicit curve, an offset of an offset) is <c>unsupported</c>; trimmed, composite, offset and intrinsic curves compile (#506). Text literals and annotation fill areas beside the curves (a dimension's value) are listed in <see cref="OpenBim.Ifc.ProductCurves.Skipped"/>, not refused (#512). Needs a native library built with the <c>mesh</c> feature; the packaged one throws <c>feature-disabled</c>.</remarks>
    public IReadOnlyList<CurvedProduct> ProductCurves(IReadOnlyList<ulong>? ids = null, string representation = "Plan")
    {
        var name = Calls.Encode(representation ?? throw new ArgumentNullException(nameof(representation)));
        var selection = ids?.ToArray();
        if (selection is { Length: 0 })
        {
            return Array.Empty<CurvedProduct>();
        }
        ulong curves;
        using (var lease = handle.Acquire())
        {
            var m = lease.Model;
            fixed (ulong* p = selection)
            fixed (byte* r = name)
            {
                Calls.Check(m, NativeMethods.openbim_ifc_v0_1_model_product_curves(m, p, (nuint)(selection?.Length ?? 0), r, (nuint)name.Length, &curves));
            }
        }
        // A copy for the lambda: a captured local cannot have its address taken.
        var set = curves;
        try
        {
            var status = Calls.Tape((n, nc, nr, s, sc, sr) =>
            {
                nuint count;
                return NativeMethods.openbim_ifc_v0_1_curves_records(set, &count, n, nc, nr, s, sc, sr);
            }, out var tape);
            if (status != IfcStatus.Ok)
            {
                throw Calls.Error(status, null);
            }
            var records = RecordDecoder.DecodeList<ProductCurves>(tape.One());
            var result = new CurvedProduct[records.Count];
            for (var i = 0; i < records.Count; i++)
            {
                result[i] = new CurvedProduct(records[i], CurvePositions(set, i));
            }
            return result;
        }
        finally
        {
            NativeMethods.openbim_ifc_v0_1_curves_destroy(set);
        }
    }

    private static float[] CurvePositions(ulong curves, int index)
    {
        nuint need;
        var status = NativeMethods.openbim_ifc_v0_1_curves_positions(curves, (nuint)index, null, 0, &need);
        if (status != IfcStatus.Ok && status != IfcStatus.BufferTooSmall)
        {
            throw Calls.Error(status, null);
        }
        var buffer = new float[checked((int)need)];
        fixed (float* p = buffer)
        {
            status = NativeMethods.openbim_ifc_v0_1_curves_positions(curves, (nuint)index, p, (nuint)buffer.Length, &need);
        }
        return status == IfcStatus.Ok ? buffer : throw Calls.Error(status, null);
    }

    private static float[] MeshPositions(ulong meshes, int index)
    {
        nuint need;
        var status = NativeMethods.openbim_ifc_v0_1_meshes_positions(meshes, (nuint)index, null, 0, &need);
        if (status != IfcStatus.Ok && status != IfcStatus.BufferTooSmall)
        {
            throw Calls.Error(status, null);
        }
        var buffer = new float[checked((int)need)];
        fixed (float* p = buffer)
        {
            status = NativeMethods.openbim_ifc_v0_1_meshes_positions(meshes, (nuint)index, p, (nuint)buffer.Length, &need);
        }
        return status == IfcStatus.Ok ? buffer : throw Calls.Error(status, null);
    }

    private static uint[] MeshIndices(ulong meshes, int index)
    {
        nuint need;
        var status = NativeMethods.openbim_ifc_v0_1_meshes_indices(meshes, (nuint)index, null, 0, &need);
        if (status != IfcStatus.Ok && status != IfcStatus.BufferTooSmall)
        {
            throw Calls.Error(status, null);
        }
        var buffer = new uint[checked((int)need)];
        fixed (uint* p = buffer)
        {
            status = NativeMethods.openbim_ifc_v0_1_meshes_indices(meshes, (nuint)index, p, (nuint)buffer.Length, &need);
        }
        return status == IfcStatus.Ok ? buffer : throw Calls.Error(status, null);
    }

    /// <summary>The property sets, quantity sets and predefined property sets of object <paramref name="id"/>: its own first, then those its type object holds, an occurrence property overriding an inherited one of the same name.</summary>
    /// <remarks>Values keep their declared IFC type. Resolved against the release the header declares (IFC2X3, IFC4 or IFC4X3).</remarks>
    public IReadOnlyList<PropertySet> PropertySets(ulong id)
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        return RecordDecoder.DecodeList<PropertySet>(Tape(m, (n, nc, nr, s, sc, sr) =>
        {
            nuint count;
            return NativeMethods.openbim_ifc_v0_1_model_property_sets(m, id, &count, n, nc, nr, s, sc, sr);
        }).One());
    }

    /// <summary>
    /// <see cref="PropertySets"/> of each of <paramref name="ids"/>, in that
    /// order, or, when null, of every object definition
    /// (<c>IfcObjectDefinition</c> and its subtypes) in file order, in one
    /// pass (#358).
    /// </summary>
    /// <remarks>
    /// The file's property relationships are validated once for the call
    /// rather than once per object, so resolving every object is linear in
    /// the model. Each <see cref="ObjectPropertySets"/> holds exactly what
    /// <see cref="PropertySets"/> returns for its object, or, in
    /// <see cref="ObjectPropertySets.Refusal"/>, the code and message it
    /// throws; only a refusal of the whole model throws. No index outlives
    /// the call, so a call after an edit sees the edit.
    /// </remarks>
    public IReadOnlyList<ObjectPropertySets> PropertySetsMany(IReadOnlyList<ulong>? ids = null)
    {
        var selection = ids?.ToArray();
        if (selection is { Length: 0 })
        {
            return Array.Empty<ObjectPropertySets>();
        }
        using var lease = handle.Acquire();
        var m = lease.Model;
        return RecordDecoder.DecodeList<ObjectPropertySets>(Tape(m, (n, nc, nr, s, sc, sr) =>
        {
            nuint count;
            fixed (ulong* p = selection)
            {
                return NativeMethods.openbim_ifc_v0_1_model_property_sets_many(m, p, (nuint)(selection?.Length ?? 0), &count, n, nc, nr, s, sc, sr);
            }
        }).One());
    }

    /// <summary>The effective unit of a <paramref name="measureType"/> value (<c>IFCAREAMEASURE</c>): <paramref name="unit"/> when given (a property's stated unit), otherwise the project default, resolved exactly to SI.</summary>
    public ResolvedUnit ResolveUnit(string measureType, ulong? unit = null)
    {
        var measure = Calls.Encode(measureType);
        var stated = unit ?? 0;
        using var lease = handle.Acquire();
        var m = lease.Model;
        return RecordDecoder.Decode<ResolvedUnit>(Tape(m, (n, nc, nr, s, sc, sr) =>
        {
            fixed (byte* name = measure)
            {
                return NativeMethods.openbim_ifc_v0_1_model_resolve_unit(m, name, (nuint)measure.Length, stated, n, nc, nr, s, sc, sr);
            }
        }).One());
    }

    /// <summary>Write and remove property and quantity values as one checked transaction: every edit, in order, or none and the model unchanged.</summary>
    /// <remarks>
    /// Returns, per edit, the entity holding the property afterwards, or null
    /// when the batch leaves none. A value an occurrence inherits from its type
    /// is overridden on the occurrence, never changed on the shared type set;
    /// pass the type object's id to change that. Values are checked against the
    /// declared release and, for a <c>Pset_</c>/<c>Qto_</c> set, its PSD/QTO
    /// catalog template, which the library embeds.
    /// </remarks>
    public IReadOnlyList<ulong?> SetProperties(IEnumerable<PropertyEdit> edits)
    {
        var list = (edits ?? throw new ArgumentNullException(nameof(edits))).ToList();
        var tape = new TapeWriter();
        tape.WriteListHead(list.Count);
        foreach (var edit in list)
        {
            if (edit is null)
            {
                throw new ArgumentException("an edit is null", nameof(edits));
            }
            var fields = edit.Remove ? 4 : edit.Value is null ? 4 : 6;
            tape.WriteListHead(fields);
            tape.WriteString(Kind.Enum, edit.Remove ? "REMOVE" : "SET");
            tape.Write(new Value.Ref(edit.Object));
            tape.WriteString(Kind.Text, edit.Set);
            tape.WriteString(Kind.Text, edit.Name);
            if (fields == 6)
            {
                tape.Write(edit.Value!);
                if (edit.SetType is null)
                {
                    tape.Write(new Value.Null());
                }
                else
                {
                    tape.WriteString(Kind.Text, edit.SetType);
                }
            }
        }
        var nodes = tape.Nodes;
        var strings = tape.Strings;
        var properties = new ulong[list.Count];
        nuint count = 0;
        using var lease = handle.Acquire();
        fixed (ValueNode* n = nodes)
        fixed (byte* s = strings)
        fixed (ulong* ids = properties)
        {
            Calls.Check(lease.Model, NativeMethods.openbim_ifc_v0_1_model_set_properties(lease.Model, n, (nuint)nodes.Length, s, (nuint)strings.Length, ids, (nuint)properties.Length, &count));
        }
        return properties.Take(checked((int)count)).Select(id => id == 0 ? (ulong?)null : id).ToList();
    }

    /// <summary>The first id of the handle range (2^62): <c>HandleBase + i</c> names the entity operation <c>i</c> of an <see cref="Author"/> batch produced.</summary>
    public const ulong HandleBase = 4611686018427387904UL;

    /// <summary>The handle of the entity operation <paramref name="index"/> of an <see cref="Author"/> batch produces, usable wherever a later operation takes an id.</summary>
    public static ulong Handle(int index) =>
        index >= 0 ? HandleBase + (ulong)index : throw new ArgumentOutOfRangeException(nameof(index));

    /// <summary>Apply authoring operations as one checked transaction against the release the header declares: every operation, in order, or none and the model unchanged.</summary>
    /// <remarks>
    /// Returns, per operation, the id of the entity it produced, or null for a
    /// removal. An operation names the entity an earlier one produced by
    /// <see cref="Handle"/>. Refusals carry the shared codes:
    /// <c>unknown-attribute</c>, <c>derived-attribute</c>,
    /// <c>missing-attribute</c>, <c>invalid-value</c>, <c>wrong-entity-type</c>,
    /// <c>missing-entity</c>, <c>missing-reference</c>, <c>invalid-model</c>,
    /// <c>still-referenced</c>, <c>unsupported-schema</c>, and
    /// <c>unsupported</c> for a representation type check that needs a curve
    /// dimension it cannot derive.
    /// </remarks>
    public IReadOnlyList<ulong?> Author(IEnumerable<AuthorOp> ops)
    {
        var list = (ops ?? throw new ArgumentNullException(nameof(ops))).ToList();
        var tape = new TapeWriter();
        tape.WriteListHead(list.Count);
        foreach (var op in list)
        {
            (op ?? throw new ArgumentException("an operation is null", nameof(ops))).Write(tape);
        }
        var nodes = tape.Nodes;
        var strings = tape.Strings;
        var produced = new ulong[list.Count];
        nuint count = 0;
        using var lease = handle.Acquire();
        fixed (ValueNode* n = nodes)
        fixed (byte* s = strings)
        fixed (ulong* ids = produced)
        {
            Calls.Check(lease.Model, NativeMethods.openbim_ifc_v0_1_model_author(lease.Model, n, (nuint)nodes.Length, s, (nuint)strings.Length, ids, (nuint)produced.Length, &count));
        }
        return produced.Take(checked((int)count)).Select(id => id == 0 ? (ulong?)null : id).ToList();
    }

    /// <summary>Create one entity of <paramref name="type"/> from named attributes, checked against the declared release (<see cref="Author"/> with one <see cref="AuthorOp.Create"/>); returns its id.</summary>
    public ulong CreateEntity(string type, IEnumerable<KeyValuePair<string, Value>>? attributes = null)
    {
        var typeBytes = Calls.Encode(type);
        var pairs = (attributes ?? Array.Empty<KeyValuePair<string, Value>>())
            .Select(pair => (Value)new Value.List(new Value.Text(pair.Key), pair.Value));
        var tape = TapeWriter.Of(new Value.List(pairs));
        var nodes = tape.Nodes;
        var strings = tape.Strings;
        ulong id = 0;
        using var lease = handle.Acquire();
        fixed (byte* t = typeBytes)
        fixed (ValueNode* n = nodes)
        fixed (byte* s = strings)
        {
            Calls.Check(lease.Model, NativeMethods.openbim_ifc_v0_1_model_create_entity(lease.Model, t, (nuint)typeBytes.Length, n, (nuint)nodes.Length, s, (nuint)strings.Length, &id));
        }
        return id;
    }

    /// <summary>Remove entity <paramref name="id"/> with the relationships that reference it, leaving nothing dangling; refused with <c>still-referenced</c> while an entity other than a relationship needs it.</summary>
    public void RemoveWithRelationships(ulong id)
    {
        using var lease = handle.Acquire();
        Calls.Check(lease.Model, NativeMethods.openbim_ifc_v0_1_entity_remove_with_relationships(lease.Model, id));
    }

    /// <summary>Remove product (or type object) <paramref name="id"/> with everything it solely owns -- its placement, representation, own property and quantity sets, material usage, openings and ports, and the relationships left without an end -- as one checked transaction, keeping what is shared (a property set another object uses, a type's representation maps). With <paramref name="dryRun"/> nothing changes.</summary>
    /// <param name="id">The <c>IfcProduct</c> or <c>IfcTypeObject</c>.</param>
    /// <param name="rebasePlacements">Re-base placements relative to a removed placement onto its parent, composing the transforms, instead of refusing with <c>still-referenced</c>.</param>
    /// <param name="untypeOccurrences">Remove a type object's typing relationship, leaving its occurrences untyped, instead of refusing with <c>still-referenced</c>.</param>
    /// <param name="dryRun">Report only.</param>
    /// <returns>What went, what stayed and why.</returns>
    public RemovalReport RemoveProduct(ulong id, bool rebasePlacements = false, bool untypeOccurrences = false, bool dryRun = false)
    {
        uint flags = (rebasePlacements ? 1u : 0u) | (untypeOccurrences ? 2u : 0u) | (dryRun ? 4u : 0u);
        using var lease = handle.Acquire();
        var m = lease.Model;
        return RecordDecoder.Decode<RemovalReport>(Tape(m, (n, nc, nr, s, sc, sr) => NativeMethods.openbim_ifc_v0_1_model_remove_product(m, id, flags, n, nc, nr, s, sc, sr)).One());
    }

    /// <summary>The entities selector <paramref name="query"/> selects as a table (#474): the key column first (<c>GlobalId</c>, or <c>id</c> with <paramref name="key"/>), then one column per value path of <paramref name="columns"/> (<c>Name</c>, <c>type.Name</c>, <c>Pset_WallCommon.FireRating</c>), each cell the value the path reads as text. Refuses with <c>invalid-selector</c> a query or path that does not parse, <c>invalid-value</c> an unknown format or key, and <c>wrong-entity-type</c> an entity without a <c>GlobalId</c> in a table keyed by it.</summary>
    /// <param name="query">The selector choosing the rows.</param>
    /// <param name="columns">One value path per column.</param>
    /// <param name="format"><c>csv</c>, <c>tsv</c> or <c>xlsx</c>.</param>
    /// <param name="key"><c>GlobalId</c> (default) or <c>id</c>.</param>
    /// <param name="null">The cell text of <c>NULL</c>.</param>
    /// <returns>The table's bytes.</returns>
    public byte[] ExportTable(string query, IEnumerable<string> columns, string format = "csv", string? key = null, string @null = "")
    {
        if (query is null)
        {
            throw new ArgumentNullException(nameof(query));
        }
        var formatCode = TableFormat(format);
        var keyCode = TableKey(key);
        var text = Calls.Encode(query);
        var nullText = Calls.Encode(@null ?? "");
        var encoded = (columns ?? throw new ArgumentNullException(nameof(columns))).Select(Calls.Encode).ToArray();
        var joined = new byte[encoded.Sum(part => part.Length) + 1];
        var lengths = new nuint[encoded.Length + 1];
        var offsets = new int[encoded.Length];
        for (int i = 0, at = 0; i < encoded.Length; at += encoded[i].Length, i++)
        {
            Buffer.BlockCopy(encoded[i], 0, joined, at, encoded[i].Length);
            offsets[i] = at;
            lengths[i] = (nuint)encoded[i].Length;
        }
        var pointers = new IntPtr[encoded.Length + 1];
        var count = (nuint)encoded.Length;
        using var lease = handle.Acquire();
        var m = lease.Model;
        return Bytes(m, (buffer, capacity, required) =>
        {
            fixed (byte* q = text)
            fixed (byte* n = nullText)
            fixed (byte* start = joined)
            fixed (nuint* sizes = lengths)
            fixed (IntPtr* table = pointers)
            {
                for (var i = 0; i < offsets.Length; i++)
                {
                    pointers[i] = (IntPtr)(start + offsets[i]);
                }
                return NativeMethods.openbim_ifc_v0_1_model_export_table(m, q, (nuint)text.Length, (byte**)table, sizes, count, formatCode, keyCode, n, (nuint)nullText.Length, buffer, capacity, required);
            }
        });
    }

    /// <summary>Apply a table back to the model by its key column (#474), as one checked transaction: every other cell is written by its header's value path, typed by the attribute's declared type or the property's stated one; a cell that reads what the model holds changes nothing, so an exported table imported unchanged is a no-op. Any refusal means nothing was written, as does <paramref name="dryRun"/>.</summary>
    /// <param name="data">The table's bytes.</param>
    /// <param name="format"><c>csv</c>, <c>tsv</c> or <c>xlsx</c>.</param>
    /// <param name="key"><c>GlobalId</c> or <c>id</c>; by default <c>GlobalId</c> when the table has one, else <c>id</c>.</param>
    /// <param name="null">The cell text of <c>NULL</c>.</param>
    /// <param name="dryRun">Plan and report only.</param>
    /// <returns>Every change and every refusal, by row and column.</returns>
    public TableImport ImportTable(byte[] data, string format = "csv", string? key = null, string @null = "", bool dryRun = false)
    {
        if (data is null)
        {
            throw new ArgumentNullException(nameof(data));
        }
        var formatCode = TableFormat(format);
        var keyCode = TableKey(key);
        var nullText = Calls.Encode(@null ?? "");
        uint flags = dryRun ? 1u : 0u;
        using var lease = handle.Acquire();
        var m = lease.Model;
        return RecordDecoder.Decode<TableImport>(Tape(m, (n, nc, nr, s, sc, sr) =>
        {
            fixed (byte* bytes = data)
            fixed (byte* nullBytes = nullText)
            {
                return NativeMethods.openbim_ifc_v0_1_model_import_table(m, bytes, (nuint)data.Length, formatCode, keyCode, nullBytes, (nuint)nullText.Length, flags, n, nc, nr, s, sc, sr);
            }
        }).One());
    }

    private static uint TableFormat(string format) => (format ?? "").ToLowerInvariant() switch
    {
        "csv" => 0u,
        "tsv" => 1u,
        "xlsx" => 2u,
        _ => throw new IfcException("invalid-value", IfcStatus.InvalidValue, "\"" + format + "\" is no table format (csv, tsv, xlsx)"),
    };

    private static uint TableKey(string? key) => key?.ToLowerInvariant() switch
    {
        null => 0u,
        "globalid" or "guid" => 1u,
        "id" => 2u,
        _ => throw new IfcException("invalid-value", IfcStatus.InvalidValue, "\"" + key + "\" is no key column (GlobalId, id)"),
    };

    /// <summary>Remove unreferenced entities as one checked transaction. Without <paramref name="classes"/>, every resource entity no <c>IfcRoot</c> reaches (nor annotates something reached); with them, the entities of those classes (subtypes included) nothing references, an <c>IfcRoot</c> only with <paramref name="includeRoots"/>. The owner history stays unless <paramref name="includeOwnerHistory"/>.</summary>
    /// <param name="classes">The entity types to purge, any case; null or empty for the default purge.</param>
    /// <param name="includeRoots">Also purge unreferenced <c>IfcRoot</c>s of <paramref name="classes"/>.</param>
    /// <param name="includeOwnerHistory">Also purge an unreferenced owner history with its actors.</param>
    /// <param name="dryRun">Report only.</param>
    /// <returns>What went and what was skipped.</returns>
    public RemovalReport Purge(IEnumerable<string>? classes = null, bool includeRoots = false, bool includeOwnerHistory = false, bool dryRun = false)
    {
        uint flags = (includeRoots ? 1u : 0u) | (includeOwnerHistory ? 2u : 0u) | (dryRun ? 4u : 0u);
        var encoded = (classes ?? Array.Empty<string>()).Select(Calls.Encode).ToArray();
        var joined = new byte[encoded.Sum(part => part.Length) + 1];
        var lengths = new nuint[encoded.Length + 1];
        var offsets = new int[encoded.Length];
        for (int i = 0, at = 0; i < encoded.Length; at += encoded[i].Length, i++)
        {
            Buffer.BlockCopy(encoded[i], 0, joined, at, encoded[i].Length);
            offsets[i] = at;
            lengths[i] = (nuint)encoded[i].Length;
        }
        var pointers = new IntPtr[encoded.Length + 1];
        var count = (nuint)encoded.Length;
        using var lease = handle.Acquire();
        var m = lease.Model;
        return RecordDecoder.Decode<RemovalReport>(Tape(m, (n, nc, nr, s, sc, sr) =>
        {
            fixed (byte* start = joined)
            fixed (nuint* sizes = lengths)
            fixed (IntPtr* table = pointers)
            {
                for (var i = 0; i < offsets.Length; i++)
                {
                    pointers[i] = (IntPtr)(start + offsets[i]);
                }
                return NativeMethods.openbim_ifc_v0_1_model_purge(m, (byte**)table, sizes, count, flags, n, nc, nr, s, sc, sr);
            }
        }).One());
    }

    /// <summary>Convert the model's length unit to <paramref name="target"/> (#469): an SI <c>METRE</c> with any prefix (<c>MILLIMETRE</c>, <c>MILLI METRE</c>) or <c>inch</c>, <c>foot</c>, <c>yard</c>, <c>mile</c>, <c>US survey foot</c>. Every length, area and volume value the release declares as one is rescaled, as one checked transaction; values with a unit of their own keep it. With <paramref name="dryRun"/> nothing changes.</summary>
    /// <param name="target">The new length unit.</param>
    /// <param name="areaAndVolume">Replace the area and volume units by the square and cube of the new length unit, rescaling their values.</param>
    /// <param name="removeReplacedUnits">Remove the replaced unit entities when nothing else refers to them.</param>
    /// <param name="dryRun">Report only.</param>
    /// <returns>What was rescaled, kept, created and removed.</returns>
    /// <exception cref="IfcException"><c>invalid-value</c> for a target that is not a length unit; <c>unsupported</c> naming the values the conversion cannot carry (a parameter on a curve whose parameter space IFC leaves undefined, a p-curve off a plane, polynomial coefficients) or an offset unit (#110); <c>invalid-model</c> when the project length unit does not resolve.</exception>
    public LengthUnitReport ConvertLengthUnit(string target, bool areaAndVolume = true, bool removeReplacedUnits = true, bool dryRun = false)
    {
        uint flags = (areaAndVolume ? 0u : 1u) | (removeReplacedUnits ? 0u : 2u) | (dryRun ? 4u : 0u);
        var bytes = Calls.Encode(target);
        using var lease = handle.Acquire();
        var m = lease.Model;
        return RecordDecoder.Decode<LengthUnitReport>(Tape(m, (n, nc, nr, s, sc, sr) =>
        {
            fixed (byte* t = bytes)
            {
                return NativeMethods.openbim_ifc_v0_1_model_convert_length_unit(m, t, (nuint)bytes.Length, flags, n, nc, nr, s, sc, sr);
            }
        }).One());
    }

    /// <summary>Convert <paramref name="value"/> from unit entity <paramref name="from"/> to unit entity <paramref name="to"/> of this model (an <c>IfcSIUnit</c>, <c>IfcConversionBasedUnit</c> or <c>IfcDerivedUnit</c>).</summary>
    /// <exception cref="IfcException"><c>type-mismatch</c> for units of different dimensions; <c>unsupported</c> for a monetary, context-dependent or offset unit; <c>missing-entity</c>.</exception>
    public double ConvertUnitValue(double value, ulong from, ulong to)
    {
        double converted = 0;
        using var lease = handle.Acquire();
        Calls.Check(lease.Model, NativeMethods.openbim_ifc_v0_1_model_convert_unit_value(lease.Model, value, from, to, &converted));
        return converted;
    }

    /// <summary>Remove shape representation <paramref name="id"/> with what only it uses (#481), as one checked transaction: taken off its product definition shape (which goes when left empty), representation map or shape aspects, then removed with its items down to what nothing else uses. Shared items, profiles, points and styles stay. With <paramref name="dryRun"/> nothing changes.</summary>
    /// <param name="id">The <c>IfcShapeModel</c>; <c>wrong-entity-type</c> otherwise, <c>still-referenced</c> for a representation map an <c>IfcMappedItem</c> still maps.</param>
    /// <param name="dryRun">Report only.</param>
    /// <returns>What went, what stayed and why.</returns>
    public RemovalReport RemoveRepresentation(ulong id, bool dryRun = false)
    {
        uint flags = dryRun ? 4u : 0u;
        using var lease = handle.Acquire();
        var m = lease.Model;
        return RecordDecoder.Decode<RemovalReport>(Tape(m, (n, nc, nr, s, sc, sr) => NativeMethods.openbim_ifc_v0_1_model_remove_representation(m, id, flags, n, nc, nr, s, sc, sr)).One());
    }

    /// <summary>Copy product (or type object) <paramref name="id"/> with everything it solely owns, as one checked transaction (#479): what <see cref="RemoveProduct"/> would remove with it is copied -- its placement, representation, own property and quantity sets, material usage, ports and, by option, its openings and the doors and windows filling them -- and what it would keep is shared, the copy joining the relationship: its type, shared property sets, classification and, by option, its container and whole. With <paramref name="dryRun"/> nothing changes.</summary>
    /// <param name="id">The <c>IfcProduct</c> or <c>IfcTypeObject</c>.</param>
    /// <param name="offset">Move the copy by this vector, in its parent placement's frame.</param>
    /// <param name="placement">Place the copy by this <c>IfcObjectPlacement</c> instead; not with <paramref name="offset"/>.</param>
    /// <param name="openings">Copy the openings voiding the product.</param>
    /// <param name="fillings">Copy the doors and windows filling a copied opening.</param>
    /// <param name="container">Contain the copy in the original's spatial structure element.</param>
    /// <param name="aggregate">Make the copy a part of the original's whole.</param>
    /// <param name="dryRun">Report only.</param>
    /// <returns>Every original with its copy, the relationships joined and skipped.</returns>
    public CopyReport CopyProduct(
        ulong id,
        (double X, double Y, double Z)? offset = null,
        ulong? placement = null,
        bool openings = true,
        bool fillings = true,
        bool container = true,
        bool aggregate = true,
        bool dryRun = false)
    {
        uint flags = (openings ? 0u : 1u) | (fillings ? 0u : 2u) | (container ? 0u : 4u) | (aggregate ? 0u : 8u) | (dryRun ? 16u : 0u);
        var vector = offset is { } o ? new[] { o.X, o.Y, o.Z } : null;
        var at = placement ?? 0;
        using var lease = handle.Acquire();
        var m = lease.Model;
        return RecordDecoder.Decode<CopyReport>(Tape(m, (n, nc, nr, s, sc, sr) =>
        {
            fixed (double* v = vector)
            {
                return NativeMethods.openbim_ifc_v0_1_model_copy_product(m, id, at, v, flags, n, nc, nr, s, sc, sr);
            }
        }).One());
    }

    /// <summary>Copy entities <paramref name="ids"/> of <paramref name="source"/> into this model with what belongs to them, as one checked transaction (#479): what they reference, their property sets, type, material and classification, their openings, fillings and ports, and what attaches to their geometry. Resources equal to this model's are reused (<paramref name="reuse"/>); relationships to objects not copied (a spatial container) are skipped and reported. Refused with <c>unsupported</c> when the models' length or plane angle units differ, <c>unsupported-schema</c> for an entity this model's release cannot hold, <c>invalid-value</c> for a kept <c>GlobalId</c> this model holds.</summary>
    /// <param name="source">The model to copy from; not this one.</param>
    /// <param name="ids">The entities to copy.</param>
    /// <param name="globalIds">Keep, merge or regenerate the copies' <c>GlobalId</c>s.</param>
    /// <param name="reuse">The resources reused when this model holds an equal one.</param>
    /// <param name="dryRun">Report only.</param>
    /// <returns>Every source entity with its copy, the resources reused, the relationships skipped.</returns>
    public CopyReport Append(IfcModel source, IEnumerable<ulong> ids, GlobalIdMode globalIds = GlobalIdMode.Keep, ReuseKinds reuse = ReuseKinds.All, bool dryRun = false)
    {
        if (source is null)
        {
            throw new ArgumentNullException(nameof(source));
        }
        var roots = (ids ?? throw new ArgumentNullException(nameof(ids))).ToArray();
        uint flags = ((uint)~reuse & (uint)ReuseKinds.All) | (dryRun ? 32u : 0u);
        using var lease = handle.Acquire();
        using var from = source.handle.Acquire();
        var m = lease.Model;
        var other = from.Model;
        return RecordDecoder.Decode<CopyReport>(Tape(m, (n, nc, nr, s, sc, sr) =>
        {
            fixed (ulong* r = roots)
            {
                return NativeMethods.openbim_ifc_v0_1_model_append(m, other, r, (nuint)roots.Length, (uint)globalIds, flags, n, nc, nr, s, sc, sr);
            }
        }).One());
    }

    /// <summary>Write one value (<see cref="SetProperties"/> with one edit); returns the id of the entity holding it.</summary>
    public ulong SetProperty(ulong obj, string set, string name, Value value, string? setType = null)
    {
        var setBytes = Calls.Encode(set);
        var nameBytes = Calls.Encode(name);
        var typeBytes = setType is null ? null : Calls.Encode(setType);
        var tape = TapeWriter.Of(value);
        var nodes = tape.Nodes;
        var strings = tape.Strings;
        ulong id = 0;
        using var lease = handle.Acquire();
        fixed (byte* s = setBytes)
        fixed (byte* p = nameBytes)
        fixed (byte* t = typeBytes)
        fixed (ValueNode* n = nodes)
        fixed (byte* str = strings)
        {
            Calls.Check(lease.Model, NativeMethods.openbim_ifc_v0_1_model_set_property(
                lease.Model, obj, s, (nuint)setBytes.Length, p, (nuint)nameBytes.Length,
                n, (nuint)nodes.Length, str, (nuint)strings.Length, t, (nuint)(typeBytes?.Length ?? 0), &id));
        }
        return id;
    }

    /// <summary>Remove one property from <paramref name="obj"/>'s own set (<see cref="SetProperties"/> with one edit).</summary>
    public void RemoveProperty(ulong obj, string set, string name)
    {
        var setBytes = Calls.Encode(set);
        var nameBytes = Calls.Encode(name);
        using var lease = handle.Acquire();
        fixed (byte* s = setBytes)
        fixed (byte* p = nameBytes)
        {
            Calls.Check(lease.Model, NativeMethods.openbim_ifc_v0_1_model_remove_property(lease.Model, obj, s, (nuint)setBytes.Length, p, (nuint)nameBytes.Length));
        }
    }

    /// <summary>The spatial containment tree: every container with its parent, sub-containers and contained elements.</summary>
    public SpatialTree SpatialTree()
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        return RecordDecoder.Decode<SpatialTree>(Tape(m, (n, nc, nr, s, sc, sr) => NativeMethods.openbim_ifc_v0_1_model_spatial_tree(m, n, nc, nr, s, sc, sr)).One());
    }

    /// <summary>The classifications of object <paramref name="id"/>: its own, then its type's.</summary>
    public IReadOnlyList<Classification> Classifications(ulong id)
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        return RecordDecoder.DecodeList<Classification>(Tape(m, (n, nc, nr, s, sc, sr) =>
        {
            nuint count;
            return NativeMethods.openbim_ifc_v0_1_model_classifications(m, id, &count, n, nc, nr, s, sc, sr);
        }).One());
    }

    /// <summary>The material association of object <paramref name="id"/>, its own or its type's, or null.</summary>
    public MaterialAssignment? Material(ulong id)
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        var value = Tape(m, (n, nc, nr, s, sc, sr) => NativeMethods.openbim_ifc_v0_1_model_material(m, id, n, nc, nr, s, sc, sr)).One();
        return value is Value.Null ? null : RecordDecoder.Decode<MaterialAssignment>(value);
    }

    /// <summary>Every system with its members and served structures, and the memberships the reader could not honour.</summary>
    public SystemsView Systems()
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        return RecordDecoder.Decode<SystemsView>(Tape(m, (n, nc, nr, s, sc, sr) => NativeMethods.openbim_ifc_v0_1_model_systems(m, n, nc, nr, s, sc, sr)).One());
    }

    /// <summary>Every cost schedule and cost item; values as authored, typed.</summary>
    public Cost Cost()
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        return RecordDecoder.Decode<Cost>(Tape(m, (n, nc, nr, s, sc, sr) => NativeMethods.openbim_ifc_v0_1_model_cost(m, n, nc, nr, s, sc, sr)).One());
    }

    /// <summary>Every coordinate operation resolved with the project length unit; empty when the model has none. IFC2X3 is read from its <c>ePSet_MapConversion</c> and <c>ePSet_ProjectedCRS</c> property sets.</summary>
    public IReadOnlyList<MapConversion> Georeferencing()
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        return RecordDecoder.DecodeList<MapConversion>(Tape(m, (n, nc, nr, s, sc, sr) =>
        {
            nuint count;
            return NativeMethods.openbim_ifc_v0_1_model_georeferencing(m, &count, n, nc, nr, s, sc, sr);
        }).One());
    }

    /// <summary>Number of entities.</summary>
    public int Count
    {
        get
        {
            using var lease = handle.Acquire();
            nuint count = 0;
            Calls.Check(lease.Model, NativeMethods.openbim_ifc_v0_1_model_len(lease.Model, &count));
            return checked((int)count);
        }
    }

    /// <summary>The first <c>FILE_SCHEMA</c> token, e.g. <c>IFC4</c>, or null.</summary>
    public string? Schema
    {
        get
        {
            using var lease = handle.Acquire();
            var m = lease.Model;
            var status = Calls.String((buffer, capacity, required) => NativeMethods.openbim_ifc_v0_1_model_schema(m, buffer, capacity, required), out var schema);
            if (status == IfcStatus.NoValue)
            {
                return null;
            }
            Calls.Check(m, status);
            return schema;
        }
    }

    /// <summary>Non-fatal problems found while reading, such as the records a lenient read skipped.</summary>
    public IReadOnlyList<string> Diagnostics
    {
        get
        {
            using var lease = handle.Acquire();
            var m = lease.Model;
            nuint count = 0;
            Calls.Check(m, NativeMethods.openbim_ifc_v0_1_model_diagnostic_count(m, &count));
            var messages = new string[checked((int)count)];
            for (var i = 0; i < messages.Length; i++)
            {
                var index = (nuint)i;
                Calls.Check(m, Calls.String((buffer, capacity, required) => NativeMethods.openbim_ifc_v0_1_model_diagnostic(m, index, buffer, capacity, required), out messages[i]));
            }
            return messages;
        }
    }

    /// <summary>Every entity id, in file order.</summary>
    public IReadOnlyList<ulong> Ids()
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        return Ids(m, (buffer, capacity, required) => NativeMethods.openbim_ifc_v0_1_model_ids(m, buffer, capacity, required));
    }

    /// <summary>Ids of every entity of exactly <paramref name="typeName"/> (case-insensitive); subtypes are not included.</summary>
    public IReadOnlyList<ulong> IdsOfType(string typeName)
    {
        var type = Calls.Encode(typeName);
        using var lease = handle.Acquire();
        var m = lease.Model;
        return Ids(m, (buffer, capacity, required) =>
        {
            fixed (byte* name = type)
            {
                return NativeMethods.openbim_ifc_v0_1_model_ids_of_type(m, name, (nuint)type.Length, buffer, capacity, required);
            }
        });
    }

    /// <summary>Ids of <paramref name="typeName"/> or any subtype, per the file's schema; <c>unsupported-schema</c> when it is not bundled.</summary>
    public IReadOnlyList<ulong> IdsOfTypeIncludingSubtypes(string typeName)
    {
        var type = Calls.Encode(typeName);
        using var lease = handle.Acquire();
        var m = lease.Model;
        return Ids(m, (buffer, capacity, required) =>
        {
            fixed (byte* name = type)
            {
                return NativeMethods.openbim_ifc_v0_1_model_ids_of_type_including_subtypes(m, name, (nuint)type.Length, buffer, capacity, required);
            }
        });
    }

    /// <summary>The entity whose <c>GlobalId</c> is <paramref name="globalId"/>, or null (#463).</summary>
    /// <remarks>The first lookup indexes every <c>GlobalId</c> once; later ones reuse the index
    /// until the model changes. Throws <c>invalid-value</c> for text that is not a <c>GlobalId</c>
    /// and <c>invalid-model</c> when several entities carry it (<see cref="DuplicateGlobalIds"/>
    /// lists them).</remarks>
    public ulong? ByGuid(string globalId)
    {
        var text = Calls.Encode(globalId);
        using var lease = handle.Acquire();
        var m = lease.Model;
        ulong id = 0;
        IfcStatus status;
        fixed (byte* pointer = text)
        {
            status = NativeMethods.openbim_ifc_v0_1_model_by_guid(m, pointer, (nuint)text.Length, &id);
        }
        return Optional(m, status, id);
    }

    /// <summary>Every <c>GlobalId</c> several entities carry, with its holders in file order; empty for a conformant file.</summary>
    public IReadOnlyList<DuplicateGlobalId> DuplicateGlobalIds()
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        return RecordDecoder.DecodeList<DuplicateGlobalId>(Tape(m, (n, nc, nr, s, sc, sr) =>
        {
            nuint count;
            return NativeMethods.openbim_ifc_v0_1_model_duplicate_global_ids(m, &count, n, nc, nr, s, sc, sr);
        }).One());
    }

    /// <summary>The type object of occurrence <paramref name="id"/> (<c>IfcRelDefinesByType</c>), or null.</summary>
    /// <remarks>Throws <c>wrong-entity-type</c> for an entity that is not an <c>IfcObject</c> and <c>invalid-model</c> when the file states two types.</remarks>
    public ulong? TypeObject(ulong id)
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        ulong found = 0;
        var status = NativeMethods.openbim_ifc_v0_1_entity_type_object(m, id, &found);
        return Optional(m, status, found);
    }

    /// <summary>Every object type object <paramref name="id"/> types, in file order.</summary>
    public IReadOnlyList<ulong> OccurrencesOf(ulong id)
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        return Ids(m, (buffer, capacity, required) => NativeMethods.openbim_ifc_v0_1_entity_occurrences(m, id, buffer, capacity, required));
    }

    /// <summary>Every opening voiding element <paramref name="id"/> (<c>IfcRelVoidsElement</c>), in file order.</summary>
    public IReadOnlyList<ulong> OpeningsOf(ulong id)
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        return Ids(m, (buffer, capacity, required) => NativeMethods.openbim_ifc_v0_1_entity_openings(m, id, buffer, capacity, required));
    }

    /// <summary>Every element filling opening <paramref name="id"/> (<c>IfcRelFillsElement</c>), in file order.</summary>
    public IReadOnlyList<ulong> FillingsOf(ulong id)
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        return Ids(m, (buffer, capacity, required) => NativeMethods.openbim_ifc_v0_1_entity_fillings(m, id, buffer, capacity, required));
    }

    /// <summary>The element opening <paramref name="id"/> voids (<c>IfcRelVoidsElement</c>), or null.</summary>
    public ulong? VoidedElementOf(ulong id)
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        ulong found = 0;
        var status = NativeMethods.openbim_ifc_v0_1_entity_voided_element(m, id, &found);
        return Optional(m, status, found);
    }

    /// <summary>A new random <c>GlobalId</c>: an RFC 9562 version-4 UUID from the operating
    /// system's generator, in IFC's 22-character form (#463).</summary>
    public static string NewGuid()
    {
        byte* buffer = stackalloc byte[GuidBytes];
        nuint required = 0;
        return GuidText(NativeMethods.openbim_ifc_v0_1_new_guid(buffer, GuidBytes, &required), buffer);
    }

    /// <summary>The <c>GlobalId</c> named by <paramref name="parts"/> (RFC 9562 version 8): the
    /// same parts give the same <c>GlobalId</c> in every binding and release, for a reproducible
    /// export. Choose parts unique together, such as a tool name and a stable key.</summary>
    public static string NameBasedGuid(params string[] parts)
    {
        if (parts is null)
        {
            throw new ArgumentNullException(nameof(parts));
        }
        var encoded = parts.Select(Calls.Encode).ToArray();
        var joined = new byte[encoded.Sum(part => part.Length)];
        var lengths = new nuint[encoded.Length];
        var offsets = new int[encoded.Length];
        for (int i = 0, at = 0; i < encoded.Length; at += encoded[i].Length, i++)
        {
            Buffer.BlockCopy(encoded[i], 0, joined, at, encoded[i].Length);
            offsets[i] = at;
            lengths[i] = (nuint)encoded[i].Length;
        }
        var pointers = new IntPtr[encoded.Length];
        byte* buffer = stackalloc byte[GuidBytes];
        nuint required = 0;
        IfcStatus status;
        fixed (byte* start = joined)
        fixed (nuint* sizes = lengths)
        fixed (IntPtr* table = pointers)
        {
            for (var i = 0; i < pointers.Length; i++)
            {
                pointers[i] = (IntPtr)(start + offsets[i]);
            }
            status = NativeMethods.openbim_ifc_v0_1_name_based_guid((byte**)table, sizes, (nuint)pointers.Length, buffer, GuidBytes, &required);
        }
        return GuidText(status, buffer);
    }

    /// <summary>A <c>GlobalId</c>'s 22 characters and the NUL.</summary>
    private const int GuidBytes = 23;

    private static string GuidText(IfcStatus status, byte* buffer)
    {
        if (status != IfcStatus.Ok)
        {
            throw Calls.Error(status, null);
        }
        return Calls.Utf8.GetString(buffer, GuidBytes - 1);
    }

    /// <summary>An optional id: <see cref="IfcStatus.NoValue"/> is none, any other failure throws.</summary>
    private static ulong? Optional(ulong model, IfcStatus status, ulong id)
    {
        if (status == IfcStatus.NoValue)
        {
            return null;
        }
        Calls.Check(model, status);
        return id;
    }

    /// <summary>The version of the selector grammar this binding parses (#464).</summary>
    public const string SelectorGrammarVersion = "1.0";

    /// <summary>The ids selector <paramref name="query"/> selects, ascending (#464).</summary>
    /// <remarks>Class facets (subtypes included) add entities, <c>!</c> removes them; attribute,
    /// property and <c>type=</c>, <c>material=</c>, <c>classification=</c>, <c>location=</c> and
    /// <c>group=</c> facets filter; <c>,</c> chains facets and <c>+</c> unites lists, as in
    /// <c>IfcWall, Pset_WallCommon.IsExternal=TRUE + IfcSlab</c>. Throws <c>invalid-selector</c>,
    /// with <see cref="IfcException.Span"/>, for a query that does not parse or names a class or
    /// attribute the file's release does not declare.</remarks>
    public IReadOnlyList<ulong> Select(string query)
    {
        var text = Calls.Encode(query);
        using var lease = handle.Acquire();
        var m = lease.Model;
        return Ids(m, (buffer, capacity, required) =>
        {
            fixed (byte* q = text)
            {
                return NativeMethods.openbim_ifc_v0_1_model_select(m, q, (nuint)text.Length, buffer, capacity, required);
            }
        });
    }

    /// <summary>The ids selector <paramref name="query"/> selects from <paramref name="baseIds"/>
    /// alone, ascending; <c>missing-entity</c> for an id the model lacks.</summary>
    public IReadOnlyList<ulong> Select(string query, IEnumerable<ulong> baseIds)
    {
        var text = Calls.Encode(query);
        var ids = (baseIds ?? throw new ArgumentNullException(nameof(baseIds))).ToArray();
        using var lease = handle.Acquire();
        var m = lease.Model;
        return Ids(m, (buffer, capacity, required) =>
        {
            fixed (byte* q = text)
            fixed (ulong* b = ids)
            {
                return NativeMethods.openbim_ifc_v0_1_model_select_from(m, q, (nuint)text.Length, b, (nuint)ids.Length, buffer, capacity, required);
            }
        });
    }

    /// <summary>The value <paramref name="path"/> reads from entity <paramref name="id"/> (#464):
    /// <c>type.Name</c>, <c>Pset_WallCommon.FireRating</c>, <c>materials.Name</c>,
    /// <c>occurrences.count</c>.</summary>
    /// <remarks>Several values are a <see cref="Value.List"/>; a property set is a
    /// <see cref="Value.List"/> of two-item lists, each the member's name and its value.</remarks>
    public Value GetValue(ulong id, string path)
    {
        var bytes = Calls.Encode(path);
        using var lease = handle.Acquire();
        var m = lease.Model;
        return Tape(m, (n, nc, nr, s, sc, sr) =>
        {
            fixed (byte* text = bytes)
            {
                return NativeMethods.openbim_ifc_v0_1_entity_get_value(m, id, text, (nuint)bytes.Length, n, nc, nr, s, sc, sr);
            }
        }).One();
    }

    /// <summary>Write <paramref name="value"/> at <paramref name="path"/> of entity
    /// <paramref name="id"/> as one checked transaction (#464).</summary>
    /// <remarks>An attribute (<c>Name</c>, <c>type.Name</c>) takes a plain value coerced as
    /// <see cref="SetAttributeByNamePlain"/> coerces it; a property
    /// (<c>Pset_WallCommon.FireRating</c>) a typed <see cref="Value"/>, or a plain one when the
    /// property already states its type. A path that names no single value throws
    /// <c>invalid-value</c>; a refused write changes nothing.</remarks>
    public void SetValue(ulong id, string path, object? value)
    {
        var bytes = Calls.Encode(path);
        var tape = TapeWriter.OfPlain(value);
        var nodes = tape.Nodes;
        var strings = tape.Strings;
        using var lease = handle.Acquire();
        fixed (byte* text = bytes)
        fixed (ValueNode* n = nodes)
        fixed (byte* s = strings)
        {
            Calls.Check(lease.Model, NativeMethods.openbim_ifc_v0_1_entity_set_value(lease.Model, id, text, (nuint)bytes.Length, n, (nuint)nodes.Length, s, (nuint)strings.Length));
        }
    }

    /// <summary>The upper-case type name of entity <paramref name="id"/>.</summary>
    public string TypeOf(ulong id)
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        Calls.Check(m, Calls.String((buffer, capacity, required) => NativeMethods.openbim_ifc_v0_1_entity_type(m, id, buffer, capacity, required), out var type));
        return type;
    }

    /// <summary>Every attribute of entity <paramref name="id"/>, in declaration order.</summary>
    public IReadOnlyList<Value> Attributes(ulong id)
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        return Tape(m, (n, nc, nr, s, sc, sr) =>
        {
            nuint count;
            return NativeMethods.openbim_ifc_v0_1_entity_attributes(m, id, &count, n, nc, nr, s, sc, sr);
        }).All();
    }

    /// <summary>Attribute <paramref name="index"/> of entity <paramref name="id"/>; <see cref="Value.Null"/> past the end.</summary>
    public Value Attribute(ulong id, int index)
    {
        if (index < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(index));
        }
        var slot = (nuint)index;
        using var lease = handle.Acquire();
        var m = lease.Model;
        return Tape(m, (n, nc, nr, s, sc, sr) => NativeMethods.openbim_ifc_v0_1_entity_attribute(m, id, slot, n, nc, nr, s, sc, sr)).One();
    }

    /// <summary>Set attribute <paramref name="index"/> of entity <paramref name="id"/>, padding a gap past the end with <c>$</c>; returns the old value.</summary>
    public Value SetAttribute(ulong id, int index, Value value)
    {
        var old = Attribute(id, index);
        var tape = TapeWriter.Of(value);
        var nodes = tape.Nodes;
        var strings = tape.Strings;
        using var lease = handle.Acquire();
        fixed (ValueNode* n = nodes)
        fixed (byte* s = strings)
        {
            Calls.Check(lease.Model, NativeMethods.openbim_ifc_v0_1_entity_set_attribute(lease.Model, id, (nuint)index, n, (nuint)nodes.Length, s, (nuint)strings.Length));
        }
        return old;
    }

    /// <summary>Every explicit attribute of entity <paramref name="id"/> in slot order, inherited first, as the release the header declares defines them; <c>INVERSE</c> attributes hold no slot and are listed by <see cref="InverseAttributeNames"/>.</summary>
    public IReadOnlyList<AttributeInfo> AttributeNames(ulong id)
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        return RecordDecoder.DecodeList<AttributeInfo>(Tape(m, (n, nc, nr, s, sc, sr) =>
        {
            nuint count;
            return NativeMethods.openbim_ifc_v0_1_entity_attribute_names(m, id, &count, n, nc, nr, s, sc, sr);
        }).One());
    }

    /// <summary>Attribute <paramref name="name"/> of entity <paramref name="id"/>, matched case-insensitively (<c>Name</c>) and resolved against the declared release; <see cref="Value.Null"/> when the record stops before its slot.</summary>
    /// <remarks>An <c>INVERSE</c> name (<c>IsDefinedBy</c>, #462) reads the entities referencing <paramref name="id"/> through the release's <c>FOR</c> clause: a <see cref="Value.List"/> of <see cref="Value.Ref"/> in ascending id order for a <c>SET</c> or <c>BAG</c> (<c>SET [0:1]</c> included: zero or one element), a <see cref="Value.Ref"/> or <see cref="Value.Null"/> for a single-valued inverse, which two referrers refuse with <c>invalid-model</c>.</remarks>
    public Value AttributeByName(ulong id, string name)
    {
        var bytes = Calls.Encode(name);
        using var lease = handle.Acquire();
        var m = lease.Model;
        return Tape(m, (n, nc, nr, s, sc, sr) =>
        {
            fixed (byte* text = bytes)
            {
                return NativeMethods.openbim_ifc_v0_1_entity_attribute_by_name(m, id, text, (nuint)bytes.Length, n, nc, nr, s, sc, sr);
            }
        }).One();
    }

    /// <summary>Every <c>INVERSE</c> attribute of entity <paramref name="id"/>, inherited ones first, with its <c>FOR</c> clause and bounds, as the declared release defines them (#462).</summary>
    public IReadOnlyList<InverseAttributeInfo> InverseAttributeNames(ulong id)
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        return RecordDecoder.DecodeList<InverseAttributeInfo>(Tape(m, (n, nc, nr, s, sc, sr) =>
        {
            nuint count;
            return NativeMethods.openbim_ifc_v0_1_entity_inverse_attribute_names(m, id, &count, n, nc, nr, s, sc, sr);
        }).One());
    }

    /// <summary>Every reference to entity <paramref name="id"/>: the referencing entity and the attribute index holding the reference, in ascending order, one per entity and attribute (#462). IfcOpenShell's <c>get_inverse</c> with attribute indices.</summary>
    public IReadOnlyList<InverseReference> InverseReferences(ulong id)
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        return RecordDecoder.DecodeList<InverseReference>(Tape(m, (n, nc, nr, s, sc, sr) =>
        {
            nuint count;
            return NativeMethods.openbim_ifc_v0_1_entity_inverse_references(m, id, &count, n, nc, nr, s, sc, sr);
        }).One());
    }

    /// <summary>How many distinct entities reference entity <paramref name="id"/>, through any attribute (#462). IfcOpenShell's <c>get_total_inverses</c>.</summary>
    public int TotalInverses(ulong id)
    {
        using var lease = handle.Acquire();
        nuint count = 0;
        Calls.Check(lease.Model, NativeMethods.openbim_ifc_v0_1_entity_total_inverses(lease.Model, id, &count));
        return checked((int)count);
    }

    /// <summary>Set attribute <paramref name="name"/> of entity <paramref name="id"/>; returns the old value. A derived attribute is refused with <c>derived-attribute</c>, an <c>INVERSE</c> one with <c>inverse-attribute</c>, an unknown name with <c>unknown-attribute</c>, and a refused write changes nothing.</summary>
    public Value SetAttributeByName(ulong id, string name, Value value)
    {
        var old = AttributeByName(id, name);
        var bytes = Calls.Encode(name);
        var tape = TapeWriter.Of(value);
        var nodes = tape.Nodes;
        var strings = tape.Strings;
        using var lease = handle.Acquire();
        fixed (byte* text = bytes)
        fixed (ValueNode* n = nodes)
        fixed (byte* s = strings)
        {
            Calls.Check(lease.Model, NativeMethods.openbim_ifc_v0_1_entity_set_attribute_by_name(lease.Model, id, text, (nuint)bytes.Length, n, (nuint)nodes.Length, s, (nuint)strings.Length));
        }
        return old;
    }

    /// <summary>
    /// Set attribute <paramref name="name"/> of entity <paramref name="id"/>
    /// from a plain .NET value, coerced against the attribute's declared type
    /// in the declared release (#342); returns the old value.
    /// </summary>
    /// <remarks>
    /// A <c>string</c> becomes a label (written bare) or the enumeration item
    /// it names in any case; an integer an <c>INTEGER</c> or a <c>REAL</c> as
    /// declared; a <c>double</c> a <c>REAL</c>; a <c>bool</c> a
    /// <c>BOOLEAN</c> or <c>LOGICAL</c>; a sequence an aggregate, element by
    /// element; an <see cref="EntityHandle"/> a reference, checked to exist
    /// and to be of an accepted type; <c>null</c> <c>$</c>. In a SELECT the
    /// one member that takes the value is written as its typed parameter. A
    /// <see cref="Value"/>, nested in a sequence too, is written exactly.
    /// Refused with <c>type-mismatch</c> when the value does not fit and
    /// <c>ambiguous-value</c> when several SELECT members take it;
    /// otherwise as <see cref="SetAttributeByName"/>. A refused write
    /// changes nothing.
    /// </remarks>
    public Value SetAttributeByNamePlain(ulong id, string name, object? value)
    {
        var old = AttributeByName(id, name);
        var bytes = Calls.Encode(name);
        var tape = TapeWriter.OfPlain(value);
        var nodes = tape.Nodes;
        var strings = tape.Strings;
        using var lease = handle.Acquire();
        fixed (byte* text = bytes)
        fixed (ValueNode* n = nodes)
        fixed (byte* s = strings)
        {
            Calls.Check(lease.Model, NativeMethods.openbim_ifc_v0_1_entity_set_attribute_by_name_plain(lease.Model, id, text, (nuint)bytes.Length, n, (nuint)nodes.Length, s, (nuint)strings.Length));
        }
        return old;
    }

    /// <summary>Append an entity of <paramref name="typeName"/>; returns its new id.</summary>
    public ulong Add(string typeName, IEnumerable<Value> attributes)
    {
        var type = Calls.Encode(typeName);
        var tape = new TapeWriter();
        var count = 0;
        foreach (var attribute in attributes ?? throw new ArgumentNullException(nameof(attributes)))
        {
            tape.Write(attribute);
            count++;
        }
        var nodes = tape.Nodes;
        var strings = tape.Strings;
        ulong id = 0;
        using var lease = handle.Acquire();
        fixed (byte* name = type)
        fixed (ValueNode* n = nodes)
        fixed (byte* s = strings)
        {
            Calls.Check(lease.Model, NativeMethods.openbim_ifc_v0_1_entity_add(lease.Model, name, (nuint)type.Length, (nuint)count, n, (nuint)nodes.Length, s, (nuint)strings.Length, &id));
        }
        return id;
    }

    /// <summary>Remove entity <paramref name="id"/>, leaving references to it dangling.</summary>
    /// <remarks>Deprecated (#478): a file with dangling references parses and is wrong. Use <see cref="RemoveWithRelationships"/> for one entity, or <see cref="RemoveProduct"/> for a product with what it solely owns. It keeps working.</remarks>
    [Obsolete("Remove leaves references dangling; use RemoveWithRelationships or RemoveProduct.")]
    public void Remove(ulong id)
    {
        using var lease = handle.Acquire();
        Calls.Check(lease.Model, NativeMethods.openbim_ifc_v0_1_entity_remove(lease.Model, id));
    }

    /// <summary>Every reference to an id the model does not contain.</summary>
    public IReadOnlyList<DanglingReference> DanglingReferences()
    {
        using var lease = handle.Acquire();
        var m = lease.Model;
        var flat = Ids(m, (buffer, capacity, required) => NativeMethods.openbim_ifc_v0_1_model_dangling_references(m, buffer, capacity, required));
        var pairs = new DanglingReference[flat.Length / 2];
        for (var i = 0; i < pairs.Length; i++)
        {
            pairs[i] = new DanglingReference(flat[2 * i], flat[(2 * i) + 1]);
        }
        return pairs;
    }

    /// <summary>Whether <see cref="Dispose"/> has released the native model.</summary>
    public bool IsDisposed => handle.IsClosed;

    /// <summary>Release the native model. Every later call throws <see cref="ObjectDisposedException"/>.</summary>
    public void Dispose() => handle.Dispose();

    /// <inheritdoc/>
    public override string ToString() => IsDisposed ? "IfcModel(disposed)" : $"IfcModel(schema={Schema ?? "none"}, entities={Count})";

    private static byte[] Bytes(ulong model, BytesCall call)
    {
        var status = Calls.Bytes(call, 0, out var buffer, out var length);
        Calls.Check(model, status);
        if (length != buffer.Length)
        {
            Array.Resize(ref buffer, length);
        }
        return buffer;
    }

    private static ulong[] Ids(ulong model, IdsCall call)
    {
        Calls.Check(model, Calls.Ids(call, out var ids));
        return ids;
    }

    private static FilledTape Tape(ulong model, TapeCall call)
    {
        Calls.Check(model, Calls.Tape(call, out var tape));
        return tape;
    }
}
