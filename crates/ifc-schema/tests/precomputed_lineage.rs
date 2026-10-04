//! The precomputed supertype chains and attribute layouts (#352) answer
//! exactly what the per-call walks they replaced answered.
//!
//! `reference` below is a frozen copy of `Schema::supertypes`,
//! `Schema::attributes` and `Schema::is_a` as they were before #352,
//! written against the public declarations only (`Schema::entity` and
//! `EntityDef`), so it shares no code with the tables it checks. Every
//! entity of every bundled release is compared, under three spellings of
//! its name, and `is_a` for every ordered pair of entities.

use std::collections::HashSet;

use ifc_schema::{ifc2x3, ifc4, ifc4x1, ifc4x2, ifc4x3, Attribute, EntityDef, Schema};

fn bundled() -> [(&'static str, &'static Schema); 5] {
    [
        ("IFC2X3", ifc2x3()),
        ("IFC4", ifc4()),
        ("IFC4X1", ifc4x1()),
        ("IFC4X2", ifc4x2()),
        ("IFC4X3", ifc4x3()),
    ]
}

/// The walks as `ifc-schema` 0.3.1 ran them on every call.
mod reference {
    use super::{Attribute, EntityDef, HashSet, Schema};

    const MAX_CHAIN_DEPTH: usize = 64;

    pub fn supertypes<'s>(schema: &'s Schema, name: &str) -> Vec<&'s str> {
        let mut seen = HashSet::new();
        seen.insert(name.to_ascii_uppercase());
        let mut out = Vec::new();
        collect_supertypes(schema, name, 0, &mut seen, &mut out);
        out
    }

    fn collect_supertypes<'s>(
        schema: &'s Schema,
        name: &str,
        depth: usize,
        seen: &mut HashSet<String>,
        out: &mut Vec<&'s str>,
    ) {
        if depth >= MAX_CHAIN_DEPTH {
            return;
        }
        let Some(def) = schema.entity(name) else {
            return;
        };
        for supertype in &def.supertypes {
            if !seen.insert(supertype.to_ascii_uppercase()) {
                continue;
            }
            match schema.entity(supertype) {
                Some(parent) => {
                    out.push(parent.name.as_str());
                    collect_supertypes(schema, &parent.name, depth + 1, seen, out);
                }
                None => out.push(supertype.as_str()),
            }
        }
    }

    pub fn attributes<'s>(schema: &'s Schema, name: &str) -> Vec<&'s Attribute> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        collect_attributes(schema, name, 0, &mut seen, &mut out);
        out
    }

    fn collect_attributes<'s>(
        schema: &'s Schema,
        name: &str,
        depth: usize,
        seen: &mut HashSet<String>,
        out: &mut Vec<&'s Attribute>,
    ) {
        if depth > MAX_CHAIN_DEPTH {
            return;
        }
        let Some(def): Option<&EntityDef> = schema.entity(name) else {
            return;
        };
        if !seen.insert(def.name.to_ascii_uppercase()) {
            return;
        }
        for supertype in &def.supertypes {
            collect_attributes(schema, supertype, depth + 1, seen, out);
        }
        out.extend(def.attributes.iter());
    }

    /// `is_a` given the reference chain of `name`.
    pub fn is_a(schema: &Schema, name: &str, chain: &[&str], ancestor: &str) -> bool {
        if name.eq_ignore_ascii_case(ancestor) {
            return schema.entity(name).is_some();
        }
        chain
            .iter()
            .any(|super_name| super_name.eq_ignore_ascii_case(ancestor))
    }
}

/// The same entity spelled as declared, as Part 21 writes it, and folded
/// down: every lookup folds case, so all three must answer alike.
fn spellings(name: &str) -> [String; 3] {
    [
        name.to_owned(),
        name.to_ascii_uppercase(),
        name.to_ascii_lowercase(),
    ]
}

#[test]
fn supertype_chains_and_layouts_equal_the_per_call_walk_on_every_bundled_release() {
    for (label, schema) in bundled() {
        let names: Vec<&str> = schema.entity_names().collect();
        assert!(names.len() > 600, "{label}: a real table");
        for &name in &names {
            let chain = reference::supertypes(schema, name);
            let layout = reference::attributes(schema, name);
            for spelled in spellings(name) {
                assert_eq!(schema.supertypes(&spelled), chain, "{label} {spelled}");
                let attributes = schema.attributes(&spelled);
                assert_eq!(attributes.len(), layout.len(), "{label} {spelled}");
                for (slot, (got, want)) in attributes.iter().zip(&layout).enumerate() {
                    // The same declaration, not merely an equal one.
                    assert!(std::ptr::eq(*got, *want), "{label} {spelled} slot {slot}");
                    assert!(
                        schema
                            .attribute_at(&spelled, slot)
                            .is_some_and(|at| std::ptr::eq(at, *want)),
                        "{label} {spelled} attribute_at {slot}"
                    );
                }
                assert_eq!(schema.attribute_count(&spelled), layout.len());
                assert!(schema.attribute_at(&spelled, layout.len()).is_none());
            }
        }
        for unknown in ["", "IfcNotAnEntity", "ifcnotanentity", &"X".repeat(300)] {
            assert!(schema.supertypes(unknown).is_empty(), "{label} {unknown}");
            assert!(schema.attributes(unknown).is_empty(), "{label} {unknown}");
            assert_eq!(schema.attribute_count(unknown), 0);
            assert!(schema.attribute_at(unknown, 0).is_none());
        }
    }
}

#[test]
fn is_a_equals_the_per_call_walk_for_every_ordered_pair_on_every_bundled_release() {
    for (label, schema) in bundled() {
        let names: Vec<&str> = schema.entity_names().collect();
        let mut ancestors: Vec<String> = names.iter().map(|name| (*name).to_owned()).collect();
        ancestors.extend(["IFCROOT".into(), "ifcroot".into(), "IfcNotAnEntity".into()]);
        for &name in &names {
            let chain = reference::supertypes(schema, name);
            let upper = name.to_ascii_uppercase();
            for ancestor in &ancestors {
                let want = reference::is_a(schema, name, &chain, ancestor);
                assert_eq!(
                    schema.is_a(name, ancestor),
                    want,
                    "{label} {name} {ancestor}"
                );
                assert_eq!(
                    schema.is_a(&upper, ancestor),
                    want,
                    "{label} {upper} {ancestor}"
                );
            }
        }
        assert!(!schema.is_a("IfcNotAnEntity", "IfcNotAnEntity"), "{label}");
    }
}

/// A mixed-case name longer than the stack fold buffer still resolves.
#[test]
fn a_long_mixed_case_name_is_folded_on_the_heap_and_still_found() {
    let long = format!("Ifc{}", "Long".repeat(40));
    let schema = Schema::new(
        "S",
        vec![
            EntityDef::new("IfcRoot").with_attribute(Attribute::new("GlobalId", "STRING")),
            EntityDef::new(long.clone()).with_supertype("IfcRoot"),
        ],
        Vec::new(),
    );
    assert!(long.len() > 128);
    assert!(schema.is_a(&long, "IFCROOT"));
    assert!(schema.is_a(&long.to_ascii_lowercase(), "IfcRoot"));
    assert_eq!(schema.supertypes(&long.to_ascii_uppercase()), ["IfcRoot"]);
    assert_eq!(schema.attribute_names(&long), ["GlobalId"]);
}
