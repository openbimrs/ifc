//! Tables of values by path (#474): the facade's `ifc::sheet`.
//!
//! [`IfcModel::export_table`] writes the entities a selector selects as a
//! table, the key column (`GlobalId`, or `id`) first and one column per
//! value path, as CSV, TSV or XLSX bytes. [`IfcModel::import_table`]
//! reads one back: rows are matched by key, every other cell is written
//! by its header's path as one checked transaction, a cell that reads what
//! the model holds changes nothing, and any refusal refuses the table with
//! nothing written. The answer is a [`TableImport`]: the planned changes
//! and every refusal by row and column, so a host shows them all, and
//! `dry_run` plans without writing.
//!
//! A table, selector or format the call cannot start from (an unknown
//! format or key name, a selector or path that does not parse, an
//! unsupported release, XLSX in a build without `sheet-xlsx`) raises the
//! usual error; refusals of the table's own rows and cells are data.

use ifc::selector::{value_text, Selector, SelectorError, SelectorIndex, SetTarget, ValuePath};
use ifc::sheet::{self, KeyColumn, Sheet, SheetFormat, SheetFormatError, SheetOptions};
use ifc::EntityId;

use crate::record::{Field, Record, ToRecord};
use crate::selector::{parse_error, selector_error};
use crate::{BindingError, IfcModel};

/// One value an import changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableChange {
    /// The one-based row, the header being row 1.
    pub row: usize,
    /// The one-based column.
    pub column: usize,
    /// The row's key as the table gives it.
    pub key: String,
    /// The column's value path.
    pub path: String,
    /// The entity the row names.
    pub entity: u64,
    /// The entity written: the row's, or one its path reaches
    /// (`type.Name` writes the type object).
    pub target_entity: u64,
    /// What is written there: an attribute name or `Set.Property`.
    pub target: String,
    /// The value now, as cell text.
    pub before: String,
    /// The value then, as cell text.
    pub after: String,
}

/// One refused table, row or cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRefusal {
    /// The one-based row; `None` for the table.
    pub row: Option<usize>,
    /// The one-based column; `None` for a row or the table.
    pub column: Option<usize>,
    /// A short, stable name for the kind: `malformed-table`,
    /// `invalid-header`, `duplicate-column`, `no-key-column`,
    /// `ragged-row`, `empty-key`, `unknown-key`, `duplicate-key`,
    /// `ambiguous-key`, or the binding error code of a refused write
    /// (`type-mismatch`, `invalid-value`, ...).
    pub code: String,
    /// Why, in words.
    pub message: String,
    /// The byte range of a header that does not parse.
    pub span: Option<(usize, usize)>,
}

/// What [`IfcModel::import_table`] planned, and whether it wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableImport {
    /// Whether the changes were written: `false` for a dry run, a refused
    /// table, or a table that changes nothing.
    pub applied: bool,
    /// Every value that changes, by row and column.
    pub changes: Vec<TableChange>,
    /// Every refusal; any refusal means nothing was written.
    pub refusals: Vec<TableRefusal>,
}

impl ToRecord for TableChange {
    fn to_record(&self) -> Record {
        Record::new(
            "TableChange",
            vec![
                ("row", Field::Count(self.row)),
                ("column", Field::Count(self.column)),
                ("key", Field::Text(self.key.clone())),
                ("path", Field::Text(self.path.clone())),
                ("entity", Field::Id(self.entity)),
                ("target_entity", Field::Id(self.target_entity)),
                ("target", Field::Text(self.target.clone())),
                ("before", Field::Text(self.before.clone())),
                ("after", Field::Text(self.after.clone())),
            ],
        )
    }
}

impl ToRecord for TableRefusal {
    fn to_record(&self) -> Record {
        Record::new(
            "TableRefusal",
            vec![
                ("row", Field::optional(self.row, Field::Count)),
                ("column", Field::optional(self.column, Field::Count)),
                ("code", Field::Text(self.code.clone())),
                ("message", Field::Text(self.message.clone())),
                (
                    "span",
                    Field::optional(self.span, |(start, end)| {
                        Field::List(vec![Field::Count(start), Field::Count(end)])
                    }),
                ),
            ],
        )
    }
}

impl ToRecord for TableImport {
    fn to_record(&self) -> Record {
        Record::new(
            "TableImport",
            vec![
                ("applied", Field::Bool(self.applied)),
                ("changes", Field::records(&self.changes)),
                ("refusals", Field::records(&self.refusals)),
            ],
        )
    }
}

/// A table format by name (`csv`, `tsv`, `xlsx`, any case).
fn format(name: &str) -> Result<SheetFormat, BindingError> {
    SheetFormat::from_name(name).ok_or_else(|| {
        BindingError::InvalidValue(format!("{name:?} is no table format (csv, tsv, xlsx)"))
    })
}

/// Options from a key name (`GlobalId`, `id`, or none) and a null text.
fn options(key: Option<&str>, null: &str) -> Result<SheetOptions, BindingError> {
    let mut options = SheetOptions::default().with_null(null);
    if let Some(name) = key {
        let key = KeyColumn::from_name(name).ok_or_else(|| {
            BindingError::InvalidValue(format!("{name:?} is no key column (GlobalId, id)"))
        })?;
        options = options.with_key(key);
    }
    Ok(options)
}

fn format_error(error: SheetFormatError) -> BindingError {
    match error {
        SheetFormatError::FeatureDisabled(feature) => BindingError::FeatureDisabled(feature),
        SheetFormatError::TooLarge(detail) => BindingError::Unsupported(detail),
        other => BindingError::Parse(other.to_string()),
    }
}

impl IfcModel {
    /// The entities `query` selects as a table in `format` (`csv`, `tsv`
    /// or `xlsx`): the key column first (`GlobalId` unless `key` names
    /// `id`), then one column per path of `columns`, each cell the value
    /// the path reads as text, `NULL` as `null`.
    ///
    /// Refused with `invalid-selector` (and its span) for a query or path
    /// that does not parse, `invalid-value` for an unknown format or key
    /// name, `wrong-entity-type` for an entity without a `GlobalId` in a
    /// table keyed by it, `feature-disabled` for XLSX in a build without
    /// `sheet-xlsx`, and as [`Self::select`] and [`Self::get_value`].
    pub fn export_table(
        &self,
        query: &str,
        columns: &[String],
        format_name: &str,
        key: Option<&str>,
        null: &str,
    ) -> Result<Vec<u8>, BindingError> {
        let format = format(format_name)?;
        let options = options(key, null)?;
        let selector = Selector::parse(query).map_err(parse_error)?;
        let paths = columns
            .iter()
            .map(|path| ValuePath::parse(path).map_err(parse_error))
            .collect::<Result<Vec<_>, _>>()?;
        let schema = self.declared_schema()?;
        let index = SelectorIndex::new(&self.inner, schema);
        let selected = index.select(&selector).map_err(selector_error)?;
        let table =
            sheet::export(&index, &selected, &paths, &options).map_err(|error| match error {
                sheet::ExportError::Value { error, .. } => selector_error(*error),
                other => BindingError::WrongEntityType(other.to_string()),
            })?;
        table.write(format).map_err(format_error)
    }

    /// Apply the table `data` (`csv`, `tsv` or `xlsx`) to the model by its
    /// key column (`key`: `GlobalId` or `id`; by default `GlobalId` when
    /// the table has one, else `id`) as one checked transaction, `null`
    /// being the cell text of `NULL`. With `dry_run`, or when anything is
    /// refused, nothing is written.
    ///
    /// Refused with `invalid-value` for an unknown format or key name,
    /// `parse` for bytes that are no table of the format,
    /// `feature-disabled` for XLSX in a build without `sheet-xlsx`, and
    /// `unsupported-schema`; every refusal of the table's header, rows and
    /// cells is in [`TableImport::refusals`].
    pub fn import_table(
        &mut self,
        data: &[u8],
        format_name: &str,
        key: Option<&str>,
        null: &str,
        dry_run: bool,
    ) -> Result<TableImport, BindingError> {
        self.import_table_seeded(data, format_name, key, null, dry_run, ifc::fresh_seed())
    }

    /// [`Self::import_table`]
    /// with the seed the `GlobalId`s of new sets and relationships derive
    /// from: a host with entropy of its own (the browser) passes it. The
    /// seed names the batch: a fixed seed reproduces its `GlobalId`s, also
    /// after a write/read cycle; reusing it where the model holds one it
    /// derives is refused (`invalid-value`).
    pub fn import_table_seeded(
        &mut self,
        data: &[u8],
        format_name: &str,
        key: Option<&str>,
        null: &str,
        dry_run: bool,
        seed: u128,
    ) -> Result<TableImport, BindingError> {
        let format = format(format_name)?;
        let options = options(key, null)?;
        let table = match Sheet::read(data, format) {
            Ok(table) => table,
            Err(SheetFormatError::Malformed {
                row,
                column,
                detail,
            }) => {
                return Ok(TableImport {
                    applied: false,
                    changes: Vec::new(),
                    refusals: vec![TableRefusal {
                        row: Some(row),
                        column,
                        code: "malformed-table".to_owned(),
                        message: detail,
                        span: None,
                    }],
                })
            }
            Err(other) => return Err(format_error(other)),
        };
        let schema = self.declared_schema()?;
        let planned = sheet::plan_import(
            &SelectorIndex::new(&self.inner, schema),
            &table,
            &options,
            seed,
        );
        let plan = match planned {
            Ok(plan) => plan,
            Err(refused) => {
                return Ok(TableImport {
                    applied: false,
                    changes: Vec::new(),
                    refusals: refused.iter().map(refusal).collect(),
                })
            }
        };
        let changes: Vec<TableChange> = plan
            .changes
            .iter()
            .map(|cell| {
                let (target_entity, target) = target(&cell.change.target);
                TableChange {
                    row: cell.row,
                    column: cell.column,
                    key: cell.key.clone(),
                    path: cell.path.clone(),
                    entity: cell.change.entity.0,
                    target_entity,
                    target,
                    before: value_text(&cell.change.before, null),
                    after: value_text(&cell.change.after, null),
                }
            })
            .collect();
        if dry_run || changes.is_empty() {
            return Ok(TableImport {
                applied: false,
                changes,
                refusals: Vec::new(),
            });
        }
        match plan.commit(&mut self.inner) {
            Ok(_) => Ok(TableImport {
                applied: true,
                changes,
                refusals: Vec::new(),
            }),
            Err(refused) => Ok(TableImport {
                applied: false,
                changes,
                refusals: refused.iter().map(refusal).collect(),
            }),
        }
    }
}

fn target(target: &SetTarget) -> (u64, String) {
    match target {
        SetTarget::Attribute {
            entity: EntityId(id),
            name,
        } => (*id, name.clone()),
        SetTarget::Property {
            object: EntityId(id),
            set,
            property,
        } => (*id, format!("{set}.{property}")),
        other => (0, format!("{other:?}")),
    }
}

fn refusal(refused: &sheet::SheetRefusal) -> TableRefusal {
    use sheet::SheetRefusalReason as R;
    let (code, span) = match &refused.reason {
        R::Empty => ("empty-table".to_owned(), None),
        R::Header(error) => (
            "invalid-header".to_owned(),
            Some((error.span.start, error.span.end)),
        ),
        R::DuplicateColumn { .. } => ("duplicate-column".to_owned(), None),
        R::NoKeyColumn(_) => ("no-key-column".to_owned(), None),
        R::RaggedRow { .. } => ("ragged-row".to_owned(), None),
        R::EmptyKey => ("empty-key".to_owned(), None),
        R::UnknownKey(_) => ("unknown-key".to_owned(), None),
        R::DuplicateKey { .. } => ("duplicate-key".to_owned(), None),
        R::AmbiguousKey(_) => ("ambiguous-key".to_owned(), None),
        R::Value(error) => (value_code(error), None),
        _ => ("invalid-value".to_owned(), None),
    };
    TableRefusal {
        row: refused.row,
        column: refused.column,
        code,
        message: refused.reason.to_string(),
        span,
    }
}

/// The binding error code a refused write would raise.
fn value_code(error: &SelectorError) -> String {
    match error {
        SelectorError::InvalidText { .. } => "type-mismatch".to_owned(),
        SelectorError::ConflictingWrite { .. } => "invalid-value".to_owned(),
        other => selector_error(other.clone()).code().to_owned(),
    }
}
