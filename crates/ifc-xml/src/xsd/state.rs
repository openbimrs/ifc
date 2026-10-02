//! Reader state: locations, references, delivery, text and element ends.

use super::*;

impl<'s> State<'s> {
    pub(super) fn new(schema: &'s Schema, profile: XmlProfile) -> Self {
        let mut model = Model::new();
        model.header_mut().schema = vec![profile.schema_token().into()];
        Self {
            schema,
            profile,
            layouts: Layouts::new(schema),
            model,
            entities: Vec::new(),
            ids: HashMap::new(),
            forward: HashMap::new(),
            forward_names: Vec::new(),
            backfills: Vec::new(),
            claims: Vec::new(),
            stack: Vec::new(),
            seen_root: false,
        }
    }

    pub(super) fn path(&self) -> String {
        let mut path = String::new();
        for segment in &self.stack {
            path.push('/');
            path.push_str(&segment.segment);
        }
        if path.is_empty() {
            path.push('/');
        }
        path
    }

    pub(super) fn path_with(&self, leaf: &str) -> String {
        let mut path = self.path();
        if path != "/" {
            path.push('/');
        }
        path.push_str(leaf);
        path
    }

    /// The number of the innermost open entity element, or 0 at the root.
    pub(super) fn enclosing_number(&self) -> u64 {
        self.stack
            .iter()
            .rev()
            .find_map(|segment| match segment.frame {
                Frame::Entity { number } => Some(number),
                _ => None,
            })
            .unwrap_or(0)
    }

    /// The document path of an entity element, rebuilt for an error.
    pub(super) fn entity_path(&self, number: u64) -> String {
        match self.entities.get(index(number.max(1))) {
            Some(pending) if number != 0 => self.entity_path_of(pending),
            _ => "/ifcXML".into(),
        }
    }

    pub(super) fn entity_path_of(&self, pending: &Pending) -> String {
        let mut parts = vec![&*pending.relative];
        let mut parent = pending.parent;
        while let Some(enclosing) = (parent != 0)
            .then(|| self.entities.get(index(parent)))
            .flatten()
        {
            parts.push(&enclosing.relative);
            parent = enclosing.parent;
        }
        let mut path = String::from("/ifcXML");
        for part in parts.iter().rev() {
            path.push('/');
            path.push_str(part);
        }
        path
    }

    pub(super) fn push(&mut self, frame: Frame, segment: String) {
        self.stack.push(Segment { frame, segment });
    }

    /// The number an id refers to, allocating a placeholder for an id whose
    /// element has not been read yet.
    pub(super) fn reference(&mut self, id: &str) -> u64 {
        if let Some(number) = self.ids.get(id) {
            return *number;
        }
        if let Some(placeholder) = self.forward.get(id) {
            return *placeholder;
        }
        let placeholder = PLACEHOLDER + self.forward_names.len() as u64;
        self.forward_names.push(id.into());
        self.forward.insert(id.into(), placeholder);
        placeholder
    }

    /// Deliver an entity value, recording the inverse it fills if any.
    pub(super) fn deliver_entity(
        &mut self,
        target: Target,
        value: Value,
        number: u64,
    ) -> Result<(), XmlError> {
        match target {
            Target::Inverse { parent, inverse } => {
                self.backfills.push(Backfill {
                    relationship: number,
                    parent,
                    inverse,
                });
                Ok(())
            }
            target => self.deliver(target, value),
        }
    }

    /// Put a finished value where its element says.
    pub(super) fn deliver(&mut self, target: Target, value: Value) -> Result<(), XmlError> {
        match target {
            Target::Slot { entity, slot } => {
                let slots = &mut self.entities[index(entity)].slots;
                if slots[slot].is_some() {
                    let name = self.entities[index(entity)].layout.slots[slot]
                        .name
                        .to_string();
                    return Err(XmlError::DuplicateSlot { name, slot });
                }
                self.entities[index(entity)].slots[slot] = Some(value);
            }
            Target::Item => match self.stack.last_mut() {
                Some(Segment {
                    frame: Frame::Value(frame),
                    ..
                }) => frame.items.push(value),
                _ => return Err(unsupported("an item outside a container")),
            },
            Target::Inverse { parent, inverse } => {
                let Value::Ref(EntityId(number)) = value else {
                    return Err(unsupported("a non-entity value in an inverse attribute"));
                };
                self.backfills.push(Backfill {
                    relationship: number,
                    parent,
                    inverse,
                });
            }
        }
        Ok(())
    }

    pub(super) fn text(&mut self, text: &str) -> Result<(), XmlError> {
        match self.stack.last_mut().map(|segment| &mut segment.frame) {
            Some(Frame::HeaderField { text: buffer, .. }) => buffer.push_str(text),
            Some(Frame::Value(frame)) if frame.text_content => frame.text.push_str(text),
            _ if text.trim().is_empty() => {}
            _ => {
                return Err(XmlError::Unsupported {
                    construct: format!("text {:?} where elements are expected", text.trim()),
                }
                .at(self.path()))
            }
        }
        Ok(())
    }

    pub(super) fn close(&mut self) -> Result<(), XmlError> {
        let Some(Segment { frame, segment }) = self.stack.pop() else {
            return Ok(());
        };
        match frame {
            Frame::HeaderField { name, text } => {
                let header = self.model.header_mut();
                match name.as_str() {
                    "name" => header.name = text,
                    "time_stamp" => header.time_stamp = text,
                    "author" => header.author.push(text),
                    "organization" => header.organization.push(text),
                    "preprocessor_version" => header.preprocessor_version = text,
                    "originating_system" => header.originating_system = text,
                    "authorization" => header.authorization = text,
                    _ => header.description.push(text),
                }
            }
            Frame::Value(frame) => {
                let target_frame = *frame;
                let ValueFrame { target, .. } = &target_frame;
                if let Target::Inverse { parent, inverse } = target {
                    for item in &target_frame.items {
                        if let Value::Ref(EntityId(number)) = item {
                            self.backfills.push(Backfill {
                                relationship: *number,
                                parent: *parent,
                                inverse: inverse.clone(),
                            });
                        }
                    }
                    return Ok(());
                }
                let (target, value) = target_frame
                    .finish()
                    .map_err(|error| error.at(self.path_with(&segment)))?;
                self.deliver(target, value)
                    .map_err(|error| error.at(self.path_with(&segment)))?;
            }
            Frame::Root | Frame::Header | Frame::Entity { .. } | Frame::Empty => {}
        }
        Ok(())
    }
}
