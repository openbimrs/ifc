//! Slot decoding that records defects instead of failing on them.
//!
//! Every slot is located by attribute NAME in the declared schema, never
//! by a constant, so the read agrees with whichever of IFC4 and IFC4X3 the
//! caller declared. A slot that does not decode yields `None` and an issue;
//! the caller keeps going with the rest of the record.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::Schema;

use super::issue::TabularIssue;

/// Whether a `None` from an empty slot is itself a defect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Need {
    Required,
    Optional,
}

/// One record being read, and the issues found on it so far.
pub(crate) struct Slots<'m, 'i> {
    model: &'m Model,
    id: EntityId,
    entity: &'m Entity,
    names: Vec<&'m str>,
    issues: &'i mut Vec<TabularIssue>,
}

impl<'m, 'i> Slots<'m, 'i> {
    /// Open `entity` as a `type_name` record, reporting an arity mismatch.
    pub(crate) fn open(
        model: &'m Model,
        schema: &'m Schema,
        id: EntityId,
        entity: &'m Entity,
        type_name: &'static str,
        issues: &'i mut Vec<TabularIssue>,
    ) -> Self {
        let names = schema.attribute_names(type_name);
        if names.len() != entity.attributes.len() {
            issues.push(TabularIssue::Arity {
                entity: id,
                type_name,
                expected: names.len(),
                found: entity.attributes.len(),
            });
        }
        Self {
            model,
            id,
            entity,
            names,
            issues,
        }
    }

    /// The non-null value of `attribute`, or `None` (reported if required).
    fn slot(&mut self, attribute: &'static str, need: Need) -> Option<&'m Value> {
        let entity = self.entity;
        let value = self
            .names
            .iter()
            .position(|name| *name == attribute)
            .and_then(|slot| entity.attribute(slot))
            .filter(|value| !matches!(value, Value::Null));
        if value.is_none() && need == Need::Required {
            self.issues.push(TabularIssue::Missing {
                entity: self.id,
                attribute,
            });
        }
        value
    }

    fn malformed(&mut self, attribute: &'static str, found: &Value) {
        self.issues.push(TabularIssue::Malformed {
            entity: self.id,
            attribute,
            found: format!("{found:?}"),
        });
    }

    /// A string-valued slot (`IfcLabel`, `IfcText`, `IfcDateTime`, ...).
    pub(crate) fn text(&mut self, attribute: &'static str, need: Need) -> Option<&'m str> {
        let value = self.slot(attribute, need)?;
        let text = value.unwrap_typed().as_text();
        if text.is_none() {
            self.malformed(attribute, value);
        }
        text
    }

    /// An enumeration token.
    pub(crate) fn enumeration(&mut self, attribute: &'static str, need: Need) -> Option<&'m str> {
        let value = self.slot(attribute, need)?;
        if let Value::Enum(token) = value {
            Some(token)
        } else {
            self.malformed(attribute, value);
            None
        }
    }

    /// An `IfcBoolean`: `.U.` is a LOGICAL state, not a BOOLEAN one.
    pub(crate) fn boolean(&mut self, attribute: &'static str, need: Need) -> Option<bool> {
        let value = self.slot(attribute, need)?;
        let flag = value.unwrap_typed().as_bool();
        if flag.is_none() {
            self.malformed(attribute, value);
        }
        flag
    }

    /// A numeric measure, integer or real.
    pub(crate) fn number(&mut self, attribute: &'static str, need: Need) -> Option<f64> {
        let value = self.slot(attribute, need)?;
        let number = value.unwrap_typed().as_f64();
        if number.is_none() {
            self.malformed(attribute, value);
        }
        number
    }

    /// A single reference that must resolve; its type is not checked.
    pub(crate) fn reference(&mut self, attribute: &'static str, need: Need) -> Option<EntityId> {
        let value = self.slot(attribute, need)?;
        let Value::Ref(target) = value else {
            self.malformed(attribute, value);
            return None;
        };
        if self.model.get(*target).is_none() {
            self.issues.push(TabularIssue::Dangling {
                entity: self.id,
                attribute,
                target: *target,
            });
        }
        Some(*target)
    }

    /// A `LIST [1:?]` of inline values, borrowed as written.
    pub(crate) fn values(&mut self, attribute: &'static str, need: Need) -> Option<&'m [Value]> {
        let value = self.slot(attribute, need)?;
        let Some(items) = value.as_list() else {
            self.malformed(attribute, value);
            return None;
        };
        if items.is_empty() {
            self.issues.push(TabularIssue::EmptyList {
                entity: self.id,
                attribute,
            });
        }
        Some(items)
    }

    /// A `LIST [1:?]` of references to `expected` records.
    ///
    /// Returns one entry per list position, `None` where the member is not
    /// a reference, dangles, or names another type, so positions (which
    /// WR1 is stated over) survive a defective member.
    pub(crate) fn records(
        &mut self,
        attribute: &'static str,
        need: Need,
        expected: &'static str,
    ) -> Vec<Option<(EntityId, &'m Entity)>> {
        let model = self.model;
        let Some(items) = self.values(attribute, need) else {
            return Vec::new();
        };
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            let Value::Ref(target) = item else {
                self.malformed(attribute, item);
                out.push(None);
                continue;
            };
            let Some(record) = model.get(*target) else {
                self.issues.push(TabularIssue::Dangling {
                    entity: self.id,
                    attribute,
                    target: *target,
                });
                out.push(None);
                continue;
            };
            if !record.is_type(expected) {
                self.issues.push(TabularIssue::WrongReferenceType {
                    entity: self.id,
                    attribute,
                    target: *target,
                    expected,
                    actual: record.type_name.to_string(),
                });
                out.push(None);
                continue;
            }
            out.push(Some((*target, record)));
        }
        out
    }
}
