//! Catalog lookup and applicability matching.

use crate::catalog::Catalog;
use crate::definition::{
    Applicability, PropertySetType, QuantitySetType, SetTemplate, SetTemplateKind,
};

/// Semantic context in which a template would be assigned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ApplicabilityContext {
    /// Values authored directly on an occurrence instance.
    Occurrence,
    /// Values authored on a type, inherited by its occurrences.
    Type,
    /// Values recording required/target performance rather than as-built state.
    PerformanceHistory,
}

/// IFC object/type tested against catalog applicability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicabilityTarget {
    /// IFC entity name of the object or type being tested.
    pub entity: String,
    /// Predefined type of the object or type being tested, if any.
    pub predefined_type: Option<String>,
    /// `None` asks only the entity/predefined-type question.
    pub context: Option<ApplicabilityContext>,
}

impl ApplicabilityTarget {
    /// Build a target with no applicability-mode context; only entity and
    /// predefined type are matched.
    pub fn new(entity: impl Into<String>, predefined_type: Option<impl Into<String>>) -> Self {
        Self {
            entity: entity.into(),
            predefined_type: predefined_type.map(Into::into),
            context: None,
        }
    }

    /// Attach an applicability-mode context, restricting matches to
    /// templates whose `set_type` supports that context.
    pub fn with_context(mut self, context: ApplicabilityContext) -> Self {
        self.context = Some(context);
        self
    }
}

/// Minimal hierarchy seam needed by catalog queries.
pub trait EntityHierarchy {
    /// True when `candidate` is `expected_supertype` or a subtype of it.
    fn is_same_or_subtype(&self, candidate: &str, expected_supertype: &str) -> bool;

    /// Return `Some(false)` when the hierarchy can prove the entity is unknown.
    fn entity_known(&self, _entity: &str) -> Option<bool> {
        None
    }
}

/// Exact entity matching when schema metadata is unavailable.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExactEntityHierarchy;

impl EntityHierarchy for ExactEntityHierarchy {
    fn is_same_or_subtype(&self, candidate: &str, expected_supertype: &str) -> bool {
        candidate.eq_ignore_ascii_case(expected_supertype)
    }
}

#[cfg(feature = "schema")]
impl EntityHierarchy for ifc_schema::Schema {
    fn is_same_or_subtype(&self, candidate: &str, expected_supertype: &str) -> bool {
        self.is_a(candidate, expected_supertype)
    }

    fn entity_known(&self, entity: &str) -> Option<bool> {
        Some(self.entity(entity).is_some())
    }
}

/// One selector that could not be evaluated because schema data was missing.
#[derive(Debug, Clone, Copy)]
pub struct UnresolvedApplicability<'a> {
    /// The template whose selector could not be resolved.
    pub template: &'a SetTemplate,
    /// The specific applicability selector that was unresolved.
    pub selector: &'a Applicability,
}

/// Structured applicability result; unknown schema entities are not conflated with no-match.
#[derive(Debug, Default)]
pub struct ApplicabilityQuery<'a> {
    /// Templates whose applicability was proven to match the target.
    pub matches: Vec<&'a SetTemplate>,
    /// Selectors that could not be resolved because the hierarchy could not
    /// confirm whether an involved entity name exists.
    pub unresolved: Vec<UnresolvedApplicability<'a>>,
}

impl Catalog {
    /// Return templates applicable to an entity and optional predefined type.
    ///
    /// Unknown schema entities are omitted. Use [`Catalog::query_applicability`]
    /// when that distinction matters.
    pub fn applicable_to<'a>(
        &'a self,
        target: &ApplicabilityTarget,
        hierarchy: &impl EntityHierarchy,
    ) -> Vec<&'a SetTemplate> {
        self.query_applicability(target, hierarchy).matches
    }

    /// Query applicability while preserving unknown-schema outcomes.
    pub fn query_applicability<'a>(
        &'a self,
        target: &ApplicabilityTarget,
        hierarchy: &impl EntityHierarchy,
    ) -> ApplicabilityQuery<'a> {
        let mut result = ApplicabilityQuery::default();
        for template in self.iter().filter(|item| context_matches(item, target)) {
            let mut matched = false;
            for selector in &template.applicability {
                match selector_match(selector, target, hierarchy) {
                    SelectorMatch::Match => matched = true,
                    SelectorMatch::NoMatch => {}
                    SelectorMatch::Unknown => result
                        .unresolved
                        .push(UnresolvedApplicability { template, selector }),
                }
            }
            if matched {
                result.matches.push(template);
            }
        }
        result
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectorMatch {
    Match,
    NoMatch,
    Unknown,
}

fn selector_match(
    selector: &Applicability,
    target: &ApplicabilityTarget,
    hierarchy: &impl EntityHierarchy,
) -> SelectorMatch {
    if !hierarchy.is_same_or_subtype(&target.entity, &selector.entity) {
        if matches!(hierarchy.entity_known(&target.entity), Some(false))
            || matches!(hierarchy.entity_known(&selector.entity), Some(false))
        {
            return SelectorMatch::Unknown;
        }
        return SelectorMatch::NoMatch;
    }
    match (&selector.predefined_type, &target.predefined_type) {
        (None, _) => SelectorMatch::Match,
        (Some(expected), Some(actual)) if expected.eq_ignore_ascii_case(actual) => {
            SelectorMatch::Match
        }
        _ => SelectorMatch::NoMatch,
    }
}

fn context_matches(template: &SetTemplate, target: &ApplicabilityTarget) -> bool {
    let Some(context) = target.context else {
        return true;
    };
    match &template.kind {
        SetTemplateKind::Property { set_type, .. } => match set_type {
            PropertySetType::TypeDrivenOverride => matches!(
                context,
                ApplicabilityContext::Occurrence | ApplicabilityContext::Type
            ),
            PropertySetType::TypeDrivenOnly => matches!(context, ApplicabilityContext::Type),
            PropertySetType::OccurrenceDriven => {
                matches!(context, ApplicabilityContext::Occurrence)
            }
            PropertySetType::PerformanceDriven => {
                matches!(context, ApplicabilityContext::PerformanceHistory)
            }
            PropertySetType::Unspecified => true,
        },
        SetTemplateKind::Quantity { set_type, .. } => match set_type {
            QuantitySetType::TypeDrivenOverride => matches!(
                context,
                ApplicabilityContext::Occurrence | ApplicabilityContext::Type
            ),
            QuantitySetType::TypeDrivenOnly => matches!(context, ApplicabilityContext::Type),
            QuantitySetType::OccurrenceDriven => {
                matches!(context, ApplicabilityContext::Occurrence)
            }
            QuantitySetType::Unspecified => true,
        },
    }
}
