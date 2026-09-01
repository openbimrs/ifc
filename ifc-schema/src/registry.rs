//! The assembled, queryable IFC schema.
//!
//! # What this owns, and what it delegates
//!
//! Supertype chains, Part 21 positional attribute order, case-insensitive
//! lookup and defined-type alias resolution are not IFC concepts: every
//! EXPRESS schema serialized as Part 21 shares them. They live in
//! [`openbim_step::SchemaGraph`], and this type delegates to it.
//!
//! What stays here is genuinely IFC: which schema *version* a file declares
//! (the `IFC2X3`/`IFC4`/`IFC4X3` tokens), and each version's independently
//! bundled tables.
//!
//! ```
//! use ifc_schema::Schema;
//!
//! let schema = Schema::from_express(
//!     "SCHEMA IFC4;\n\
//!      ENTITY IfcRoot; GlobalId : IfcGloballyUniqueId; END_ENTITY;\n\
//!      ENTITY IfcWall SUBTYPE OF (IfcRoot); Name : IfcLabel; END_ENTITY;\n\
//!      END_SCHEMA;",
//! );
//!
//! assert!(schema.is_a("IFCWALL", "IfcRoot"));
//! assert_eq!(schema.attribute_names("IfcWall"), ["GlobalId", "Name"]);
//! ```

use std::collections::HashSet;

use openbim_step::express::{Attribute, EntityDef, ParsedSchema, TypeDef, TypeKind};
use openbim_step::SchemaGraph;

use crate::version::SchemaVersion;

/// An IFC schema: the entity and type tables, queryable.
#[derive(Debug, Clone)]
pub struct Schema {
    graph: SchemaGraph,
}

impl Schema {
    /// Wraps an already-parsed schema.
    #[must_use]
    pub fn from_parsed(parsed: ParsedSchema) -> Self {
        Self {
            graph: SchemaGraph::new(parsed),
        }
    }

    /// Parses EXPRESS source into a schema.
    #[must_use]
    pub fn from_express(source: &str) -> Self {
        Self {
            graph: SchemaGraph::from_express(source),
        }
    }

    /// Parses EXPRESS source that is not valid UTF-8.
    ///
    /// The normative `IFC4.exp` is Latin-1: it contains `°` and similar in
    /// comments. Decoding byte-per-char is correct for the ASCII structure
    /// this parser reads and cannot fail.
    #[must_use]
    pub fn from_express_bytes(bytes: &[u8]) -> Self {
        let text: String = bytes.iter().map(|&byte| byte as char).collect();
        Self::from_express(&text)
    }

    /// The declared schema name, e.g. `IFC4`.
    #[must_use]
    pub fn name(&self) -> &str {
        self.graph.name()
    }

    /// The IFC schema version this table describes, when recognized.
    ///
    /// This is the one genuinely IFC-specific query on this type: it maps a
    /// declared schema name onto the versions this crate knows about.
    #[must_use]
    pub fn version(&self) -> Option<SchemaVersion> {
        SchemaVersion::from_header_token(self.graph.name())
    }

    /// How many entity declarations the schema holds.
    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.graph.entity_count()
    }

    /// How many type declarations the schema holds.
    #[must_use]
    pub fn type_count(&self) -> usize {
        self.graph.type_count()
    }

    /// The entity declaration for `name`, if the schema declares one.
    #[must_use]
    pub fn entity(&self, name: &str) -> Option<&EntityDef> {
        self.graph.entity(name)
    }

    /// The type declaration for `name`, if the schema declares one.
    #[must_use]
    pub fn type_def(&self, name: &str) -> Option<&TypeDef> {
        self.graph.type_def(name)
    }

    /// Every entity name the schema declares, in unspecified order.
    pub fn entity_names(&self) -> impl Iterator<Item = &str> {
        self.graph.entity_names()
    }

    /// Whether a candidate entity or defined type satisfies a declared type.
    ///
    /// This walks entity inheritance, defined-type aliases, and nested SELECTs.
    /// Unknown declarations and cyclic aliases fail closed.
    #[must_use]
    pub fn accepts_type(&self, declared: &str, candidate: &str) -> bool {
        self.accepts_type_inner(declared, candidate, &mut HashSet::new(), 32)
    }

    fn accepts_type_inner(
        &self,
        declared: &str,
        candidate: &str,
        seen: &mut HashSet<(String, String)>,
        depth: usize,
    ) -> bool {
        if declared.eq_ignore_ascii_case(candidate) {
            return self.entity(declared).is_some() || self.type_def(declared).is_some();
        }
        if depth == 0
            || !seen.insert((
                declared.to_ascii_uppercase(),
                candidate.to_ascii_uppercase(),
            ))
        {
            return false;
        }
        if self.entity(declared).is_some() && self.entity(candidate).is_some() {
            return self.is_a(candidate, declared);
        }
        if let Some(definition) = self.type_def(declared) {
            match &definition.kind {
                TypeKind::Defined(alias) => {
                    if self.accepts_type_inner(alias, candidate, seen, depth - 1) {
                        return true;
                    }
                }
                TypeKind::Select(members) => {
                    if members
                        .iter()
                        .any(|member| self.accepts_type_inner(member, candidate, seen, depth - 1))
                    {
                        return true;
                    }
                }
                TypeKind::Enumeration(_) => {}
            }
        }
        if let Some(definition) = self.type_def(candidate) {
            if let TypeKind::Defined(alias) = &definition.kind {
                return self.accepts_type_inner(declared, alias, seen, depth - 1);
            }
        }
        false
    }

    /// Whether `name` is `ancestor`, or inherits from it.
    #[must_use]
    pub fn is_a(&self, name: &str, ancestor: &str) -> bool {
        self.graph.is_a(name, ancestor)
    }

    /// The supertype chain above `name`, nearest parent first.
    #[must_use]
    pub fn supertypes(&self, name: &str) -> Vec<&str> {
        self.graph.supertypes(name)
    }

    /// Every attribute slot in Part 21 positional order, inherited first.
    #[must_use]
    pub fn attributes(&self, name: &str) -> Vec<&Attribute> {
        self.graph.attributes(name)
    }

    /// Attribute names in positional order.
    #[must_use]
    pub fn attribute_names(&self, name: &str) -> Vec<&str> {
        self.graph.attribute_names(name)
    }

    /// Resolves a defined type to the base it ultimately aliases.
    ///
    /// `IfcPositiveLengthMeasure` -> `IfcLengthMeasure` -> `REAL`.
    #[must_use]
    pub fn resolve_defined(&self, name: &str) -> String {
        self.graph.resolve_defined(name)
    }

    /// The underlying schema graph.
    ///
    /// Exposed so schema-neutral consumers can work against the generic type
    /// rather than this IFC-flavoured wrapper.
    #[must_use]
    pub fn graph(&self) -> &SchemaGraph {
        &self.graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHAIN: &str = "\
SCHEMA IFC4;
ENTITY IfcRoot
 ABSTRACT SUPERTYPE OF (ONEOF(IfcObjectDefinition));
  GlobalId : IfcGloballyUniqueId;
  OwnerHistory : OPTIONAL IfcOwnerHistory;
  Name : OPTIONAL IfcLabel;
  Description : OPTIONAL IfcText;
END_ENTITY;
ENTITY IfcObjectDefinition
 ABSTRACT SUPERTYPE OF (ONEOF(IfcObject))
 SUBTYPE OF (IfcRoot);
END_ENTITY;
ENTITY IfcObject
 SUBTYPE OF (IfcObjectDefinition);
  ObjectType : OPTIONAL IfcLabel;
END_ENTITY;
TYPE IfcLengthMeasure = REAL; END_TYPE;
TYPE IfcPositiveLengthMeasure = IfcLengthMeasure; END_TYPE;
END_SCHEMA;";

    #[test]
    fn the_declared_schema_name_maps_onto_a_known_ifc_version() {
        let schema = Schema::from_express(CHAIN);
        assert_eq!(schema.name(), "IFC4");
        assert_eq!(schema.version(), Some(SchemaVersion::Ifc4));
    }

    /// A schema this crate does not recognize still parses and queries.
    #[test]
    fn an_unrecognized_schema_name_has_no_version_but_still_works() {
        let schema = Schema::from_express(
            "SCHEMA AP242; ENTITY Product; Id : Identifier; END_ENTITY; END_SCHEMA;",
        );
        assert_eq!(schema.version(), None, "not an IFC schema");
        assert_eq!(schema.attribute_names("Product"), ["Id"]);
    }

    #[test]
    fn inherited_attributes_come_first_in_positional_order() {
        assert_eq!(
            Schema::from_express(CHAIN).attribute_names("IFCOBJECT"),
            [
                "GlobalId",
                "OwnerHistory",
                "Name",
                "Description",
                "ObjectType"
            ],
        );
    }

    #[test]
    fn defined_types_resolve_through_the_alias_chain() {
        assert_eq!(
            Schema::from_express(CHAIN).resolve_defined("IfcPositiveLengthMeasure"),
            "REAL"
        );
    }
}
