//! EXPRESS source parsing, behind the opt-in `express` feature.
//!
//! Extraction is delegated to `openbim_step::express` and its result is
//! converted straight into this crate's own declaration types. No
//! `openbim-step` type crosses this crate's public API, so a parser release
//! cannot change it; and without the feature `openbim-step` is not linked at
//! all -- the bundled tables decode without it.
//!
//! The conversion keeps exactly what the bundled artifacts record: explicit
//! redeclarations of inherited attributes (`SELF\X.a : T;`) are dropped,
//! because they add no Part 21 slot and no bundled table carries them.

use openbim_step::express::{self as step, ParsedSchema};

use crate::attribute::Attribute;
use crate::entity::{EntityDef, WhereRule};
use crate::registry::Schema;
use crate::types::{TypeDef, TypeKind};

/// Parses EXPRESS source text into an owned [`Schema`].
pub(crate) fn parse(source: &str) -> Schema {
    convert(step::parse(source))
}

fn convert(parsed: ParsedSchema) -> Schema {
    let entities = parsed.entities.into_iter().map(entity).collect();
    let types = parsed.types.into_iter().map(type_def).collect();
    Schema::new(parsed.name, entities, types)
}

fn entity(parsed: step::EntityDef) -> EntityDef {
    let mut def = EntityDef::new(parsed.name);
    def.supertypes = parsed.supertypes;
    def.abstract_ = parsed.abstract_;
    def.attributes = parsed.attributes.into_iter().map(attribute).collect();
    def.derived = parsed.derived;
    def.where_rules = parsed
        .where_rules
        .into_iter()
        .map(|rule| WhereRule::new(rule.label, rule.expression))
        .collect();
    def
}

fn attribute(parsed: step::Attribute) -> Attribute {
    let mut built = Attribute::new(parsed.name, parsed.type_name);
    built.optional = parsed.optional;
    built.aggregate = parsed.aggregate;
    built
}

fn type_def(parsed: step::TypeDef) -> TypeDef {
    let kind = match parsed.kind {
        step::TypeKind::Defined(alias) => TypeKind::Defined(alias),
        step::TypeKind::Enumeration(members) => TypeKind::Enumeration(members),
        step::TypeKind::Select(members) => TypeKind::Select(members),
    };
    TypeDef::new(parsed.name, kind)
}
