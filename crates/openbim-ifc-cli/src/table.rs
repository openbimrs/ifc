//! `export-table` and `import-table`: the facade's `ifc::sheet`.
//!
//! `export-table` writes the entities a selector selects as a table, the
//! key column first (`GlobalId`, or `id` with `--key id`), then one column
//! per `--column` path, each headed by the path. `import-table` reads a
//! table back: rows are matched by their key, every other cell is written
//! by its header's path as one checked transaction, a cell that reads what
//! the model holds changes nothing, and any refusal refuses the run with
//! nothing written (exit 3).

use std::io::Write;
use std::path::Path;

use ifc::selector::{value_text, Selector, SelectorIndex, ValuePath};
use ifc::sheet::{self, KeyColumn, Sheet, SheetFormat, SheetFormatError, SheetOptions};

use crate::cli::{ExportTableArgs, ImportTableArgs, KeyArg, SheetFormatArg};
use crate::error::{CliError, CliResult, Outcome};
use crate::input;
use crate::output;
use crate::report::{Change, Refusal, Report};
use crate::select::selector_error;

/// The columns `export-table` writes without `--column`.
const DEFAULT_COLUMNS: [&str; 2] = ["class", "Name"];

fn key(arg: KeyArg) -> KeyColumn {
    match arg {
        KeyArg::GlobalId => KeyColumn::GlobalId,
        KeyArg::Id => KeyColumn::Id,
    }
}

fn format(arg: SheetFormatArg) -> SheetFormat {
    match arg {
        SheetFormatArg::Csv => SheetFormat::Csv,
        SheetFormatArg::Tsv => SheetFormat::Tsv,
        SheetFormatArg::Xlsx => SheetFormat::Xlsx,
    }
}

/// The format `named` gives, else the one `path`'s extension implies.
fn format_for(named: Option<SheetFormatArg>, path: &Path) -> Option<SheetFormat> {
    named.map(format).or_else(|| {
        path.extension()
            .and_then(|extension| extension.to_str())
            .and_then(SheetFormat::from_extension)
    })
}

fn format_error(path: &Path, error: SheetFormatError) -> CliError {
    let shown = path.display().to_string();
    match error {
        SheetFormatError::FeatureDisabled(_) | SheetFormatError::TooLarge(_) => {
            CliError::Unsupported(format!("{shown}: {error}"))
        }
        other => CliError::Parse {
            path: shown,
            detail: other.to_string(),
        },
    }
}

/// Run `export-table`.
pub(crate) fn run_export(args: &ExportTableArgs, out: &mut impl Write) -> CliResult<Outcome> {
    let selector = Selector::parse(&args.query)
        .map_err(|error| CliError::InvalidSelector(format!("{:?}: {error}", args.query)))?;
    let texts: Vec<&str> = if args.columns.is_empty() {
        DEFAULT_COLUMNS.to_vec()
    } else {
        args.columns.iter().map(String::as_str).collect()
    };
    let columns = texts
        .iter()
        .map(|text| {
            ValuePath::parse(text)
                .map_err(|error| CliError::InvalidSelector(format!("--column {text:?}: {error}")))
        })
        .collect::<CliResult<Vec<_>>>()?;
    let to_stdout = args.output.as_os_str() == "-";
    let table_format = format_for(args.to, &args.output).unwrap_or(SheetFormat::Csv);
    if !to_stdout && !args.force && args.output.exists() {
        return Err(CliError::Usage(format!(
            "{} exists; pass --force to replace it",
            args.output.display()
        )));
    }
    let loaded = input::read(&args.file, args.input.input_layout)?;
    let schema = input::declared_schema(&loaded.model)?;
    let index = SelectorIndex::new(&loaded.model, schema);
    let selected = index
        .select(&selector)
        .map_err(|error| selector_error(&args.query, error))?;
    let options = SheetOptions::default()
        .with_key(key(args.key))
        .with_null(args.null.clone());
    let table =
        sheet::export(&index, &selected, &columns, &options).map_err(|error| match error {
            sheet::ExportError::Value { error, .. } => selector_error(&args.query, *error),
            other => CliError::WrongEntityType(other.to_string()),
        })?;
    let bytes = table
        .write(table_format)
        .map_err(|error| format_error(&args.output, error))?;
    if to_stdout {
        out.write_all(&bytes)
            .map_err(|error| CliError::stdout(&error))?;
    } else {
        std::fs::write(&args.output, &bytes).map_err(|error| CliError::Io {
            path: args.output.display().to_string(),
            detail: error.to_string(),
        })?;
        eprintln!(
            "openbim-ifc: {} rows x {} columns -> {} ({})",
            table.rows.len(),
            table.header.len(),
            args.output.display(),
            table_format.name()
        );
    }
    Ok(Outcome::Clean)
}

/// Run `import-table`.
pub(crate) fn run_import(args: &ImportTableArgs, out: &mut impl Write) -> CliResult<Outcome> {
    let table_format = format_for(args.from, &args.table).ok_or_else(|| {
        CliError::Usage(format!(
            "cannot tell the table format from {}; name it with --from csv, tsv or xlsx",
            args.table.display()
        ))
    })?;
    let destination = output::destination(&args.output, &args.file)?;
    let bytes = std::fs::read(&args.table).map_err(|error| CliError::Io {
        path: args.table.display().to_string(),
        detail: error.to_string(),
    })?;
    let table =
        Sheet::read(&bytes, table_format).map_err(|error| format_error(&args.table, error))?;
    let mut loaded = input::read(&args.file, args.input.input_layout)?;
    let schema = input::declared_schema(&loaded.model)?;
    let mut options = SheetOptions::default().with_null(args.null.clone());
    if let Some(arg) = args.key {
        options = options.with_key(key(arg));
    }
    let mut report = Report {
        command: "import-table",
        file: loaded.path.clone(),
        table: Some(args.table.display().to_string()),
        selected: None,
        dry_run: args.output.dry_run,
        written: None,
        changes: Vec::new(),
        refusals: Vec::new(),
    };
    let planned = sheet::plan_import(
        &SelectorIndex::new(&loaded.model, schema),
        &table,
        &options,
        ifc::fresh_seed(),
    );
    let plan = match planned {
        Ok(plan) => plan,
        Err(refused) => {
            report.refusals = refused
                .iter()
                .map(|refusal| Refusal::of_sheet(refusal.row, refusal.column, &refusal.reason))
                .collect();
            return report.emit(args.format, out);
        }
    };
    report.changes = plan
        .changes
        .iter()
        .map(|cell| Change {
            cell: Some((cell.row, cell.column)),
            entity: cell.change.entity,
            path: cell.path.clone(),
            target: cell.change.target.clone(),
            before: value_text(&cell.change.before, &args.null),
            after: value_text(&cell.change.after, &args.null),
        })
        .collect();
    if let Some(path) = destination {
        if let Err(refused) = plan.commit(&mut loaded.model) {
            report.refusals = refused
                .iter()
                .map(|refusal| Refusal::of_sheet(refusal.row, refusal.column, &refusal.reason))
                .collect();
            return report.emit(args.format, out);
        }
        output::write_edited(&loaded.model, &loaded, &path, args.output.in_place)?;
        report.written = Some(path.display().to_string());
    }
    report.emit(args.format, out)
}
