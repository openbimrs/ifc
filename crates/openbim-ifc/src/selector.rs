//! A selector query language for filtering elements, and values read and
//! written by path (#464).
//!
//! A selector is text such as
//! `IfcWall, Pset_WallCommon.IsExternal=TRUE, location="Level 1"`: walls
//! that are external and sit on Level 1. It parses
//! ([`Selector::parse`]) into a typed query, with every error a
//! [`ParseError`] carrying the byte span it refuses, and is evaluated
//! ([`SelectorIndex::select`]) against a model and the release its header
//! declares. A [`ValuePath`] such as `type.Name` or
//! `Pset_WallCommon.FireRating` reads one value of an entity
//! ([`SelectorIndex::value`]) and writes it ([`set_value`]) through a
//! transaction, checked against the release.
//!
//! # Grammar, version [`GRAMMAR_VERSION`]
//!
//! ```text
//! selector   = list { "+" list } [ "+" ]
//! list       = facet { "," facet }
//! facet      = [ "!" ] GLOBALID                 instance
//!            | [ "!" ] CLASS                    class, subtypes included
//!            | ATTRIBUTE comparison value       explicit attribute
//!            | key "." key comparison value    property or quantity
//!            | KEYWORD comparison value         type material classification
//!                                               location container group
//!            | "query:" path comparison value  a value path
//! path       = key { "." key }
//! comparison = [ "!" ] "=" | [ "!" ] "*=" | ">" | ">=" | "<" | "<="
//! value      = "NULL" | "TRUE" | "FALSE" | quoted | pattern | unquoted
//! key        = quoted | pattern | unquoted
//! quoted     = '"' ... '"'                     \" and \\ escape
//! pattern    = "/" ... "/"                      matched from the start
//! ```
//!
//! `GLOBALID` is 22 characters of IFC's alphabet; `CLASS` is `Ifc` (any
//! case) followed by letters, digits and `_`; `ATTRIBUTE` starts with an
//! upper-case letter. Whitespace between tokens and `/* ... */` comments
//! are ignored. The `docs/guide/selectors.md` page has the full grammar,
//! the comparison rules and every difference from IfcOpenShell's
//! `ifcopenshell.util.selector`.
//!
//! # Semantics
//!
//! A facet list works on one set, left to right: a class or `GlobalId`
//! facet adds to it (removes with `!`), every other facet keeps only the
//! entities it holds for, and a list that starts with a filter or a `!`
//! facet starts from every `IfcProduct` and `IfcTypeProduct` (or the base
//! set a caller passes). The lists' sets are united, and the answer is in
//! ascending id order.
//!
//! # Cost
//!
//! Class facets read the model's type index, subtypes through the
//! release's tree; `GlobalId` facets a [`GlobalIdIndex`](crate::GlobalIdIndex);
//! property facets the exact resolver's `PropertyIndex` (feature
//! `properties`); relationship facets one scan per relationship
//! kind. Each index is built once per [`SelectorIndex`], on first use.
//!
//! ```
//! # #[cfg(all(feature = "step", feature = "ifc4"))] {
//! use ifc::selector::{Selector, SelectorIndex, SelectedValue, ValuePath};
//! use ifc::{Codec, EntityId, StepCodec};
//!
//! let model = StepCodec.read_bytes(b"ISO-10303-21;HEADER;FILE_DESCRIPTION((''),'2;1');\
//!     FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;DATA;\
//!     #1=IFCWALLTYPE('0dGA15OQf9_P5UdNj5qcto',$,'WT',$,$,$,$,$,$,.SOLIDWALL.);\
//!     #2=IFCWALL('31t64Uzj98DhdKplMxBpF5',$,'Wall',$,$,$,$,$,$);\
//!     #3=IFCRELDEFINESBYTYPE('0xtMgJ4Tb6LQGWVe9NZXe6',$,$,$,(#2),#1);\
//!     #4=IFCSLAB('1kTvXnbbzCWw8lcMd1dR4o',$,'Slab',$,$,$,$,$,$);\
//!     ENDSEC;END-ISO-10303-21;").unwrap();
//! let index = SelectorIndex::new(&model, ifc::schema::ifc4());
//!
//! let walls = Selector::parse("IfcWall, type=WT + IfcSlab").unwrap();
//! assert_eq!(index.select(&walls).unwrap(), [EntityId(2), EntityId(4)]);
//!
//! let path = ValuePath::parse("type.Name").unwrap();
//! assert_eq!(index.value(EntityId(2), &path).unwrap(), SelectedValue::Text("WT".into()));
//! # }
//! ```
#![cfg(feature = "selector")]

mod ast;
mod error;
mod eval;
mod exact;
mod index;
mod parse;
mod path;
mod set;
mod value;

pub use ast::{
    Comparison, Facet, FacetKind, FacetList, Key, Keyword, Literal, Operator, PathKey, Pattern,
    Selector, ValuePath,
};
pub use error::{ParseError, ParseErrorKind, SelectorError};
pub use index::SelectorIndex;
pub use set::{set_value, SetTarget};
pub use value::{compare, SelectedValue};

use ifc_model::{EntityId, Model};
use ifc_schema::Schema;

/// The version of the grammar this module parses.
///
/// The minor version grows when the grammar accepts more; the major
/// version when a selector that parsed before parses differently or not
/// at all.
pub const GRAMMAR_VERSION: &str = "1.0";

/// The entities `selector` selects from `model`, read against `schema`,
/// in ascending id order. Builds a [`SelectorIndex`] for the one query;
/// keep one to run several.
///
/// # Errors
///
/// As [`SelectorIndex::select`].
pub fn select(
    model: &Model,
    schema: &Schema,
    selector: &Selector,
) -> Result<Vec<EntityId>, SelectorError> {
    SelectorIndex::new(model, schema).select(selector)
}

/// The value `path` reads from `entity`.
///
/// # Errors
///
/// As [`SelectorIndex::value`].
pub fn get_value(
    model: &Model,
    schema: &Schema,
    entity: EntityId,
    path: &ValuePath,
) -> Result<SelectedValue, SelectorError> {
    SelectorIndex::new(model, schema).value(entity, path)
}

#[cfg(test)]
mod tests;
