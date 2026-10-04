// Records beyond attribute values: read options, the STEP header,
// validation, unreachable products and dangling references. Each mirrors a
// record of the shared binding core, so the JavaScript and Python bindings
// carry the same fields under their own names.

namespace OpenBim.Ifc;

/// <summary>How a STEP read treats a record it cannot parse.</summary>
public enum OnMalformed
{
    /// <summary>Fail the read (the strict default).</summary>
    Abort,
    /// <summary>Drop the record and report it in <see cref="IfcModel.Diagnostics"/>.</summary>
    Skip,
}

/// <summary>
/// How a STEP read treats damaged input. The defaults are strict; what a
/// lenient read recovers from is listed by <see cref="IfcModel.Diagnostics"/>.
/// </summary>
/// <param name="OnMalformed">Fail on a record that cannot be parsed, or skip it.</param>
/// <param name="CheckReferences">Report duplicate ids and references to undefined ids. Nothing is dropped.</param>
/// <param name="AcceptRealWithoutPoint">Read a real written without its decimal point (<c>1E-05</c>) as that real.</param>
public sealed record ParseOptions(
    OnMalformed OnMalformed = OnMalformed.Abort,
    bool CheckReferences = false,
    bool AcceptRealWithoutPoint = false)
{
    /// <summary>Any malformed record fails the read.</summary>
    public static ParseOptions Strict { get; } = new();

    /// <summary>Skip malformed records and accept reals without a point.</summary>
    public static ParseOptions Lenient { get; } = new(OnMalformed.Skip, AcceptRealWithoutPoint: true);

    /// <summary>The <c>OPENBIM_IFC_PARSE_*</c> flag bits.</summary>
    internal uint Flags =>
        (OnMalformed == OnMalformed.Skip ? Native.ParseFlags.SkipMalformed : 0u)
        | (CheckReferences ? Native.ParseFlags.CheckReferences : 0u)
        | (AcceptRealWithoutPoint ? Native.ParseFlags.AcceptRealWithoutPoint : 0u);
}

/// <summary>
/// The STEP file header: <c>FILE_DESCRIPTION</c>, <c>FILE_NAME</c> and
/// <c>FILE_SCHEMA</c>, in STEP order. Change a field with <c>with</c> and
/// assign the result to <see cref="IfcModel.Header"/>.
/// </summary>
/// <param name="Description">FILE_DESCRIPTION description.</param>
/// <param name="ImplementationLevel">FILE_DESCRIPTION implementation level, e.g. <c>2;1</c>.</param>
/// <param name="Name">FILE_NAME name.</param>
/// <param name="TimeStamp">FILE_NAME time stamp.</param>
/// <param name="Author">FILE_NAME author.</param>
/// <param name="Organization">FILE_NAME organization.</param>
/// <param name="PreprocessorVersion">FILE_NAME preprocessor version.</param>
/// <param name="OriginatingSystem">FILE_NAME originating system.</param>
/// <param name="Authorization">FILE_NAME authorization.</param>
/// <param name="Schema">FILE_SCHEMA schema identifiers, e.g. <c>IFC4</c>.</param>
public sealed record Header(
    EquatableList<string> Description,
    string ImplementationLevel,
    string Name,
    string TimeStamp,
    EquatableList<string> Author,
    EquatableList<string> Organization,
    string PreprocessorVersion,
    string OriginatingSystem,
    string Authorization,
    EquatableList<string> Schema);

/// <summary>
/// One validation finding. <see cref="Entity"/> is null for the file as a whole.
/// </summary>
/// <param name="Severity"><c>error</c>, <c>evaluation-error</c>, <c>warning</c> or <c>unsupported</c>.</param>
/// <param name="Rule">The check's stable id.</param>
/// <param name="Entity">The entity, or null for the file.</param>
/// <param name="AttributeIndex">The attribute slot, when the finding has one.</param>
/// <param name="AttributeName">The attribute name, when the finding has one.</param>
/// <param name="Path">Where in the value the finding is.</param>
/// <param name="Message">A one-line explanation.</param>
public sealed record ValidationFinding(
    string Severity,
    string Rule,
    ulong? Entity,
    long? AttributeIndex,
    string? AttributeName,
    string Path,
    string Message);

/// <summary>
/// The result of <see cref="IfcModel.Validate"/>. <see cref="Conformant"/>
/// means no errors and no evaluation errors; unsupported rules do not count
/// against it. <see cref="Truncated"/> means the finding budget was reached,
/// so the counts are lower bounds.
/// </summary>
/// <param name="Conformant">No errors and no evaluation errors.</param>
/// <param name="Truncated">The run stopped at its finding budget.</param>
/// <param name="Errors">Schema violations.</param>
/// <param name="EvaluationErrors">Implemented rules that could not be decided for an instance.</param>
/// <param name="Warnings">Legal but suspicious conditions.</param>
/// <param name="Unsupported">Rules this validator does not evaluate.</param>
/// <param name="Findings">The findings, sorted by severity, rule, entity and slot.</param>
public sealed record ValidationReport(
    bool Conformant,
    bool Truncated,
    long Errors,
    long EvaluationErrors,
    long Warnings,
    long Unsupported,
    EquatableList<ValidationFinding> Findings);

/// <summary>A product no viewer will draw.</summary>
/// <param name="Id">The product.</param>
/// <param name="Reason"><c>not-contained-in-spatial-structure</c>, <c>no-representation-in-model-context</c> or <c>representation-without-context</c>.</param>
/// <param name="FoundViews">The target views the geometry sits in instead, for the second reason.</param>
/// <param name="Message">A one-line explanation.</param>
public sealed record UnreachableProduct(
    ulong Id,
    string Reason,
    EquatableList<string> FoundViews,
    string Message);

/// <summary>A reference to an entity the model does not contain.</summary>
/// <param name="From">The referencing entity.</param>
/// <param name="To">The missing id it points to.</param>
public readonly record struct DanglingReference(ulong From, ulong To);

/// <summary>The C ABI and library versions of the loaded native library.</summary>
/// <param name="Abi">The C ABI version; this package binds <c>0.1.x</c>.</param>
/// <param name="Library">The <c>openbim-ifc-capi</c> crate version it was built from.</param>
public sealed record LibraryVersion(System.Version Abi, System.Version Library);
