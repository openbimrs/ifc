//! EXPRESS source parsing, behind the opt-in `express` feature.
//!
//! Extraction is delegated to `openbim_step::express` and its result is
//! converted straight into this crate's own declaration types. No
//! `openbim-step` type crosses this crate's public API, so a parser release
//! cannot change it; and without the feature `openbim-step` is not linked at
//! all -- the bundled tables decode without it.
//!
//! The conversion keeps what the bundled artifacts record: attributes with
//! their aggregation levels and bounds, derived names, WHERE rules, INVERSE
//! attributes and UNIQUE rules. Explicit redeclarations of inherited
//! attributes (`SELF\X.a : T;`) are dropped: they add no Part 21 slot and no
//! bundled table carries them.
//!
//! `openbim-step` is pinned exactly (`=0.10.0`), and its declaration enums
//! are `#[non_exhaustive]`: a variant this conversion does not name cannot
//! occur in the pinned version, and a new one must be mapped here before the
//! pin moves. The wildcard arms say so rather than guess.

use openbim_step::express::{self as step, ParsedSchema};

use crate::attribute::{AggregateKind, Aggregation, Attribute, Bound};
use crate::entity::{EntityDef, InverseAttribute, UniqueRule, WhereRule};
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
    def.inverses = parsed.inverses.into_iter().map(inverse).collect();
    def.unique_rules = parsed
        .unique_rules
        .into_iter()
        .map(|rule| {
            let mut owned = UniqueRule::unlabelled(rule.attributes);
            owned.label = rule.label;
            owned
        })
        .collect();
    def
}

fn attribute(parsed: step::Attribute) -> Attribute {
    let mut built = Attribute::new(parsed.name, parsed.type_name);
    built.optional = parsed.optional;
    built.aggregate = parsed.aggregate;
    built.aggregation = parsed.aggregation.into_iter().map(aggregation).collect();
    built
}

fn inverse(parsed: step::InverseAttribute) -> InverseAttribute {
    let mut built = InverseAttribute::new(parsed.name, parsed.entity, parsed.for_attribute);
    built.redeclares = parsed.redeclares;
    built.aggregation = parsed.aggregation.map(aggregation);
    built
}

fn aggregation(parsed: step::Aggregation) -> Aggregation {
    let kind = match parsed.kind {
        step::AggregateKind::List => AggregateKind::List,
        step::AggregateKind::Set => AggregateKind::Set,
        step::AggregateKind::Bag => AggregateKind::Bag,
        step::AggregateKind::Array => AggregateKind::Array,
        other => unreachable!("openbim-step =0.10.0 declares no aggregate kind {other:?}"),
    };
    let mut built = Aggregation::new(kind, bound(parsed.lower), bound(parsed.upper));
    built.unique = parsed.unique;
    built.optional_elements = parsed.optional_elements;
    built
}

fn bound(parsed: step::Bound) -> Bound {
    match parsed {
        step::Bound::Integer(value) => Bound::Integer(value),
        step::Bound::Unbounded => Bound::Unbounded,
        step::Bound::Expression(text) => Bound::Expression(text),
        other => unreachable!("openbim-step =0.10.0 declares no bound form {other:?}"),
    }
}

fn type_def(parsed: step::TypeDef) -> TypeDef {
    let kind = match parsed.kind {
        step::TypeKind::Defined(alias) => TypeKind::Defined(alias),
        step::TypeKind::Enumeration(members) => TypeKind::Enumeration(members),
        step::TypeKind::Select(members) => TypeKind::Select(members),
        other => unreachable!("openbim-step =0.10.0 declares no type kind {other:?}"),
    };
    TypeDef::new(parsed.name, kind)
}
