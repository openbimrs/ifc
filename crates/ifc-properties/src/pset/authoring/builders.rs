//! Constructors and builder setters for the drafts of the parent module,
//! kept apart so the writer stays under the 800-line limit.

#[allow(clippy::wildcard_imports)]
use super::*;

impl<'a> TableValueDraft<'a> {
    /// Starts a draft from its required `name`; every other field is unset.
    #[must_use]
    pub fn new(name: &'a str) -> Self {
        Self {
            name,
            specification: None,
            defining: None,
            defined: None,
            defining_unit: None,
            defined_unit: None,
            expression: None,
            interpolation: None,
        }
    }

    /// Sets `specification`.
    ///
    /// `Specification`.
    #[must_use]
    pub fn specification(mut self, value: &'a str) -> Self {
        self.specification = Some(value);
        self
    }

    /// Sets `defining`.
    ///
    /// `DefiningValues`, the independent variable.
    #[must_use]
    pub fn defining(mut self, value: Vec<Value>) -> Self {
        self.defining = Some(value);
        self
    }

    /// Sets `defined`.
    ///
    /// `DefinedValues`, the dependent variable.
    #[must_use]
    pub fn defined(mut self, value: Vec<Value>) -> Self {
        self.defined = Some(value);
        self
    }

    /// Sets `defining_unit`.
    ///
    /// `DefiningUnit`.
    #[must_use]
    pub fn defining_unit(mut self, value: EntityId) -> Self {
        self.defining_unit = Some(value);
        self
    }

    /// Sets `defined_unit`.
    ///
    /// `DefinedUnit`.
    #[must_use]
    pub fn defined_unit(mut self, value: EntityId) -> Self {
        self.defined_unit = Some(value);
        self
    }

    /// Sets `expression`.
    ///
    /// `Expression`, the closed form when one exists.
    #[must_use]
    pub fn expression(mut self, value: &'a str) -> Self {
        self.expression = Some(value);
        self
    }

    /// Sets `interpolation`.
    ///
    /// `CurveInterpolation`, an `IfcCurveInterpolationEnum` token.
    #[must_use]
    pub fn interpolation(mut self, value: &'a str) -> Self {
        self.interpolation = Some(value);
        self
    }
}
