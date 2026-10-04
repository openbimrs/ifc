using System;

namespace OpenBim.Ifc;

/// <summary>
/// A refused call. <see cref="Code"/> is the stable code every binding
/// shares (ADR 0013): the JavaScript <c>err.code</c>, the Python
/// <c>IfcError.code</c> and this one name the same failure the same way, and
/// a code is never renamed or reused.
/// </summary>
/// <remarks>
/// The shared codes are <c>parse</c>, <c>write</c>, <c>missing-entity</c>,
/// <c>invalid-value</c>, <c>out-of-range</c>, <c>unsupported-schema</c>,
/// <c>io</c>, <c>unsupported-profile</c>, <c>feature-disabled</c>, from a
/// domain view <c>invalid-model</c>, <c>missing-reference</c>,
/// <c>budget-exceeded</c>, <c>unsupported</c> and <c>wrong-entity-type</c>,
/// from a property edit <c>template-violation</c> and
/// <c>missing-property</c>, and from a by-name attribute access
/// <c>unknown-attribute</c> and <c>derived-attribute</c>. A status that only the C boundary has carries
/// its own name (<c>panic</c>, <c>invalid-argument</c>, ...).
/// </remarks>
public sealed class IfcException : Exception
{
    /// <summary>A refusal with its code, C status and message.</summary>
    public IfcException(string code, IfcStatus status, string message)
        : base(message)
    {
        Code = code;
        Status = status;
    }

    /// <summary>The stable error code, e.g. <c>missing-entity</c>.</summary>
    public string Code { get; }

    /// <summary>The C ABI status the call returned.</summary>
    public IfcStatus Status { get; }

    /// <summary>The code a status stands for, when no model holds a better one.</summary>
    internal static string CodeOf(IfcStatus status) => status switch
    {
        IfcStatus.Parse => "parse",
        IfcStatus.Write => "write",
        IfcStatus.MissingEntity => "missing-entity",
        IfcStatus.InvalidValue => "invalid-value",
        IfcStatus.OutOfRange => "out-of-range",
        IfcStatus.UnsupportedSchema => "unsupported-schema",
        IfcStatus.Io => "io",
        IfcStatus.UnsupportedProfile => "unsupported-profile",
        IfcStatus.FeatureDisabled => "feature-disabled",
        IfcStatus.InvalidModel => "invalid-model",
        IfcStatus.MissingReference => "missing-reference",
        IfcStatus.BudgetExceeded => "budget-exceeded",
        IfcStatus.Unsupported => "unsupported",
        IfcStatus.WrongEntityType => "wrong-entity-type",
        IfcStatus.TemplateViolation => "template-violation",
        IfcStatus.MissingProperty => "missing-property",
        IfcStatus.CatalogNotLoaded => "catalog-not-loaded",
        IfcStatus.UnknownAttribute => "unknown-attribute",
        IfcStatus.DerivedAttribute => "derived-attribute",
        IfcStatus.NullPointer => "null-pointer",
        IfcStatus.InvalidArgument => "invalid-argument",
        IfcStatus.InvalidHandle => "invalid-handle",
        IfcStatus.BufferTooSmall => "buffer-too-small",
        IfcStatus.NoValue => "no-value",
        IfcStatus.Panic => "panic",
        _ => "status-" + ((int)status).ToString(System.Globalization.CultureInfo.InvariantCulture),
    };

    /// <summary>Whether the C ABI records this status as the model's last
    /// error: the shared binding errors do, boundary misuse does not.</summary>
    internal static bool IsBindingError(IfcStatus status) =>
        status is >= IfcStatus.Parse and <= IfcStatus.DerivedAttribute and not IfcStatus.NoValue;
}
