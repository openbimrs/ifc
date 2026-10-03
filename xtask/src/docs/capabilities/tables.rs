//! The lowering tables of `ifc-geometry`, read as Rust rather than as text.
//!
//! `IMPLEMENTED`, `PLANNED` and `PARTIAL` in `lower/dispatch.rs`, and the two
//! profile tables in `lower/profile.rs`, are the capability claims. Parsing
//! them with `syn` reads exactly the values the compiler sees: string escapes
//! and line continuations are resolved, and a name in a comment is not a row.

use std::collections::BTreeMap;

use super::{ADMITTED, IMPLEMENTED, PARTIAL, PLANNED, REFUSED};
use crate::rust_source::{self, Consts};
use crate::text::is_word;
use crate::workspace::Workspace;

const DISPATCH: &str = "crates/ifc-geometry/src/lower/dispatch.rs";
const PROFILE: &str = "crates/ifc-geometry/src/lower/profile.rs";
/// Correctly-cased entity names, already asserted against the schema.
/// Re-deriving casing from upper-case STEP names would need a word-splitting
/// heuristic that fails silently on the next entity.
const CASING: &str = "crates/ifc-geometry/tests/schema_coverage.rs";

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
    unlowered_profiles: Vec<(String, String)>,
}

impl Lowering {
    pub(super) fn read(workspace: &Workspace) -> Result<Self, String> {
        let read = |rel: &str| {
            std::fs::read_to_string(workspace.root.join(rel))
                .map_err(|error| format!("cannot read {rel}: {error}"))
        };
        let dispatch = Consts::read(workspace, DISPATCH)?;
        let profile = Consts::read(workspace, PROFILE)?;
        Ok(Self {
            casing: casing(&read(CASING)?),
            implemented: dispatch.strings("IMPLEMENTED")?,
            planned: dispatch.pairs("PLANNED")?,
            partial: Self::variants(&dispatch, "PARTIAL")?,
            profiles: profile.strings("IMPLEMENTED_PROFILES")?,
            planned_profiles: profile.pairs("PLANNED_PROFILES")?,
            unlowered_profiles: profile.pairs("UNLOWERED")?,
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
        for (entity, reason) in &self.unlowered_profiles {
            rows.push(format!(
                "| `{}` | {PLANNED} \u{2014} {reason} |",
                self.name(entity)
            ));
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

impl Lowering {
    fn variants(dispatch: &Consts, name: &str) -> Result<BTreeMap<String, Vec<Variant>>, String> {
        let mut out: BTreeMap<String, Vec<Variant>> = BTreeMap::new();
        let Ok(records) = dispatch.records(name) else {
            return Ok(out);
        };
        for record in records {
            let text = |wanted: &str| record.field(wanted).and_then(|e| dispatch.text(e));
            let (Some(family), Some(variant), Some(rationale), Some(support)) = (
                text("family"),
                text("variant"),
                text("rationale"),
                record.field("support").and_then(rust_source::variant),
            ) else {
                return Err(dispatch.shape(name, "records with family/variant/support/rationale"));
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
            out.entry(family).or_default().push(Variant {
                variant,
                admitted: support.0 == "Admitted",
                rationale,
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn casing_reads_quoted_names_only() {
        let map = casing(r#"let a = ["IfcWall", "IfcSlab"]; // IfcNot"#);
        assert_eq!(map.get("IFCWALL").map(String::as_str), Some("IfcWall"));
        assert!(!map.contains_key("IFCNOT"));
    }
}
