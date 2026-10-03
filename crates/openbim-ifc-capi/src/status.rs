//! Status codes, the version record, and the panic boundary.

use std::panic::{catch_unwind, AssertUnwindSafe};

use openbim_ifc_binding_core::BindingError;

/// Result of every ABI call. `Ok` is zero; every failure is non-zero.
///
/// The values from `Parse` to `FeatureDisabled`, `InvalidModel`, and
/// `MissingReference` to `CatalogNotLoaded`, are the binding errors shared
/// with the JavaScript and Python bindings; the rest describe misuse of the
/// C boundary itself.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenbimIfcStatus {
    /// Success.
    Ok = 0,
    /// A required pointer was null.
    NullPointer = 1,
    /// An argument was malformed (bad UTF-8, a malformed value tape, ...).
    InvalidArgument = 2,
    /// The model handle is zero, stale, or already destroyed.
    InvalidHandle = 3,
    /// The output buffer is smaller than `*out_required`.
    BufferTooSmall = 4,
    /// The STEP input could not be parsed (`parse`).
    Parse = 10,
    /// The model could not be serialized (`write`).
    Write = 11,
    /// No entity has the given id (`missing-entity`).
    MissingEntity = 12,
    /// A value did not follow the encoding (`invalid-value`).
    InvalidValue = 13,
    /// An id or index is outside the representable range (`out-of-range`).
    OutOfRange = 14,
    /// The file's schema is not bundled, or a domain view does not read it
    /// (`unsupported-schema`).
    UnsupportedSchema = 15,
    /// A file could not be opened or read (`io`).
    Io = 16,
    /// No ifcXML XSD profile has that name (`unsupported-profile`).
    UnsupportedProfile = 17,
    /// This build leaves out the feature the call needs (`feature-disabled`).
    FeatureDisabled = 18,
    /// A domain view refused the file's data as malformed, ambiguous or
    /// unprovable (`invalid-model`).
    InvalidModel = 19,
    /// The requested value does not exist (no schema token, no error, ...).
    NoValue = 20,
    /// A domain view followed a reference to an entity the file lacks
    /// (`missing-reference`).
    MissingReference = 21,
    /// A domain view stopped at a cycle or its depth budget
    /// (`budget-exceeded`).
    BudgetExceeded = 22,
    /// A domain view met a construct it does not interpret (`unsupported`).
    Unsupported = 23,
    /// A domain query named an entity of a type it does not accept, or a
    /// property edit a set type the set does not have (`wrong-entity-type`).
    WrongEntityType = 24,
    /// A property edit wrote a value its PSD/QTO template or property
    /// enumeration refuses (`template-violation`).
    TemplateViolation = 25,
    /// A property edit removed a property the object does not state
    /// (`missing-property`).
    MissingProperty = 26,
    /// A property edit wrote to a `Pset_`/`Qto_` set before its release's
    /// catalog was loaded (`catalog-not-loaded`). This library embeds the
    /// catalog, so it never returns this; the value is reserved so every
    /// binding code has one.
    CatalogNotLoaded = 27,
    /// A Rust panic was contained at the boundary. Report it as a bug.
    Panic = 255,
}

impl From<&BindingError> for OpenbimIfcStatus {
    fn from(error: &BindingError) -> Self {
        match error {
            BindingError::Parse(_) => Self::Parse,
            BindingError::Write(_) => Self::Write,
            BindingError::MissingEntity(_) => Self::MissingEntity,
            BindingError::InvalidValue(_) => Self::InvalidValue,
            BindingError::OutOfRange(_) => Self::OutOfRange,
            BindingError::UnsupportedSchema(_) => Self::UnsupportedSchema,
            BindingError::Io(_) => Self::Io,
            BindingError::UnsupportedProfile(_) => Self::UnsupportedProfile,
            BindingError::FeatureDisabled(_) => Self::FeatureDisabled,
            BindingError::InvalidModel(_) => Self::InvalidModel,
            BindingError::MissingReference(_) => Self::MissingReference,
            BindingError::BudgetExceeded(_) => Self::BudgetExceeded,
            BindingError::Unsupported(_) => Self::Unsupported,
            BindingError::WrongEntityType(_) => Self::WrongEntityType,
            BindingError::TemplateViolation(_) => Self::TemplateViolation,
            BindingError::MissingProperty(_) => Self::MissingProperty,
            BindingError::CatalogNotLoaded(_) => Self::CatalogNotLoaded,
        }
    }
}

/// ABI and crate versions, reported separately: the ABI version changes only
/// when the C surface does, the crate version on every release.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OpenbimIfcVersion {
    /// ABI major; also the `v0_1` in every symbol name.
    pub abi_major: u16,
    /// ABI minor.
    pub abi_minor: u16,
    /// ABI patch.
    pub abi_patch: u16,
    /// Crate major.
    pub crate_major: u16,
    /// Crate minor.
    pub crate_minor: u16,
    /// Crate patch.
    pub crate_patch: u16,
}

/// Run one export, containing any panic.
pub(crate) fn boundary(operation: impl FnOnce() -> OpenbimIfcStatus) -> OpenbimIfcStatus {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or(OpenbimIfcStatus::Panic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_is_contained() {
        assert_eq!(boundary(|| panic!("contained")), OpenbimIfcStatus::Panic);
    }

    #[test]
    fn binding_errors_keep_their_shared_meaning() {
        let cases = [
            (BindingError::Parse(String::new()), OpenbimIfcStatus::Parse),
            (BindingError::Write(String::new()), OpenbimIfcStatus::Write),
            (
                BindingError::MissingEntity(1),
                OpenbimIfcStatus::MissingEntity,
            ),
            (
                BindingError::InvalidValue(String::new()),
                OpenbimIfcStatus::InvalidValue,
            ),
            (
                BindingError::OutOfRange(String::new()),
                OpenbimIfcStatus::OutOfRange,
            ),
            (
                BindingError::UnsupportedSchema(String::new()),
                OpenbimIfcStatus::UnsupportedSchema,
            ),
            (BindingError::Io(String::new()), OpenbimIfcStatus::Io),
            (
                BindingError::UnsupportedProfile(String::new()),
                OpenbimIfcStatus::UnsupportedProfile,
            ),
            (
                BindingError::FeatureDisabled(""),
                OpenbimIfcStatus::FeatureDisabled,
            ),
            (
                BindingError::InvalidModel(String::new()),
                OpenbimIfcStatus::InvalidModel,
            ),
            (
                BindingError::MissingReference(String::new()),
                OpenbimIfcStatus::MissingReference,
            ),
            (
                BindingError::BudgetExceeded(String::new()),
                OpenbimIfcStatus::BudgetExceeded,
            ),
            (
                BindingError::Unsupported(String::new()),
                OpenbimIfcStatus::Unsupported,
            ),
            (
                BindingError::WrongEntityType(String::new()),
                OpenbimIfcStatus::WrongEntityType,
            ),
            (
                BindingError::TemplateViolation(String::new()),
                OpenbimIfcStatus::TemplateViolation,
            ),
            (
                BindingError::MissingProperty(String::new()),
                OpenbimIfcStatus::MissingProperty,
            ),
            (
                BindingError::CatalogNotLoaded(String::new()),
                OpenbimIfcStatus::CatalogNotLoaded,
            ),
        ];
        for (error, status) in cases {
            assert_eq!(OpenbimIfcStatus::from(&error), status, "{}", error.code());
        }
    }
}
