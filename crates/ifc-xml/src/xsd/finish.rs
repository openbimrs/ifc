//! The end of the document: forward references, inverses, reference types.

use super::*;

impl State<'_> {
    pub(super) fn finish(mut self) -> Result<Model, XmlError> {
        if !self.seen_root || !self.stack.is_empty() {
            return Err(XmlError::Root { found: None });
        }
        // Forward references: every placeholder must name a defined id.
        let mut resolved = Vec::with_capacity(self.forward_names.len());
        for id in &self.forward_names {
            let number = self
                .ids
                .get(id)
                .copied()
                .ok_or_else(|| XmlError::UnresolvedReference { id: id.clone() })?;
            resolved.push(number);
        }
        let resolve = |number: u64| -> u64 {
            if number >= PLACEHOLDER {
                resolved[usize::try_from(number - PLACEHOLDER).unwrap_or(usize::MAX)]
            } else {
                number
            }
        };
        if !resolved.is_empty() {
            for pending in &mut self.entities {
                for value in pending.slots.iter_mut().flatten() {
                    rewrite(value, &resolve);
                }
            }
        }
        for claim in &mut self.claims {
            claim.target = resolve(claim.target);
        }
        for backfill in &mut self.backfills {
            backfill.relationship = resolve(backfill.relationship);
        }

        let backfills = std::mem::take(&mut self.backfills);
        for backfill in &backfills {
            self.apply_backfill(backfill).map_err(|error| {
                error.at(format!(
                    "{}/{}",
                    self.entity_path(backfill.parent),
                    backfill.inverse.name
                ))
            })?;
        }

        let claims = std::mem::take(&mut self.claims);
        for claim in &claims {
            let target = &self.entities[index(claim.target)].layout;
            if !self.schema.is_a(&target.name, &claim.claimed) {
                return Err(XmlError::TypeMismatch {
                    declared: claim.claimed.to_string(),
                    found: format!("a reference to an entity `{}`", target.name),
                }
                .at(self.entity_path(claim.referrer)));
            }
        }

        for (position, pending) in self.entities.iter().enumerate() {
            self.check_references(pending)
                .map_err(|error| error.at(self.entity_path(position as u64 + 1)))?;
        }
        let mut entities = std::mem::take(&mut self.entities);
        let mut model = self.model;
        for (position, pending) in entities.drain(..).enumerate() {
            let values = pending
                .slots
                .into_iter()
                .zip(&pending.layout.slots)
                .map(|(value, slot)| match value {
                    _ if slot.derived => Value::Derived,
                    Some(value) => value,
                    None => Value::Null,
                })
                .collect();
            model.insert(
                EntityId(position as u64 + 1),
                Entity::new(pending.layout.upper.clone(), values),
            );
        }
        Ok(model)
    }

    /// Supply the attribute an inverse inverts: the relationship's
    /// `for_attribute` names the entity that held the inverse.
    pub(super) fn apply_backfill(&mut self, backfill: &Backfill) -> Result<(), XmlError> {
        let relationship = &self.entities[index(backfill.relationship)];
        let layout = relationship.layout.clone();
        let inverse = &backfill.inverse;
        let conflict = |detail: &str| XmlError::InverseConflict {
            inverse: inverse.name.to_string(),
            relationship: layout.name.to_string(),
            attribute: inverse.for_attribute.to_string(),
            detail: detail.into(),
        };
        if !self.schema.is_a(&layout.name, &inverse.entity) {
            return Err(XmlError::TypeMismatch {
                declared: inverse.entity.to_string(),
                found: format!("an entity `{}` in inverse `{}`", layout.name, inverse.name),
            });
        }
        let slot = layout
            .slot(&inverse.for_attribute)
            .ok_or_else(|| conflict("is not an explicit attribute of it"))?;
        let shape = &layout.slots[slot].shape;
        let parent = Value::Ref(EntityId(backfill.parent));
        let current = &mut self.entities[index(backfill.relationship)].slots[slot];
        if shape.levels.is_empty() {
            match current {
                None | Some(Value::Null) => *current = Some(parent),
                Some(existing) if *existing == parent => {}
                Some(_) => return Err(conflict("already names a different entity")),
            }
            return Ok(());
        }
        if shape.levels.len() != 1 {
            return Err(conflict("is a nested aggregate"));
        }
        match current {
            None | Some(Value::Null) => *current = Some(Value::List(vec![parent])),
            Some(Value::List(items)) => {
                if items.contains(&parent) {
                    return Ok(());
                }
                match shape.levels[0].kind {
                    AggregateKind::Set | AggregateKind::Bag => items.push(parent),
                    _ => return Err(conflict(
                        "is ordered, so the position of an entity implied by an inverse is unknown",
                    )),
                }
            }
            Some(_) => return Err(conflict("holds a value that is not a list")),
        }
        Ok(())
    }

    /// Every reference resolves to an entity its declared type admits.
    pub(super) fn check_references(&self, pending: &Pending) -> Result<(), XmlError> {
        let mut references = Vec::new();
        for (value, slot) in pending.slots.iter().zip(&pending.layout.slots) {
            let Some(value) = value else { continue };
            references.clear();
            typing::references(self.schema, &slot.shape, value, &mut references).map_err(
                |error| error.at(format!("{}/{}", self.entity_path_of(pending), slot.name)),
            )?;
            for (EntityId(number), declared) in &references {
                let target = &self.entities[index(*number)].layout;
                if !self.schema.accepts_type(declared, &target.name) {
                    return Err(XmlError::TypeMismatch {
                        declared: declared.to_string(),
                        found: format!("a reference to an entity `{}`", target.name),
                    }
                    .at(format!(
                        "{}/{}",
                        self.entity_path_of(pending),
                        slot.name
                    )));
                }
            }
        }
        Ok(())
    }
}

impl ValueFrame {
    pub(super) fn new(
        target: Target,
        shape: Shape,
        attribute: Arc<str>,
        text_content: bool,
        single: bool,
    ) -> Self {
        Self {
            target,
            shape,
            text_content,
            single,
            wrap: None,
            items: Vec::new(),
            nested_items: None,
            text: String::new(),
            array_size: None,
            attribute,
        }
    }

    /// The value this element denotes.
    pub(super) fn finish(self) -> Result<(Target, Value), XmlError> {
        let value = if self.text_content {
            if self.shape.levels.is_empty() {
                if self.shape.leaf == Leaf::Binary {
                    let token = self.text.trim();
                    Value::Binary(typing::hex_binary(token).ok_or_else(|| {
                        XmlError::InvalidScalar {
                            kind: "BINARY".into(),
                            value: self.text.clone(),
                        }
                    })?)
                } else {
                    typing::scalar(&self.shape.leaf, &self.text, Lexical::Xsd)?
                }
            } else {
                typing::list_text(&self.shape, &self.text, Lexical::Xsd)?
            }
        } else if self.single {
            let mut items = self.items;
            if items.len() != 1 {
                return Err(XmlError::Unsupported {
                    construct: format!("an empty SELECT value `{}`", self.attribute),
                });
            }
            items.remove(0)
        } else if self.nested_items == Some(true) {
            Value::List(self.items)
        } else {
            typing::nest(&self.shape, self.items, self.array_size.as_deref())?
        };
        let value = match self.wrap {
            Some(type_name) => Value::Typed {
                type_name,
                value: Box::new(value),
            },
            None => value,
        };
        Ok((self.target, value))
    }
}
