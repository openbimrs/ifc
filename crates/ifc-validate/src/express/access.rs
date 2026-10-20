//! Reading attributes: explicit slots, DERIVE expressions and INVERSE
//! referrers, and typing the file's values by their declarations.
//!
//! An explicit value is typed by its declared type, so `TYPEOF(SELF.Name)`
//! answers `IFC4.IFCLABEL` for a bare string in an `IfcLabel` slot, as it
//! would for `IFCLABEL('x')` in a SELECT. A derived attribute evaluates its
//! expression with `SELF` bound to the instance; a file instance's value is
//! cached for the run. An INVERSE resolves through the reverse index with
//! the same rule as the facade's `attribute_value_by_name` (#462): the
//! referrers holding the instance in the `FOR` attribute's slot whose type
//! is the `FOR` entity or a subtype.

use std::rc::Rc;

use ifc_model::{EntityId, Value};

use super::ast::{AggKind, Expr, TypeExpr};
use super::eval::Cx;
use super::machine::{Fault, Machine, R};
use super::tables::AttrRef;
use super::value::{Agg, Inst, LocalInst, Val};

impl Machine<'_> {
    /// `base.name`; `?` when the base is `?` or has no such attribute.
    pub(crate) fn attribute(&self, base: &Val, name: &str) -> R<Val> {
        let Val::Inst(inst) = base.bare() else {
            return Ok(Val::Indet);
        };
        let entity = self.entity_of(inst)?;
        let Some(attr) = self.tables.attrs[entity].get(name).copied() else {
            return Ok(Val::Indet);
        };
        match (inst, attr) {
            (Inst::Model(id), AttrRef::Explicit { slot, owner, idx }) => {
                // Typed once per run: a rule iterating an attribute reads it
                // once per element, and a 65,536-pixel texture must not be
                // converted 65,536 times.
                if let Some(found) = self.explicit.borrow().get(&(*id, slot)) {
                    return Ok(found.clone());
                }
                let record = self
                    .model
                    .get(*id)
                    .ok_or_else(|| Fault::Data(format!("{id} is not in the file")))?;
                let Some(raw) = record.attribute(slot) else {
                    return Ok(Val::Indet);
                };
                let ty = &self.tables.schema.entities[owner].explicit[idx].ty;
                let value = self.typed(raw, ty)?;
                self.explicit
                    .borrow_mut()
                    .insert((*id, slot), value.clone());
                Ok(value)
            }
            (Inst::Local(local), AttrRef::Explicit { .. }) => Ok(local
                .borrow()
                .attrs
                .get(name)
                .cloned()
                .unwrap_or(Val::Indet)),
            (_, AttrRef::Derived { owner, idx }) => self.derived_value(inst, owner, idx),
            (Inst::Model(id), AttrRef::Inverse { owner, idx }) => self.inverse(*id, owner, idx),
            (Inst::Local(_), AttrRef::Inverse { owner, idx }) => {
                // Nothing in the file references an instance a function built.
                let inverse = &self.tables.schema.entities[owner].inverses[idx];
                Ok(match inverse.aggregate {
                    Some((kind, _, _)) => Val::agg(kind, Vec::new()),
                    None => Val::Indet,
                })
            }
        }
    }

    fn derived_value(&self, inst: &Inst, owner: usize, idx: usize) -> R<Val> {
        if let Inst::Model(id) = inst {
            if let Some(found) = self.derived.borrow().get(&(*id, owner, idx)) {
                return Ok(found.clone());
            }
        }
        let derived = &self.tables.schema.entities[owner].derived[idx];
        // A file instance whose derived value depends on itself (a trimmed
        // curve trimming itself) has none; say so rather than recurse to
        // the depth limit.
        if let Inst::Model(id) = inst {
            if !self.deriving.borrow_mut().insert((*id, owner, idx)) {
                return Err(Fault::Data(format!(
                    "{id}.{} depends on itself",
                    derived.spelling
                )));
            }
        }
        let value = self.nested(|| {
            let mut cx = Cx::new(Val::Inst(inst.clone()), derived.frame);
            self.eval(&derived.expr, &mut cx)
        });
        if let Inst::Model(id) = inst {
            self.deriving.borrow_mut().remove(&(*id, owner, idx));
            if let Ok(value) = &value {
                self.derived
                    .borrow_mut()
                    .insert((*id, owner, idx), value.clone());
            }
        }
        value
    }

    fn inverse(&self, id: EntityId, owner: usize, idx: usize) -> R<Val> {
        let inverse = &self.tables.schema.entities[owner].inverses[idx];
        let found = self.referrers(id, &inverse.entity, &inverse.for_attribute)?;
        match inverse.aggregate {
            Some((kind, _, _)) => Ok(Val::agg(
                kind,
                found
                    .into_iter()
                    .map(|id| Val::Inst(Inst::Model(id)))
                    .collect(),
            )),
            None => match found.as_slice() {
                [] => Ok(Val::Indet),
                [one] => Ok(Val::Inst(Inst::Model(*one))),
                many => Err(Fault::Data(format!(
                    "{id}.{} is a single INVERSE, but {} instances reference it",
                    inverse.name,
                    many.len()
                ))),
            },
        }
    }

    /// The instances of `entity` (or a subtype) holding `id` in their
    /// `attribute` slot, ascending; `USEDIN` and INVERSE both read this.
    pub(crate) fn referrers(
        &self,
        id: EntityId,
        entity: &str,
        attribute: &str,
    ) -> R<Vec<EntityId>> {
        let Some(entity) = self.tables.entity_index(entity) else {
            return Ok(Vec::new());
        };
        let slot = match self.tables.attrs[entity].get(attribute) {
            Some(AttrRef::Explicit { slot, .. }) => *slot,
            _ => return Ok(Vec::new()),
        };
        let mut found = Vec::new();
        for referrer in self.reverse().referrers(id) {
            if referrer.slot != slot || found.last() == Some(&referrer.from) {
                continue;
            }
            let Some(record) = self.model.get(referrer.from) else {
                continue;
            };
            if self
                .entity_of_name(&record.type_name)
                .is_some_and(|source| self.tables.is_a(source, entity))
            {
                found.push(referrer.from);
            }
        }
        Ok(found)
    }

    /// Every instance referencing `id` through any attribute.
    pub(crate) fn all_referrers(&self, id: EntityId) -> Vec<EntityId> {
        let mut found: Vec<EntityId> = self
            .reverse()
            .referrers(id)
            .iter()
            .map(|referrer| referrer.from)
            .collect();
        found.dedup();
        found
    }

    /// A file value typed by its declaration.
    pub(crate) fn typed(&self, raw: &Value, ty: &TypeExpr) -> R<Val> {
        match (raw, ty) {
            (Value::Null | Value::Derived, _) => Ok(Val::Indet),
            (_, TypeExpr::Named(name)) => {
                if let Some(index) = self.tables.globals.types.get(name) {
                    return self.typed_as(raw, *index);
                }
                self.untyped(raw)
            }
            (Value::List(items), TypeExpr::Aggregate { kind, lower, elem }) => {
                // Only an ARRAY's bounds are indices; a LIST's, SET's or
                // BAG's are sizes, and those index from 1.
                let lower = match lower.as_deref() {
                    _ if *kind != AggKind::Array => 1,
                    None => 1,
                    Some(Expr::Int(value)) => *value,
                    Some(_) => {
                        return Err(Fault::Unsupported(
                            "an explicit ARRAY attribute with a computed lower bound".into(),
                        ))
                    }
                };
                let items = items
                    .iter()
                    .map(|item| {
                        self.step()?;
                        self.typed(item, elem)
                    })
                    .collect::<R<Vec<_>>>()?;
                Ok(Val::Agg(Rc::new(Agg {
                    kind: *kind,
                    lower,
                    items,
                })))
            }
            (Value::Integer(value), TypeExpr::Simple(super::ast::Simple::Real)) => {
                // A REAL written without a decimal point.
                #[allow(clippy::cast_precision_loss)]
                Ok(Val::Real(*value as f64))
            }
            _ => self.untyped(raw),
        }
    }

    /// A file value of defined, enumeration or select type `index`.
    fn typed_as(&self, raw: &Value, index: usize) -> R<Val> {
        let decl = &self.tables.schema.types[index];
        match &decl.underlying {
            TypeExpr::Select(_) => self.untyped(raw),
            TypeExpr::Enumeration(_) => Ok(Val::Typed(Rc::new((index, self.untyped(raw)?)))),
            underlying => {
                // `IFCLABEL('x')` in a slot declared `IfcLabel` is allowed
                // by some writers; the wrapper says the same as the slot.
                let inner = match raw {
                    Value::Typed { value, .. } => value,
                    other => other,
                };
                let value = self.typed(inner, underlying)?;
                Ok(Val::Typed(Rc::new((index, value))))
            }
        }
    }

    /// A file value without a declaration to type it by: a SELECT member
    /// carries its own wrapper, anything else is read as written.
    pub(crate) fn untyped(&self, raw: &Value) -> R<Val> {
        Ok(match raw {
            Value::Null | Value::Derived => Val::Indet,
            Value::Bool(value) => Val::logical(*value),
            Value::LogicalUnknown => Val::Logical(super::ast::Logical::Unknown),
            Value::Integer(value) => Val::Int(*value),
            Value::Real(value) => Val::Real(*value),
            Value::Text(text) => Val::Str(text.as_ref().into()),
            Value::Binary(bits) => Val::Binary(bits.as_ref().into()),
            Value::Enum(item) => Val::Enum(item.to_ascii_uppercase().into()),
            Value::Ref(id) => Val::Inst(Inst::Model(*id)),
            Value::List(items) => Val::agg(
                AggKind::List,
                items
                    .iter()
                    .map(|item| self.untyped(item))
                    .collect::<R<Vec<_>>>()?,
            ),
            Value::Typed { type_name, value } => match self.tables.type_index(type_name) {
                Some(index) => self.typed_as(value, index)?,
                None => self.untyped(value)?,
            },
        })
    }

    /// An instance from partial entity constructors joined by `||`.
    pub(crate) fn construct(&self, partials: Vec<(usize, Vec<Val>)>) -> R<Val> {
        let leaf = partials
            .iter()
            .map(|(entity, _)| *entity)
            .find(|candidate| {
                partials
                    .iter()
                    .all(|(other, _)| self.tables.is_a(*candidate, *other))
            })
            .ok_or_else(|| {
                Fault::Unsupported("a complex instance of more than one leaf entity".into())
            })?;
        let mut attrs = std::collections::HashMap::new();
        for (entity, args) in partials {
            let own: Vec<_> = self.tables.schema.entities[entity]
                .explicit
                .iter()
                .filter(|attr| attr.redeclares.is_none())
                .collect();
            if own.len() != args.len() {
                return Err(Fault::Unsupported(format!(
                    "a {} constructor with {} arguments for {} attributes",
                    self.tables.schema.entities[entity].spelling,
                    args.len(),
                    own.len()
                )));
            }
            for (attr, value) in own.into_iter().zip(args) {
                attrs.insert(attr.name.clone(), value);
            }
        }
        Ok(Val::Inst(Inst::Local(Rc::new(std::cell::RefCell::new(
            LocalInst {
                entity: leaf,
                attrs,
            },
        )))))
    }
}
