//! Exact enumerated, list, bounded, table and reference values (#150).
//!
//! Each kind is read by attribute name from the bound release's table, so a
//! slot the release does not declare is never read and a slot it declares
//! `OPTIONAL` is the only one that may be `$`:
//!
//! ```text
//! IFC2X3 TC1                              IFC4 ADD2 TC1 / IFC4X3 ADD2
//! EnumeratedValue  2 EnumerationValues    2 OPTIONAL EnumerationValues
//!                  3 OPT EnumerationReference   (same)
//! ListValue        2 ListValues           2 OPTIONAL ListValues
//!                  3 OPT Unit                   (same)
//! BoundedValue     2 OPT UpperBoundValue  3 OPT LowerBoundValue  4 OPT Unit
//!                                         5 OPTIONAL SetPointValue (IFC4+)
//! TableValue       2 DefiningValues       2 OPTIONAL DefiningValues
//!                  3 DefinedValues        3 OPTIONAL DefinedValues
//!                  4 OPT Expression  5 OPT DefiningUnit  6 OPT DefinedUnit
//!                                         7 OPTIONAL CurveInterpolation (IFC4+)
//! ReferenceValue   2 OPT UsageName        3 PropertyReference
//!                                         3 OPTIONAL PropertyReference (IFC4+)
//! ```
//!
//! Every value in a list is an `IfcValue` accepted as a single value's
//! `NominalValue` is. The WHERE rules that decide how the values are read
//! (one type per list, bounds of one type, table columns of equal length,
//! selected values drawn from the referenced enumeration) and the `UNIQUE`
//! list constraints are checked; a violation is
//! [`ExactPropertyError::InconsistentValues`], never a partial answer.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::{Attribute, SchemaVersion, TypeKind};

use super::refs::text_at;
use super::release::Release;
use super::value::{typed_value, unit_at, ResolvedValue};
use super::{
    ExactBoundedValue, ExactEntityRef, ExactEnumeratedValue, ExactEnumeration, ExactPropertyError,
    ExactReferenceValue, ExactTableRow, ExactTableValue, ExactTypedValue, ExactValue,
};

/// The value of a property of one of the five kinds, or `None` for any
/// other kind.
///
/// # Errors
///
/// Any structural, value, unit or rule violation of that kind.
pub(super) fn composite_value(
    model: &Model,
    release: Release,
    id: EntityId,
    entity: &Entity,
) -> Result<Option<ResolvedValue>, ExactPropertyError> {
    let reader = Reader {
        model,
        release,
        id,
        entity,
    };
    let resolved = if entity.is_type("IFCPROPERTYENUMERATEDVALUE") {
        reader.enumerated()?
    } else if entity.is_type("IFCPROPERTYLISTVALUE") {
        reader.list()?
    } else if entity.is_type("IFCPROPERTYBOUNDEDVALUE") {
        reader.bounded()?
    } else if entity.is_type("IFCPROPERTYTABLEVALUE") {
        reader.table()?
    } else if entity.is_type("IFCPROPERTYREFERENCEVALUE") {
        reader.reference()?
    } else {
        return Ok(None);
    };
    Ok(Some(resolved))
}

/// One property record whose arity the release already confirmed.
struct Reader<'m> {
    model: &'m Model,
    release: Release,
    id: EntityId,
    entity: &'m Entity,
}

impl<'m> Reader<'m> {
    /// The slot and declaration of attribute `name`, if the release
    /// declares it for this entity.
    fn declared(&self, name: &str) -> Option<(&'m Value, &'static Attribute)> {
        let (slot, attribute) = self.release.attribute(&self.entity.type_name, name)?;
        Some((&self.entity.attributes[slot], attribute))
    }

    /// An attribute every bundled release declares for this entity.
    fn slot(&self, name: &str) -> (&'m Value, &'static Attribute) {
        self.declared(name)
            .unwrap_or_else(|| panic!("{}.{name} is in every release", self.entity.type_name))
    }

    /// A rule label: IFC2X3 TC1 labels its WHERE rules differently from
    /// IFC4 ADD2 TC1 and IFC4X3 ADD2, which agree for every rule read here.
    fn rule(&self, ifc2x3: &'static str, later: &'static str) -> ExactPropertyError {
        self.inconsistent(self.id, ifc2x3, later)
    }

    fn inconsistent(
        &self,
        entity: EntityId,
        ifc2x3: &'static str,
        later: &'static str,
    ) -> ExactPropertyError {
        let rule = if self.release.version == SchemaVersion::Ifc2x3 {
            ifc2x3
        } else {
            later
        };
        ExactPropertyError::InconsistentValues { entity, rule }
    }

    fn enumerated(&self) -> Result<ResolvedValue, ExactPropertyError> {
        let values = typed_list(self.release, self.id, self.slot("EnumerationValues"))?;
        let (reference, attribute) = self.slot("EnumerationReference");
        let (enumeration, unit_id) = match reference {
            Value::Null => (None, None),
            Value::Ref(target) => {
                let entity = self.target(*target, attribute)?;
                let (enumeration, unit) = self.enumeration(*target, entity)?;
                (Some(enumeration), unit)
            }
            _ => return Err(ExactPropertyError::UnsupportedValue { property: self.id }),
        };
        if let Some(enumeration) = &enumeration {
            if !values
                .iter()
                .all(|value| enumeration.values.iter().any(|v| same(v, value)))
            {
                return Err(self.rule("WR1", "WR21"));
            }
        }
        Ok(composite(
            ExactValue::Enumerated(ExactEnumeratedValue {
                values,
                enumeration,
            }),
            unit_id,
        ))
    }

    /// A referenced `IfcPropertyEnumeration` and its `Unit`.
    fn enumeration(
        &self,
        id: EntityId,
        entity: &Entity,
    ) -> Result<(ExactEnumeration, Option<EntityId>), ExactPropertyError> {
        let reader = Reader {
            model: self.model,
            release: self.release,
            id,
            entity,
        };
        let (name, _) = reader.slot("Name");
        let name = text_at(id, Some(name), "Name")?;
        let values = typed_list(self.release, id, reader.slot("EnumerationValues"))?;
        if !unique(&values) {
            return Err(ExactPropertyError::InconsistentValues {
                entity: id,
                rule: "EnumerationValues UNIQUE",
            });
        }
        if !one_type(&values) {
            return Err(self.inconsistent(id, "WR01", "WR01"));
        }
        let unit = unit_at(self.model, self.release, id, reader.slot("Unit").0)?;
        let enumeration = ExactEnumeration {
            id,
            name: name.into(),
            values,
        };
        Ok((enumeration, unit))
    }

    fn list(&self) -> Result<ResolvedValue, ExactPropertyError> {
        let values = typed_list(self.release, self.id, self.slot("ListValues"))?;
        if !one_type(&values) {
            return Err(self.rule("WR31", "WR31"));
        }
        let unit = unit_at(self.model, self.release, self.id, self.slot("Unit").0)?;
        Ok(composite(ExactValue::List(values), unit))
    }

    fn bounded(&self) -> Result<ResolvedValue, ExactPropertyError> {
        let upper = self.optional_value(self.slot("UpperBoundValue").0)?;
        let lower = self.optional_value(self.slot("LowerBoundValue").0)?;
        let set_point = match self.declared("SetPointValue") {
            Some((value, _)) => self.optional_value(value)?,
            None => None,
        };
        let unit = unit_at(self.model, self.release, self.id, self.slot("Unit").0)?;
        let differ = |a: &Option<ExactTypedValue>, b: &Option<ExactTypedValue>| matches!((a, b), (Some(a), Some(b)) if !a.value_type.eq_ignore_ascii_case(&b.value_type));
        if differ(&upper, &lower) {
            return Err(self.rule("WR21", "SameUnitUpperLower"));
        }
        if differ(&upper, &set_point) {
            return Err(self.rule("WR21", "SameUnitUpperSet"));
        }
        if differ(&lower, &set_point) {
            return Err(self.rule("WR21", "SameUnitLowerSet"));
        }
        // IFC2X3 alone requires a bound; IFC4 dropped the rule.
        if self.release.version == SchemaVersion::Ifc2x3 && upper.is_none() && lower.is_none() {
            return Err(self.rule("WR22", "WR22"));
        }
        let bounds = ExactBoundedValue {
            lower,
            upper,
            set_point,
        };
        Ok(composite(ExactValue::Bounded(Box::new(bounds)), unit))
    }

    fn table(&self) -> Result<ResolvedValue, ExactPropertyError> {
        let defining = typed_list(self.release, self.id, self.slot("DefiningValues"))?;
        let defined = typed_list(self.release, self.id, self.slot("DefinedValues"))?;
        if defining.len() != defined.len() {
            return Err(self.rule("WR1", "WR21"));
        }
        if !one_type(&defining) {
            return Err(self.rule("WR2", "WR22"));
        }
        if !one_type(&defined) {
            return Err(self.rule("WR3", "WR23"));
        }
        if !unique(&defining) {
            return Err(ExactPropertyError::InconsistentValues {
                entity: self.id,
                rule: "DefiningValues UNIQUE",
            });
        }
        let expression = self.optional_text(self.slot("Expression").0)?;
        let defining_unit = unit_at(
            self.model,
            self.release,
            self.id,
            self.slot("DefiningUnit").0,
        )?;
        let defined_unit = unit_at(
            self.model,
            self.release,
            self.id,
            self.slot("DefinedUnit").0,
        )?;
        let interpolation = match self.declared("CurveInterpolation") {
            Some((value, attribute)) => self.optional_enum(value, attribute)?,
            None => None,
        };
        let rows = defining
            .into_iter()
            .zip(defined)
            .map(|(defining, defined)| ExactTableRow { defining, defined })
            .collect();
        let table = ExactTableValue {
            rows,
            expression,
            defining_unit,
            defined_unit,
            interpolation,
        };
        Ok(composite(ExactValue::Table(table), None))
    }

    fn reference(&self) -> Result<ResolvedValue, ExactPropertyError> {
        let usage_name = self.optional_text(self.slot("UsageName").0)?;
        let target = match self.slot("PropertyReference") {
            (Value::Null, attribute) if attribute.optional => None,
            (Value::Null, _) => {
                return Err(ExactPropertyError::MissingValueSlot { property: self.id })
            }
            (Value::Ref(target), attribute) => {
                let entity = self.target(*target, attribute)?;
                Some(ExactEntityRef {
                    id: *target,
                    type_name: entity.type_name.clone(),
                })
            }
            _ => return Err(ExactPropertyError::UnsupportedValue { property: self.id }),
        };
        Ok(composite(
            ExactValue::Reference(ExactReferenceValue { usage_name, target }),
            None,
        ))
    }

    /// A referenced entity: present, of the release, accepted by the
    /// attribute's declared type, with its exact arity.
    fn target(
        &self,
        target: EntityId,
        attribute: &Attribute,
    ) -> Result<&'m Entity, ExactPropertyError> {
        entity_target(self.model, self.release, self.id, target, attribute)
    }

    fn optional_value(&self, value: &Value) -> Result<Option<ExactTypedValue>, ExactPropertyError> {
        match value {
            Value::Null => Ok(None),
            value => typed_value(self.release, self.id, value).map(Some),
        }
    }

    fn optional_text(
        &self,
        value: &Value,
    ) -> Result<Option<std::sync::Arc<str>>, ExactPropertyError> {
        match value {
            Value::Null => Ok(None),
            Value::Text(text) => Ok(Some(text.clone())),
            _ => Err(ExactPropertyError::UnsupportedValue { property: self.id }),
        }
    }

    /// An optional enumeration constant of the attribute's declared
    /// enumeration type in this release.
    fn optional_enum(
        &self,
        value: &Value,
        attribute: &Attribute,
    ) -> Result<Option<std::sync::Arc<str>>, ExactPropertyError> {
        match value {
            Value::Null => Ok(None),
            Value::Enum(member) if enum_accepts(self.release, &attribute.type_name, member) => {
                Ok(Some(member.clone()))
            }
            _ => Err(ExactPropertyError::UnsupportedValue { property: self.id }),
        }
    }
}

/// A referenced entity held by attribute `attribute` of `holder`: present,
/// of the release, accepted by the declared type, with its exact arity.
///
/// # Errors
///
/// A dangling reference, an entity foreign to the release, an entity the
/// declared type does not accept, or a wrong arity.
pub(super) fn entity_target<'m>(
    model: &'m Model,
    release: Release,
    holder: EntityId,
    target: EntityId,
    attribute: &Attribute,
) -> Result<&'m Entity, ExactPropertyError> {
    let entity = model
        .get(target)
        .ok_or(ExactPropertyError::MissingReference {
            from: holder,
            to: target,
        })?;
    if release.schema.entity(entity.type_name.as_ref()).is_none() {
        return Err(release.not_in_schema(target, entity.type_name.clone()));
    }
    if !release
        .schema
        .accepts_type(&attribute.type_name, entity.type_name.as_ref())
    {
        return Err(ExactPropertyError::UnsupportedValue { property: holder });
    }
    release.require_exact_slots(target, entity)?;
    Ok(entity)
}

/// Whether `member` is a constant of enumeration type `declared` in this
/// release.
pub(super) fn enum_accepts(release: Release, declared: &str, member: &str) -> bool {
    matches!(
        release.schema.type_def(declared).map(|definition| &definition.kind),
        Some(TypeKind::Enumeration(members))
            if members.iter().any(|candidate| candidate.eq_ignore_ascii_case(member))
    )
}

/// A list of `IfcValue`s: `$` only where the attribute is optional (read as
/// empty), otherwise a nonempty list of accepted typed values.
fn typed_list(
    release: Release,
    property: EntityId,
    (value, attribute): (&Value, &'static Attribute),
) -> Result<Vec<ExactTypedValue>, ExactPropertyError> {
    let malformed = ExactPropertyError::MalformedAggregate {
        entity: property,
        attribute: attribute.name.as_str(),
    };
    match value {
        Value::Null if attribute.optional => Ok(Vec::new()),
        Value::List(items) if !items.is_empty() => items
            .iter()
            .map(|item| typed_value(release, property, item))
            .collect(),
        _ => Err(malformed),
    }
}

/// Whether every value is declared with the first one's type (`TYPEOF`
/// equality: a defined type's name is part of its `TYPEOF` set).
fn one_type(values: &[ExactTypedValue]) -> bool {
    values
        .iter()
        .all(|value| value.value_type.eq_ignore_ascii_case(&values[0].value_type))
}

/// Whether no two values are equal in type and payload.
fn unique(values: &[ExactTypedValue]) -> bool {
    values
        .iter()
        .enumerate()
        .all(|(i, value)| values[..i].iter().all(|earlier| !same(earlier, value)))
}

/// EXPRESS instance equality of two select values: same type, same value.
fn same(a: &ExactTypedValue, b: &ExactTypedValue) -> bool {
    a.value_type.eq_ignore_ascii_case(&b.value_type) && a.value == b.value
}

/// A composite value carries its types inside, so the property states none.
fn composite(value: ExactValue, unit_id: Option<EntityId>) -> ResolvedValue {
    ResolvedValue {
        value,
        value_type: None,
        unit_id,
    }
}
