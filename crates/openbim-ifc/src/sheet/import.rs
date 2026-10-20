//! [`plan_import`] and [`import`]: a [`Sheet`] written back by key.

use std::collections::HashMap;
use std::fmt;

use ifc_model::{Applied, EntityId, Model, Transaction};
use ifc_schema::Schema;

use super::grid::column_name;
use super::{KeyColumn, Sheet, SheetOptions};
use crate::selector::{
    EditRefusal, ParseError, PlannedChange, SelectorError, SelectorIndex, TextEdit, ValuePath,
};
use crate::DuplicateGlobalId;

/// One cell that changes the model.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct CellChange {
    /// The one-based row, the header being row 1.
    pub row: usize,
    /// The one-based column.
    pub column: usize,
    /// The row's key as the sheet gives it.
    pub key: String,
    /// The column's value path, canonical.
    pub path: String,
    /// What changes. A path such as `type.Name` changes another entity
    /// than the row's; a cell whose path reaches several entities changes
    /// each.
    pub change: PlannedChange,
}

/// A planned import: what changes, and the one transaction that changes
/// it.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ImportPlan {
    /// The column the rows were matched by.
    pub key: KeyColumn,
    /// Every change, by row and then column.
    pub changes: Vec<CellChange>,
    /// The writes, staged against the model the plan read; empty when
    /// nothing changes.
    pub transaction: Transaction,
}

impl ImportPlan {
    /// Whether the import changes nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Commit the plan as one transaction; `None` when it changes nothing,
    /// which leaves the model and its revision untouched.
    ///
    /// # Errors
    ///
    /// One [`SheetRefusal`] carrying [`SelectorError::Conflict`] when the
    /// transaction fails its preflight, such as for a model that changed
    /// since the plan was made. The model is unchanged then.
    pub fn commit(self, model: &mut Model) -> Result<Option<Applied>, Vec<SheetRefusal>> {
        if self.transaction.is_empty() {
            return Ok(None);
        }
        self.transaction
            .commit(model)
            .map(Some)
            .map_err(|conflicts| {
                vec![SheetRefusal {
                    row: None,
                    column: None,
                    reason: SheetRefusalReason::Value(SelectorError::Conflict(conflicts)),
                }]
            })
    }
}

/// Why a sheet, a row or a cell was refused. A refused import writes
/// nothing.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct SheetRefusal {
    /// The one-based row, the header being row 1; `None` for the sheet.
    pub row: Option<usize>,
    /// The one-based column; `None` for a whole row or the sheet.
    pub column: Option<usize>,
    /// Why.
    pub reason: SheetRefusalReason,
}

/// The reason a [`SheetRefusal`] gives.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SheetRefusalReason {
    /// The sheet has no header row.
    Empty,
    /// A header is no value path; the span is in the header's text.
    Header(ParseError),
    /// A header names the same path as an earlier column.
    DuplicateColumn {
        /// The earlier column, one-based.
        first: usize,
    },
    /// The sheet has no key column: no `GlobalId` and no `id` column, or
    /// not the one the options name.
    NoKeyColumn(Option<KeyColumn>),
    /// A row has another number of cells than the header.
    RaggedRow {
        /// The row's cells.
        cells: usize,
        /// The header's columns.
        columns: usize,
    },
    /// A row's key is empty.
    EmptyKey,
    /// A row's key names no entity of the model.
    UnknownKey(String),
    /// A row's key is also an earlier row's.
    DuplicateKey {
        /// The earlier row, one-based.
        first: usize,
    },
    /// A row's `GlobalId` is carried by several entities.
    AmbiguousKey(DuplicateGlobalId),
    /// A cell's write was refused, or the batch's: a value that does not
    /// read as its target's type, a path that names no value to write, a
    /// conflicting write, a property edit's refusal.
    Value(SelectorError),
}

impl fmt::Display for SheetRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.row, self.column) {
            (Some(row), Some(column)) => {
                write!(f, "row {row}, column {}: ", column_name(column))?;
            }
            (Some(row), None) => write!(f, "row {row}: ")?,
            (None, Some(column)) => write!(f, "column {}: ", column_name(column))?,
            (None, None) => {}
        }
        match &self.reason {
            SheetRefusalReason::Empty => f.write_str("the sheet has no header row"),
            SheetRefusalReason::Header(error) => write!(f, "header: {error}"),
            SheetRefusalReason::DuplicateColumn { first } => {
                write!(f, "the same path as column {}", column_name(*first))
            }
            SheetRefusalReason::NoKeyColumn(Some(key)) => {
                write!(f, "no {} column to match rows by", key.header())
            }
            SheetRefusalReason::NoKeyColumn(None) => {
                f.write_str("no GlobalId or id column to match rows by")
            }
            SheetRefusalReason::RaggedRow { cells, columns } => {
                write!(f, "{cells} cells under {columns} columns")
            }
            SheetRefusalReason::EmptyKey => f.write_str("the key is empty"),
            SheetRefusalReason::UnknownKey(key) => write!(f, "no entity has the key {key:?}"),
            SheetRefusalReason::DuplicateKey { first } => {
                write!(f, "the same key as row {first}")
            }
            SheetRefusalReason::AmbiguousKey(duplicate) => write!(f, "{duplicate}"),
            SheetRefusalReason::Value(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SheetRefusal {}

/// A row's cell, as a write.
struct Origin {
    row: usize,
    column: usize,
    key: String,
}

/// Plan writing `sheet` back to the model `index` reads, matching rows by
/// the key column and writing every other cell by its header's path as
/// one transaction.
///
/// The key column is `options.key`, else `GlobalId` when the header has
/// one, else `id` (`12` or `#12`). The key column is never written. A
/// cell whose text is what the path reads now changes nothing; a row of
/// empty cells is skipped. Every other cell is read as a value of its
/// target's type and planned as
/// [`SelectorIndex::plan_text_edits`] plans it, `options.null` writing
/// `$`.
///
/// # Errors
///
/// Every refusal found, by row and column: the sheet's, its header's
/// ([`SheetRefusalReason::Header`] with the span), each row's key, and
/// each cell's write. Nothing is planned when anything is refused.
pub fn plan_import(
    index: &SelectorIndex<'_>,
    sheet: &Sheet,
    options: &SheetOptions,
) -> Result<ImportPlan, Vec<SheetRefusal>> {
    let refusal = |row, column, reason| SheetRefusal {
        row,
        column,
        reason,
    };
    if sheet.header.is_empty() {
        return Err(vec![refusal(None, None, SheetRefusalReason::Empty)]);
    }
    let mut refusals = Vec::new();
    let mut paths = Vec::with_capacity(sheet.header.len());
    let mut seen: HashMap<String, usize> = HashMap::new();
    for (index, text) in sheet.header.iter().enumerate() {
        let column = index + 1;
        match ValuePath::parse(text) {
            Ok(path) => {
                if let Some(&first) = seen.get(&path.to_string()) {
                    refusals.push(refusal(
                        Some(1),
                        Some(column),
                        SheetRefusalReason::DuplicateColumn { first },
                    ));
                    paths.push(None);
                    continue;
                }
                seen.insert(path.to_string(), column);
                paths.push(Some(path));
            }
            Err(error) => {
                refusals.push(refusal(
                    Some(1),
                    Some(column),
                    SheetRefusalReason::Header(error),
                ));
                paths.push(None);
            }
        }
    }
    let found = |key: KeyColumn| {
        paths
            .iter()
            .position(|path| path.as_ref().is_some_and(|path| key.is(path)))
    };
    let key_column = match options.key {
        Some(key) => found(key).map(|column| (key, column)),
        None => found(KeyColumn::GlobalId)
            .map(|column| (KeyColumn::GlobalId, column))
            .or_else(|| found(KeyColumn::Id).map(|column| (KeyColumn::Id, column))),
    };
    let Some((key, key_at)) = key_column else {
        refusals.push(refusal(
            Some(1),
            None,
            SheetRefusalReason::NoKeyColumn(options.key),
        ));
        return Err(refusals);
    };

    let mut edits = Vec::new();
    let mut origins = Vec::new();
    let mut rows_by_key: HashMap<EntityId, usize> = HashMap::new();
    for (index_in_rows, cells) in sheet.rows.iter().enumerate() {
        let row = index_in_rows + 2;
        if cells.iter().all(String::is_empty) {
            continue;
        }
        if cells.len() != paths.len() {
            refusals.push(refusal(
                Some(row),
                None,
                SheetRefusalReason::RaggedRow {
                    cells: cells.len(),
                    columns: paths.len(),
                },
            ));
            continue;
        }
        let key_text = cells[key_at].trim();
        let entity = match resolve(index, key, key_text) {
            Ok(entity) => entity,
            Err(reason) => {
                refusals.push(refusal(Some(row), Some(key_at + 1), reason));
                continue;
            }
        };
        if let Some(first) = rows_by_key.insert(entity, row) {
            refusals.push(refusal(
                Some(row),
                Some(key_at + 1),
                SheetRefusalReason::DuplicateKey { first },
            ));
            continue;
        }
        for (column, (cell, path)) in cells.iter().zip(&paths).enumerate() {
            let Some(path) = path else { continue };
            if column == key_at {
                continue;
            }
            edits.push(TextEdit::new(entity, path.clone(), cell.clone()));
            origins.push(Origin {
                row,
                column: column + 1,
                key: key_text.to_owned(),
            });
        }
    }

    let planned = index.plan_text_edits(&edits, &options.null);
    if let Err(refused) = &planned {
        refusals.extend(refused.iter().map(|EditRefusal { edit, error }| {
            let origin = edit.and_then(|edit| origins.get(edit));
            refusal(
                origin.map(|origin| origin.row),
                origin.map(|origin| origin.column),
                SheetRefusalReason::Value(error.clone()),
            )
        }));
    }
    let plan = match planned {
        Ok(plan) if refusals.is_empty() => plan,
        _ => {
            refusals.sort_by_key(|refusal| (refusal.row, refusal.column));
            return Err(refusals);
        }
    };
    let changes = plan
        .changes
        .into_iter()
        .map(|change| {
            let origin = &origins[change.edit];
            CellChange {
                row: origin.row,
                column: origin.column,
                key: origin.key.clone(),
                path: edits[change.edit].path.to_string(),
                change,
            }
        })
        .collect();
    Ok(ImportPlan {
        key,
        changes,
        transaction: plan.transaction,
    })
}

/// The entity a row's key names.
fn resolve(
    index: &SelectorIndex<'_>,
    key: KeyColumn,
    text: &str,
) -> Result<EntityId, SheetRefusalReason> {
    if text.is_empty() {
        return Err(SheetRefusalReason::EmptyKey);
    }
    let unknown = || SheetRefusalReason::UnknownKey(text.to_owned());
    match key {
        KeyColumn::GlobalId => {
            let guid = crate::Guid::parse(text).ok_or_else(unknown)?;
            match index.guids().get(&guid) {
                Ok(Some(entity)) => Ok(entity),
                Ok(None) => Err(unknown()),
                Err(duplicate) => Err(SheetRefusalReason::AmbiguousKey(duplicate)),
            }
        }
        KeyColumn::Id => {
            let digits = text.strip_prefix('#').unwrap_or(text);
            let id = digits.parse::<u64>().map_err(|_| unknown())?;
            index
                .model()
                .contains(EntityId(id))
                .then_some(EntityId(id))
                .ok_or_else(unknown)
        }
    }
}

/// Plan `sheet` against `model` and commit it as one transaction: every
/// cell or none. Returns what changed; an empty list leaves the model and
/// its revision untouched.
///
/// # Errors
///
/// As [`plan_import`], and a refusal carrying [`SelectorError::Conflict`]
/// when the transaction fails its preflight. The model is unchanged on
/// every error.
pub fn import(
    model: &mut Model,
    schema: &Schema,
    sheet: &Sheet,
    options: &SheetOptions,
) -> Result<Vec<CellChange>, Vec<SheetRefusal>> {
    let plan = plan_import(&SelectorIndex::new(model, schema), sheet, options)?;
    let changes = plan.changes.clone();
    plan.commit(model)?;
    Ok(changes)
}
