//! The tape's fields into the facade's operation, and the facade's
//! refusals into the shared codes.

use ifc::{AuthorOp, AuthoringError, AuthoringFailure as F, EntityId, OwnerHistoryOp};

use super::ops::{invalid, Arg, Fields};
use crate::BindingError;

pub(super) fn op(mut fields: Fields) -> Result<AuthorOp, BindingError> {
    let name = fields.spec.name;
    let mut text = |key: &str| match fields.take(key) {
        Some(Arg::Text(text)) => Some(text),
        _ => None,
    };
    // The required fields are checked by `ops::read`; a missing one here
    // is an internal disagreement, reported rather than unwrapped.
    let need = |value: Option<String>, key: &str| {
        value.ok_or_else(|| invalid(format!("`{name}` needs `{key}`")))
    };
    Ok(match name {
        "owner_history" => {
            let person_identification = text("person_identification");
            let family_name = text("family_name");
            let given_name = text("given_name");
            let organization = need(text("organization"), "organization")?;
            let application_name = need(text("application_name"), "application_name")?;
            let application_version = need(text("application_version"), "application_version")?;
            let application_identifier =
                need(text("application_identifier"), "application_identifier")?;
            let change_action = text("change_action");
            let integer = |fields: &mut Fields, key: &str| match fields.take(key) {
                Some(Arg::Integer(value)) => Some(value),
                _ => None,
            };
            let creation_date = integer(&mut fields, "creation_date")
                .ok_or_else(|| invalid("`owner_history` needs `creation_date`"))?;
            let mut history = OwnerHistoryOp::new(
                organization,
                application_name,
                application_version,
                application_identifier,
                creation_date,
            );
            history.person_identification = person_identification;
            history.family_name = family_name;
            history.given_name = given_name;
            history.change_action = change_action;
            history.last_modified_date = integer(&mut fields, "last_modified_date");
            AuthorOp::OwnerHistory(history)
        }
        "placement" => {
            let reals = |fields: &mut Fields, key: &str| match fields.take(key) {
                Some(Arg::Reals(values)) => Some(values),
                _ => None,
            };
            let location = reals(&mut fields, "location").unwrap_or([0.0; 3]);
            let axes = match (
                reals(&mut fields, "axis"),
                reals(&mut fields, "ref_direction"),
            ) {
                (Some(axis), Some(ref_direction)) => Some((axis, ref_direction)),
                (None, None) => None,
                _ => {
                    return Err(invalid(
                        "`placement` takes `axis` and `ref_direction` both or neither",
                    ))
                }
            };
            AuthorOp::Placement {
                relative_to: id(&mut fields, "relative_to"),
                location,
                axes,
            }
        }
        _ => structural(name, fields)?,
    })
}

/// The operations made of ids, a type and attributes.
fn structural(name: &str, mut fields: Fields) -> Result<AuthorOp, BindingError> {
    let type_name = match fields.take("type") {
        Some(Arg::Text(text)) => text,
        _ => String::new(),
    };
    let attributes = match fields.take("attributes") {
        Some(Arg::Attributes(pairs)) => pairs
            .into_iter()
            .map(|(key, value)| Ok((key, value.into_value()?)))
            .collect::<Result<Vec<_>, BindingError>>()?,
        _ => Vec::new(),
    };
    let owner_history = id(&mut fields, "owner_history");
    let mut required =
        |key: &str| id(&mut fields, key).ok_or_else(|| invalid(format!("`{name}` needs `{key}`")));
    Ok(match name {
        "create" => AuthorOp::Create {
            type_name,
            attributes,
        },
        "edit" => AuthorOp::Edit {
            entity: required("entity")?,
            attributes,
        },
        "remove" => AuthorOp::Remove {
            entity: required("entity")?,
        },
        "project" => AuthorOp::Project {
            attributes,
            owner_history,
        },
        "spatial" => AuthorOp::Spatial {
            type_name,
            parent: required("parent")?,
            attributes,
            placement: id(&mut fields, "placement"),
            owner_history,
        },
        "product" => AuthorOp::Product {
            type_name,
            container: id(&mut fields, "container"),
            attributes,
            placement: id(&mut fields, "placement"),
            type_object: id(&mut fields, "type_object"),
            owner_history,
        },
        "type_object" => AuthorOp::TypeObject {
            type_name,
            attributes,
            owner_history,
        },
        "assign_type" => AuthorOp::AssignType {
            type_object: required("type_object")?,
            objects: ids(&mut fields, "objects"),
            owner_history,
        },
        "contain" => AuthorOp::Contain {
            structure: required("structure")?,
            elements: ids(&mut fields, "elements"),
            owner_history,
        },
        "aggregate" => AuthorOp::Aggregate {
            parent: required("parent")?,
            parts: ids(&mut fields, "parts"),
            owner_history,
        },
        other => return Err(invalid(format!("no operation `{other}`"))),
    })
}

fn id(fields: &mut Fields, key: &str) -> Option<EntityId> {
    match fields.take(key) {
        Some(Arg::Id(id)) => Some(EntityId(id)),
        _ => None,
    }
}

fn ids(fields: &mut Fields, key: &str) -> Vec<EntityId> {
    match fields.take(key) {
        Some(Arg::Ids(ids)) => ids.into_iter().map(EntityId).collect(),
        _ => Vec::new(),
    }
}

/// A facade refusal as the shared code, its message naming the operation.
pub(super) fn error(error: AuthoringError) -> BindingError {
    use ifc::author::AuthorError as A;
    let detail = error.to_string();
    match *error.failure {
        F::UnknownEntity { .. } => BindingError::UnsupportedSchema(detail),
        F::AbstractEntity { .. } | F::NotA { .. } | F::WrongReferenceType { .. } => {
            BindingError::WrongEntityType(detail)
        }
        F::Author(author) => match author {
            A::UnknownAttribute { .. } => BindingError::UnknownAttribute(detail),
            A::DerivedAttribute { .. } => BindingError::DerivedAttribute(detail),
            A::MissingRequired { .. } => BindingError::MissingAttribute(detail),
            A::UnknownEntity { .. } => BindingError::UnsupportedSchema(detail),
            A::MissingEntity { id } => BindingError::MissingEntity(id.0),
            A::ArityMismatch { .. } => BindingError::InvalidModel(detail),
            _ => BindingError::InvalidValue(detail),
        },
        F::MissingEntity(id) => BindingError::MissingEntity(id.0),
        F::MissingReference { .. } => BindingError::MissingReference(detail),
        F::StillReferenced { .. } => BindingError::StillReferenced(detail),
        F::AlreadyRelated { .. } | F::Conflict(_) => BindingError::InvalidModel(detail),
        F::IdRangeExhausted { .. } => BindingError::OutOfRange(detail),
        F::Cardinality { .. }
        | F::DuplicateGlobalId { .. }
        | F::InvalidRelationship { .. }
        | F::InvalidHandle { .. }
        | F::InvalidPlacement { .. } => BindingError::InvalidValue(detail),
        // `AuthoringFailure` is non-exhaustive; a new refusal is reported,
        // never dropped, until it earns a code of its own.
        _ => BindingError::InvalidModel(detail),
    }
}
