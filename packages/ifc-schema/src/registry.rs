//! The assembled, queryable schema.
//!
//! Owns the entity and type tables for one [`crate::SchemaVersion`] and answers
//! the two questions consumers actually ask:
//!
//! 1. Is this entity a kind of that one? (`is_a`)
//! 2. What is the name of attribute slot `i`? (`attribute_names`)
//!
//! The second is what makes a conformant ifcXML writer possible: STEP records
//! are positional, XML is named, so crossing between them requires the schema.

use crate::attribute::Attribute;
use crate::entity::EntityDef;
use crate::express::{self, ParsedSchema};
use crate::types::TypeDef;
use crate::SchemaVersion;
use std::collections::HashMap;

/// A queryable schema for one IFC version.
#[derive(Debug)]
pub struct Schema {
    version: Option<SchemaVersion>,
    name: String,
    entities: HashMap<String, EntityDef>,
    types: HashMap<String, TypeDef>,
}

impl Schema {
    /// Build from a parsed EXPRESS document.
    ///
    /// Lookup keys are upper-cased because STEP writes `IFCWALL` while EXPRESS
    /// declares `IfcWall`; normalizing once here avoids every call site
    /// remembering to.
    pub fn from_parsed(parsed: ParsedSchema) -> Self {
        let version = SchemaVersion::from_header_token(&parsed.name);
        let entities = parsed
            .entities
            .into_iter()
            .map(|e| (e.name.to_ascii_uppercase(), e))
            .collect();
        let types = parsed
            .types
            .into_iter()
            .map(|t| (t.name.to_ascii_uppercase(), t))
            .collect();
        Self {
            version,
            name: parsed.name,
            entities,
            types,
        }
    }

    /// Parse an EXPRESS schema document directly.
    pub fn from_express(source: &str) -> Self {
        Self::from_parsed(express::parse(source))
    }

    /// The schema name as declared, e.g. `IFC4`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The recognized version, if the name maps to one we know.
    pub fn version(&self) -> Option<SchemaVersion> {
        self.version
    }

    /// How many entity types are declared.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// How many types are declared.
    pub fn type_count(&self) -> usize {
        self.types.len()
    }

    /// Look up an entity declaration, case-insensitively.
    pub fn entity(&self, name: &str) -> Option<&EntityDef> {
        self.entities.get(&name.to_ascii_uppercase())
    }

    /// Look up a type declaration, case-insensitively.
    pub fn type_def(&self, name: &str) -> Option<&TypeDef> {
        self.types.get(&name.to_ascii_uppercase())
    }

    /// Is `name` the same as, or a subtype of, `ancestor`?
    ///
    /// The most-called query in any IFC tool: every filter and rule is
    /// ultimately "give me the walls, including subtypes". Returns `false` for
    /// unknown entities rather than erroring, so a file containing a
    /// future-schema entity still answers queries about the entities it does
    /// know.
    pub fn is_a(&self, name: &str, ancestor: &str) -> bool {
        let target = ancestor.to_ascii_uppercase();
        let mut current = self.entities.get(&name.to_ascii_uppercase());
        // Bounded to guard against a malformed schema with a cyclic chain.
        for _ in 0..64 {
            let Some(def) = current else { return false };
            if def.name.to_ascii_uppercase() == target {
                return true;
            }
            let Some(sup) = def.supertype.as_ref() else {
                return false;
            };
            current = self.entities.get(&sup.to_ascii_uppercase());
        }
        false
    }

    /// The supertype chain from `name` upward, excluding `name` itself.
    pub fn supertypes(&self, name: &str) -> Vec<&str> {
        let mut out = Vec::new();
        let mut current = self.entities.get(&name.to_ascii_uppercase());
        for _ in 0..64 {
            let Some(def) = current else { break };
            let Some(sup) = def.supertype.as_ref() else {
                break;
            };
            match self.entities.get(&sup.to_ascii_uppercase()) {
                Some(parent) => {
                    out.push(parent.name.as_str());
                    current = Some(parent);
                }
                None => break,
            }
        }
        out
    }

    /// Every attribute slot in **STEP positional order**, inherited first.
    ///
    /// This is the ordering rule that makes positional records work: a record
    /// lists the supertype's attributes before the subtype's own, most-general
    /// first. Getting this backwards misreads every attribute of every
    /// inheriting entity, which is why it is tested against a real chain.
    pub fn attributes(&self, name: &str) -> Vec<&Attribute> {
        let mut chain: Vec<&EntityDef> = Vec::new();
        let mut current = self.entities.get(&name.to_ascii_uppercase());
        for _ in 0..64 {
            let Some(def) = current else { break };
            chain.push(def);
            let Some(sup) = def.supertype.as_ref() else {
                break;
            };
            current = self.entities.get(&sup.to_ascii_uppercase());
        }
        chain.reverse();
        chain.iter().flat_map(|d| d.attributes.iter()).collect()
    }

    /// Attribute names in positional order.
    ///
    /// The bridge STEP-to-XML needs: slot `i` is called `names[i]`.
    pub fn attribute_names(&self, name: &str) -> Vec<&str> {
        self.attributes(name)
            .into_iter()
            .map(|a| a.name.as_str())
            .collect()
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
END_SCHEMA;
";

    fn schema() -> Schema {
        Schema::from_express(CHAIN)
    }

    #[test]
    fn is_a_walks_the_whole_chain() {
        let s = schema();
        assert!(s.is_a("IfcObject", "IfcRoot"), "grandparent");
        assert!(s.is_a("IfcObject", "IfcObject"), "reflexive");
        assert!(!s.is_a("IfcRoot", "IfcObject"), "not upward");
    }

    /// STEP writes IFCWALL, EXPRESS declares IfcWall.
    #[test]
    fn lookups_are_case_insensitive() {
        let s = schema();
        assert!(s.is_a("IFCOBJECT", "ifcroot"));
        assert!(s.entity("IFCROOT").is_some());
    }

    #[test]
    fn unknown_entities_answer_false_rather_than_panicking() {
        let s = schema();
        assert!(!s.is_a("IfcFutureThing", "IfcRoot"));
        assert!(s.attributes("IfcFutureThing").is_empty());
    }

    /// The ordering rule positional records depend on.
    #[test]
    fn inherited_attributes_come_first_and_in_order() {
        let s = schema();
        assert_eq!(
            s.attribute_names("IfcObject"),
            [
                "GlobalId",
                "OwnerHistory",
                "Name",
                "Description",
                "ObjectType"
            ],
            "supertype slots must precede the subtype's own"
        );
    }

    #[test]
    fn supertype_chain_is_reported_upward() {
        let s = schema();
        assert_eq!(
            s.supertypes("IfcObject"),
            ["IfcObjectDefinition", "IfcRoot"]
        );
    }
}
