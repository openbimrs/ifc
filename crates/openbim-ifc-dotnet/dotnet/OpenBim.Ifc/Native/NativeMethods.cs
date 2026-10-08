// P/Invoke declarations for the openbim-ifc C ABI v0.1 (openbim_ifc.h).
//
// One declaration per export, named exactly as in the header, with blittable
// parameters only: pointers, integers and the two structs below. Nothing is
// marshalled, so every call is a plain native call. The crate's
// `tests/conformance.rs` holds this file to the header: every function and
// status code declared, each with the header's parameter count.

using System.Runtime.InteropServices;

namespace OpenBim.Ifc.Native;

/// <summary>One node of a value tape (<c>OpenbimIfcValueNode</c>).</summary>
[StructLayout(LayoutKind.Sequential)]
internal struct ValueNode
{
    public int Kind;
    public uint ChildCount;
    public long IntValue;
    public double RealValue;
    public ulong StrOffset;
    public ulong StrLen;
}

/// <summary>Counts from one validation run (<c>OpenbimIfcValidationSummary</c>).</summary>
[StructLayout(LayoutKind.Sequential)]
internal struct NativeValidationSummary
{
    public nuint FindingCount;
    public nuint Errors;
    public nuint EvaluationErrors;
    public nuint Warnings;
    public nuint Unsupported;
    public uint Conformant;
    public uint Truncated;
}

/// <summary>ABI and crate versions (<c>OpenbimIfcVersion</c>).</summary>
[StructLayout(LayoutKind.Sequential)]
internal struct NativeVersion
{
    public ushort AbiMajor;
    public ushort AbiMinor;
    public ushort AbiPatch;
    public ushort CrateMajor;
    public ushort CrateMinor;
    public ushort CratePatch;
}

/// <summary>The <c>OPENBIM_IFC_KIND_*</c> codes of a tape node.</summary>
internal static class Kind
{
    public const int Null = 0;
    public const int Derived = 1;
    public const int Bool = 2;
    public const int Unknown = 3;
    public const int Integer = 4;
    public const int Real = 5;
    public const int Text = 6;
    public const int Binary = 7;
    public const int Enum = 8;
    public const int Ref = 9;
    public const int List = 10;
    public const int Typed = 11;
}

/// <summary>The <c>OPENBIM_IFC_PARSE_*</c> flag bits.</summary>
internal static class ParseFlags
{
    public const uint SkipMalformed = 1;
    public const uint CheckReferences = 2;
    public const uint AcceptRealWithoutPoint = 4;
}

internal static unsafe partial class NativeMethods
{
    /// <summary>The C ABI's shared library: <c>openbim_ifc_capi.dll</c>,
    /// <c>libopenbim_ifc_capi.so</c> or <c>libopenbim_ifc_capi.dylib</c>.</summary>
    internal const string Library = "openbim_ifc_capi";

    static NativeMethods() => NativeLibraryLoader.Register();

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_entity_add(ulong model, byte* type_name, nuint type_len, nuint attribute_count, ValueNode* nodes, nuint node_count, byte* strings, nuint string_len, ulong* out_id);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_entity_attribute(ulong model, ulong id, nuint index, ValueNode* nodes, nuint node_capacity, nuint* out_nodes_required, byte* strings, nuint string_capacity, nuint* out_strings_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_entity_attribute_by_name(ulong model, ulong id, byte* name, nuint name_len, ValueNode* nodes, nuint node_capacity, nuint* out_nodes_required, byte* strings, nuint string_capacity, nuint* out_strings_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_entity_attribute_names(ulong model, ulong id, nuint* out_count, ValueNode* nodes, nuint node_capacity, nuint* out_nodes_required, byte* strings, nuint string_capacity, nuint* out_strings_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_entity_attributes(ulong model, ulong id, nuint* out_count, ValueNode* nodes, nuint node_capacity, nuint* out_nodes_required, byte* strings, nuint string_capacity, nuint* out_strings_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_entity_remove(ulong model, ulong id);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_entity_remove_with_relationships(ulong model, ulong id);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_entity_set_attribute(ulong model, ulong id, nuint index, ValueNode* nodes, nuint node_count, byte* strings, nuint string_len);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_entity_set_attribute_by_name(ulong model, ulong id, byte* name, nuint name_len, ValueNode* nodes, nuint node_count, byte* strings, nuint string_len);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_entity_type(ulong model, ulong id, byte* buffer, nuint capacity, nuint* out_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_last_error_code(ulong model, byte* buffer, nuint capacity, nuint* out_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_last_error_message(ulong model, byte* buffer, nuint capacity, nuint* out_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_live_models(nuint* out_count);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_meshes_destroy(ulong meshes);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_meshes_indices(ulong meshes, nuint index, uint* buffer, nuint capacity, nuint* out_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_meshes_positions(ulong meshes, nuint index, float* buffer, nuint capacity, nuint* out_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_meshes_records(ulong meshes, nuint* out_count, ValueNode* nodes, nuint node_capacity, nuint* out_nodes_required, byte* strings, nuint string_capacity, nuint* out_strings_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_classifications(ulong model, ulong @object, nuint* out_count, ValueNode* nodes, nuint node_capacity, nuint* out_nodes_required, byte* strings, nuint string_capacity, nuint* out_strings_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_cost(ulong model, ValueNode* nodes, nuint node_capacity, nuint* out_nodes_required, byte* strings, nuint string_capacity, nuint* out_strings_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_create(ulong* out_model);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_dangling_references(ulong model, ulong* buffer, nuint capacity, nuint* out_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_destroy(ulong model);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_diagnostic(ulong model, nuint index, byte* buffer, nuint capacity, nuint* out_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_diagnostic_count(ulong model, nuint* out_count);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_georeferencing(ulong model, nuint* out_count, ValueNode* nodes, nuint node_capacity, nuint* out_nodes_required, byte* strings, nuint string_capacity, nuint* out_strings_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_header(ulong model, ValueNode* nodes, nuint node_capacity, nuint* out_nodes_required, byte* strings, nuint string_capacity, nuint* out_strings_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_ids(ulong model, ulong* buffer, nuint capacity, nuint* out_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_ids_of_type(ulong model, byte* type_name, nuint type_len, ulong* buffer, nuint capacity, nuint* out_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_ids_of_type_including_subtypes(ulong model, byte* type_name, nuint type_len, ulong* buffer, nuint capacity, nuint* out_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_len(ulong model, nuint* out_count);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_material(ulong model, ulong @object, ValueNode* nodes, nuint node_capacity, nuint* out_nodes_required, byte* strings, nuint string_capacity, nuint* out_strings_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_open(byte* path, nuint path_len, ulong* out_model, byte* error_buffer, nuint capacity);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_open_mapped(byte* path, nuint path_len, ulong* out_model, byte* error_buffer, nuint capacity);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_open_mapped_with_options(byte* path, nuint path_len, uint flags, ulong* out_model, byte* error_buffer, nuint capacity);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_open_with_options(byte* path, nuint path_len, uint flags, ulong* out_model, byte* error_buffer, nuint capacity);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_parse(byte* data, nuint len, ulong* out_model, byte* error_buffer, nuint capacity);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_parse_ifcxml(byte* data, nuint len, byte* profile, nuint profile_len, ulong* out_model, byte* error_buffer, nuint capacity);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_parse_with_options(byte* data, nuint len, uint flags, ulong* out_model, byte* error_buffer, nuint capacity);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_product_meshes(ulong model, ulong* ids, nuint id_count, ulong* out_meshes);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_product_placements(ulong model, ulong* ids, nuint id_count, nuint* out_count, ValueNode* nodes, nuint node_capacity, nuint* out_nodes_required, byte* strings, nuint string_capacity, nuint* out_strings_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_property_sets(ulong model, ulong @object, nuint* out_count, ValueNode* nodes, nuint node_capacity, nuint* out_nodes_required, byte* strings, nuint string_capacity, nuint* out_strings_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_remove_property(ulong model, ulong @object, byte* set, nuint set_len, byte* name, nuint name_len);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_resolve_unit(ulong model, byte* measure_type, nuint measure_len, ulong unit, ValueNode* nodes, nuint node_capacity, nuint* out_nodes_required, byte* strings, nuint string_capacity, nuint* out_strings_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_schema(ulong model, byte* buffer, nuint capacity, nuint* out_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_set_header(ulong model, ValueNode* nodes, nuint node_count, byte* strings, nuint string_len);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_author(ulong model, ValueNode* nodes, nuint node_count, byte* strings, nuint string_len, ulong* out_ids, nuint ids_capacity, nuint* out_count);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_create_entity(ulong model, byte* type_name, nuint type_len, ValueNode* nodes, nuint node_count, byte* strings, nuint string_len, ulong* out_id);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_set_properties(ulong model, ValueNode* nodes, nuint node_count, byte* strings, nuint string_len, ulong* out_properties, nuint properties_capacity, nuint* out_count);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_set_property(ulong model, ulong @object, byte* set, nuint set_len, byte* name, nuint name_len, ValueNode* nodes, nuint node_count, byte* strings, nuint string_len, byte* set_type, nuint set_type_len, ulong* out_id);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_spatial_tree(ulong model, ValueNode* nodes, nuint node_capacity, nuint* out_nodes_required, byte* strings, nuint string_capacity, nuint* out_strings_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_systems(ulong model, ValueNode* nodes, nuint node_capacity, nuint* out_nodes_required, byte* strings, nuint string_capacity, nuint* out_strings_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_unreachable_products(ulong model, nuint* out_count, ValueNode* nodes, nuint node_capacity, nuint* out_nodes_required, byte* strings, nuint string_capacity, nuint* out_strings_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_validate(ulong model, nuint max_findings, NativeValidationSummary* out_summary, ValueNode* nodes, nuint node_capacity, nuint* out_nodes_required, byte* strings, nuint string_capacity, nuint* out_strings_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_write(ulong model, byte* buffer, nuint capacity, nuint* out_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_model_write_ifcxml(ulong model, byte* profile, nuint profile_len, byte* buffer, nuint capacity, nuint* out_required);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IfcStatus openbim_ifc_v0_1_version(NativeVersion* out_version);
}
