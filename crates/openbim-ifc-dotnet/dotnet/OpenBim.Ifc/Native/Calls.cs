// The C ABI's buffer protocol, once: every variable-size output takes
// (buffer, capacity, out_required); a short buffer returns BufferTooSmall and
// is left untouched. Each helper tries a small buffer first, then one of the
// exact size, so a short answer costs one call.

using System;
using System.Text;

namespace OpenBim.Ifc.Native;

internal unsafe delegate IfcStatus BytesCall(byte* buffer, nuint capacity, nuint* required);

internal unsafe delegate IfcStatus IdsCall(ulong* buffer, nuint capacity, nuint* required);

internal unsafe delegate IfcStatus TapeCall(ValueNode* nodes, nuint nodeCapacity, nuint* nodesRequired, byte* strings, nuint stringCapacity, nuint* stringsRequired);

/// <summary>A tape the C ABI filled: <see cref="Count"/> nodes are valid.</summary>
internal readonly struct FilledTape
{
    internal FilledTape(ValueNode[] nodes, int count, byte[] strings)
    {
        Nodes = nodes;
        Count = count;
        Strings = strings;
    }

    internal ValueNode[] Nodes { get; }

    internal int Count { get; }

    internal byte[] Strings { get; }

    /// <summary>The tape's single value.</summary>
    internal Value One() => TapeReader.ReadOne(Nodes, Count, Strings);

    /// <summary>Every top-level value, for a call returning several.</summary>
    internal Value[] All() => TapeReader.ReadAll(Nodes, Count, Strings);
}

internal static unsafe class Calls
{
    /// <summary>Bounds the retries when a model changes between the size
    /// query and the fetch; the C ABI serializes calls, so one suffices.</summary>
    private const int Attempts = 4;

    internal static readonly UTF8Encoding Utf8 = new(encoderShouldEmitUTF8Identifier: false, throwOnInvalidBytes: true);

    /// <summary>Bytes from a buffer call: the status, and on success the
    /// buffer and the length used.</summary>
    internal static IfcStatus Bytes(BytesCall call, int initial, out byte[] buffer, out int length)
    {
        buffer = new byte[initial];
        for (var attempt = 0; ; attempt++)
        {
            nuint required = 0;
            IfcStatus status;
            fixed (byte* pointer = buffer)
            {
                status = call(pointer, (nuint)buffer.Length, &required);
            }
            if (status == IfcStatus.BufferTooSmall && attempt < Attempts && required > (nuint)buffer.Length)
            {
                buffer = new byte[checked((int)required)];
                continue;
            }
            length = status == IfcStatus.Ok ? checked((int)required) : 0;
            return status;
        }
    }

    /// <summary>A NUL-terminated string from a buffer call.</summary>
    internal static IfcStatus String(BytesCall call, out string text)
    {
        var status = Bytes(call, 256, out var buffer, out var length);
        if (length > 0 && buffer[length - 1] == 0)
        {
            length--;
        }
        text = status == IfcStatus.Ok ? Utf8.GetString(buffer, 0, length) : string.Empty;
        return status;
    }

    /// <summary>Ids from an id-buffer call.</summary>
    internal static IfcStatus Ids(IdsCall call, out ulong[] ids)
    {
        var buffer = new ulong[64];
        for (var attempt = 0; ; attempt++)
        {
            nuint required = 0;
            IfcStatus status;
            fixed (ulong* pointer = buffer)
            {
                status = call(pointer, (nuint)buffer.Length, &required);
            }
            if (status == IfcStatus.BufferTooSmall && attempt < Attempts && required > (nuint)buffer.Length)
            {
                buffer = new ulong[checked((int)required)];
                continue;
            }
            if (status != IfcStatus.Ok)
            {
                ids = Array.Empty<ulong>();
                return status;
            }
            var count = checked((int)required);
            if (count != buffer.Length)
            {
                Array.Resize(ref buffer, count);
            }
            ids = buffer;
            return status;
        }
    }

    /// <summary>A value tape from a tape call.</summary>
    internal static IfcStatus Tape(TapeCall call, out FilledTape tape)
    {
        var nodes = new ValueNode[64];
        var strings = new byte[1024];
        for (var attempt = 0; ; attempt++)
        {
            nuint nodesRequired = 0;
            nuint stringsRequired = 0;
            IfcStatus status;
            fixed (ValueNode* nodePointer = nodes)
            fixed (byte* stringPointer = strings)
            {
                status = call(nodePointer, (nuint)nodes.Length, &nodesRequired, stringPointer, (nuint)strings.Length, &stringsRequired);
            }
            if (status == IfcStatus.BufferTooSmall && attempt < Attempts
                && (nodesRequired > (nuint)nodes.Length || stringsRequired > (nuint)strings.Length))
            {
                if (nodesRequired > (nuint)nodes.Length)
                {
                    nodes = new ValueNode[checked((int)nodesRequired)];
                }
                if (stringsRequired > (nuint)strings.Length)
                {
                    strings = new byte[checked((int)stringsRequired)];
                }
                continue;
            }
            tape = status == IfcStatus.Ok
                ? new FilledTape(nodes, checked((int)nodesRequired), strings)
                : default;
            return status;
        }
    }

    /// <summary>UTF-8 bytes of a string argument.</summary>
    internal static byte[] Encode(string text) => Utf8.GetBytes(text ?? throw new ArgumentNullException(nameof(text)));

    /// <summary>The message a load wrote into its error buffer.</summary>
    internal static string ErrorText(byte[] buffer)
    {
        var length = Array.IndexOf(buffer, (byte)0);
        return Encoding.UTF8.GetString(buffer, 0, length < 0 ? buffer.Length : length);
    }

    /// <summary>The exception for a failed call on <paramref name="model"/>:
    /// the model's last error code and message, which the C ABI records for
    /// every shared binding error.</summary>
    internal static IfcException Error(ulong model, IfcStatus status)
    {
        if (IfcException.IsBindingError(status)
            && String((buffer, capacity, required) => NativeMethods.openbim_ifc_v0_1_last_error_code(model, buffer, capacity, required), out var code) == IfcStatus.Ok
            && String((buffer, capacity, required) => NativeMethods.openbim_ifc_v0_1_last_error_message(model, buffer, capacity, required), out var message) == IfcStatus.Ok)
        {
            return new IfcException(code, status, message);
        }
        return Error(status, null);
    }

    /// <summary>The exception for a failed call with no model to ask.</summary>
    internal static IfcException Error(IfcStatus status, string? message) =>
        new(IfcException.CodeOf(status), status, string.IsNullOrEmpty(message) ? "the C ABI returned " + status : message!);

    /// <summary>Throw unless <paramref name="status"/> is <see cref="IfcStatus.Ok"/>.</summary>
    internal static void Check(ulong model, IfcStatus status)
    {
        if (status != IfcStatus.Ok)
        {
            throw Error(model, status);
        }
    }
}
