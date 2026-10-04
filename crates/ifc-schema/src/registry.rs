//! The assembled, queryable IFC schema.
//!
//! # What this owns
//!
//! Supertype chains, Part 21 positional attribute order, case-insensitive
//! lookup and defined-type alias resolution. They are not IFC concepts --
//! every EXPRESS schema serialized as Part 21 shares them -- but this crate
//! answers them over its own declaration types, so decoding and querying a
//! bundled table needs no parser crate. The walks follow the same rules as
//! `openbim_step::SchemaGraph` (ISO 10303-21 §12.2.5.2 slot order,
//! case-folded lookup, bounded chains).
//!
//! What is genuinely IFC here: which schema *version* a file declares (the
//! `FILE_SCHEMA` tokens), and each version's independently bundled tables.
//!
//! Parsing EXPRESS text (`Schema::from_express`) needs the opt-in `express`
//! feature. A schema can always be assembled from declarations:
//!
//! ```
//! use ifc_schema::{Attribute, EntityDef, Schema};
//!
//! let schema = Schema::new(
//!     "IFC4",
//!     vec![
//!         EntityDef::new("IfcRoot")
//!             .with_attribute(Attribute::new("GlobalId", "IfcGloballyUniqueId")),
//!         EntityDef::new("IfcWall")
//!             .with_supertype("IfcRoot")
//!             .with_attribute(Attribute::new("Name", "IfcLabel")),
//!     ],
//!     Vec::new(),
//! );
//!
//! assert!(schema.is_a("IFCWALL", "IfcRoot"));
//! assert_eq!(schema.attribute_names("IfcWall"), ["GlobalId", "Name"]);
//! ```

use std::collections::{HashMap, HashSet};

use crate::attribute::Attribute;
use crate::entity::EntityDef;
use crate::types::{TypeDef, TypeKind};
use crate::version::SchemaVersion;

/// Longest supertype or alias chain walked before giving up.
///
/// A cyclic `SUBTYPE OF` is not legal EXPRESS, but a malformed source must
/// not hang a consumer. Real schemas nest around a dozen levels.
const MAX_CHAIN_DEPTH: usize = 64;

/// An IFC schema: the entity and type tables, queryable.
///
/// Every lookup folds ASCII case: Part 21 spells `IFCWALL`, the schema
/// `IfcWall`, and either may be passed.
#[derive(Debug, Clone)]
pub struct Schema {
    name: String,
    /// Declarations in source order. A name declared twice keeps its first
    /// position and its last declaration.
    entities: Vec<EntityDef>,
    types: Vec<TypeDef>,
    /// Upper-cased name to index into `entities` / `types`.
    entity_index: HashMap<String, usize>,
    type_index: HashMap<String, usize>,
    /// Upper-cased supertype name to the declared names of its direct
    /// subtypes, sorted. Derived once: `EntityDef` records only the upward
    /// edge, and scanning every declaration per query is quadratic.
    children: HashMap<String, Vec<String>>,
    /// Per entity, parallel to `entities`: its supertype chain and its
    /// positional attribute slots, precomputed (#352). Both are pure
    /// functions of the declarations, so they are derived once here and
    /// [`Self::is_a`], [`Self::supertypes`] and [`Self::attributes`] read
    /// them instead of walking the declarations on every call.
    lineage: Vec<Lineage>,
}

/// One entity's precomputed supertype chain and attribute layout.
#[derive(Debug, Clone, Default)]
struct Lineage {
    /// [`Schema::supertypes`], nearest parent first.
    supertypes: Box<[Ancestor]>,
    /// [`Schema::attributes`] as `(entity index, attribute index)` pairs
    /// into `Schema::entities`, in Part 21 positional order.
    slots: Box<[(u32, u32)]>,
}

/// One member of a supertype chain.
#[derive(Debug, Clone)]
enum Ancestor {
    /// A declared entity, by index into `Schema::entities`.
    Declared(usize),
    /// A supertype the schema names but never declares, as written.
    Undeclared(Box<str>),
}

/// Equal when the name and the declarations, in order, are equal.
impl PartialEq for Schema {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.entities == other.entities && self.types == other.types
    }
}

impl Eq for Schema {}

impl Schema {
    /// Assembles a schema from declarations in source order.
    ///
    /// A name declared more than once keeps its first position and its last
    /// declaration, so counts are of distinct names.
    #[must_use]
    pub fn new(name: impl Into<String>, entities: Vec<EntityDef>, types: Vec<TypeDef>) -> Self {
        let (entities, entity_index) = index_by_name(entities, |entity| &entity.name);
        let (types, type_index) = index_by_name(types, |type_def| &type_def.name);
        let mut children: HashMap<String, Vec<String>> = HashMap::new();
        for entity in &entities {
            for supertype in &entity.supertypes {
                children
                    .entry(supertype.to_ascii_uppercase())
                    .or_default()
                    .push(entity.name.clone());
            }
        }
        for names in children.values_mut() {
            names.sort_unstable_by_key(|name| name.to_ascii_uppercase());
            // `SUBTYPE OF (a, a)` is illegal but must not double-report.
            names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        }
        let mut schema = Self {
            name: name.into(),
            entities,
            types,
            entity_index,
            type_index,
            children,
            lineage: Vec::new(),
        };
        schema.lineage = (0..schema.entities.len())
            .map(|index| schema.derive_lineage(index))
            .collect();
        schema
    }

    /// Walks one entity's supertype chain and attribute layout, once.
    ///
    /// These are the walks `supertypes` and `attributes` ran per call
    /// before #352, unchanged: the same visiting order, the same
    /// first-reached-wins rule and the same depth bound, so every answer
    /// read from the precomputed tables is the answer the walk gave.
    fn derive_lineage(&self, index: usize) -> Lineage {
        let name = self.entities[index].name.as_str();
        let mut seen = HashSet::new();
        seen.insert(name.to_ascii_uppercase());
        let mut supertypes = Vec::new();
        self.collect_supertypes(name, 0, &mut seen, &mut supertypes);
        let mut seen = HashSet::new();
        let mut slots = Vec::new();
        self.collect_attributes(name, 0, &mut seen, &mut slots);
        Lineage {
            supertypes: supertypes.into_boxed_slice(),
            slots: slots.into_boxed_slice(),
        }
    }

    /// The index into `entities` of the declaration for `name`.
    ///
    /// Case-folded without allocating: an upper-case name (the Part 21
    /// spelling) is looked up as given, and a short mixed-case one is
    /// folded on the stack.
    fn entity_position(&self, name: &str) -> Option<usize> {
        with_folded(name, |key| self.entity_index.get(key).copied())
    }

    /// The precomputed lineage of `name`, when it is declared.
    fn lineage_of(&self, name: &str) -> Option<&Lineage> {
        self.entity_position(name).map(|index| &self.lineage[index])
    }

    fn ancestor_name<'s>(&'s self, ancestor: &'s Ancestor) -> &'s str {
        match ancestor {
            Ancestor::Declared(index) => self.entities[*index].name.as_str(),
            Ancestor::Undeclared(name) => name,
        }
    }

    fn slot(&self, (entity, attribute): (u32, u32)) -> &Attribute {
        &self.entities[entity as usize].attributes[attribute as usize]
    }

    /// Parses EXPRESS source into a schema.
    ///
    /// Requires the `express` feature, which links the `openbim-step`
    /// EXPRESS extractor. Its result is converted into this crate's own
    /// types; explicit attribute redeclarations are not recorded.
    #[cfg(feature = "express")]
    #[must_use]
    pub fn from_express(source: &str) -> Self {
        crate::express::parse(source)
    }

    /// Parses EXPRESS source that is not valid UTF-8.
    ///
    /// The normative `IFC4.exp` is Latin-1: it contains `°` and similar in
    /// comments. Decoding byte-per-char is correct for the ASCII structure
    /// this parser reads and cannot fail. Requires the `express` feature.
    #[cfg(feature = "express")]
    #[must_use]
    pub fn from_express_bytes(bytes: &[u8]) -> Self {
        let text: String = bytes.iter().map(|&byte| byte as char).collect();
        Self::from_express(&text)
    }

    /// The declared schema name, e.g. `IFC4`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The IFC schema version this table describes, when recognized.
    ///
    /// This is the one genuinely IFC-specific query on this type: it maps a
    /// declared schema name onto the versions this crate knows about.
    #[must_use]
    pub fn version(&self) -> Option<SchemaVersion> {
        SchemaVersion::from_header_token(&self.name)
    }

    /// How many entity declarations the schema holds.
    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// How many type declarations the schema holds.
    #[must_use]
    pub fn type_count(&self) -> usize {
        self.types.len()
    }

    /// The entity declaration for `name`, if the schema declares one.
    #[must_use]
    pub fn entity(&self, name: &str) -> Option<&EntityDef> {
        self.entity_position(name)
            .map(|index| &self.entities[index])
    }

    /// The type declaration for `name`, if the schema declares one.
    #[must_use]
    pub fn type_def(&self, name: &str) -> Option<&TypeDef> {
        with_folded(name, |key| self.type_index.get(key).copied()).map(|index| &self.types[index])
    }

    /// Every entity declaration, in source order.
    pub fn entities(&self) -> impl Iterator<Item = &EntityDef> {
        self.entities.iter()
    }

    /// Every type declaration, in source order.
    pub fn types(&self) -> impl Iterator<Item = &TypeDef> {
        self.types.iter()
    }

    /// Every entity name the schema declares.
    ///
    /// Source order today; callers needing a particular order must sort.
    pub fn entity_names(&self) -> impl Iterator<Item = &str> {
        self.entities.iter().map(|entity| entity.name.as_str())
    }

    /// Whether a candidate entity or defined type satisfies a declared type.
    ///
    /// This walks entity inheritance, defined-type aliases, and nested SELECTs.
    /// Unknown declarations and cyclic aliases fail closed.
    #[must_use]
    pub fn accepts_type(&self, declared: &str, candidate: &str) -> bool {
        // Two distinct entities: the walk below would record the pair as
        // seen and answer `is_a` at once, so answer it without the walk's
        // allocations. This is the common case for an entity-typed slot.
        if !declared.eq_ignore_ascii_case(candidate)
            && self.entity_position(declared).is_some()
            && self.entity_position(candidate).is_some()
        {
            return self.is_a(candidate, declared);
        }
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
    ///
    /// Reflexive for declared entities. An entity the schema never declares
    /// is not a subtype of anything, including itself -- otherwise a typo
    /// would silently satisfy every check made against it.
    #[must_use]
    pub fn is_a(&self, name: &str, ancestor: &str) -> bool {
        let Some(lineage) = self.lineage_of(name) else {
            return false;
        };
        name.eq_ignore_ascii_case(ancestor)
            || lineage.supertypes.iter().any(|super_name| {
                self.ancestor_name(super_name)
                    .eq_ignore_ascii_case(ancestor)
            })
    }

    /// The supertype chain above `name`, nearest parent first.
    ///
    /// Depth-first in `SUBTYPE OF` order, each ancestor once, excluding
    /// `name`. A supertype the schema names but never declares is reported
    /// but cannot be walked past. Terminates on a malformed cyclic schema.
    #[must_use]
    pub fn supertypes(&self, name: &str) -> Vec<&str> {
        self.lineage_of(name)
            .map(|lineage| {
                lineage
                    .supertypes
                    .iter()
                    .map(|ancestor| self.ancestor_name(ancestor))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn collect_supertypes(
        &self,
        name: &str,
        depth: usize,
        seen: &mut HashSet<String>,
        out: &mut Vec<Ancestor>,
    ) {
        if depth >= MAX_CHAIN_DEPTH {
            return;
        }
        let Some(def) = self.entity(name) else {
            return;
        };
        for supertype in &def.supertypes {
            if !seen.insert(supertype.to_ascii_uppercase()) {
                continue;
            }
            match self.entity_position(supertype) {
                Some(parent) => {
                    out.push(Ancestor::Declared(parent));
                    self.collect_supertypes(&self.entities[parent].name, depth + 1, seen, out);
                }
                None => out.push(Ancestor::Undeclared(supertype.as_str().into())),
            }
        }
    }

    /// Entities declaring `SUBTYPE OF (name)` directly, sorted by name.
    #[must_use]
    pub fn direct_subtypes(&self, name: &str) -> Vec<&str> {
        with_folded(name, |key| self.children.get(key))
            .map(|names| names.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// Every entity inheriting from `name` at any depth, excluding `name`.
    ///
    /// Sorted depth-first pre-order. The inverse of [`Self::is_a`].
    #[must_use]
    pub fn subtypes(&self, name: &str) -> Vec<&str> {
        let mut seen = HashSet::new();
        seen.insert(name.to_ascii_uppercase());
        let mut out = Vec::new();
        // Children are pushed in reverse so the sorted first sibling pops
        // first, which is what makes this a pre-order and not a post-order.
        let mut stack: Vec<&str> = self.direct_subtypes(name).into_iter().rev().collect();
        while let Some(current) = stack.pop() {
            if !seen.insert(current.to_ascii_uppercase()) {
                continue;
            }
            out.push(current);
            stack.extend(self.direct_subtypes(current).into_iter().rev());
        }
        out
    }

    /// Every attribute slot in Part 21 positional order, inherited first.
    ///
    /// ISO 10303-21:2016 §12.2.5.2: each supertype is laid out in
    /// `SUBTYPE OF` order, its own supertypes first, and a supertype reached
    /// a second time contributes nothing more. Derived redeclarations are
    /// included (they keep their slot and are written `*`); use
    /// [`EntityDef::is_derived`] on the owning entity to tell them apart.
    #[must_use]
    pub fn attributes(&self, name: &str) -> Vec<&Attribute> {
        self.lineage_of(name)
            .map(|lineage| lineage.slots.iter().map(|&slot| self.slot(slot)).collect())
            .unwrap_or_default()
    }

    /// How many positional slots [`Self::attributes`] lists for `name`;
    /// `0` for an entity the schema does not declare.
    ///
    /// Read from the precomputed layout, without allocating.
    #[must_use]
    pub fn attribute_count(&self, name: &str) -> usize {
        self.lineage_of(name)
            .map_or(0, |lineage| lineage.slots.len())
    }

    /// The attribute at positional `slot` of `name`, as
    /// `attributes(name).get(slot)` answers, without allocating.
    #[must_use]
    pub fn attribute_at(&self, name: &str, slot: usize) -> Option<&Attribute> {
        let lineage = self.lineage_of(name)?;
        lineage.slots.get(slot).map(|&at| self.slot(at))
    }

    fn collect_attributes(
        &self,
        name: &str,
        depth: usize,
        seen: &mut HashSet<String>,
        out: &mut Vec<(u32, u32)>,
    ) {
        if depth > MAX_CHAIN_DEPTH {
            return;
        }
        let Some(index) = self.entity_position(name) else {
            return;
        };
        let def = &self.entities[index];
        if !seen.insert(def.name.to_ascii_uppercase()) {
            return;
        }
        for supertype in &def.supertypes {
            self.collect_attributes(supertype, depth + 1, seen, out);
        }
        let owner = u32::try_from(index).expect("fewer than 2^32 entity declarations");
        out.extend((0..def.attributes.len()).map(|attribute| {
            (
                owner,
                u32::try_from(attribute).expect("fewer than 2^32 attributes"),
            )
        }));
    }

    /// Attribute names in positional order.
    #[must_use]
    pub fn attribute_names(&self, name: &str) -> Vec<&str> {
        self.attributes(name)
            .into_iter()
            .map(|attribute| attribute.name.as_str())
            .collect()
    }

    /// Resolves a defined type to the base it ultimately aliases.
    ///
    /// `IfcPositiveLengthMeasure` -> `IfcLengthMeasure` -> `REAL`. Returns
    /// the final right-hand side verbatim (including any aggregate syntax);
    /// for a declaration that is not an alias, the type's own name. Bounded
    /// so a cyclic alias cannot hang.
    #[must_use]
    pub fn resolve_defined(&self, name: &str) -> String {
        let mut current = name.to_string();
        for _ in 0..MAX_CHAIN_DEPTH {
            let Some(def) = self.type_def(&current) else {
                return current;
            };
            let TypeKind::Defined(target) = &def.kind else {
                return current;
            };
            let next = target.trim().to_string();
            if next.eq_ignore_ascii_case(&current) {
                return current;
            }
            current = next;
        }
        current
    }
}

/// Longest name folded on the stack; a longer one is folded on the heap.
const FOLD_BUFFER: usize = 128;

/// Calls `f` with `name` upper-cased (ASCII), allocating only for a
/// mixed-case name longer than [`FOLD_BUFFER`] bytes.
///
/// Upper-casing ASCII bytes leaves every other byte alone, so the folded
/// buffer is valid UTF-8 whenever `name` is.
fn with_folded<R>(name: &str, f: impl FnOnce(&str) -> R) -> R {
    if !name.bytes().any(|byte| byte.is_ascii_lowercase()) {
        return f(name);
    }
    if name.len() <= FOLD_BUFFER {
        let mut buffer = [0u8; FOLD_BUFFER];
        let folded = &mut buffer[..name.len()];
        folded.copy_from_slice(name.as_bytes());
        folded.make_ascii_uppercase();
        let folded = std::str::from_utf8(folded).expect("ASCII folding keeps UTF-8 valid");
        return f(folded);
    }
    f(&name.to_ascii_uppercase())
}

/// Deduplicates declarations by case-folded name (first position, last
/// declaration wins) and builds the lookup index.
fn index_by_name<T>(
    declarations: Vec<T>,
    name: impl Fn(&T) -> &String,
) -> (Vec<T>, HashMap<String, usize>) {
    let mut kept: Vec<T> = Vec::with_capacity(declarations.len());
    let mut index = HashMap::with_capacity(declarations.len());
    for declaration in declarations {
        let key = name(&declaration).to_ascii_uppercase();
        match index.get(&key) {
            Some(&position) => kept[position] = declaration,
            None => {
                index.insert(key, kept.len());
                kept.push(declaration);
            }
        }
    }
    (kept, index)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real three-level chain: two levels cannot catch ordering bugs.
    fn chain() -> Schema {
        Schema::new(
            "IFC4",
            vec![
                EntityDef::new("IfcRoot")
                    .abstract_entity()
                    .with_attribute(Attribute::new("GlobalId", "IfcGloballyUniqueId"))
                    .with_attribute(Attribute::new("OwnerHistory", "IfcOwnerHistory").optional())
                    .with_attribute(Attribute::new("Name", "IfcLabel").optional())
                    .with_attribute(Attribute::new("Description", "IfcText").optional()),
                EntityDef::new("IfcObjectDefinition")
                    .abstract_entity()
                    .with_supertype("IfcRoot"),
                EntityDef::new("IfcObject")
                    .with_supertype("IfcObjectDefinition")
                    .with_attribute(Attribute::new("ObjectType", "IfcLabel").optional()),
            ],
            vec![
                TypeDef::new("IfcLengthMeasure", TypeKind::Defined("REAL".into())),
                TypeDef::new(
                    "IfcPositiveLengthMeasure",
                    TypeKind::Defined("IfcLengthMeasure".into()),
                ),
            ],
        )
    }

    #[test]
    fn the_declared_schema_name_maps_onto_a_known_ifc_version() {
        let schema = chain();
        assert_eq!(schema.name(), "IFC4");
        assert_eq!(schema.version(), Some(SchemaVersion::Ifc4));
    }

    /// A schema this crate does not recognize still assembles and queries.
    #[test]
    fn an_unrecognized_schema_name_has_no_version_but_still_works() {
        let schema = Schema::new(
            "AP242",
            vec![EntityDef::new("Product").with_attribute(Attribute::new("Id", "Identifier"))],
            Vec::new(),
        );
        assert_eq!(schema.version(), None, "not an IFC schema");
        assert_eq!(schema.attribute_names("Product"), ["Id"]);
    }

    #[test]
    fn inherited_attributes_come_first_in_positional_order() {
        assert_eq!(
            chain().attribute_names("IFCOBJECT"),
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
        assert_eq!(chain().resolve_defined("IfcPositiveLengthMeasure"), "REAL");
    }

    #[test]
    fn subtype_tests_cross_intermediate_levels_and_are_reflexive_only_when_declared() {
        let schema = chain();
        assert!(schema.is_a("IFCOBJECT", "IfcRoot"));
        assert!(schema.is_a("IfcObject", "IfcObject"));
        assert!(!schema.is_a("IfcRoot", "IfcObject"));
        assert!(!schema.is_a("NotAThing", "NotAThing"));
        assert_eq!(
            schema.subtypes("ifcroot"),
            ["IfcObjectDefinition", "IfcObject"]
        );
        assert_eq!(
            schema.supertypes("IfcObject"),
            ["IfcObjectDefinition", "IfcRoot"]
        );
    }

    #[test]
    fn cyclic_chains_terminate() {
        let schema = Schema::new(
            "S",
            vec![
                EntityDef::new("A").with_supertype("B"),
                EntityDef::new("B").with_supertype("A"),
            ],
            vec![
                TypeDef::new("X", TypeKind::Defined("Y".into())),
                TypeDef::new("Y", TypeKind::Defined("X".into())),
            ],
        );
        assert_eq!(schema.supertypes("A"), ["B"]);
        assert_eq!(schema.subtypes("A"), ["B"]);
        let resolved = schema.resolve_defined("X");
        assert!(resolved == "X" || resolved == "Y");
    }

    #[test]
    fn an_undeclared_supertype_is_still_named_in_both_directions() {
        let schema = Schema::new(
            "S",
            vec![EntityDef::new("A").with_supertype("Missing")],
            Vec::new(),
        );
        assert_eq!(schema.supertypes("A"), ["Missing"]);
        assert!(schema.is_a("A", "Missing"));
        assert_eq!(schema.subtypes("Missing"), ["A"]);
    }

    #[test]
    fn a_repeated_name_keeps_its_first_position_and_last_declaration() {
        let schema = Schema::new(
            "S",
            vec![
                EntityDef::new("A"),
                EntityDef::new("B"),
                EntityDef::new("a").with_attribute(Attribute::new("X", "INTEGER")),
            ],
            Vec::new(),
        );
        assert_eq!(schema.entity_count(), 2);
        assert_eq!(schema.entity_names().collect::<Vec<_>>(), ["a", "B"]);
        assert_eq!(schema.attribute_names("A"), ["X"]);
    }
}

/// The owned walks must answer exactly as `openbim_step::SchemaGraph` did,
/// which this crate delegated to before owning its types.
#[cfg(all(test, feature = "express"))]
mod parity_with_step {
    use super::*;

    const TREE: &str = "\
SCHEMA TREE;
ENTITY Root; Id : INTEGER; END_ENTITY;
ENTITY Wall SUBTYPE OF (Root); Name : OPTIONAL STRING; END_ENTITY;
ENTITY Door SUBTYPE OF (Root); END_ENTITY;
ENTITY WallStandardCase SUBTYPE OF (Wall); Tag : LIST [1:?] OF STRING; END_ENTITY;
ENTITY WallElementedCase SUBTYPE OF (Wall); END_ENTITY;
ENTITY Orphan SUBTYPE OF (Missing); END_ENTITY;
ENTITY Unrelated; END_ENTITY;
TYPE Count = INTEGER; END_TYPE;
TYPE PositiveCount = Count; END_TYPE;
TYPE Colour = ENUMERATION OF (RED, GREEN); END_TYPE;
END_SCHEMA;";

    #[test]
    fn every_query_matches_the_step_schema_graph() {
        let owned = Schema::from_express(TREE);
        let graph = openbim_step::SchemaGraph::from_express(TREE);
        assert_eq!(owned.name(), graph.name());
        assert_eq!(owned.entity_count(), graph.entity_count());
        assert_eq!(owned.type_count(), graph.type_count());
        let mut names: Vec<&str> = graph.entity_names().collect();
        names.extend(["Missing", "NotAThing"]);
        for &name in &names {
            assert_eq!(owned.supertypes(name), graph.supertypes(name), "{name}");
            assert_eq!(owned.subtypes(name), graph.subtypes(name), "{name}");
            assert_eq!(
                owned.direct_subtypes(name),
                graph.direct_subtypes(name),
                "{name}"
            );
            assert_eq!(
                owned.attribute_names(name),
                graph.attribute_names(name),
                "{name}"
            );
            for &ancestor in &names {
                assert_eq!(
                    owned.is_a(name, ancestor),
                    graph.is_a(name, ancestor),
                    "{name} / {ancestor}"
                );
            }
        }
        for name in ["PositiveCount", "Colour", "Count", "Unknown"] {
            assert_eq!(owned.resolve_defined(name), graph.resolve_defined(name));
        }
    }
}
