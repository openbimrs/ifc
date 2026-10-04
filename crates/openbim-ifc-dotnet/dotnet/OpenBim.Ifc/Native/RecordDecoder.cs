// Records from the tape, with one generic function, as the shared binding
// core intends (#123): a record crosses the C ABI as a LIST of its field
// values in declaration order, and each C# record declares the same fields
// in the same order as positional parameters. The crate's
// tests/conformance.rs checks those declarations against the core's.
//
// A field's C# type says how to read it: ulong is a REF (an entity id), long
// or int (a count or slot) an INTEGER, double a REAL, bool a BOOL, string a TEXT, Value any IFC value
// as it is, EquatableList<T> a LIST of T, and any other type a nested record.
// A nullable field reads NULL as null.

using System;
using System.Collections.Concurrent;
using System.Linq;
using System.Reflection;

namespace OpenBim.Ifc.Native;

internal static class RecordDecoder
{
    private static readonly ConcurrentDictionary<Type, ConstructorInfo> Constructors = new();

    /// <summary>The record of type <typeparamref name="T"/> a tape value holds.</summary>
    internal static T Decode<T>(Value value) => (T)Decode(value, typeof(T))!;

    /// <summary>A list of records.</summary>
    internal static EquatableList<T> DecodeList<T>(Value value) => (EquatableList<T>)Decode(value, typeof(EquatableList<T>))!;

    private static object? Decode(Value value, Type type)
    {
        var underlying = Nullable.GetUnderlyingType(type);
        if (underlying is not null)
        {
            return value is Value.Null ? null : Decode(value, underlying);
        }
        if (type == typeof(Value))
        {
            return value;
        }
        if (value is Value.Null && !type.IsValueType)
        {
            // An optional text or record. A required one is never NULL: the
            // core writes NULL only for absent optional fields.
            return null;
        }
        if (type == typeof(ulong))
        {
            return value.ExpectRef();
        }
        if (type == typeof(int))
        {
            return value is Value.Integer count ? checked((int)count.Value) : throw value.Mismatch("INTEGER");
        }
        if (type == typeof(long))
        {
            return value is Value.Integer integer ? integer.Value : throw value.Mismatch("INTEGER");
        }
        if (type == typeof(double))
        {
            return value is Value.Real real ? real.Value : throw value.Mismatch("REAL");
        }
        if (type == typeof(bool))
        {
            return value is Value.Bool flag ? flag.Value : throw value.Mismatch("BOOL");
        }
        if (type == typeof(string))
        {
            return value is Value.Text text ? text.Value : throw value.Mismatch("TEXT");
        }
        if (type.IsGenericType && type.GetGenericTypeDefinition() == typeof(EquatableList<>))
        {
            var item = type.GetGenericArguments()[0];
            var items = value is Value.List list ? list.Items : throw value.Mismatch("LIST");
            var array = Array.CreateInstance(item, items.Count);
            for (var i = 0; i < items.Count; i++)
            {
                array.SetValue(Decode(items[i], item), i);
            }
            return typeof(EquatableList<>).MakeGenericType(item)
                .GetMethod("Wrap", BindingFlags.NonPublic | BindingFlags.Static)!
                .Invoke(null, new object[] { array });
        }
        var fields = value is Value.List record ? record.Items : throw value.Mismatch("LIST (a " + type.Name + " record)");
        var constructor = Constructors.GetOrAdd(type, Primary);
        var parameters = constructor.GetParameters();
        if (parameters.Length != fields.Count)
        {
            throw new InvalidOperationException(
                "the C ABI returned " + fields.Count + " fields for " + type.Name + ", which has " + parameters.Length);
        }
        var arguments = new object?[parameters.Length];
        for (var i = 0; i < parameters.Length; i++)
        {
            arguments[i] = Decode(fields[i], parameters[i].ParameterType);
        }
        return constructor.Invoke(arguments);
    }

    /// <summary>A record's primary constructor: the public one that is not
    /// the compiler's copy constructor.</summary>
    private static ConstructorInfo Primary(Type type) =>
        type.GetConstructors(BindingFlags.Public | BindingFlags.Instance)
            .Where(c => !(c.GetParameters().Length == 1 && c.GetParameters()[0].ParameterType == type))
            .OrderByDescending(c => c.GetParameters().Length)
            .First();
}
