//! Entity descriptors: name, supertype, attribute slots.

use crate::attribute::Attribute;

/// One entity type in the schema, e.g. `IfcWall`.
///
/// Attribute order is significant: STEP records are positional, so slot `n`
/// here corresponds to the `n`th value in the record body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityDef {
    /// Canonical name as written in the EXPRESS schema (e.g. `IfcWall`).
    pub name: String,
    /// Direct supertype, if any. `IfcRoot` has none.
    pub supertype: Option<String>,
    /// Whether the type is abstract and thus never instantiated directly.
    pub is_abstract: bool,
    /// Positional attribute slots, including inherited ones, in STEP order.
    pub attributes: Vec<Attribute>,
}
