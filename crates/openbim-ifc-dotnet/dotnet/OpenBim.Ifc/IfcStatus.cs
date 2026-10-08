namespace OpenBim.Ifc;

/// <summary>
/// The C ABI's status of a call (<c>OpenbimIfcStatus</c>), carried by
/// <see cref="IfcException.Status"/>. The values equal the C header's.
/// </summary>
/// <remarks>
/// <see cref="Parse"/> to <see cref="FeatureDisabled"/>, <see cref="InvalidModel"/>
/// and <see cref="MissingReference"/> to <see cref="AmbiguousValue"/> are the
/// errors every binding shares, each with a stable <see cref="IfcException.Code"/>.
/// The rest describe misuse of the C boundary, which this binding does not
/// commit; <see cref="Panic"/> is a bug in the library, worth reporting.
/// </remarks>
public enum IfcStatus
{
    /// <summary>Success.</summary>
    Ok = 0,
    /// <summary>A required pointer was null.</summary>
    NullPointer = 1,
    /// <summary>An argument was malformed (bad UTF-8, a malformed value tape, ...).</summary>
    InvalidArgument = 2,
    /// <summary>The model handle is zero, stale, or already destroyed.</summary>
    InvalidHandle = 3,
    /// <summary>The output buffer is smaller than required.</summary>
    BufferTooSmall = 4,
    /// <summary>The STEP or ifcXML input could not be parsed (<c>parse</c>).</summary>
    Parse = 10,
    /// <summary>The model could not be serialized (<c>write</c>).</summary>
    Write = 11,
    /// <summary>No entity has the given id (<c>missing-entity</c>).</summary>
    MissingEntity = 12,
    /// <summary>A value did not follow the encoding (<c>invalid-value</c>).</summary>
    InvalidValue = 13,
    /// <summary>An id or index is outside the representable range (<c>out-of-range</c>).</summary>
    OutOfRange = 14,
    /// <summary>The file's schema is not bundled, or a domain view does not read it (<c>unsupported-schema</c>).</summary>
    UnsupportedSchema = 15,
    /// <summary>A file could not be opened or read (<c>io</c>).</summary>
    Io = 16,
    /// <summary>No ifcXML XSD profile has that name (<c>unsupported-profile</c>).</summary>
    UnsupportedProfile = 17,
    /// <summary>The library leaves out the feature the call needs (<c>feature-disabled</c>).</summary>
    FeatureDisabled = 18,
    /// <summary>A domain view refused the file's data as malformed, ambiguous or unprovable (<c>invalid-model</c>).</summary>
    InvalidModel = 19,
    /// <summary>The requested value does not exist.</summary>
    NoValue = 20,
    /// <summary>A domain view followed a reference to an entity the file lacks (<c>missing-reference</c>).</summary>
    MissingReference = 21,
    /// <summary>A domain view stopped at a cycle or its depth budget (<c>budget-exceeded</c>).</summary>
    BudgetExceeded = 22,
    /// <summary>A domain view met a construct it does not interpret (<c>unsupported</c>).</summary>
    Unsupported = 23,
    /// <summary>A query named an entity of a type it does not accept, or a property edit a set type the set does not have (<c>wrong-entity-type</c>).</summary>
    WrongEntityType = 24,
    /// <summary>A property edit wrote a value its PSD/QTO template or enumeration refuses (<c>template-violation</c>).</summary>
    TemplateViolation = 25,
    /// <summary>A property edit removed a property the object does not state (<c>missing-property</c>).</summary>
    MissingProperty = 26,
    /// <summary>Reserved: a property edit before its catalog was loaded (<c>catalog-not-loaded</c>). This package embeds the catalog and never returns it.</summary>
    CatalogNotLoaded = 27,
    /// <summary>A by-name attribute access named no explicit attribute of the entity's type (<c>unknown-attribute</c>).</summary>
    UnknownAttribute = 28,
    /// <summary>A by-name attribute write named a slot the entity's type derives, written <c>*</c> (<c>derived-attribute</c>).</summary>
    DerivedAttribute = 29,
    /// <summary>An authoring batch left a required attribute of the declared release unset (<c>missing-attribute</c>).</summary>
    MissingAttribute = 30,
    /// <summary>An authoring batch removed an entity that an entity other than a relationship still references (<c>still-referenced</c>).</summary>
    StillReferenced = 31,
    /// <summary>A plain value written by name does not fit the attribute's declared type (<c>type-mismatch</c>).</summary>
    TypeMismatch = 32,
    /// <summary>A plain value written by name fits several members of the attribute's SELECT; write it exactly instead (<c>ambiguous-value</c>).</summary>
    AmbiguousValue = 33,
    /// <summary>A Rust panic was contained at the boundary.</summary>
    Panic = 255,
}
