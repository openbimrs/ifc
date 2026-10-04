// Schema-checked entity creation (#330): one operation of an authoring
// batch, written to the C ABI's tape form (`openbim_ifc_v0_1_model_author`):
// a LIST of the operation's ENUM name, then field names and values.

using System;
using System.Collections.Generic;
using System.Linq;
using OpenBim.Ifc.Native;

namespace OpenBim.Ifc;

/// <summary>
/// One operation of <see cref="IfcModel.Author"/>, built with the factory
/// methods below.
/// </summary>
/// <remarks>
/// Ids are entity ids or <see cref="IfcModel.Handle"/> values: the entity an
/// earlier operation of the same batch produced. Attributes map attribute
/// names (any case) to <see cref="Value"/>s. A builder writes the
/// <c>ownerHistory</c> it is given on every record it creates; none is
/// invented, and IFC2X3, which requires one, refuses a record without.
/// </remarks>
public sealed class AuthorOp
{
    private readonly List<KeyValuePair<string, object>> fields = new();

    private AuthorOp(string op, params (string Key, object? Value)[] values)
    {
        Op = op;
        foreach (var (key, value) in values)
        {
            if (value is not null)
            {
                fields.Add(new KeyValuePair<string, object>(key, value));
            }
        }
    }

    /// <summary>The operation's name, e.g. <c>product</c>.</summary>
    public string Op { get; }

    /// <summary>One entity of <paramref name="type"/> from named attributes; an <c>IfcRoot</c> without a <c>GlobalId</c> gets one.</summary>
    public static AuthorOp Create(string type, IEnumerable<KeyValuePair<string, Value>>? attributes = null) =>
        new("create", ("type", type), ("attributes", Pairs(attributes)));

    /// <summary>Replace named attributes of <paramref name="entity"/>; the whole entity is checked again.</summary>
    public static AuthorOp Edit(ulong entity, IEnumerable<KeyValuePair<string, Value>> attributes) =>
        new("edit", ("entity", entity), ("attributes", Pairs(attributes ?? throw new ArgumentNullException(nameof(attributes)))));

    /// <summary>Remove <paramref name="entity"/> with the relationships that reference it.</summary>
    public static AuthorOp Remove(ulong entity) => new("remove", ("entity", entity));

    /// <summary>The model's one <c>IfcProject</c>.</summary>
    public static AuthorOp Project(IEnumerable<KeyValuePair<string, Value>>? attributes = null, ulong? ownerHistory = null) =>
        new("project", ("attributes", Pairs(attributes)), ("owner_history", ownerHistory));

    /// <summary>A spatial element aggregated under <paramref name="parent"/> (<c>IfcRelAggregates</c>).</summary>
    public static AuthorOp Spatial(
        string type,
        ulong parent,
        IEnumerable<KeyValuePair<string, Value>>? attributes = null,
        ulong? placement = null,
        ulong? ownerHistory = null) =>
        new("spatial", ("type", type), ("parent", parent), ("attributes", Pairs(attributes)), ("placement", placement), ("owner_history", ownerHistory));

    /// <summary>A product, contained in <paramref name="container"/> (<c>IfcRelContainedInSpatialStructure</c>) and typed by <paramref name="typeObject"/> (<c>IfcRelDefinesByType</c>).</summary>
    public static AuthorOp Product(
        string type,
        IEnumerable<KeyValuePair<string, Value>>? attributes = null,
        ulong? container = null,
        ulong? placement = null,
        ulong? typeObject = null,
        ulong? ownerHistory = null) =>
        new(
            "product",
            ("type", type),
            ("attributes", Pairs(attributes)),
            ("container", container),
            ("placement", placement),
            ("type_object", typeObject),
            ("owner_history", ownerHistory));

    /// <summary>A type object (<c>IfcWallType</c>, ...).</summary>
    public static AuthorOp TypeObject(string type, IEnumerable<KeyValuePair<string, Value>>? attributes = null, ulong? ownerHistory = null) =>
        new("type_object", ("type", type), ("attributes", Pairs(attributes)), ("owner_history", ownerHistory));

    /// <summary><c>IfcRelDefinesByType</c>; an object already typed is refused.</summary>
    public static AuthorOp AssignType(ulong typeObject, IEnumerable<ulong> objects, ulong? ownerHistory = null) =>
        new("assign_type", ("type_object", typeObject), ("objects", Ids(objects)), ("owner_history", ownerHistory));

    /// <summary><c>IfcRelContainedInSpatialStructure</c>; an element already contained is refused.</summary>
    public static AuthorOp Contain(ulong structure, IEnumerable<ulong> elements, ulong? ownerHistory = null) =>
        new("contain", ("structure", structure), ("elements", Ids(elements)), ("owner_history", ownerHistory));

    /// <summary><c>IfcRelAggregates</c>; a part already aggregated is refused.</summary>
    public static AuthorOp Aggregate(ulong parent, IEnumerable<ulong> parts, ulong? ownerHistory = null) =>
        new("aggregate", ("parent", parent), ("parts", Ids(parts)), ("owner_history", ownerHistory));

    /// <summary>An <c>IfcLocalPlacement</c>; <paramref name="axis"/> and <paramref name="refDirection"/> both or neither.</summary>
    public static AuthorOp Placement(
        ulong? relativeTo = null,
        (double X, double Y, double Z)? location = null,
        (double X, double Y, double Z)? axis = null,
        (double X, double Y, double Z)? refDirection = null) =>
        new(
            "placement",
            ("relative_to", relativeTo),
            ("location", Triple(location)),
            ("axis", Triple(axis)),
            ("ref_direction", Triple(refDirection)));

    /// <summary>An <c>IfcOwnerHistory</c> with its person, organization and application.</summary>
    public static AuthorOp OwnerHistory(
        string organization,
        string applicationName,
        string applicationVersion,
        string applicationIdentifier,
        long creationDate,
        string? personIdentification = null,
        string? familyName = null,
        string? givenName = null,
        string? changeAction = null,
        long? lastModifiedDate = null) =>
        new(
            "owner_history",
            ("organization", organization),
            ("application_name", applicationName),
            ("application_version", applicationVersion),
            ("application_identifier", applicationIdentifier),
            ("creation_date", creationDate),
            ("person_identification", personIdentification),
            ("family_name", familyName),
            ("given_name", givenName),
            ("change_action", changeAction),
            ("last_modified_date", lastModifiedDate));

    private static Value? Pairs(IEnumerable<KeyValuePair<string, Value>>? attributes) =>
        attributes is null
            ? null
            : new Value.List(attributes.Select(pair => (Value)new Value.List(new Value.Text(pair.Key), pair.Value)));

    private static Value Ids(IEnumerable<ulong> ids) =>
        new Value.List((ids ?? throw new ArgumentNullException(nameof(ids))).Select(id => (Value)new Value.Ref(id)));

    private static Value? Triple((double X, double Y, double Z)? point) =>
        point is { } p ? new Value.List(new Value.Real(p.X), new Value.Real(p.Y), new Value.Real(p.Z)) : null;

    /// <summary>Append the operation's tape form.</summary>
    internal void Write(TapeWriter tape)
    {
        tape.WriteListHead(1 + 2 * fields.Count);
        tape.WriteString(Kind.Enum, Op);
        foreach (var field in fields)
        {
            tape.WriteString(Kind.Text, field.Key);
            switch (field.Value)
            {
                case string text:
                    tape.WriteString(Kind.Text, text);
                    break;
                case ulong id:
                    tape.Write(new Value.Ref(id));
                    break;
                case long integer:
                    tape.Write(new Value.Integer(integer));
                    break;
                case Value value:
                    tape.Write(value);
                    break;
                default:
                    throw new InvalidOperationException("unexpected field " + field.Key);
            }
        }
    }
}
