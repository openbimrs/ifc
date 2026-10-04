// Values across the C ABI: a pre-order array of nodes plus one UTF-8 string
// buffer (crates/openbim-ifc-capi/README.md, "Values: tapes"). Unused node
// fields stay zero, as the ABI requires.

using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

namespace OpenBim.Ifc.Native;

/// <summary>Values flattened for a call that takes a tape.</summary>
internal sealed class TapeWriter
{
    private static readonly UTF8Encoding Utf8 = new(encoderShouldEmitUTF8Identifier: false, throwOnInvalidBytes: true);

    private readonly List<ValueNode> nodes = new();
    private readonly MemoryStream strings = new();

    /// <summary>The nodes written so far.</summary>
    internal ValueNode[] Nodes => nodes.ToArray();

    /// <summary>The string buffer written so far.</summary>
    internal byte[] Strings => strings.ToArray();

    /// <summary>A tape of one value.</summary>
    internal static TapeWriter Of(Value value)
    {
        var tape = new TapeWriter();
        tape.Write(value);
        return tape;
    }

    /// <summary>Append <paramref name="value"/> and its children.</summary>
    internal void Write(Value value)
    {
        switch (value ?? throw new ArgumentNullException(nameof(value)))
        {
            case Value.Null:
                nodes.Add(new ValueNode { Kind = Kind.Null });
                break;
            case Value.Derived:
                nodes.Add(new ValueNode { Kind = Kind.Derived });
                break;
            case Value.Unknown:
                nodes.Add(new ValueNode { Kind = Kind.Unknown });
                break;
            case Value.Bool b:
                nodes.Add(new ValueNode { Kind = Kind.Bool, IntValue = b.Value ? 1 : 0 });
                break;
            case Value.Integer i:
                nodes.Add(new ValueNode { Kind = Kind.Integer, IntValue = i.Value });
                break;
            case Value.Real r:
                nodes.Add(new ValueNode { Kind = Kind.Real, RealValue = r.Value });
                break;
            case Value.Text t:
                WriteString(Kind.Text, t.Value);
                break;
            case Value.Binary b:
                WriteString(Kind.Binary, b.Value);
                break;
            case Value.Enum e:
                WriteString(Kind.Enum, e.Value);
                break;
            case Value.Ref r:
                // The C ABI carries ids as int64; anything larger is refused
                // there with out-of-range, as in the other hosts.
                nodes.Add(new ValueNode { Kind = Kind.Ref, IntValue = unchecked((long)r.Id) });
                break;
            case Value.List list:
                nodes.Add(new ValueNode { Kind = Kind.List, ChildCount = checked((uint)list.Items.Count) });
                foreach (var item in list.Items)
                {
                    Write(item);
                }
                break;
            case Value.Typed typed:
                WriteString(Kind.Typed, typed.Type);
                Write(typed.Value);
                break;
            default:
                throw new ArgumentException("not an OpenBim.Ifc value: " + value.GetType(), nameof(value));
        }
    }

    /// <summary>Append a node of <paramref name="kind"/> holding <paramref name="text"/>.</summary>
    internal void WriteString(int kind, string text)
    {
        var bytes = Utf8.GetBytes(text ?? throw new ArgumentNullException(nameof(text)));
        nodes.Add(new ValueNode
        {
            Kind = kind,
            StrOffset = (ulong)strings.Length,
            StrLen = (ulong)bytes.Length,
        });
        strings.Write(bytes, 0, bytes.Length);
    }

    /// <summary>Append a <c>LIST</c> head of <paramref name="children"/> items.</summary>
    internal void WriteListHead(int children) =>
        nodes.Add(new ValueNode { Kind = Kind.List, ChildCount = checked((uint)children) });
}

/// <summary>Values read back from a tape the C ABI filled.</summary>
internal static class TapeReader
{
    private static readonly UTF8Encoding Utf8 = new(encoderShouldEmitUTF8Identifier: false, throwOnInvalidBytes: true);

    /// <summary>Every top-level value of the tape, in order.</summary>
    internal static Value[] ReadAll(ValueNode[] nodes, int nodeCount, byte[] strings)
    {
        var values = new List<Value>();
        var index = 0;
        while (index < nodeCount)
        {
            values.Add(Read(nodes, nodeCount, strings, ref index));
        }
        return values.ToArray();
    }

    /// <summary>The single value of a one-value tape.</summary>
    internal static Value ReadOne(ValueNode[] nodes, int nodeCount, byte[] strings)
    {
        var index = 0;
        var value = Read(nodes, nodeCount, strings, ref index);
        if (index != nodeCount)
        {
            throw new InvalidOperationException("the C ABI returned more than one value");
        }
        return value;
    }

    private static Value Read(ValueNode[] nodes, int nodeCount, byte[] strings, ref int index)
    {
        if (index >= nodeCount)
        {
            throw new InvalidOperationException("the C ABI returned a truncated tape");
        }
        var node = nodes[index++];
        switch (node.Kind)
        {
            case Kind.Null:
                return new Value.Null();
            case Kind.Derived:
                return new Value.Derived();
            case Kind.Unknown:
                return new Value.Unknown();
            case Kind.Bool:
                return new Value.Bool(node.IntValue != 0);
            case Kind.Integer:
                return new Value.Integer(node.IntValue);
            case Kind.Real:
                return new Value.Real(node.RealValue);
            case Kind.Text:
                return new Value.Text(String(node, strings));
            case Kind.Binary:
                return new Value.Binary(String(node, strings));
            case Kind.Enum:
                return new Value.Enum(String(node, strings));
            case Kind.Ref:
                return new Value.Ref(unchecked((ulong)node.IntValue));
            case Kind.List:
                var items = new Value[node.ChildCount];
                for (var i = 0; i < items.Length; i++)
                {
                    items[i] = Read(nodes, nodeCount, strings, ref index);
                }
                return new Value.List(EquatableList<Value>.Wrap(items));
            case Kind.Typed:
                var type = String(node, strings);
                return new Value.Typed(type, Read(nodes, nodeCount, strings, ref index));
            default:
                throw new InvalidOperationException("the C ABI returned an unknown value kind " + node.Kind);
        }
    }

    private static string String(ValueNode node, byte[] strings) =>
        Utf8.GetString(strings, checked((int)node.StrOffset), checked((int)node.StrLen));
}
