//! [`ExpressRules`]: the rules of one release, read from its EXPRESS text.

use std::collections::BTreeSet;
use std::sync::Arc;

use ifc_schema::Schema;

use super::coverage::{self, Coverage};
use super::error::ExpressError;
use super::options::EvalLimits;
use super::resolve::{self, Globals};
use super::tables::Tables;

/// The WHERE rules, type rules, global rules and functions of one IFC
/// release, ready to evaluate.
///
/// # Where the rule text comes from
///
/// The IFC schemas are published by buildingSMART under CC BY-ND 4.0, and
/// this crate does not redistribute them or anything derived from their
/// rule text: the bundled tables carry rule labels only. The caller
/// supplies the release's `.exp` file -- `scripts/fetch-ifc-schemas.sh`
/// fetches and checksums the five releases -- and this type reads it at run
/// time.
///
/// # Pairing with the tables
///
/// The text must declare the same schema as the bundled tables it is
/// paired with, and agree with them on every entity's attribute layout:
/// [`ExpressRules::parse`] refuses `IFC4.exp` paired with IFC4X3 tables,
/// and a modified file whose slots would be read wrong.
///
/// Cheap to clone: the parsed schema is shared. `Send` and `Sync`, so one
/// parse serves every thread.
#[derive(Clone)]
pub struct ExpressRules {
    pub(crate) tables: Arc<Tables>,
    pub(crate) limits: EvalLimits,
    ids: Arc<BTreeSet<String>>,
}

impl std::fmt::Debug for ExpressRules {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExpressRules")
            .field("schema", &self.schema_name())
            .field("rules", &self.ids.len())
            .field("limits", &self.limits)
            .finish()
    }
}

impl ExpressRules {
    /// Reads `source`, the EXPRESS text of `schema`'s release.
    ///
    /// # Errors
    ///
    /// [`ExpressError`] when the text does not parse, names another schema,
    /// disagrees with the tables, or uses a name that binds to nothing.
    pub fn parse(source: &str, schema: &Schema) -> Result<Self, ExpressError> {
        let mut decl = super::parse::schema(source)?;
        if !decl.name.eq_ignore_ascii_case(schema.name()) {
            return Err(ExpressError::SchemaMismatch {
                source: decl.name.to_string(),
                tables: schema.name().to_owned(),
            });
        }
        let globals = Globals::new(&decl);
        resolve::schema(&mut decl, &globals)?;
        let tables = Tables::new(decl, globals, schema)?;
        let mut ids = BTreeSet::new();
        for entity in &tables.schema.entities {
            for rule in &entity.rules {
                ids.insert(format!("{}.{}", entity.spelling, rule.label));
            }
        }
        for decl in &tables.schema.types {
            for rule in &decl.rules {
                ids.insert(format!("{}.{}", decl.spelling, rule.label));
            }
        }
        for rule in &tables.schema.rules {
            ids.insert(format!("global.{}", rule.spelling));
        }
        Ok(Self {
            tables: Arc::new(tables),
            limits: EvalLimits::DEFAULT,
            ids: Arc::new(ids),
        })
    }

    /// Reads EXPRESS text that is not UTF-8.
    ///
    /// The normative IFC files are ASCII, but some copies are Latin-1
    /// (a `°` in a remark); each byte is read as one character, which is
    /// exact for everything outside remarks and string literals.
    ///
    /// # Errors
    ///
    /// As [`Self::parse`].
    pub fn parse_bytes(bytes: &[u8], schema: &Schema) -> Result<Self, ExpressError> {
        let text: String = bytes.iter().map(|&byte| char::from(byte)).collect();
        Self::parse(&text, schema)
    }

    /// The same rules evaluated under other limits.
    #[must_use]
    pub fn with_limits(mut self, limits: EvalLimits) -> Self {
        self.limits = limits;
        self
    }

    /// The limits evaluation runs under.
    #[must_use]
    pub fn limits(&self) -> EvalLimits {
        self.limits
    }

    /// The schema name the text declares (`IFC4X3_ADD2`).
    #[must_use]
    pub fn schema_name(&self) -> &str {
        &self.tables.schema.name
    }

    /// Every rule id these rules can report, sorted: `Entity.Label` for an
    /// entity WHERE rule, `Type.Label` for a type WHERE rule, and
    /// `global.Rule` for a global rule.
    pub fn rule_ids(&self) -> impl Iterator<Item = &str> {
        self.ids.iter().map(String::as_str)
    }

    /// Whether these rules report under `id`; a hand-written rule with the
    /// same id is superseded by them.
    #[must_use]
    pub fn covers(&self, id: &str) -> bool {
        self.ids.contains(id) || self.ids.contains(&format!("global.{id}"))
    }

    /// How many declarations this evaluator can evaluate, and which it
    /// cannot, by name.
    #[must_use]
    pub fn coverage(&self) -> Coverage {
        coverage::measure(&self.tables)
    }
}

#[cfg(test)]
mod tests {
    use super::ExpressRules;

    #[test]
    fn rules_can_be_shared_between_threads() {
        fn shared<T: Send + Sync>() {}
        shared::<ExpressRules>();
    }
}
