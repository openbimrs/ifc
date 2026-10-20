//! The report of `edit` and `import-table`: the planned changes, every
//! refusal with its kind and location, and what was written.
//!
//! Human output lists one change per line on standard output and one
//! refusal per line on standard error, then a summary; `--format json`
//! writes one document on standard output either way. A run with any
//! refusal writes nothing and exits 3.

use std::io::Write;
use std::ops::Range;

use ifc::selector::{SelectorError, SetTarget};
use ifc::sheet::{column_name, SheetRefusalReason};
use ifc::EntityId;

use crate::cli::TextFormat;
use crate::error::{CliError, CliResult, Outcome};
use crate::json::Json;

/// One value that changes.
pub(crate) struct Change {
    /// The table row and column, for `import-table`.
    pub(crate) cell: Option<(usize, usize)>,
    /// The entity the path starts from.
    pub(crate) entity: EntityId,
    /// The path, canonical.
    pub(crate) path: String,
    /// What is written.
    pub(crate) target: SetTarget,
    /// The value now, as cell text.
    pub(crate) before: String,
    /// The value then, as cell text.
    pub(crate) after: String,
}

/// One refused write, row, header or batch.
pub(crate) struct Refusal {
    /// The table row and column, either optional, for `import-table`.
    pub(crate) row: Option<usize>,
    pub(crate) column: Option<usize>,
    /// The entity and path of a refused `--set`, for `edit`.
    pub(crate) entity: Option<EntityId>,
    pub(crate) path: Option<String>,
    /// A short, stable name for the kind.
    pub(crate) kind: &'static str,
    /// Why.
    pub(crate) detail: String,
    /// The byte range of a header or path that does not parse.
    pub(crate) span: Option<Range<usize>>,
}

impl Refusal {
    /// A refused write of a selector error.
    pub(crate) fn of(error: &SelectorError) -> Self {
        let span = match error {
            SelectorError::UnknownClass { span, .. }
            | SelectorError::UnknownAttribute { span, .. } => Some(span.clone()),
            _ => None,
        };
        Self {
            row: None,
            column: None,
            entity: None,
            path: None,
            kind: selector_kind(error),
            detail: error.to_string(),
            span,
        }
    }

    /// A refused sheet, row or cell.
    pub(crate) fn of_sheet(
        row: Option<usize>,
        column: Option<usize>,
        reason: &SheetRefusalReason,
    ) -> Self {
        let (kind, span) = match reason {
            SheetRefusalReason::Empty => ("empty-table", None),
            SheetRefusalReason::Header(error) => ("invalid-header", Some(error.span.clone())),
            SheetRefusalReason::DuplicateColumn { .. } => ("duplicate-column", None),
            SheetRefusalReason::NoKeyColumn(_) => ("no-key-column", None),
            SheetRefusalReason::RaggedRow { .. } => ("ragged-row", None),
            SheetRefusalReason::EmptyKey => ("empty-key", None),
            SheetRefusalReason::UnknownKey(_) => ("unknown-key", None),
            SheetRefusalReason::DuplicateKey { .. } => ("duplicate-key", None),
            SheetRefusalReason::AmbiguousKey(_) => ("ambiguous-key", None),
            SheetRefusalReason::Value(error) => {
                let mut refusal = Self::of(error);
                refusal.row = row;
                refusal.column = column;
                return refusal;
            }
            _ => ("refused", None),
        };
        let detail = reason.to_string();
        Self {
            row,
            column,
            entity: None,
            path: None,
            kind,
            detail,
            span,
        }
    }

    fn location(&self) -> String {
        let mut parts = Vec::new();
        if let Some(row) = self.row {
            parts.push(format!("row {row}"));
        }
        if let Some(column) = self.column {
            parts.push(format!("column {}", column_name(column)));
        }
        if let Some(entity) = self.entity {
            parts.push(format!("#{}", entity.0));
        }
        if let Some(path) = &self.path {
            parts.push(format!("--set {path}"));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("{}: ", parts.join(", "))
        }
    }
}

/// A stable kind name for a selector refusal.
pub(crate) fn selector_kind(error: &SelectorError) -> &'static str {
    match error {
        SelectorError::UnknownClass { .. } | SelectorError::UnknownAttribute { .. } => {
            "invalid-selector"
        }
        SelectorError::MissingEntity(_) => "missing-entity",
        SelectorError::MissingReference { .. } => "missing-reference",
        SelectorError::UnsupportedValue { .. } | SelectorError::FeatureDisabled(_) => "unsupported",
        SelectorError::NotSettable { .. } => "not-settable",
        SelectorError::Attribute(_) => "attribute",
        SelectorError::TypeMismatch { .. } => "type-mismatch",
        SelectorError::UntypedValue { .. } => "untyped-value",
        SelectorError::InvalidText { .. } => "invalid-text",
        SelectorError::ConflictingWrite { .. } => "conflicting-write",
        SelectorError::PropertyEdit(_) => "property-edit",
        SelectorError::Property { error, .. } => crate::psets::property_error(error.clone()).kind(),
        SelectorError::Conflict(_) => "conflict",
        _ => "invalid-model",
    }
}

/// What an edit command did.
pub(crate) struct Report {
    pub(crate) command: &'static str,
    pub(crate) file: String,
    pub(crate) table: Option<String>,
    pub(crate) selected: Option<usize>,
    pub(crate) dry_run: bool,
    pub(crate) written: Option<String>,
    pub(crate) changes: Vec<Change>,
    pub(crate) refusals: Vec<Refusal>,
}

impl Report {
    /// Write the report and return how the run ends.
    pub(crate) fn emit(&self, format: TextFormat, out: &mut impl Write) -> CliResult<Outcome> {
        let text = match format {
            TextFormat::Json => self.json().pretty(),
            TextFormat::Human => {
                for refusal in &self.refusals {
                    eprintln!(
                        "openbim-ifc: refused: {}{}: {}",
                        refusal.location(),
                        refusal.kind,
                        refusal.detail
                    );
                }
                self.human()
            }
        };
        out.write_all(text.as_bytes())
            .map_err(|error| CliError::stdout(&error))?;
        Ok(if self.refusals.is_empty() {
            Outcome::Clean
        } else {
            Outcome::Refused
        })
    }

    fn human(&self) -> String {
        let mut text = String::new();
        for change in &self.changes {
            if let Some((row, column)) = change.cell {
                text.push_str(&format!("row {row}, column {}: ", column_name(column)));
            }
            text.push_str(&format!(
                "#{} {}{}: {:?} -> {:?}\n",
                change.entity.0,
                change.path,
                elsewhere(change.entity, &change.target),
                change.before,
                change.after
            ));
        }
        let count = |n: usize, what: &str| {
            if n == 1 {
                format!("1 {what}")
            } else {
                format!("{n} {what}s")
            }
        };
        if !self.refusals.is_empty() {
            text.push_str(&format!(
                "{}; nothing written\n",
                count(self.refusals.len(), "refusal")
            ));
        } else if self.dry_run {
            text.push_str(&format!(
                "{} planned (dry run; nothing written)\n",
                count(self.changes.len(), "change")
            ));
        } else if let Some(written) = &self.written {
            text.push_str(&format!(
                "{} written to {written}\n",
                count(self.changes.len(), "change")
            ));
        }
        text
    }

    fn json(&self) -> Json {
        let changes = self
            .changes
            .iter()
            .map(|change| {
                Json::object([
                    ("row", Json::opt_uint(change.cell.map(|cell| cell.0 as u64))),
                    (
                        "column",
                        Json::opt_uint(change.cell.map(|cell| cell.1 as u64)),
                    ),
                    ("entity", Json::uint(change.entity.0)),
                    ("path", Json::str(change.path.clone())),
                    ("target", target_json(&change.target)),
                    ("before", Json::str(change.before.clone())),
                    ("after", Json::str(change.after.clone())),
                ])
            })
            .collect();
        let refusals = self
            .refusals
            .iter()
            .map(|refusal| {
                Json::object([
                    ("row", Json::opt_uint(refusal.row.map(|row| row as u64))),
                    (
                        "column",
                        Json::opt_uint(refusal.column.map(|column| column as u64)),
                    ),
                    (
                        "entity",
                        Json::opt_uint(refusal.entity.map(|entity| entity.0)),
                    ),
                    ("path", Json::opt_str(refusal.path.clone())),
                    ("kind", Json::str(refusal.kind)),
                    ("detail", Json::str(refusal.detail.clone())),
                    (
                        "span",
                        refusal.span.as_ref().map_or(Json::Null, |span| {
                            Json::Array(vec![
                                Json::uint(span.start as u64),
                                Json::uint(span.end as u64),
                            ])
                        }),
                    ),
                ])
            })
            .collect();
        Json::object([
            ("command", Json::str(self.command)),
            ("file", Json::str(self.file.clone())),
            ("table", Json::opt_str(self.table.clone())),
            ("selected", Json::opt_uint(self.selected.map(|n| n as u64))),
            ("dry_run", Json::Bool(self.dry_run)),
            ("written", Json::opt_str(self.written.clone())),
            ("changes", Json::Array(changes)),
            ("refusals", Json::Array(refusals)),
        ])
    }
}

/// ` (#16 Name)` when the target is not on the path's own entity.
fn elsewhere(entity: EntityId, target: &SetTarget) -> String {
    match target {
        SetTarget::Attribute { entity: on, name } if *on != entity => {
            format!(" (#{} {name})", on.0)
        }
        SetTarget::Property {
            object,
            set,
            property,
        } if *object != entity => format!(" (#{} {set}.{property})", object.0),
        _ => String::new(),
    }
}

fn target_json(target: &SetTarget) -> Json {
    match target {
        SetTarget::Attribute { entity, name } => Json::object([
            ("entity", Json::uint(entity.0)),
            ("attribute", Json::str(name.clone())),
        ]),
        SetTarget::Property {
            object,
            set,
            property,
        } => Json::object([
            ("entity", Json::uint(object.0)),
            ("set", Json::str(set.clone())),
            ("property", Json::str(property.clone())),
        ]),
        other => Json::str(format!("{other:?}")),
    }
}
