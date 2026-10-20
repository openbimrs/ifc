//! Lookup tables derived once from the parsed schema.
//!
//! Positional slots follow the same rule as `ifc-schema`'s bundled tables
//! (ISO 10303-21 12.2.5.2: supertypes first, in `SUBTYPE OF` order, each
//! once; redeclarations add no slot), and [`Tables::new`] refuses a source
//! whose layout differs from the bundled table it is paired with. A rule
//! therefore reads exactly the slot `structure` and `type_check` judged.

use std::collections::HashMap;
use std::sync::Arc;

use ifc_schema::Schema;

use super::ast::{Name, SchemaDecl, TypeExpr};
use super::error::ExpressError;
use super::resolve::Globals;

/// Where a named attribute of an entity comes from, most specific
/// declaration first: a DERIVE redeclaring an explicit attribute wins.
#[derive(Debug, Clone, Copy)]
pub(crate) enum AttrRef {
    Explicit {
        slot: usize,
        owner: usize,
        idx: usize,
    },
    Derived {
        owner: usize,
        idx: usize,
    },
    Inverse {
        owner: usize,
        idx: usize,
    },
}

/// The parsed schema and everything evaluation looks up in it.
#[derive(Debug)]
pub(crate) struct Tables {
    pub schema: SchemaDecl,
    pub globals: Globals,
    /// Per entity: itself, then every supertype (depth-first).
    pub ancestors: Vec<Vec<usize>>,
    /// Per entity: explicit attributes in positional order, as
    /// `(owner entity, attribute index)`.
    pub layout: Vec<Vec<(usize, usize)>>,
    /// Per entity: every attribute name it answers to.
    pub attrs: Vec<HashMap<Name, AttrRef>>,
    /// Per entity: its `TYPEOF` names.
    pub entity_typeof: Vec<Vec<Arc<str>>>,
    /// Per type: its `TYPEOF` names.
    pub type_typeof: Vec<Vec<Arc<str>>>,
}

impl std::fmt::Debug for Globals {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Globals")
            .field("entities", &self.entities.len())
            .finish_non_exhaustive()
    }
}

impl Tables {
    /// Builds the tables, checking the layout against `bundled`.
    pub(crate) fn new(
        schema: SchemaDecl,
        globals: Globals,
        bundled: &Schema,
    ) -> Result<Self, ExpressError> {
        if !schema.name.eq_ignore_ascii_case(bundled.name()) {
            return Err(ExpressError::SchemaMismatch {
                source: schema.name.to_string(),
                tables: bundled.name().to_owned(),
            });
        }
        let count = schema.entities.len();
        let ancestors: Vec<Vec<usize>> = (0..count)
            .map(|index| ancestors(&schema, &globals, index))
            .collect();
        let layout: Vec<Vec<(usize, usize)>> = (0..count)
            .map(|index| layout(&schema, &globals, index))
            .collect();
        for (index, slots) in layout.iter().enumerate() {
            let entity = &schema.entities[index];
            let ours: Vec<&str> = slots
                .iter()
                .map(|&(owner, idx)| &*schema.entities[owner].explicit[idx].spelling)
                .collect();
            let theirs = bundled.attribute_names(&entity.name);
            let agrees = bundled.entity(&entity.name).is_some()
                && ours.len() == theirs.len()
                && ours
                    .iter()
                    .zip(&theirs)
                    .all(|(a, b)| a.eq_ignore_ascii_case(b));
            if !agrees {
                return Err(ExpressError::TableMismatch {
                    entity: entity.spelling.to_string(),
                    detail: format!("source slots {ours:?}, tables {theirs:?}"),
                });
            }
        }
        if bundled.entity_count() != count {
            return Err(ExpressError::TableMismatch {
                entity: schema.name.to_string(),
                detail: format!(
                    "{count} entities in the source, {} in the tables",
                    bundled.entity_count()
                ),
            });
        }
        let attrs = (0..count)
            .map(|index| attributes(&schema, &ancestors[index], &layout[index]))
            .collect();
        let qualifier = format!("{}.", schema.name.to_ascii_uppercase());
        let selects = containing_selects(&schema);
        let entity_typeof = (0..count)
            .map(|index| {
                let names = ancestors[index]
                    .iter()
                    .map(|&a| schema.entities[a].name.clone())
                    .collect();
                qualify(&qualifier, close_over_selects(names, &selects), None)
            })
            .collect();
        let type_typeof = (0..schema.types.len())
            .map(|index| type_names(&schema, &globals, &qualifier, &selects, index))
            .collect();
        Ok(Self {
            schema,
            globals,
            ancestors,
            layout,
            attrs,
            entity_typeof,
            type_typeof,
        })
    }

    /// Whether entity `entity` is `ancestor` or a subtype of it.
    pub(crate) fn is_a(&self, entity: usize, ancestor: usize) -> bool {
        self.ancestors[entity].contains(&ancestor)
    }

    /// The entity index for a name in any case.
    pub(crate) fn entity_index(&self, name: &str) -> Option<usize> {
        self.globals
            .entities
            .get(name)
            .or_else(|| {
                self.globals
                    .entities
                    .get(name.to_ascii_uppercase().as_str())
            })
            .copied()
    }

    /// The type index for a name in any case.
    pub(crate) fn type_index(&self, name: &str) -> Option<usize> {
        self.globals
            .types
            .get(name)
            .or_else(|| self.globals.types.get(name.to_ascii_uppercase().as_str()))
            .copied()
    }
}

fn ancestors(schema: &SchemaDecl, globals: &Globals, index: usize) -> Vec<usize> {
    let mut out = vec![index];
    let mut stack = vec![index];
    while let Some(current) = stack.pop() {
        for supertype in schema.entities[current].supertypes.iter().rev() {
            if let Some(&parent) = globals.entities.get(supertype) {
                if !out.contains(&parent) {
                    out.push(parent);
                    stack.push(parent);
                }
            }
        }
    }
    out
}

fn layout(schema: &SchemaDecl, globals: &Globals, index: usize) -> Vec<(usize, usize)> {
    fn walk(
        schema: &SchemaDecl,
        globals: &Globals,
        index: usize,
        seen: &mut Vec<usize>,
        out: &mut Vec<(usize, usize)>,
    ) {
        if seen.contains(&index) {
            return;
        }
        seen.push(index);
        for supertype in &schema.entities[index].supertypes {
            if let Some(&parent) = globals.entities.get(supertype) {
                walk(schema, globals, parent, seen, out);
            }
        }
        for (idx, attr) in schema.entities[index].explicit.iter().enumerate() {
            if attr.redeclares.is_none() {
                out.push((index, idx));
            }
        }
    }
    let mut out = Vec::new();
    walk(schema, globals, index, &mut Vec::new(), &mut out);
    out
}

fn attributes(
    schema: &SchemaDecl,
    ancestors: &[usize],
    layout: &[(usize, usize)],
) -> HashMap<Name, AttrRef> {
    let mut map = HashMap::new();
    for &owner in ancestors {
        let entity = &schema.entities[owner];
        for (idx, derived) in entity.derived.iter().enumerate() {
            map.entry(derived.name.clone())
                .or_insert(AttrRef::Derived { owner, idx });
        }
        for (idx, inverse) in entity.inverses.iter().enumerate() {
            map.entry(inverse.name.clone())
                .or_insert(AttrRef::Inverse { owner, idx });
        }
    }
    for (slot, &(owner, idx)) in layout.iter().enumerate() {
        let name = schema.entities[owner].explicit[idx].name.clone();
        map.entry(name)
            .or_insert(AttrRef::Explicit { slot, owner, idx });
    }
    map
}

/// Name to the SELECT types listing it directly.
fn containing_selects(schema: &SchemaDecl) -> HashMap<Name, Vec<Name>> {
    let mut map: HashMap<Name, Vec<Name>> = HashMap::new();
    for decl in &schema.types {
        if let TypeExpr::Select(members) = &decl.underlying {
            for member in members {
                map.entry(member.clone())
                    .or_default()
                    .push(decl.name.clone());
            }
        }
    }
    map
}

/// `names` plus every SELECT reaching any of them, transitively.
fn close_over_selects(mut names: Vec<Name>, selects: &HashMap<Name, Vec<Name>>) -> Vec<Name> {
    let mut at = 0;
    while at < names.len() {
        if let Some(parents) = selects.get(&names[at]) {
            for parent in parents {
                if !names.contains(parent) {
                    names.push(parent.clone());
                }
            }
        }
        at += 1;
    }
    names
}

fn qualify(qualifier: &str, names: Vec<Name>, base: Option<&str>) -> Vec<Arc<str>> {
    let mut out: Vec<Arc<str>> = names
        .iter()
        .map(|name| Arc::from(format!("{qualifier}{name}")))
        .collect();
    if let Some(base) = base {
        out.push(Arc::from(base));
    }
    out
}

/// A defined type, the defined types it is based on, the SELECTs reaching
/// any of them, and its base type's keyword (`REAL`, `LIST`).
fn type_names(
    schema: &SchemaDecl,
    globals: &Globals,
    qualifier: &str,
    selects: &HashMap<Name, Vec<Name>>,
    index: usize,
) -> Vec<Arc<str>> {
    let mut names = vec![schema.types[index].name.clone()];
    let mut current = &schema.types[index].underlying;
    let mut base = None;
    for _ in 0..64 {
        match current {
            TypeExpr::Named(name) => match globals.types.get(name) {
                Some(&next) if !names.contains(name) => {
                    names.push(name.clone());
                    current = &schema.types[next].underlying;
                }
                _ => break,
            },
            TypeExpr::Simple(simple) => {
                base = Some(simple.keyword());
                break;
            }
            TypeExpr::Aggregate { kind, .. } => {
                base = Some(kind.keyword());
                break;
            }
            _ => break,
        }
    }
    qualify(qualifier, close_over_selects(names, selects), base)
}
