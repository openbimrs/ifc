//! Applicability selectors from PSD/QTO catalogs.

use thiserror::Error;

/// An entity selector, optionally restricted by IFC predefined type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, bincode::Encode, bincode::Decode)]
pub struct Applicability {
    /// Original selector text retained for provenance and round trips.
    pub raw: String,
    /// IFC entity name.
    pub entity: String,
    /// Optional predefined type after `/` in publication data.
    pub predefined_type: Option<String>,
}

impl Applicability {
    /// Construct an unrestricted entity selector.
    pub fn entity(entity: impl Into<String>) -> Self {
        let entity = entity.into();
        Self {
            raw: entity.clone(),
            entity,
            predefined_type: None,
        }
    }

    /// Parse `IfcEntity` or `IfcEntity/PREDEFINEDTYPE`.
    pub fn parse(raw: impl Into<String>) -> Result<Self, ApplicabilityError> {
        let raw = raw.into();
        let value = raw.trim();
        if value.is_empty() {
            return Err(ApplicabilityError::Empty);
        }
        let (entity, predefined_type) = match value.split_once('/') {
            Some((entity, predefined))
                if !entity.trim().is_empty() && !predefined.trim().is_empty() =>
            {
                if predefined.contains('/') {
                    return Err(ApplicabilityError::Invalid(raw));
                }
                (entity.trim().to_owned(), Some(predefined.trim().to_owned()))
            }
            Some(_) => return Err(ApplicabilityError::Invalid(raw)),
            None => (value.to_owned(), None),
        };
        Ok(Self {
            raw,
            entity,
            predefined_type,
        })
    }
}

/// Invalid catalog applicability syntax.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ApplicabilityError {
    #[error("applicability selector is empty")]
    Empty,
    #[error("invalid applicability selector `{0}`")]
    Invalid(String),
}
