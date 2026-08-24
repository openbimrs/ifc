//! Entity descriptors: name, supertype, attribute slots.

use crate::attribute::Attribute;

/// One entity type in the schema, e.g. `IfcWall`.
///
/// Attribute order is significant: STEP records are positional, so slot `i`
/// here must be slot `i` in the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityDef {
    /// Name as declared in EXPRESS, e.g. `IfcWall`.
    pub name: String,
    /// Direct supertype, if any. IFC uses single inheritance for entities.
    pub supertype: Option<String>,
    /// Whether the entity is `ABSTRACT` and so may not be instantiated.
    pub abstract_: bool,
    /// Explicit attributes **excluding** inherited ones, in declaration order.
    ///
    /// A STEP record carries inherited attributes first, so a full positional
    /// list must be assembled by walking the supertype chain. That assembly
    /// lives in the registry, which can see the whole schema.
    pub attributes: Vec<Attribute>,
}

impl EntityDef {
    /// A concrete entity with no supertype and no attributes.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            supertype: None,
            abstract_: false,
            attributes: Vec::new(),
        }
    }

    /// Set the direct supertype.
    pub fn with_supertype(mut self, supertype: impl Into<String>) -> Self {
        self.supertype = Some(supertype.into());
        self
    }

    /// Append an attribute slot.
    pub fn with_attribute(mut self, attribute: Attribute) -> Self {
        self.attributes.push(attribute);
        self
    }
}
