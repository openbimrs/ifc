use crate::definition::QuantityKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MemberForm {
    SingleValue,
    BoundedValue,
    EnumeratedValue,
    ListValue,
    ReferenceValue,
    TableValue,
    Complex,
    Quantity(QuantityKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedMember {
    pub name: String,
    pub form: MemberForm,
    /// Observed IFC type metadata. An empty vector means "not observed";
    /// validation then skips type conformance rather than proving it.
    pub data_types: Vec<String>,
    pub enumeration_value: Option<String>,
}

impl ObservedMember {
    pub fn property(name: impl Into<String>, form: MemberForm) -> Self {
        Self {
            name: name.into(),
            form,
            data_types: Vec::new(),
            enumeration_value: None,
        }
    }
    pub fn quantity(name: impl Into<String>, kind: QuantityKind) -> Self {
        Self::property(name, MemberForm::Quantity(kind))
    }
    pub fn with_data_type(mut self, type_name: impl Into<String>) -> Self {
        self.data_types.push(type_name.into());
        self
    }
    pub fn with_enumeration_value(mut self, value: impl Into<String>) -> Self {
        self.enumeration_value = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedSet {
    pub name: String,
    pub members: Vec<ObservedMember>,
}
impl ObservedSet {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            members: Vec::new(),
        }
    }
    pub fn with_member(mut self, member: ObservedMember) -> Self {
        self.members.push(member);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UnexpectedMemberPolicy {
    Ignore,
    Warning,
    Error,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationPolicy {
    pub require_all_members: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationSeverity {
    Warning,
    Error,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationCode {
    SetNameMismatch,
    DuplicateMember,
    UnexpectedMember,
    MissingMember,
    FormMismatch,
    DataTypeMismatch,
    InvalidEnumerationValue,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub code: ValidationCode,
    pub severity: ValidationSeverity,
    pub member: Option<String>,
    pub message: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValidationReport {
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
