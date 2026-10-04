using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace OpenBim.Ifc;

/// <summary>
/// An IFC attribute value, one case per STEP form. The cases keep every
/// distinction IFC makes (ADR 0013): <see cref="Null"/> (<c>$</c>) is not
/// <see cref="Derived"/> (<c>*</c>), <see cref="Unknown"/> (<c>.U.</c>) is not
/// <see cref="Bool"/>, <see cref="Integer"/> is not <see cref="Real"/>, and a
/// <see cref="Typed"/> wrapper such as <c>IFCLENGTHMEASURE(2.5)</c> is not its
/// payload. A file read and written back through .NET is unchanged.
/// </summary>
/// <remarks>
/// The hierarchy is closed: match on it with <c>switch</c>. Equality is by
/// value, lists included. No host value converts implicitly, since <c>3</c>
/// could be an <see cref="Integer"/> or a <see cref="Real"/> and <c>"x"</c> a
/// <see cref="Text"/> or an <see cref="Enum"/>.
/// </remarks>
public abstract record Value
{
    private Value()
    {
    }

    /// <summary><c>$</c>: the attribute is not set.</summary>
    public sealed record Null : Value
    {
        /// <inheritdoc/>
        public override string ToString() => "$";
    }

    /// <summary><c>*</c>: derived in a supertype; distinct from <c>$</c>.</summary>
    public sealed record Derived : Value
    {
        /// <inheritdoc/>
        public override string ToString() => "*";
    }

    /// <summary><c>.T.</c> or <c>.F.</c>.</summary>
    /// <param name="Value">The boolean.</param>
    public sealed record Bool(bool Value) : Value
    {
        /// <inheritdoc/>
        public override string ToString() => Value ? ".T." : ".F.";
    }

    /// <summary><c>.U.</c>: the third logical state, never a <see cref="Bool"/>.</summary>
    public sealed record Unknown : Value
    {
        /// <inheritdoc/>
        public override string ToString() => ".U.";
    }

    /// <summary>An integer literal (64-bit).</summary>
    /// <param name="Value">The integer.</param>
    public sealed record Integer(long Value) : Value
    {
        /// <inheritdoc/>
        public override string ToString() => Value.ToString(CultureInfo.InvariantCulture);
    }

    /// <summary>A real literal; finite, or the library refuses it with <c>invalid-value</c>.</summary>
    /// <param name="Value">The real.</param>
    public sealed record Real(double Value) : Value
    {
        /// <inheritdoc/>
        public override string ToString() => Value.ToString("R", CultureInfo.InvariantCulture);
    }

    /// <summary>A string literal, decoded.</summary>
    /// <param name="Value">The text.</param>
    public sealed record Text(string Value) : Value
    {
        /// <inheritdoc/>
        public override string ToString() => "'" + Value.Replace("'", "''") + "'";
    }

    /// <summary>A binary literal: its hexadecimal digits, as written in the file.</summary>
    /// <param name="Value">The digits, e.g. <c>0123ABC</c>.</param>
    public sealed record Binary(string Value) : Value
    {
        /// <inheritdoc/>
        public override string ToString() => "\"" + Value + "\"";
    }

    /// <summary>An enumeration value, e.g. <c>.ELEMENT.</c>.</summary>
    /// <param name="Value">The name without dots, e.g. <c>ELEMENT</c>.</param>
    public sealed record Enum(string Value) : Value
    {
        /// <inheritdoc/>
        public override string ToString() => "." + Value + ".";
    }

    /// <summary>A reference to an entity, <c>#42</c>.</summary>
    /// <param name="Id">The entity id.</param>
    public sealed record Ref(ulong Id) : Value
    {
        /// <inheritdoc/>
        public override string ToString() => "#" + Id.ToString(CultureInfo.InvariantCulture);
    }

    /// <summary>An aggregate, <c>(1,2)</c>.</summary>
    /// <param name="Items">The items, in order.</param>
    public sealed record List(EquatableList<Value> Items) : Value
    {
        /// <summary>An aggregate of <paramref name="items"/>.</summary>
        public List(params Value[] items)
            : this(new EquatableList<Value>(items))
        {
        }

        /// <summary>An aggregate of <paramref name="items"/>.</summary>
        public List(IEnumerable<Value> items)
            : this(new EquatableList<Value>(items))
        {
        }

        /// <inheritdoc/>
        public override string ToString() => "(" + string.Join(",", Items.Select(item => item.ToString())) + ")";
    }

    /// <summary>A typed value, <c>IFCLENGTHMEASURE(2.5)</c>.</summary>
    /// <param name="Type">The type name, e.g. <c>IFCLENGTHMEASURE</c>.</param>
    /// <param name="Value">The wrapped value.</param>
    public sealed record Typed(string Type, Value Value) : Value
    {
        /// <inheritdoc/>
        public override string ToString() => Type + "(" + Value + ")";
    }

    /// <summary>The value as a <see cref="Ref"/> id, or an exception naming what it is.</summary>
    internal ulong ExpectRef() => this is Ref reference ? reference.Id : throw Mismatch("REF");

    internal InvalidOperationException Mismatch(string expected) =>
        new("expected " + expected + " from the C ABI, found " + GetType().Name);
}
