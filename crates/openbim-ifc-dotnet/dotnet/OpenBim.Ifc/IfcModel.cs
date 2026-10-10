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

    /// <summary>Parse a STEP (<c>.ifc</c>) file from its bytes; <paramref name="options"/> relaxes the strict read.</summary>
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

    /// <summary>Each product's Body as Axiolid's neutral geometry graph (#367), for <paramref name="ids"/> or, when null, every product with a shape, in id order, encoded as <paramref name="encoding"/> in Axiolid's wire format (1.0, or 1.1 when a station carries a seam-snapping window).</summary>
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
    public IReadOnlyList<MeshedProduct> ProductMeshes(IReadOnlyList<ulong>? ids = null)
    {
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
            {
                Calls.Check(m, NativeMethods.openbim_ifc_v0_1_model_product_meshes(m, p, (nuint)(selection?.Length ?? 0), &meshes));
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
    /// <c>still-referenced</c>, <c>unsupported-schema</c>.
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

    /// <summary>Every coordinate operation resolved with the project length unit; empty when the model has none.</summary>
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

    /// <summary>Every explicit attribute of entity <paramref name="id"/> in slot order, inherited first, as the release the header declares defines them; <c>INVERSE</c> attributes hold no slot and are not listed.</summary>
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

    /// <summary>Set attribute <paramref name="name"/> of entity <paramref name="id"/>; returns the old value. A derived attribute is refused with <c>derived-attribute</c>, an unknown name with <c>unknown-attribute</c>, and a refused write changes nothing.</summary>
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
