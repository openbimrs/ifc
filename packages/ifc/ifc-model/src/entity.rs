//! The entity record — a type name plus positional attributes.
//!
//! # Why entities are not Rust structs
//!
//! IFC4 declares 776 entity types; IFC4x3 declares 876, and renames some of
//! IFC4's. Generating a struct per entity per schema version is what makes
//! IfcOpenShell heavy, and it forces a recompile to support a new schema.
//!
//! Here an entity is a type name and an attribute vector. The schema explains
//! what the slots mean; domain crates interpret them. The immediate benefit is
//! requirement 3 of the design: **an entity whose type nothing understands is
//! still stored perfectly and written back unchanged.**

use crate::value::{EntityId, Value};
use std::sync::Arc;

/// One IFC entity instance.
#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    /// Upper-cased type name exactly as it appeared (`IFCWALL`).
    ///
    /// Case is normalized because STEP is case-insensitive for keywords but
    /// real files are inconsistent; comparing normalized names avoids a class
    /// of silent lookup misses.
    pub type_name: Arc<str>,
    /// Positional attributes, in declaration order.
    ///
    /// Order is significant: STEP records are positional, so slot 3 of
    /// `IFCWALL` is its name regardless of what any other entity looks like.
    pub attributes: Vec<Value>,
}

impl Entity {
    /// Build an entity from a type name and its attributes.
    pub fn new(type_name: impl Into<Arc<str>>, attributes: Vec<Value>) -> Self {
        Self {
            type_name: type_name.into(),
            attributes,
        }
    }

    /// Attribute at `index`, or `None` when the slot does not exist.
    ///
    /// Returns `None` rather than panicking because real files are routinely
    /// short a trailing optional attribute, and a reader that panics on that
    /// is useless in practice.
    pub fn attribute(&self, index: usize) -> Option<&Value> {
        self.attributes.get(index)
    }

    /// Attribute at `index` interpreted as text.
    pub fn text(&self, index: usize) -> Option<&str> {
        self.attribute(index)?.unwrap_typed().as_text()
    }

    /// Attribute at `index` interpreted as a number.
    pub fn number(&self, index: usize) -> Option<f64> {
        self.attribute(index)?.unwrap_typed().as_f64()
    }

    /// Attribute at `index` interpreted as an entity reference.
    pub fn reference(&self, index: usize) -> Option<EntityId> {
        self.attribute(index)?.as_ref_id()
    }

    /// Every entity this one refers to, at any nesting depth.
    pub fn references(&self) -> Vec<EntityId> {
        let mut out = Vec::new();
        for attr in &self.attributes {
            attr.for_each_ref(&mut |id| out.push(id));
        }
        out
    }

    /// Case-insensitive type-name test.
    pub fn is_type(&self, name: &str) -> bool {
        self.type_name.eq_ignore_ascii_case(name)
    }
}
