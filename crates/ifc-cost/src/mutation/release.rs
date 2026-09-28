//! The IFC release cost authoring writes against (#190, #202).
//!
//! The same binding `ifc-properties` quantity authoring uses, so the two
//! writers of `IfcQuantity*` agree in every release (the facade test
//! `quantity_writer_agreement.rs` compares them byte for byte). Sibling
//! crates cannot share the code, only the rule, from `FILE_SCHEMA`:
//! - one recognised declaration binds that release's bundled table;
//! - one unrecognised declaration fails with
//!   [`CostAuthoringError::UnsupportedSchema`];
//! - several fail with [`CostAuthoringError::MultipleSchemas`];
//! - none at all (an in-memory [`Model::new`]) binds IFC4.
//!
//! Attributes are placed by name from that table, so an IFC2X3 quantity has
//! four attributes and no `Formula`, `IfcQuantityNumber` exists only in
//! IFC4X3, and an IFC2X3 `IfcCostItem` has five attributes and a required
//! `IfcRoot.OwnerHistory` (#202). A value the release cannot hold is
//! refused, never dropped, and a required one is never invented.

use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_schema::{for_version, Attribute, Schema, SchemaVersion, TypeKind};

use super::validate::projected_type;
use super::{CostAuthoringError, CostAuthoringResult};

/// The bound release and its table.
#[derive(Debug, Clone, Copy)]
pub(super) struct Release {
    version: SchemaVersion,
    schema: &'static Schema,
}

/// Bind `model`'s declared release.
pub(super) fn bind(model: &Model) -> CostAuthoringResult<Release> {
    let version = match model.header().schema.as_slice() {
        [] => SchemaVersion::Ifc4,
        [token] => SchemaVersion::from_header_token(token).ok_or_else(|| {
            CostAuthoringError::UnsupportedSchema {
                schema: token.clone(),
            }
        })?,
        tokens => {
            return Err(CostAuthoringError::MultipleSchemas {
                schemas: tokens.len(),
            })
        }
    };
    let schema = for_version(version).ok_or_else(|| CostAuthoringError::UnsupportedSchema {
        schema: format!("{version:?}"),
    })?;
    Ok(Release { version, schema })
}

impl Release {
    /// The bound table.
    pub(super) const fn schema(self) -> &'static Schema {
        self.schema
    }

    /// Fail with `EntityNotInSchema` unless the release instantiates
    /// `entity`.
    pub(super) fn require_entity(self, entity: &'static str) -> CostAuthoringResult<()> {
        if self.schema.entity(entity).is_none_or(|e| e.abstract_) {
            return Err(CostAuthoringError::EntityNotInSchema {
                entity,
                schema: self.version,
            });
        }
        Ok(())
    }

    /// Position and declaration of the attribute this crate knows by its
    /// IFC4 name, or `AuthoringNotInSchema`.
    pub(super) fn declared(
        self,
        entity: &'static str,
        attribute: &'static str,
    ) -> CostAuthoringResult<(usize, &'static Attribute)> {
        self.require_entity(entity)?;
        let name = release_name(self.version, entity, attribute);
        self.schema
            .attributes(entity)
            .into_iter()
            .enumerate()
            .find(|(_, declared)| declared.name.eq_ignore_ascii_case(name))
            .ok_or(CostAuthoringError::AuthoringNotInSchema {
                entity,
                attribute,
                schema: self.version,
            })
    }

    /// Build a record of `entity` in this release's layout from values named
    /// by their IFC4 attribute names.
    ///
    /// A value for an attribute the release does not declare is refused
    /// (`AuthoringNotInSchema`) unless it is `$`; one the declaration cannot
    /// hold, such as text where IFC2X3 declares an `IfcDateTimeSelect`, is
    /// `AuthoringValueType`; a `$` in a required slot, such as the IFC2X3
    /// `IfcRoot.OwnerHistory`, is `AuthoringRequired`. Unnamed optional
    /// slots are `$`.
    pub(super) fn record(
        self,
        entity: &'static str,
        values: Vec<(&'static str, Value)>,
    ) -> CostAuthoringResult<Entity> {
        self.require_entity(entity)?;
        let declared = self.schema.attributes(entity);
        let mut attributes = vec![Value::Null; declared.len()];
        for (attribute, value) in values {
            let (slot, declaration) = match self.declared(entity, attribute) {
                Ok(found) => found,
                Err(_) if value == Value::Null => continue,
                Err(error) => return Err(error),
            };
            if !conforms(self.schema, declaration, &value) {
                return Err(CostAuthoringError::AuthoringValueType {
                    entity,
                    attribute,
                    declared: declaration.type_name.as_str(),
                    schema: self.version,
                });
            }
            attributes[slot] = value;
        }
        if let Some((declaration, _)) = declared
            .iter()
            .zip(&attributes)
            .find(|(declaration, value)| !declaration.optional && **value == Value::Null)
        {
            return Err(CostAuthoringError::AuthoringRequired {
                entity,
                attribute: declaration.name.as_str(),
                schema: self.version,
            });
        }
        Ok(Entity::new(entity, attributes))
    }

    /// Fail unless `id` is, in the model or staged on `tx`, a record the
    /// release accepts as `entity.OwnerHistory`: an `IfcOwnerHistory`.
    pub(super) fn require_owner_history(
        self,
        tx: &Transaction,
        model: &Model,
        entity: &'static str,
        id: EntityId,
    ) -> CostAuthoringResult<()> {
        let actual = projected_type(tx, model, id).ok_or(CostAuthoringError::MissingReference {
            entity,
            attribute: "OwnerHistory",
            target: id,
        })?;
        let (_, declaration) = self.declared(entity, "OwnerHistory")?;
        if self.schema.accepts_type(&declaration.type_name, &actual) {
            return Ok(());
        }
        Err(CostAuthoringError::WrongReferenceType {
            entity,
            attribute: "OwnerHistory",
            target: id,
            actual,
            expected: "IFCOWNERHISTORY",
        })
    }
}

/// The name `release` gives the attribute this crate knows by its IFC4
/// name.
///
/// IFC4 renamed the IFC2X3 `IfcCostSchedule.ID` to `Identification` and
/// promoted it to `IfcControl`. The IFC4 ADD2 TC1 documentation of
/// `IfcCostSchedule` states: "Attribute ID renamed to Identification and
/// promoted to supertype IfcControl". It keeps its meaning, the schedule's
/// identifier, which IFC2X3 requires. IFC2X3 `IfcCostItem` declares no
/// identifier at all.
fn release_name(release: SchemaVersion, entity: &str, attribute: &'static str) -> &'static str {
    match (release, entity, attribute) {
        (SchemaVersion::Ifc2x3, "IFCCOSTSCHEDULE", "Identification") => "ID",
        _ => attribute,
    }
}

/// Whether `value` is a legal instance of `declared` in `schema`.
fn conforms(schema: &Schema, declared: &Attribute, value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::List(items) => {
            declared.aggregate
                && items
                    .iter()
                    .all(|item| scalar(schema, &declared.type_name, item))
        }
        _ => !declared.aggregate && scalar(schema, &declared.type_name, value),
    }
}

/// Whether a single (non-aggregate) `value` is a legal `declared`.
fn scalar(schema: &Schema, declared: &str, value: &Value) -> bool {
    let base = schema.resolve_defined(declared).to_ascii_uppercase();
    let kind = schema.type_def(declared).map(|t| &t.kind);
    match value {
        Value::Text(_) => base.starts_with("STRING"),
        Value::Enum(token) => matches!(kind, Some(TypeKind::Enumeration(members))
            if members.iter().any(|m| m.eq_ignore_ascii_case(token))),
        Value::Bool(_) => base == "BOOLEAN" || base == "LOGICAL",
        Value::Integer(_) => base == "INTEGER" || base == "NUMBER",
        Value::Real(_) => base.starts_with("REAL") || base == "NUMBER",
        Value::Ref(_) => admits_entity(schema, declared, 8),
        Value::Typed { type_name, .. } => {
            matches!(kind, Some(TypeKind::Select(_))) && schema.accepts_type(declared, type_name)
        }
        _ => false,
    }
}

/// Whether `declared` is an entity, or a SELECT that reaches one.
fn admits_entity(schema: &Schema, declared: &str, depth: usize) -> bool {
    if schema.entity(declared).is_some() {
        return true;
    }
    match schema.type_def(declared).map(|t| &t.kind) {
        Some(TypeKind::Select(members)) if depth > 0 => members
            .iter()
            .any(|member| admits_entity(schema, member, depth - 1)),
        _ => false,
    }
}
