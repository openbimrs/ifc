//! Reading `const` tables out of Rust source with `syn`.
//!
//! Several capability claims are data tables in the crates themselves
//! (lowering families, appearance declarations, validation rules). Parsing
//! them as Rust reads exactly the values the compiler sees: escapes and line
//! continuations resolve, a name in a comment is not a row, and a string held
//! in a named constant is followed to its value.

use std::collections::BTreeMap;

use syn::visit::Visit;
use syn::{Expr, ExprLit, ExprStruct, ItemConst, Lit, Member};

use crate::workspace::Workspace;

/// The `const` items of one source file, by name.
pub(crate) struct Consts {
    file: String,
    items: BTreeMap<String, Expr>,
}

impl Consts {
    pub(crate) fn read(workspace: &Workspace, rel: &str) -> Result<Self, String> {
        let source = std::fs::read_to_string(workspace.root.join(rel))
            .map_err(|error| format!("cannot read {rel}: {error}"))?;
        Self::parse(rel, &source)
    }

    pub(crate) fn parse(file: &str, source: &str) -> Result<Self, String> {
        let syntax = syn::parse_file(source).map_err(|error| format!("{file}: {error}"))?;
        let mut collector = Collector::default();
        collector.visit_file(&syntax);
        Ok(Self {
            file: file.to_owned(),
            items: collector.items,
        })
    }

    /// The array elements of `const NAME: &[…] = &[…];`.
    pub(crate) fn elements(&self, name: &str) -> Result<Vec<&Expr>, String> {
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

    pub(crate) fn strings(&self, name: &str) -> Result<Vec<String>, String> {
        self.elements(name)?
            .into_iter()
            .map(|element| {
                self.text(element)
                    .ok_or_else(|| self.shape(name, "string literals"))
            })
            .collect()
    }

    pub(crate) fn pairs(&self, name: &str) -> Result<Vec<(String, String)>, String> {
        self.elements(name)?
            .into_iter()
            .map(|element| match element {
                Expr::Tuple(tuple) if tuple.elems.len() == 2 => {
                    match (self.text(&tuple.elems[0]), self.text(&tuple.elems[1])) {
                        (Some(a), Some(b)) => Ok((a, b)),
                        _ => Err(self.shape(name, "pairs of strings")),
                    }
                }
                _ => Err(self.shape(name, "pairs of strings")),
            })
            .collect()
    }

    /// The struct-literal elements of an array table.
    pub(crate) fn records(&self, name: &str) -> Result<Vec<Record<'_>>, String> {
        self.elements(name)?
            .into_iter()
            .map(|element| match element {
                Expr::Struct(record) => Ok(Record { expr: record }),
                _ => Err(self.shape(name, "struct literals")),
            })
            .collect()
    }

    /// A string literal, or the value of a named string constant.
    pub(crate) fn text(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Lit(ExprLit {
                lit: Lit::Str(literal),
                ..
            }) => Some(literal.value()),
            Expr::Path(path) => {
                let name = path.path.get_ident()?.to_string();
                self.text(self.items.get(&name)?)
            }
            _ => None,
        }
    }

    pub(crate) fn shape(&self, name: &str, expected: &str) -> String {
        format!("{}: {name} must hold {expected}", self.file)
    }
}

/// One struct literal in a table.
pub(crate) struct Record<'a> {
    expr: &'a ExprStruct,
}

impl<'a> Record<'a> {
    pub(crate) fn field(&self, name: &str) -> Option<&'a Expr> {
        self.expr
            .fields
            .iter()
            .find_map(|field| match &field.member {
                Member::Named(ident) if ident == name => Some(&field.expr),
                _ => None,
            })
    }
}

/// The last path segment of an enum value: `Support::Admitted` → `Admitted`,
/// `Support::Unsupported(REASON)` → `Unsupported`, with its first argument.
pub(crate) fn variant(expr: &Expr) -> Option<(String, Option<&Expr>)> {
    match expr {
        Expr::Path(path) => Some((path.path.segments.last()?.ident.to_string(), None)),
        Expr::Call(call) => {
            let Expr::Path(path) = &*call.func else {
                return None;
            };
            Some((
                path.path.segments.last()?.ident.to_string(),
                call.args.first(),
            ))
        }
        _ => None,
    }
}

/// `Some(x)` → `x`, `None` → `None`.
pub(crate) fn option(expr: &Expr) -> Option<&Expr> {
    match expr {
        Expr::Call(call) => call.args.first(),
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
            const WHY: &str = "because";
            pub const IMPLEMENTED: &[&str] = &[
                "IFCA", // "IFCNOTAROW"
                "IFCB",
            ];
            pub const PLANNED: &[(&str, &str)] = &[("IFCC", "wrapped \
                reason with \"quotes\""), ("IFCD", WHY)];
            pub const RULES: &[Rule] = &[Rule { id: "r", entity: Some("IfcX"), support: Support::Unsupported(WHY) }];
        "#;
        let consts = Consts::parse("test.rs", source).unwrap();
        assert_eq!(consts.strings("IMPLEMENTED").unwrap(), ["IFCA", "IFCB"]);
        assert_eq!(
            consts.pairs("PLANNED").unwrap(),
            [
                (
                    "IFCC".to_owned(),
                    "wrapped reason with \"quotes\"".to_owned()
                ),
                ("IFCD".to_owned(), "because".to_owned())
            ]
        );
        let rules = consts.records("RULES").unwrap();
        let entity = rules[0]
            .field("entity")
            .and_then(option)
            .and_then(|e| consts.text(e));
        assert_eq!(entity.as_deref(), Some("IfcX"));
        let (name, argument) = variant(rules[0].field("support").unwrap()).unwrap();
        assert_eq!(name, "Unsupported");
        assert_eq!(
            argument.and_then(|a| consts.text(a)).as_deref(),
            Some("because")
        );
    }
}
