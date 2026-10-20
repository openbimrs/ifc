//! The tape's fields into the facade's operation, and the facade's
//! refusals into the shared codes.

use ifc::{
    AssignTypeOptions, AuthorOp, AuthoringError, AuthoringFailure as F, CopyPlacement,
    CopyPolicy, CopyProductOptions, EntityId, LayerAxis, OwnerHistoryOp, PredefinedTypeMode,
    RelateOptions,
};

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
        "shape_representation" => {
            let identifier = text("identifier");
            let representation_type = text("representation_type");
            AuthorOp::ShapeRepresentation {
                context: id(&mut fields, "context")
                    .ok_or_else(|| invalid("`shape_representation` needs `context`"))?,
                items: ids(&mut fields, "items"),
                identifier,
                representation_type,
            }
        }
        "copy_product" => copy_product(&mut fields)?,
        "copy_entity" => {
            let texts = |fields: &mut Fields, key: &str| match fields.take(key) {
                Some(Arg::Texts(texts)) => texts,
                _ => Vec::new(),
            };
            let entity = id(&mut fields, "entity")
                .ok_or_else(|| invalid("`copy_entity` needs `entity`"))?;
            AuthorOp::CopyEntity {
                entity,
                policy: CopyPolicy::new()
                    .copy(texts(&mut fields, "copy"))
                    .share(texts(&mut fields, "share"))
                    .copy_roots(flag(&mut fields, "copy_roots"))
                    .copy_definitions(flag(&mut fields, "copy_definitions")),
            }
        }
        _ => structural(name, fields)?,
    })
}

/// `copy_product` (#479): its flags default to on, so only an explicit
/// `false` turns one off.
fn copy_product(fields: &mut Fields) -> Result<AuthorOp, BindingError> {
    let product =
        id(fields, "product").ok_or_else(|| invalid("`copy_product` needs `product`"))?;
    let offset = match fields.take("offset") {
        Some(Arg::Reals(offset)) => Some(offset),
        _ => None,
    };
    let placement = match (offset, id(fields, "placement")) {
        (Some(_), Some(_)) => {
            return Err(invalid(
                "`copy_product` takes `offset` or `placement`, not both",
            ))
        }
        (Some(offset), None) => CopyPlacement::Offset(offset),
        (None, Some(placement)) => CopyPlacement::At(placement),
        (None, None) => CopyPlacement::Same,
    };
    let on = |fields: &mut Fields, key: &str| !matches!(fields.take(key), Some(Arg::Bool(false)));
    Ok(AuthorOp::CopyProduct {
        product,
        options: CopyProductOptions::new()
            .placement(placement)
            .openings(on(fields, "openings"))
            .fillings(on(fields, "fillings"))
            .container(on(fields, "container"))
            .aggregate(on(fields, "aggregate")),
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
        "assign_type" => {
            let type_object = required("type_object")?;
            AuthorOp::AssignType {
                type_object,
                objects: ids(&mut fields, "objects"),
                owner_history,
                options: assign_type_options(&mut fields)?,
            }
        }
        "contain" => {
            let structure = required("structure")?;
            AuthorOp::Contain {
                structure,
                elements: ids(&mut fields, "elements"),
                owner_history,
                options: RelateOptions::new().replace(flag(&mut fields, "replace")),
            }
        }
        "aggregate" => {
            let parent = required("parent")?;
            AuthorOp::Aggregate {
                parent,
                parts: ids(&mut fields, "parts"),
                owner_history,
                options: RelateOptions::new().replace(flag(&mut fields, "replace")),
            }
        }
        "assign_representation" => AuthorOp::AssignRepresentation {
            representation: required("representation")?,
            to: required("to")?,
        },
        "unassign_representation" => AuthorOp::UnassignRepresentation {
            representation: required("representation")?,
            from: required("from")?,
        },
        "copy_representation" => AuthorOp::CopyRepresentation {
            representation: required("representation")?,
        },
        "remove_representation" => AuthorOp::RemoveRepresentation {
            representation: required("representation")?,
        },
        other => return Err(invalid(format!("no operation `{other}`"))),
    })
}

/// The propagation options of `assign_type` (#480).
fn assign_type_options(fields: &mut Fields) -> Result<AssignTypeOptions, BindingError> {
    let text = |fields: &mut Fields, key: &str| match fields.take(key) {
        Some(Arg::Text(text)) => Some(text),
        _ => None,
    };
    let direction = text(fields, "layer_set_direction")
        .map(|name| {
            LayerAxis::parse(&name).ok_or_else(|| {
                invalid(format!(
                    "`assign_type.layer_set_direction` `{name}` is not AXIS1, AXIS2 or AXIS3"
                ))
            })
        })
        .transpose()?;
    let mode = text(fields, "predefined_type")
        .map(|name| {
            PredefinedTypeMode::parse(&name).ok_or_else(|| {
                invalid(format!(
                    "`assign_type.predefined_type` `{name}` is not ignore, validate or sync"
                ))
            })
        })
        .transpose()?
        .unwrap_or_default();
    Ok(AssignTypeOptions::new()
        .map_representations(flag(fields, "map_representations"))
        .mapping_target(id(fields, "mapping_target"))
        .material_usage(flag(fields, "material_usage"))
        .layer_set_direction(direction)
        .predefined_type(mode)
        .replace(flag(fields, "replace")))
}

fn flag(fields: &mut Fields, key: &str) -> bool {
    matches!(fields.take(key), Some(Arg::Bool(true)))
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
        F::UnknownEntity { .. } | F::IncompatibleRelease { .. } => {
            BindingError::UnsupportedSchema(detail)
        }
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
        F::StillReferenced { .. }
        | F::PlacementHasDependents { .. }
        | F::TypeHasOccurrences { .. } => BindingError::StillReferenced(detail),
        F::UnsupportedRebase { .. }
        | F::UnsupportedCopy { .. }
        | F::UnitMismatch { .. }
        | F::UnsupportedRepresentationMap { .. }
        | F::MaterialUsageUndetermined { .. }
        | F::UndecidableRepresentationType { .. } => BindingError::Unsupported(detail),
        F::AlreadyRelated { .. }
        | F::RepresentationConflict { .. }
        | F::PredefinedTypeConflict { .. }
        | F::Conflict(_)
        | F::SharedProductShape { .. }
        | F::NotAssigned { .. } => BindingError::InvalidModel(detail),
        F::RepresentationTypeMismatch { .. }
        | F::NoRepresentationType { .. }
        | F::MissingRepresentationIdentifier { .. }
        | F::InvalidRepresentationItem { .. } => BindingError::InvalidValue(detail),
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
