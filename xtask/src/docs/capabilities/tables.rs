//! The lowering tables of `ifc-geometry`, read as Rust rather than as text.
//!
//! `IMPLEMENTED`, `PLANNED` and `PARTIAL` in `lower/dispatch.rs`, and the two
//! profile tables in `lower/profile.rs`, are the capability claims. Parsing
//! them with `syn` reads exactly the values the compiler sees: string escapes
//! and line continuations are resolved, and a name in a comment is not a row.

use std::collections::BTreeMap;

use syn::visit::Visit;
use syn::{Expr, ExprLit, ItemConst, Lit};

use super::{ADMITTED, IMPLEMENTED, PARTIAL, PLANNED, REFUSED};
use crate::text::is_word;
use crate::workspace::Workspace;

const DISPATCH: &str = "ifc-geometry/src/lower/dispatch.rs";
const PROFILE: &str = "ifc-geometry/src/lower/profile.rs";
/// Correctly-cased entity names, already asserted against the schema.
/// Re-deriving casing from upper-case STEP names would need a word-splitting
/// heuristic that fails silently on the next entity.
const CASING: &str = "ifc-geometry/tests/schema_coverage.rs";

/// One row of the `PARTIAL` variant catalogue.
struct Variant {
    variant: String,
    admitted: bool,
    rationale: String,
}

/// Every lowering table the capability page publishes.
pub(super) struct Lowering {
    casing: BTreeMap<String, String>,
    implemented: Vec<String>,
    planned: Vec<(String, String)>,
    partial: BTreeMap<String, Vec<Variant>>,
    profiles: Vec<String>,
    planned_profiles: Vec<(String, String)>,
}

impl Lowering {
    pub(super) fn read(workspace: &Workspace) -> Result<Self, String> {
        let read = |rel: &str| {
            std::fs::read_to_string(workspace.root.join(rel))
                .map_err(|error| format!("cannot read {rel}: {error}"))
        };
        let dispatch = Consts::parse(DISPATCH, &read(DISPATCH)?)?;
        let profile = Consts::parse(PROFILE, &read(PROFILE)?)?;
        Ok(Self {
            casing: casing(&read(CASING)?),
            implemented: dispatch.strings("IMPLEMENTED")?,
            planned: dispatch.pairs("PLANNED")?,
            partial: dispatch.variants("PARTIAL")?,
            profiles: profile.strings("IMPLEMENTED_PROFILES")?,
            planned_profiles: profile.pairs("PLANNED_PROFILES")?,
        })
    }

    fn name<'a>(&'a self, entity: &'a str) -> &'a str {
        self.casing.get(entity).map_or(entity, String::as_str)
    }

    pub(super) fn geometry_table(&self) -> String {
        let mut rows = vec!["| Family | Status |".to_owned(), "| --- | --- |".to_owned()];
        for entity in &self.implemented {
            let name = self.name(entity);
            match self.partial.get(entity) {
                Some(variants) => {
                    let refused = variants.iter().filter(|v| !v.admitted).count();
                    rows.push(format!(
                        "| `{name}` | {PARTIAL} \u{2014} {refused} authored form(s) refused; \
                         see [variants](#partially-supported-variants) |"
                    ));
                }
                None => rows.push(format!("| `{name}` | {IMPLEMENTED} |")),
            }
        }
        for (entity, reason) in &self.planned {
            rows.push(format!(
                "| `{}` | {PLANNED} \u{2014} {reason} |",
                self.name(entity)
            ));
        }
        rows.join("\n")
    }

    pub(super) fn variant_table(&self) -> String {
        let mut rows = vec![
            "| Family | Variant | Status | Rationale |".to_owned(),
            "| --- | --- | --- | --- |".to_owned(),
        ];
        for (entity, variants) in &self.partial {
            for variant in variants {
                let badge = if variant.admitted { ADMITTED } else { REFUSED };
                rows.push(format!(
                    "| `{}` | {} | {badge} | {} |",
                    self.name(entity),
                    variant.variant,
                    variant.rationale
                ));
            }
        }
        rows.join("\n")
    }

    pub(super) fn profile_table(&self) -> String {
        let mut rows = vec![
            "| Profile family | Status |".to_owned(),
            "| --- | --- |".to_owned(),
        ];
        for entity in &self.profiles {
            rows.push(format!("| `{}` | {IMPLEMENTED} |", self.name(entity)));
        }
        for (entity, reason) in &self.planned_profiles {
            rows.push(format!(
                "| `{}` | {REFUSED} — {reason} |",
                self.name(entity)
            ));
        }
        rows.join("\n")
    }
}

/// Every `"Ifc…"` string literal in the casing source, keyed upper-case.
fn casing(source: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut rest = source;
    while let Some(start) = rest.find("\"Ifc") {
        let after = &rest[start + 1..];
        let length = after.find(|c: char| !is_word(c)).unwrap_or(after.len());
        let name = &after[..length];
        if after[length..].starts_with('"') {
            out.insert(name.to_uppercase(), name.to_owned());
        }
        rest = &after[length..];
    }
    out
}

/// The `const` items of one source file, by name.
struct Consts {
    file: &'static str,
    items: BTreeMap<String, Expr>,
}

impl Consts {
    fn parse(file: &'static str, source: &str) -> Result<Self, String> {
        let syntax = syn::parse_file(source).map_err(|error| format!("{file}: {error}"))?;
        let mut collector = Collector::default();
        collector.visit_file(&syntax);
        Ok(Self {
            file,
            items: collector.items,
        })
    }

    /// The array elements of `const NAME: &[…] = &[…];`.
    fn elements(&self, name: &str) -> Result<Vec<&Expr>, String> {
        let expr = self
            .items
            .get(name)
            .ok_or_else(|| format!("{}: {name} not found", self.file))?;
        let array = match expr {
            Expr::Reference(reference) => &*reference.expr,
            other => other,
        };
        match array {
            Expr::Array(array) => Ok(array.elems.iter().collect()),
            _ => Err(format!("{}: {name} is not an array literal", self.file)),
        }
    }

    fn strings(&self, name: &str) -> Result<Vec<String>, String> {
        self.elements(name)?
            .into_iter()
            .map(|element| string(element).ok_or_else(|| self.shape(name, "string literals")))
            .collect()
    }

    fn pairs(&self, name: &str) -> Result<Vec<(String, String)>, String> {
        self.elements(name)?
            .into_iter()
            .map(|element| match element {
                Expr::Tuple(tuple) if tuple.elems.len() == 2 => {
                    match (string(&tuple.elems[0]), string(&tuple.elems[1])) {
                        (Some(entity), Some(reason)) => Ok((entity, reason)),
                        _ => Err(self.shape(name, "(entity, reason) string pairs")),
                    }
                }
                _ => Err(self.shape(name, "(entity, reason) string pairs")),
            })
            .collect()
    }

    fn variants(&self, name: &str) -> Result<BTreeMap<String, Vec<Variant>>, String> {
        let mut out: BTreeMap<String, Vec<Variant>> = BTreeMap::new();
        let Ok(elements) = self.elements(name) else {
            return Ok(out);
        };
        for element in elements {
            let Expr::Struct(record) = element else {
                return Err(self.shape(name, "Variant { .. } records"));
            };
            let field = |wanted: &str| {
                record.fields.iter().find_map(|field| match &field.member {
                    syn::Member::Named(ident) if ident == wanted => Some(&field.expr),
                    _ => None,
                })
            };
            let text = |wanted: &str| field(wanted).and_then(string);
            let (Some(family), Some(variant), Some(rationale), Some(support)) = (
                text("family"),
                text("variant"),
                text("rationale"),
                field("support"),
            ) else {
                return Err(self.shape(name, "records with family/variant/support/rationale"));
            };
            // A doubled continuation escapes the backslash instead of
            // continuing the string, leaving a literal one in the prose.
            for (label, value) in [("variant", &variant), ("rationale", &rationale)] {
                if value.contains('\\') {
                    return Err(format!(
                        "{family}: {label} carries a literal backslash; a wrapped string \
                         line must end with a single backslash"
                    ));
                }
            }
            let admitted = match support {
                Expr::Path(path) => path
                    .path
                    .segments
                    .last()
                    .is_some_and(|s| s.ident == "Admitted"),
                _ => false,
            };
            out.entry(family).or_default().push(Variant {
                variant,
                admitted,
                rationale,
            });
        }
        Ok(out)
    }

    fn shape(&self, name: &str, expected: &str) -> String {
        format!("{}: {name} must hold {expected}", self.file)
    }
}

fn string(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(literal),
            ..
        }) => Some(literal.value()),
        _ => None,
    }
}

#[derive(Default)]
struct Collector {
    items: BTreeMap<String, Expr>,
}

impl<'ast> Visit<'ast> for Collector {
    fn visit_item_const(&mut self, item: &'ast ItemConst) {
        self.items
            .insert(item.ident.to_string(), (*item.expr).clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_are_read_as_values_not_text() {
        let source = r#"
            pub const IMPLEMENTED: &[&str] = &[
                "IFCA", // "IFCNOTAROW"
                "IFCB",
            ];
            pub const PLANNED: &[(&str, &str)] = &[("IFCC", "wrapped \
                reason with \"quotes\"")];
        "#;
        let consts = Consts::parse("test.rs", source).unwrap();
        assert_eq!(consts.strings("IMPLEMENTED").unwrap(), ["IFCA", "IFCB"]);
        assert_eq!(
            consts.pairs("PLANNED").unwrap(),
            [(
                "IFCC".to_owned(),
                "wrapped reason with \"quotes\"".to_owned()
            )]
        );
        assert!(consts.variants("PARTIAL").unwrap().is_empty());
    }

    #[test]
    fn casing_reads_quoted_names_only() {
        let map = casing(r#"let a = ["IfcWall", "IfcSlab"]; // IfcNot"#);
        assert_eq!(map.get("IFCWALL").map(String::as_str), Some("IfcWall"));
        assert!(!map.contains_key("IFCNOT"));
    }
}
