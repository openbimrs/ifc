use crate::definition::QuantityKind;

/// Value form of one observed member, mirrors [`crate::definition::PropertyKind`]
/// or a quantity value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MemberForm {
    /// Matches [`crate::definition::PropertyKind::SingleValue`].
    SingleValue,
    /// Matches [`crate::definition::PropertyKind::BoundedValue`].
    BoundedValue,
    /// Matches [`crate::definition::PropertyKind::EnumeratedValue`].
    EnumeratedValue,
    /// Matches [`crate::definition::PropertyKind::ListValue`].
    ListValue,
    /// Matches [`crate::definition::PropertyKind::ReferenceValue`].
    ReferenceValue,
    /// Matches [`crate::definition::PropertyKind::TableValue`].
    TableValue,
    /// Matches [`crate::definition::PropertyKind::Complex`].
    Complex,
    /// A quantity member of the given physical family.
    Quantity(QuantityKind),
}

/// One authored property or quantity as observed in a caller's own model,
/// used as validation input against a [`crate::definition::SetTemplate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedMember {
    /// Dotted path matching [`super::apply_template`]'s `path` argument for
    /// nested `Complex` properties, or the bare name at the top level.
    pub name: String,
    /// Observed value form.
    pub form: MemberForm,
    /// Observed IFC type metadata. An empty vector means "not observed";
    /// validation then skips type conformance rather than proving it.
    pub data_types: Vec<String>,
    /// Observed enumeration value, if `form` is `EnumeratedValue` and one was read.
    pub enumeration_value: Option<String>,
}

impl ObservedMember {
    /// Start an observed property member with no data types or enumeration value set.
    pub fn property(name: impl Into<String>, form: MemberForm) -> Self {
        Self {
            name: name.into(),
            form,
            data_types: Vec::new(),
            enumeration_value: None,
        }
    }
    /// Start an observed quantity member of the given physical family.
    pub fn quantity(name: impl Into<String>, kind: QuantityKind) -> Self {
        Self::property(name, MemberForm::Quantity(kind))
    }
    /// Record one observed IFC type name for this member.
    pub fn with_data_type(mut self, type_name: impl Into<String>) -> Self {
        self.data_types.push(type_name.into());
        self
    }
    /// Record the observed enumeration value for this member.
    pub fn with_enumeration_value(mut self, value: impl Into<String>) -> Self {
        self.enumeration_value = Some(value.into());
        self
    }
}

/// One authored property or quantity set as observed in a caller's own
/// model, the [`validate`](super::validate) input paired with a template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedSet {
    /// Observed set name, compared against the template's `name`.
    pub name: String,
    /// Observed members, in the order they were added.
    pub members: Vec<ObservedMember>,
}
impl ObservedSet {
    /// Start an observed set with no members.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            members: Vec::new(),
        }
    }
    /// Append one observed member.
    pub fn with_member(mut self, member: ObservedMember) -> Self {
        self.members.push(member);
        self
    }
}

/// How [`validate`](super::validate) treats an observed member that the
/// template does not declare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UnexpectedMemberPolicy {
    /// Silently accept the extra member; no issue is reported.
    Ignore,
    /// Report the extra member as a [`ValidationSeverity::Warning`].
    Warning,
    /// Report the extra member as a [`ValidationSeverity::Error`].
    Error,
}
/// Validation rules applied by [`validate`](super::validate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationPolicy {
    /// When true, every template-declared member must appear in the
    /// observed set or a [`ValidationCode::MissingMember`] error is reported.
    pub require_all_members: bool,
    /// How to treat an observed member the template does not declare.
    pub unexpected_members: UnexpectedMemberPolicy,
}
impl Default for ValidationPolicy {
    fn default() -> Self {
        Self {
            require_all_members: false,
            unexpected_members: UnexpectedMemberPolicy::Warning,
        }
    }
}

/// Severity of one [`ValidationIssue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationSeverity {
    /// Notable but does not make [`ValidationReport::is_valid`] return false.
    Warning,
    /// Makes [`ValidationReport::is_valid`] return false.
    Error,
}
/// Category of one [`ValidationIssue`], identifying which check produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationCode {
    /// The observed set's `name` does not match the template's `name`.
    SetNameMismatch,
    /// The same member name appeared more than once in the observed set.
    DuplicateMember,
    /// An observed member is not declared by the template; see [`UnexpectedMemberPolicy`].
    UnexpectedMember,
    /// A template-declared member is absent from the observed set, under
    /// [`ValidationPolicy::require_all_members`].
    MissingMember,
    /// The observed member's [`MemberForm`] does not match the template's.
    FormMismatch,
    /// The observed member's data type(s) do not match the template's.
    DataTypeMismatch,
    /// The observed enumeration value is not among the template's declared values.
    InvalidEnumerationValue,
}
/// One deviation found while validating an [`ObservedSet`] against a template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    /// Which check produced this issue.
    pub code: ValidationCode,
    /// Whether this issue counts against [`ValidationReport::is_valid`].
    pub severity: ValidationSeverity,
    /// Member path the issue concerns, `None` for set-level issues.
    pub member: Option<String>,
    /// Human-readable description of the deviation.
    pub message: String,
}
/// Every deviation found by one [`validate`](super::validate) call.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValidationReport {
    /// Issues found, in the order they were detected.
    pub issues: Vec<ValidationIssue>,
}
impl ValidationReport {
    /// Returns true when validation found no errors in the metadata the caller supplied.
    /// Missing observed type metadata is unresolved, not proof of type conformance.
    pub fn is_valid(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|issue| issue.severity == ValidationSeverity::Error)
    }
}
