//! Start tags: the root, the header, and an entity's attribute elements.

use super::*;

impl State<'_> {
    pub(super) fn open(
        &mut self,
        reader: &NsReader<&[u8]>,
        namespace: Option<String>,
        element: &BytesStart<'_>,
    ) -> Result<(), XmlError> {
        let name = element.local_name().as_ref().to_owned();
        self.check_namespace(namespace, &name)?;
        let mut attributes = self.attributes(reader, element, &name)?;
        let Some(top) = self.stack.last() else {
            return self.open_root(&name, &mut attributes);
        };
        let result = match &top.frame {
            Frame::Root => self.open_top_level(&name, attributes),
            Frame::Header => self.open_header_field(&name, &attributes),
            Frame::Entity { number } => {
                let number = *number;
                self.open_attribute(number, &name, attributes)
            }
            Frame::Value(_) => self.open_item(&name, attributes),
            Frame::HeaderField { .. } | Frame::Empty => Err(XmlError::Unsupported {
                construct: format!("element `{name}` inside a value that has no elements"),
            }),
        };
        result.map_err(|error| error.at(self.path_with(&name)))
    }

    pub(super) fn check_namespace(
        &self,
        found: Option<String>,
        element: &str,
    ) -> Result<(), XmlError> {
        if found
            .as_deref()
            .is_some_and(|found| self.profile.namespaces().contains(&found))
        {
            return Ok(());
        }
        Err(XmlError::Namespace {
            element: element.into(),
            expected: self.profile.namespace(),
            found,
        }
        .at(self.path_with(element)))
    }

    /// Sort a start tag's attributes, normalizing values as XML 1.0 does.
    pub(super) fn attributes(
        &self,
        reader: &NsReader<&[u8]>,
        element: &BytesStart<'_>,
        name: &str,
    ) -> Result<Attributes, XmlError> {
        let mut out = Attributes::default();
        for attribute in element.attributes() {
            let attribute = attribute
                .map_err(|error| XmlError::Malformed(error.to_string()).at(self.path_with(name)))?;
            if attribute.key.as_namespace_binding().is_some() {
                continue;
            }
            let value = attribute
                .normalized_value(XmlVersion::Implicit1_0)
                .map_err(|error| XmlError::Malformed(error.to_string()).at(self.path_with(name)))?
                .into_owned();
            let (namespace, local) = reader.resolver().resolve_attribute(attribute.key);
            let local = local.as_ref().to_owned();
            match namespace {
                ResolveResult::Unbound => out.plain.push((local, value)),
                ResolveResult::Bound(namespace) if namespace.as_ref() == XSI => {
                    match local.as_str() {
                        "type" => out.xsi_type = Some(self.resolve_type_name(reader, &value)?),
                        "nil" => {
                            out.xsi_nil = Some(match value.trim() {
                                "true" | "1" => true,
                                "false" | "0" => false,
                                _ => {
                                    return Err(XmlError::InvalidScalar {
                                        kind: "xsi:nil".into(),
                                        value,
                                    }
                                    .at(self.path_with(name)))
                                }
                            });
                        }
                        "schemaLocation" | "noNamespaceSchemaLocation" => {
                            out.xsi_schema_location = true;
                        }
                        _ => return Err(self.unknown_xml_attribute(name, &format!("xsi:{local}"))),
                    }
                }
                // The aggregate attributes are global attribute declarations
                // of the XSD, so a valid document qualifies them with the
                // IFC namespace (`ifc:arraySize`); unqualified ones are read
                // too.
                ResolveResult::Bound(namespace)
                    if self.profile.namespaces().contains(&namespace.as_ref())
                        && matches!(local.as_str(), "arraySize" | "itemType" | "cType") =>
                {
                    out.plain.push((local, value));
                }
                _ => {
                    let qualified = attribute.key.as_ref().to_owned();
                    return Err(self.unknown_xml_attribute(name, &qualified));
                }
            }
        }
        Ok(out)
    }

    pub(super) fn unknown_xml_attribute(&self, element: &str, attribute: &str) -> XmlError {
        XmlError::UnknownAttribute {
            entity: self.enclosing_entity().unwrap_or_else(|| element.into()),
            element: element.into(),
            attribute: attribute.into(),
        }
        .at(self.path_with(element))
    }

    pub(super) fn enclosing_entity(&self) -> Option<String> {
        self.stack
            .iter()
            .rev()
            .find_map(|segment| match segment.frame {
                Frame::Entity { number } => self
                    .entities
                    .get(index(number))
                    .map(|pending| pending.layout.name.to_string()),
                _ => None,
            })
    }

    /// An `xsi:type` QName, which must name a type in the IFC namespace.
    pub(super) fn resolve_type_name(
        &self,
        reader: &NsReader<&[u8]>,
        value: &str,
    ) -> Result<String, XmlError> {
        let value = value.trim();
        let (namespace, local) = reader.resolver().resolve_element(QName(value));
        let in_ifc = matches!(
            namespace,
            ResolveResult::Bound(namespace) if self
                .profile
                .namespaces()
                .iter()
                .any(|candidate| *candidate == namespace.as_ref())
        );
        if !in_ifc {
            return Err(XmlError::TypeMismatch {
                declared: format!("an xsi:type in {}", self.profile.namespace()),
                found: format!("xsi:type {value:?}"),
            });
        }
        Ok(local.as_ref().to_owned())
    }

    pub(super) fn open_root(
        &mut self,
        name: &str,
        attributes: &mut Attributes,
    ) -> Result<(), XmlError> {
        if self.seen_root || name != "ifcXML" {
            return Err(XmlError::Root {
                found: Some(name.into()),
            });
        }
        self.seen_root = true;
        for known in ["id", "express", "configuration"] {
            attributes.take(known);
        }
        if let Some((attribute, _)) = attributes.plain.first() {
            return Err(XmlError::UnknownAttribute {
                entity: "ifcXML".into(),
                element: "ifcXML".into(),
                attribute: attribute.clone(),
            });
        }
        if attributes.xsi_type.is_some() || attributes.xsi_nil.is_some() {
            return Err(unsupported("xsi:type or xsi:nil on the ifcXML root"));
        }
        self.push(Frame::Root, "ifcXML".into());
        Ok(())
    }

    pub(super) fn open_top_level(
        &mut self,
        name: &str,
        attributes: Attributes,
    ) -> Result<(), XmlError> {
        if name == "header" {
            if !attributes.plain.is_empty() || attributes.xsi_type.is_some() {
                return Err(unsupported("attributes on the header element"));
            }
            self.push(Frame::Header, "header".into());
            return Ok(());
        }
        if attributes.xsi_schema_location {
            return Err(unsupported("xsi:schemaLocation below the root"));
        }
        let layout = self.entity_layout(name)?;
        if attributes
            .xsi_type
            .as_deref()
            .is_some_and(|claimed| claimed != name)
        {
            return Err(unsupported("xsi:type on a top-level entity element"));
        }
        self.entity_element(name, &layout, attributes, None)
    }

    pub(super) fn open_header_field(
        &mut self,
        name: &str,
        attributes: &Attributes,
    ) -> Result<(), XmlError> {
        const FIELDS: &[&str] = &[
            "name",
            "time_stamp",
            "author",
            "organization",
            "preprocessor_version",
            "originating_system",
            "authorization",
            "documentation",
        ];
        if !FIELDS.contains(&name) {
            return Err(XmlError::UnknownAttribute {
                entity: "header".into(),
                element: "header".into(),
                attribute: name.into(),
            });
        }
        if !attributes.plain.is_empty() || attributes.xsi_type.is_some() {
            return Err(unsupported("attributes on a header field"));
        }
        self.push(
            Frame::HeaderField {
                name: name.into(),
                text: String::new(),
            },
            name.into(),
        );
        Ok(())
    }

    /// The layout of an element named by its entity type.
    pub(super) fn entity_layout(&mut self, name: &str) -> Result<Arc<EntityLayout>, XmlError> {
        self.layouts
            .entity(name, true)?
            .ok_or_else(|| XmlError::UnknownEntity { name: name.into() })
    }

    /// A child element of an entity element: one of its explicit or inverse
    /// attributes.
    pub(super) fn open_attribute(
        &mut self,
        parent: u64,
        name: &str,
        attributes: Attributes,
    ) -> Result<(), XmlError> {
        let layout = self.entities[index(parent)].layout.clone();
        if let Some(slot) = layout.slot(name) {
            let slot_layout = &layout.slots[slot];
            if slot_layout.derived {
                return Err(XmlError::TypeMismatch {
                    declared: format!("`{name}`, derived in {}", layout.name),
                    found: "a value".into(),
                });
            }
            if self.entities[index(parent)].slots[slot].is_some() {
                return Err(XmlError::DuplicateSlot {
                    name: name.into(),
                    slot,
                });
            }
            let shape = slot_layout.shape.clone();
            let target = Target::Slot {
                entity: parent,
                slot,
            };
            return self.open_declared(name, shape, target, attributes);
        }
        if let Some(inverse) = layout.inverse(name) {
            let inverse = inverse.clone();
            return self.open_inverse(parent, name, inverse, attributes);
        }
        Err(XmlError::UnknownAttribute {
            entity: layout.name.to_string(),
            element: self.stack.last().map_or_else(String::new, |top| {
                top.segment
                    .split('[')
                    .next()
                    .unwrap_or_default()
                    .to_string()
            }),
            attribute: name.into(),
        })
    }

    /// An explicit attribute's element, by the form its declared type takes.
    pub(super) fn open_declared(
        &mut self,
        name: &str,
        shape: Shape,
        target: Target,
        mut attributes: Attributes,
    ) -> Result<(), XmlError> {
        let attribute: Arc<str> = name.into();
        if shape.levels.is_empty() {
            match &shape.leaf {
                Leaf::Entity(declared) => {
                    let declared = declared.clone();
                    return self.entity_value(name, &declared, attributes, target);
                }
                Leaf::Binary => {
                    let extra = attributes.take("extraBits");
                    self.no_more_attributes(name, &attributes, true)?;
                    if attributes.xsi_nil == Some(true) {
                        return self.deliver_nil(name, target);
                    }
                    if extra.is_some_and(|extra| extra.trim() != "0") {
                        return Err(unsupported(
                            "a binary whose length is not a whole number of bytes (extraBits)",
                        ));
                    }
                    let frame = ValueFrame::new(target, shape, attribute, true, false);
                    self.push(Frame::Value(Box::new(frame)), name.into());
                    return Ok(());
                }
                Leaf::Select(_) => {
                    self.no_more_attributes(name, &attributes, true)?;
                    if attributes.xsi_nil == Some(true) {
                        return self.deliver_nil(name, target);
                    }
                    let frame = ValueFrame::new(target, shape, attribute, false, true);
                    self.push(Frame::Value(Box::new(frame)), name.into());
                    return Ok(());
                }
                _ => {
                    return Err(XmlError::WrongForm {
                        attribute: name.into(),
                        expected: "an XML attribute",
                        found: "a child element",
                    })
                }
            }
        }
        // An aggregate: a container of items.
        let array_size = self.container_attributes(name, &mut attributes)?;
        if attributes.xsi_nil == Some(true) {
            return self.deliver_nil(name, target);
        }
        let mut frame = ValueFrame::new(target, shape, attribute, false, false);
        frame.array_size = array_size;
        self.push(Frame::Value(Box::new(frame)), name.into());
        Ok(())
    }

    /// `itemType`, `cType` and `arraySize` on an aggregate container.
    pub(super) fn container_attributes(
        &self,
        name: &str,
        attributes: &mut Attributes,
    ) -> Result<Option<Vec<usize>>, XmlError> {
        attributes.take("itemType");
        attributes.take("cType");
        let array_size = attributes
            .take("arraySize")
            .map(|text| {
                text.split_ascii_whitespace()
                    .map(|size| {
                        size.parse::<usize>().map_err(|_| XmlError::InvalidScalar {
                            kind: "arraySize".into(),
                            value: text.clone(),
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?;
        self.no_more_attributes(name, attributes, true)?;
        Ok(array_size)
    }

    /// Refuse anything left on a container or wrapper start tag.
    pub(super) fn no_more_attributes(
        &self,
        name: &str,
        attributes: &Attributes,
        nil_allowed: bool,
    ) -> Result<(), XmlError> {
        if let Some((attribute, _)) = attributes.plain.first() {
            if matches!(attribute.as_str(), "id" | "pos" | "path") {
                return Err(XmlError::Unsupported {
                    construct: format!("`{attribute}` on the value element `{name}`"),
                });
            }
            return Err(XmlError::UnknownAttribute {
                entity: self.enclosing_entity().unwrap_or_default(),
                element: name.into(),
                attribute: attribute.clone(),
            });
        }
        if attributes.xsi_type.is_some() {
            return Err(XmlError::Unsupported {
                construct: format!("xsi:type on the value element `{name}`"),
            });
        }
        if attributes.xsi_nil.is_some() && !nil_allowed {
            return Err(XmlError::Unsupported {
                construct: format!("xsi:nil on the value element `{name}`"),
            });
        }
        if attributes.xsi_schema_location {
            return Err(unsupported("xsi:schemaLocation below the root"));
        }
        Ok(())
    }

    pub(super) fn deliver_nil(&mut self, name: &str, target: Target) -> Result<(), XmlError> {
        match target {
            Target::Slot { .. } => self.deliver(target, Value::Null)?,
            Target::Item | Target::Inverse { .. } => {
                return Err(unsupported("an unset (xsi:nil) aggregate item"));
            }
        }
        self.push(Frame::Empty, name.into());
        Ok(())
    }

    /// An inverse attribute's element: a container of relationships, or the
    /// relationship itself when the configuration types the element directly.
    pub(super) fn open_inverse(
        &mut self,
        parent: u64,
        name: &str,
        inverse: InverseLayout,
        mut attributes: Attributes,
    ) -> Result<(), XmlError> {
        let direct = attributes.xsi_type.is_some()
            || attributes
                .plain
                .iter()
                .any(|(key, _)| !matches!(key.as_str(), "itemType" | "cType" | "arraySize"));
        if direct {
            let declared = inverse.entity.clone();
            return self.entity_value(
                name,
                &declared,
                attributes,
                Target::Inverse { parent, inverse },
            );
        }
        let array_size = self.container_attributes(name, &mut attributes)?;
        if array_size.is_some() {
            return Err(unsupported("arraySize on an inverse attribute"));
        }
        if attributes.xsi_nil == Some(true) {
            self.push(Frame::Empty, name.into());
            return Ok(());
        }
        let shape = Shape {
            levels: vec![typing::Level {
                kind: AggregateKind::Set,
                fixed: None,
                count: (0, None),
            }],
            leaf: Leaf::Entity(inverse.entity.clone()),
            named: inverse.entity.clone(),
        };
        let frame = ValueFrame::new(
            Target::Inverse { parent, inverse },
            shape,
            name.into(),
            false,
            false,
        );
        self.push(Frame::Value(Box::new(frame)), name.into());
        Ok(())
    }
}
