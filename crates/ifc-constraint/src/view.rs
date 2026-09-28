//! Shared borrowed model view and strict by-name decoders.

use std::collections::HashSet;

use ifc_model::{Entity, EntityId, Model, Value};

use crate::release::{Binding, Layout};
use crate::{ConstraintError, ConstraintResult};

/// Borrowed entry point for constraint projections and inverse-style queries.
///
/// Every projection reads against the release the model's header declares
/// (IFC2X3, IFC4 or IFC4X3; none reads as IFC4). A header this crate cannot
/// bind surfaces as [`ConstraintError::UnsupportedSchema`] or
/// [`ConstraintError::MultipleSchemas`] from the first lookup.
#[derive(Debug, Clone, Copy)]
pub struct ConstraintView<'m> {
    model: &'m Model,
    binding: Binding<'m>,
}

impl<'m> ConstraintView<'m> {
    /// Borrow constraint semantics from a model snapshot.
    #[must_use]
    pub fn new(model: &'m Model) -> Self {
        Self {
            model,
            binding: Binding::of(model),
        }
    }

    pub(crate) const fn model(self) -> &'m Model {
        self.model
    }

    /// The bound release, or why none could be bound.
    pub(crate) fn layout(self) -> ConstraintResult<Layout> {
        self.binding.layout()
    }
}

pub(crate) fn wrong(expected: &'static str, entity: &Entity) -> ConstraintError {
    ConstraintError::WrongEntityType {
        expected,
        actual: entity.type_name.to_string(),
    }
}

pub(crate) fn invalid(
    kind: &'static str,
    id: EntityId,
    attribute: &'static str,
    value: &Value,
) -> ConstraintError {
    ConstraintError::InvalidValue {
        entity: kind,
        id,
        attribute,
        value: format!("{value:?}"),
    }
}

/// One record read against its release.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Record<'m> {
    pub(crate) kind: &'static str,
    pub(crate) id: EntityId,
    pub(crate) entity: &'m Entity,
    pub(crate) layout: Layout,
}

impl<'m> Record<'m> {
    /// The value of `attribute` (IFC4 name): `NotInSchema` when the release
    /// does not declare it, `None` when the record leaves it `$` or ends
    /// before it.
    pub(crate) fn value(self, attribute: &'static str) -> ConstraintResult<Option<&'m Value>> {
        let (slot, _) =
            self.layout
                .declared(self.kind, attribute)
                .ok_or(ConstraintError::NotInSchema {
                    entity: self.kind,
                    id: self.id,
                    attribute,
                    schema: self.layout.version(),
                })?;
        Ok(match self.entity.attribute(slot) {
            None | Some(Value::Null) => None,
            Some(value) => Some(value),
        })
    }

    /// Whether the release declares `attribute` (IFC4 name).
    pub(crate) fn declares(self, attribute: &'static str) -> bool {
        self.layout.declared(self.kind, attribute).is_some()
    }

    /// Whether the release types `attribute` as an entity record (or a
    /// SELECT reaching one), such as IFC2X3's `IfcDateTimeSelect`.
    pub(crate) fn admits_record(self, attribute: &'static str) -> bool {
        self.layout
            .declared(self.kind, attribute)
            .is_some_and(|(_, declared)| self.layout.admits_entity(&declared.type_name, 8))
    }

    /// Whether the release declares `attribute` as an aggregate.
    pub(crate) fn aggregate(self, attribute: &'static str) -> bool {
        self.layout
            .declared(self.kind, attribute)
            .is_some_and(|(_, declared)| declared.aggregate)
    }

    /// The (element) type the release declares for `attribute`, or `""`.
    pub(crate) fn declared_type(self, attribute: &'static str) -> &'static str {
        self.layout
            .declared(self.kind, attribute)
            .map_or("", |(_, declared)| declared.type_name.as_str())
    }

    /// Whether the release requires `attribute`.
    pub(crate) fn requires(self, attribute: &'static str) -> bool {
        self.layout
            .declared(self.kind, attribute)
            .is_some_and(|(_, declared)| !declared.optional)
    }

    pub(crate) fn optional_text(
        self,
        attribute: &'static str,
    ) -> ConstraintResult<Option<&'m str>> {
        match self.value(attribute)? {
            None => Ok(None),
            Some(Value::Ref(target)) if self.admits_record(attribute) => {
                Err(ConstraintError::StructuredValue {
                    entity: self.kind,
                    id: self.id,
                    attribute,
                    target: *target,
                })
            }
            Some(value) => value
                .unwrap_typed()
                .as_text()
                .map(Some)
                .ok_or_else(|| invalid(self.kind, self.id, attribute, value)),
        }
    }

    pub(crate) fn required_text(self, attribute: &'static str) -> ConstraintResult<&'m str> {
        self.optional_text(attribute)?
            .ok_or(ConstraintError::MissingAttribute {
                entity: self.kind,
                id: self.id,
                attribute,
            })
    }

    pub(crate) fn optional_ref(
        self,
        attribute: &'static str,
    ) -> ConstraintResult<Option<EntityId>> {
        match self.value(attribute)? {
            None => Ok(None),
            Some(Value::Ref(target)) => Ok(Some(*target)),
            Some(value) => Err(invalid(self.kind, self.id, attribute, value)),
        }
    }

    pub(crate) fn required_ref(self, attribute: &'static str) -> ConstraintResult<EntityId> {
        self.optional_ref(attribute)?
            .ok_or(ConstraintError::MissingAttribute {
                entity: self.kind,
                id: self.id,
                attribute,
            })
    }

    /// A non-empty, duplicate-free set of references. Where the release
    /// declares a single reference, that one reference is the set.
    pub(crate) fn required_refs(self, attribute: &'static str) -> ConstraintResult<Vec<EntityId>> {
        let value = self
            .value(attribute)?
            .ok_or(ConstraintError::MissingAttribute {
                entity: self.kind,
                id: self.id,
                attribute,
            })?;
        let single = self
            .layout
            .declared(self.kind, attribute)
            .is_some_and(|(_, declared)| !declared.aggregate);
        let values = match value {
            Value::Ref(target) if single => return Ok(vec![*target]),
            Value::List(values) if !single && !values.is_empty() => values,
            _ => return Err(invalid(self.kind, self.id, attribute, value)),
        };
        let mut seen = HashSet::new();
        let mut out = Vec::with_capacity(values.len());
        for item in values {
            let Value::Ref(target) = item else {
                return Err(invalid(self.kind, self.id, attribute, item));
            };
            if !seen.insert(*target) {
                return Err(ConstraintError::InvalidValue {
                    entity: self.kind,
                    id: self.id,
                    attribute,
                    value: format!("duplicate {target}"),
                });
            }
            out.push(*target);
        }
        Ok(out)
    }

    /// Fail unless `target` resolves to a type the release accepts as
    /// `expected`.
    pub(crate) fn validate_target(
        self,
        model: &Model,
        attribute: &'static str,
        target: EntityId,
        expected: &'static str,
    ) -> ConstraintResult<()> {
        let actual = model
            .get(target)
            .ok_or(ConstraintError::DanglingReference {
                entity: self.kind,
                id: self.id,
                attribute,
                target,
            })?;
        let accepted = if expected == "IfcDefinitionSelect" {
            self.layout.is_definition(&actual.type_name)
        } else {
            self.layout
                .schema()
                .accepts_type(expected, &actual.type_name)
        };
        if accepted {
            Ok(())
        } else {
            Err(ConstraintError::ReferenceType {
                entity: self.kind,
                id: self.id,
                attribute,
                target,
                expected,
                actual: actual.type_name.to_string(),
            })
        }
    }
}
