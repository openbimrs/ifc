//! Container items, wrappers, and entities referenced or defined in place.

use super::*;

impl State<'_> {
    /// A child of a container or wrapper: an entity, reference or wrapper.
    pub(super) fn open_item(&mut self, name: &str, attributes: Attributes) -> Result<(), XmlError> {
        let Some(Segment {
            frame: Frame::Value(frame),
            ..
        }) = self.stack.last()
        else {
            return Err(unsupported("an item outside a container"));
        };
        if frame.text_content {
            return Err(XmlError::Unsupported {
                construct: format!(
                    "element `{name}` inside the text value `{}`",
                    frame.attribute
                ),
            });
        }
        if frame.single && !frame.items.is_empty() {
            return Err(XmlError::Unsupported {
                construct: format!("a second value in the SELECT `{}`", frame.attribute),
            });
        }
        if let Target::Inverse { inverse, .. } = &frame.target {
            if frame.items.is_empty() && self.schema.entity(name).is_none() {
                let inverse = inverse.clone();
                if let Some(layout) = self.layouts.entity(&inverse.entity, false)? {
                    if layout.slot(name).is_some() || layout.inverse(name).is_some() {
                        return self.open_direct_inverse(name, attributes);
                    }
                }
            }
        }
        let shape = frame.shape.clone();
        let attribute = frame.attribute.clone();
        if let Some(type_name) = name.strip_suffix("-wrapper") {
            return self.open_wrapper(name, type_name, &shape, &attribute, attributes);
        }
        // An entity element named by its type.
        let layout = self.entity_layout(name)?;
        let admitted = match &shape.leaf {
            Leaf::Entity(declared) => self.schema.is_a(&layout.name, declared),
            Leaf::Select(select) => self.schema.accepts_type(select, &layout.name),
            _ => false,
        };
        if !admitted {
            return Err(XmlError::TypeMismatch {
                declared: shape.describe(),
                found: format!("an entity `{}`", layout.name),
            });
        }
        self.mark_flat_item()?;
        let declared = layout.name.clone();
        self.entity_value(name, &declared, attributes, Target::Item)
    }

    /// An inverse element taken for a container whose first child is an
    /// attribute of the relationship: the element is the relationship
    /// itself, written without XML attributes (`<StyledByItem><Styles>`).
    pub(super) fn open_direct_inverse(
        &mut self,
        name: &str,
        attributes: Attributes,
    ) -> Result<(), XmlError> {
        let Some(Segment {
            frame: Frame::Value(frame),
            segment,
        }) = self.stack.pop()
        else {
            return Err(unsupported("an inverse element outside an entity"));
        };
        let Target::Inverse { parent, inverse } = frame.target else {
            return Err(unsupported("an inverse element outside an entity"));
        };
        let declared = inverse.entity.clone();
        self.entity_value(
            &segment,
            &declared,
            Attributes::default(),
            Target::Inverse { parent, inverse },
        )?;
        let Some(Segment {
            frame: Frame::Entity { number },
            ..
        }) = self.stack.last()
        else {
            return Err(unsupported("an inverse element that is not an entity"));
        };
        let number = *number;
        self.open_attribute(number, name, attributes)
    }

    /// Record that the enclosing container received a flat leaf item.
    pub(super) fn mark_flat_item(&mut self) -> Result<(), XmlError> {
        if let Some(Segment {
            frame: Frame::Value(frame),
            ..
        }) = self.stack.last_mut()
        {
            if frame.nested_items == Some(true) {
                return Err(unsupported("flat items mixed with Seq- wrapped lists"));
            }
            frame.nested_items = Some(false);
        }
        Ok(())
    }

    pub(super) fn open_wrapper(
        &mut self,
        name: &str,
        type_name: &str,
        container: &Shape,
        attribute: &Arc<str>,
        mut attributes: Attributes,
    ) -> Result<(), XmlError> {
        let extra = attributes.take("extraBits");
        self.no_more_attributes(name, &attributes, false)?;
        if extra.is_some_and(|extra| extra.trim() != "0") {
            return Err(unsupported(
                "a binary whose length is not a whole number of bytes (extraBits)",
            ));
        }
        // `Seq-T-wrapper`: one inner list of a nested aggregate of T.
        if let Some(inner) = type_name.strip_prefix("Seq-") {
            if container.levels.len() < 2
                || inner != &*container.named
                || !container.leaf.is_simple()
            {
                return Err(XmlError::TypeMismatch {
                    declared: container.describe(),
                    found: format!("a `{name}`"),
                });
            }
            if let Some(Segment {
                frame: Frame::Value(frame),
                ..
            }) = self.stack.last_mut()
            {
                if frame.nested_items == Some(false) {
                    return Err(unsupported("Seq- wrapped lists mixed with flat items"));
                }
                frame.nested_items = Some(true);
            }
            let frame = ValueFrame::new(
                Target::Item,
                container.inner(),
                attribute.clone(),
                true,
                false,
            );
            self.push(Frame::Value(Box::new(frame)), name.into());
            return Ok(());
        }
        let Some(definition) = self
            .schema
            .type_def(type_name)
            .filter(|definition| definition.name == type_name)
        else {
            return Err(XmlError::TypeMismatch {
                declared: container.describe(),
                found: format!("a `{name}`, which names no defined type"),
            });
        };
        let (shape, wrap) = match &container.leaf {
            Leaf::Select(select) => {
                if !self.schema.accepts_type(select, &definition.name) {
                    return Err(XmlError::TypeMismatch {
                        declared: container.describe(),
                        found: format!("a `{name}`"),
                    });
                }
                let shape = typing::type_shape(self.schema, &definition.name)?;
                (shape, Some(Arc::from(definition.name.to_ascii_uppercase())))
            }
            _ if *container.named == *definition.name && !container.levels.is_empty() => {
                // An aggregate of a defined type: the wrapper is the bare item.
                (container.inner_leaf(), None)
            }
            _ => {
                return Err(XmlError::TypeMismatch {
                    declared: container.describe(),
                    found: format!("a `{name}`"),
                })
            }
        };
        self.mark_flat_item()?;
        let text_content = shape.leaf.is_simple();
        let mut frame =
            ValueFrame::new(Target::Item, shape, attribute.clone(), text_content, false);
        frame.wrap = wrap;
        self.push(Frame::Value(Box::new(frame)), name.into());
        Ok(())
    }

    /// An element denoting an entity of (at least) type `declared`: a
    /// reference, an unset value, or an entity defined in place.
    pub(super) fn entity_value(
        &mut self,
        name: &str,
        declared: &Arc<str>,
        mut attributes: Attributes,
        target: Target,
    ) -> Result<(), XmlError> {
        if attributes.xsi_schema_location {
            return Err(unsupported("xsi:schemaLocation below the root"));
        }
        let claimed = match attributes.xsi_type.take() {
            Some(claimed) => {
                let layout = self.entity_layout(&claimed)?;
                if !self.schema.is_a(&layout.name, declared) {
                    return Err(XmlError::TypeMismatch {
                        declared: declared.to_string(),
                        found: format!("xsi:type `{claimed}`"),
                    });
                }
                layout.name.clone()
            }
            None => declared.clone(),
        };
        let reference = match (attributes.take("ref"), attributes.take("href")) {
            (Some(_), Some(_)) => return Err(unsupported("both ref and href on one element")),
            (Some(reference), None) => Some(reference.trim().to_string()),
            (None, Some(href)) => Some(local_href(&href)?),
            (None, None) => None,
        };
        if let Some(reference) = reference {
            if let Some((attribute, _)) = attributes.plain.first() {
                return Err(XmlError::Unsupported {
                    construct: format!(
                        "attribute `{attribute}` on a reference element; a reference carries none"
                    ),
                });
            }
            let target_number = self.reference(&reference);
            self.claims.push(Claim {
                target: target_number,
                claimed: claimed.clone(),
                referrer: self.enclosing_number(),
            });
            let value = Value::Ref(EntityId(target_number));
            self.deliver_entity(target, value, target_number)?;
            self.push(Frame::Empty, format!("{name}[@ref='{reference}']"));
            return Ok(());
        }
        if attributes.xsi_nil == Some(true) {
            if !attributes.plain.is_empty() {
                return Err(unsupported("attributes on an unset (xsi:nil) element"));
            }
            return self.deliver_nil(name, target);
        }
        let layout = self.entity_layout(&claimed)?;
        self.entity_element(name, &layout, attributes, Some(target))
    }

    /// Define an entity in place: allocate it, deliver it, read its XML
    /// attributes and open its frame.
    pub(super) fn entity_element(
        &mut self,
        element: &str,
        layout: &Arc<EntityLayout>,
        mut attributes: Attributes,
        target: Option<Target>,
    ) -> Result<(), XmlError> {
        if layout.abstract_ {
            return Err(XmlError::AbstractEntity {
                name: layout.name.to_string(),
            });
        }
        for addressing in ["pos", "path"] {
            if attributes.take(addressing).is_some() {
                return Err(XmlError::Unsupported {
                    construct: format!("`{addressing}` addressing on `{element}`"),
                });
            }
        }
        if attributes.xsi_nil == Some(true) {
            return Err(unsupported("xsi:nil on an entity defined in place"));
        }
        let number = self.entities.len() as u64 + 1;
        let id = attributes.take("id");
        let segment = match &id {
            Some(id) => format!("{element}[@id='{id}']"),
            None => format!("{element}[#{number}]"),
        };
        let path = self.path_with(&segment);
        let start = self
            .stack
            .iter()
            .rposition(|segment| matches!(segment.frame, Frame::Entity { .. }))
            .map_or(1, |position| position + 1);
        let mut relative: Vec<&str> = self
            .stack
            .iter()
            .skip(start)
            .map(|segment| segment.segment.as_str())
            .collect();
        relative.push(&segment);
        let relative = relative.join("/").into_boxed_str();
        if let Some(id) = &id {
            if self.ids.insert(id.clone(), number).is_some() {
                return Err(XmlError::DuplicateId { id: id.clone() });
            }
        }
        self.entities.push(Pending {
            layout: layout.clone(),
            slots: vec![None; layout.slots.len()],
            parent: self.enclosing_number(),
            relative,
        });
        if let Some(target) = target {
            self.deliver_entity(target, Value::Ref(EntityId(number)), number)?;
        }
        for (attribute, text) in std::mem::take(&mut attributes.plain) {
            self.simple_attribute(number, element, layout, &attribute, &text)
                .map_err(|error| error.at(format!("{path}/@{attribute}")))?;
        }
        self.push(Frame::Entity { number }, segment);
        Ok(())
    }

    /// An XML attribute of an entity element: a simple explicit attribute.
    pub(super) fn simple_attribute(
        &mut self,
        number: u64,
        element: &str,
        layout: &EntityLayout,
        attribute: &str,
        text: &str,
    ) -> Result<(), XmlError> {
        let Some(slot) = layout.slot(attribute) else {
            return Err(XmlError::UnknownAttribute {
                entity: layout.name.to_string(),
                element: element.into(),
                attribute: attribute.into(),
            });
        };
        let declared = &layout.slots[slot];
        let shape = &declared.shape;
        // The form the configuration gives the attribute; an aggregate of
        // simple values it writes as a container is also read from a list
        // attribute, as some exporters write it.
        let attribute_form = declared.form == XsdForm::Attribute
            || (!shape.levels.is_empty() && shape.leaf.is_simple());
        if !attribute_form {
            return Err(XmlError::WrongForm {
                attribute: attribute.into(),
                expected: "a child element",
                found: "an XML attribute",
            });
        }
        let value = if shape.levels.is_empty() {
            typing::scalar(&shape.leaf, text, Lexical::Xsd)?
        } else {
            typing::list_text(shape, text, Lexical::Xsd)?
        };
        if declared.derived {
            // The XSD derives a subtype that redeclares an attribute DERIVE
            // by restricting its supertype's content model. A restriction
            // drops the supertype's child elements but inherits its XML
            // attributes, so the XSD admits a value here that EXPRESS
            // defines by derivation: STEP writes the slot `*`. The value is
            // typed, so malformed text is still refused, and not stored.
            return Ok(());
        }
        let slots = &mut self.entities[index(number)].slots;
        if slots[slot].is_some() {
            return Err(XmlError::DuplicateSlot {
                name: attribute.into(),
                slot,
            });
        }
        slots[slot] = Some(value);
        Ok(())
    }
}
