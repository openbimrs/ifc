using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;

namespace OpenBim.Ifc;

/// <summary>
/// An immutable list that compares by its items, so a record holding one
/// (<see cref="Value.List"/>, <see cref="Header"/>, every domain record)
/// keeps value equality: two reads of the same model are equal.
/// </summary>
/// <typeparam name="T">The item type.</typeparam>
public sealed class EquatableList<T> : IReadOnlyList<T>, IEquatable<EquatableList<T>>
{
    private readonly T[] items;

    /// <summary>The empty list.</summary>
    public static EquatableList<T> Empty { get; } = new(Array.Empty<T>());

    /// <summary>A list of <paramref name="items"/>, copied.</summary>
    public EquatableList(IEnumerable<T> items)
    {
        this.items = items?.ToArray() ?? throw new ArgumentNullException(nameof(items));
    }

    private EquatableList(T[] items, bool owned)
    {
        _ = owned;
        this.items = items;
    }

    /// <summary>A list taking ownership of an array nobody else holds.</summary>
    internal static EquatableList<T> Wrap(T[] items) => items.Length == 0 ? Empty : new EquatableList<T>(items, owned: true);

    /// <inheritdoc/>
    public T this[int index] => items[index];

    /// <inheritdoc/>
    public int Count => items.Length;

    /// <inheritdoc/>
    public IEnumerator<T> GetEnumerator() => ((IEnumerable<T>)items).GetEnumerator();

    IEnumerator IEnumerable.GetEnumerator() => items.GetEnumerator();

    /// <inheritdoc/>
    public bool Equals(EquatableList<T>? other) =>
        other is not null && (ReferenceEquals(this, other) || items.SequenceEqual(other.items));

    /// <inheritdoc/>
    public override bool Equals(object? obj) => Equals(obj as EquatableList<T>);

    /// <inheritdoc/>
    public override int GetHashCode()
    {
        unchecked
        {
            var hash = 17;
            foreach (var item in items)
            {
                hash = (hash * 31) + (item is null ? 0 : EqualityComparer<T>.Default.GetHashCode(item));
            }
            return hash;
        }
    }

    /// <summary>The items, as <c>[a, b]</c>.</summary>
    public override string ToString() => "[" + string.Join(", ", items.Select(item => item?.ToString())) + "]";
}
