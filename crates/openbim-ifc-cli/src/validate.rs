//! `validate`: every file against the schema its own header declares.
//!
//! The verdict is the validator's: a file fails on an error or an
//! evaluation error (a rule that could not be decided is not a rule that
//! passed), on a truncated report (its counts are lower bounds), and with
//! `--deny-warnings` on a warning. Rules the validator does not evaluate
//! are counted, never a failure. A file that cannot be read or declares no
//! bundled schema is reported and makes the run exit 2, after every other
//! file has been checked.
//!
//! `--express FILE.exp` (#538) also evaluates every WHERE, type and global
//! rule of the file's release from the EXPRESS text the caller supplies.
//! The `.exp` files are read once, before any model; each model is checked
//! with the first one that declares its release (not refused with a
//! schema mismatch), and refused with `express-mismatch` when none does.
//! An `.exp` that cannot be read ends the run; one that does not parse
//! refuses the files it is tried on with `invalid-express`. Rules that
//! evaluate to UNKNOWN are `indeterminate` findings: reported and counted,
//! never a failure.

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

use ifc::validate::express::{ExpressError, ExpressRules};
use ifc::validate::{validate_with_options, Budget, Path, Report, Severity, ValidateOptions};
use ifc::Schema;

use crate::cli::{ReportFormat, ValidateArgs};
use crate::error::{CliError, CliResult, Outcome};
use crate::input::{self, Loaded};
use crate::json::Json;
use crate::sarif::{self, Level};

/// One finding, flattened for every output format.
struct Item {
    severity: Severity,
    rule: String,
    path: String,
    entity: Option<u64>,
    attribute_index: Option<usize>,
    attribute_name: Option<String>,
    line: Option<usize>,
    message: String,
}

/// One file's result.
struct Checked {
    schema: String,
    conformant: bool,
    truncated: bool,
    passed: bool,
    errors: usize,
    evaluation_errors: usize,
    warnings: usize,
    indeterminate: usize,
    unsupported: usize,
    items: Vec<Item>,
}

/// The `--express` files, read once, and their parses per release.
struct ExpressFiles {
    /// Each file's path as given, and its text.
    files: Vec<(PathBuf, String)>,
    /// The outcome of parsing file `.0` against the tables named `.1`.
    parsed: HashMap<(usize, String), Result<ExpressRules, ExpressError>>,
}

impl ExpressFiles {
    /// Read every file; the first that cannot be read ends the run.
    fn read(paths: &[PathBuf]) -> CliResult<Self> {
        let files = paths
            .iter()
            .map(|path| {
                let bytes = std::fs::read(path).map_err(|error| CliError::Io {
                    path: path.display().to_string(),
                    detail: error.to_string(),
                })?;
                // The normative files are ASCII, but some copies are
                // Latin-1: one character per byte then.
                let text = String::from_utf8(bytes).unwrap_or_else(|error| {
                    error.into_bytes().iter().map(|&b| char::from(b)).collect()
                });
                Ok((path.clone(), text))
            })
            .collect::<CliResult<_>>()?;
        Ok(Self {
            files,
            parsed: HashMap::new(),
        })
    }

    /// The rules for a model whose header declares `schema`.
    fn rules_for(&mut self, schema: &Schema) -> CliResult<ExpressRules> {
        for (index, (path, text)) in self.files.iter().enumerate() {
            let outcome = self
                .parsed
                .entry((index, schema.name().to_owned()))
                .or_insert_with(|| ExpressRules::parse(text, schema));
            match outcome {
                Ok(rules) => return Ok(rules.clone()),
                Err(ExpressError::SchemaMismatch { .. }) => {}
                Err(error @ ExpressError::TableMismatch { .. }) => {
                    return Err(CliError::ExpressMismatch(format!(
                        "{}: {error}",
                        path.display()
                    )));
                }
                Err(error) => {
                    return Err(CliError::InvalidExpress(format!(
                        "{}: {error}",
                        path.display()
                    )));
                }
            }
        }
        let given: Vec<String> = self
            .files
            .iter()
            .map(|(path, _)| path.display().to_string())
            .collect();
        Err(CliError::ExpressMismatch(format!(
            "no --express file declares {} (given: {})",
            schema.name(),
            given.join(", ")
        )))
    }
}

/// Run `validate`.
pub(crate) fn run(args: &ValidateArgs, out: &mut impl Write) -> CliResult<Outcome> {
    let budget = args
        .max_findings
        .map_or(Budget::DEFAULT, |max_findings| Budget { max_findings });
    let files: Vec<String> = args
        .files
        .iter()
        .map(|file| file.display().to_string())
        .collect();
    let mut express = ExpressFiles::read(&args.express)?;
    let results: Vec<CliResult<Checked>> = args
        .files
        .iter()
        .map(|file| {
            let loaded = input::read(file, args.input.input_layout)?;
            check(&loaded, budget, args, &mut express)
        })
        .collect();

    match args.format {
        ReportFormat::Human => human(&files, &results, !args.express.is_empty(), out),
        ReportFormat::Json => json(&files, &results, out),
        ReportFormat::Sarif => sarif(&files, &results, out),
    }
    .map_err(|error| CliError::stdout(&error))?;

    Ok(verdict(&results))
}

/// The run's outcome: 2 if a file could not be checked, else 1 if one
/// failed, else 0.
fn verdict(results: &[CliResult<Checked>]) -> Outcome {
    if results.iter().any(Result::is_err) {
        Outcome::Incomplete
    } else if results.iter().flatten().any(|checked| !checked.passed) {
        Outcome::Findings
    } else {
        Outcome::Clean
    }
}

fn check(
    loaded: &Loaded,
    budget: Budget,
    args: &ValidateArgs,
    express: &mut ExpressFiles,
) -> CliResult<Checked> {
    let schema = input::declared_schema(&loaded.model)?;
    let rules = if express.files.is_empty() {
        None
    } else {
        Some(express.rules_for(schema)?)
    };
    let mut options = ValidateOptions::new().budget(budget);
    if let Some(rules) = &rules {
        options = options.express(rules);
    }
    let report: Report = validate_with_options(&loaded.model, schema, &options);
    let summary = report.summary();
    let line = |id: u64| loaded.lines.as_ref().and_then(|lines| lines.line(id));
    let items = report
        .sorted()
        .into_iter()
        .filter(|finding| args.include_unsupported || finding.severity != Severity::Unsupported)
        .map(|finding| {
            let (entity, attribute_index, attribute_name) = match &finding.path {
                Path::Entity(id) => (Some(id.0), None, None),
                Path::Attribute {
                    entity,
                    index,
                    name,
                } => (Some(entity.0), Some(*index), name.clone()),
                // The file, or a location kind added later: `path` still
                // says where.
                _ => (None, None, None),
            };
            Item {
                severity: finding.severity,
                rule: finding.rule.clone(),
                path: finding.path.to_string(),
                entity,
                attribute_index,
                attribute_name,
                line: entity.and_then(line),
                message: finding.message.clone(),
            }
        })
        .collect();
    let conformant = report.is_conformant();
    let truncated = report.is_truncated();
    Ok(Checked {
        schema: loaded
            .model
            .header()
            .schema_token()
            .unwrap_or_default()
            .to_owned(),
        conformant,
        truncated,
        passed: conformant && !truncated && !(args.deny_warnings && summary.warnings > 0),
        errors: summary.errors,
        evaluation_errors: summary.evaluation_errors,
        warnings: summary.warnings,
        indeterminate: summary.indeterminate,
        unsupported: summary.unsupported,
        items,
    })
}

fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

fn human(
    files: &[String],
    results: &[CliResult<Checked>],
    express: bool,
    out: &mut impl Write,
) -> std::io::Result<()> {
    for (file, result) in files.iter().zip(results) {
        let checked = match result {
            Ok(checked) => checked,
            Err(error) => {
                eprintln!("openbim-ifc: {file}: not validated: {error}");
                continue;
            }
        };
        for item in &checked.items {
            let at = match item.line {
                Some(line) => format!("{file}:{line}"),
                None => file.clone(),
            };
            let target = if item.entity.is_some() {
                format!(" {}", item.path)
            } else {
                String::new()
            };
            writeln!(
                out,
                "{at}: {} [{}]{target}: {}",
                item.severity, item.rule, item.message
            )?;
        }
        let verdict = match (checked.passed, checked.conformant) {
            (true, _) => "conformant",
            (false, true) if checked.truncated => "verdict not final (report truncated)",
            (false, true) => "fails on warnings",
            (false, false) => "not conformant",
        };
        let truncated = if checked.truncated {
            " (truncated: counts are lower bounds)"
        } else {
            ""
        };
        // Only a run that evaluates EXPRESS rules can decide "indeterminate".
        let indeterminate = if express {
            format!(", {} indeterminate", checked.indeterminate)
        } else {
            String::new()
        };
        writeln!(
            out,
            "{file}: {}: {verdict}: {}, {}, {}{indeterminate}; {} not evaluated{truncated}",
            checked.schema,
            plural(checked.errors, "error", "errors"),
            plural(
                checked.evaluation_errors,
                "evaluation error",
                "evaluation errors"
            ),
            plural(checked.warnings, "warning", "warnings"),
            plural(checked.unsupported, "rule", "rules"),
        )?;
    }
    Ok(())
}

fn json(
    files: &[String],
    results: &[CliResult<Checked>],
    out: &mut impl Write,
) -> std::io::Result<()> {
    let reports = files
        .iter()
        .zip(results)
        .map(|(file, result)| match result {
            Ok(checked) => Json::object([
                ("file", Json::str(file.clone())),
                ("schema", Json::str(checked.schema.clone())),
                ("passed", Json::Bool(checked.passed)),
                ("conformant", Json::Bool(checked.conformant)),
                ("truncated", Json::Bool(checked.truncated)),
                (
                    "summary",
                    Json::object([
                        ("errors", Json::uint(checked.errors as u64)),
                        (
                            "evaluation_errors",
                            Json::uint(checked.evaluation_errors as u64),
                        ),
                        ("warnings", Json::uint(checked.warnings as u64)),
                        ("indeterminate", Json::uint(checked.indeterminate as u64)),
                        ("unsupported", Json::uint(checked.unsupported as u64)),
                    ]),
                ),
                (
                    "findings",
                    Json::Array(
                        checked
                            .items
                            .iter()
                            .map(|item| {
                                Json::object([
                                    ("severity", Json::str(item.severity.to_string())),
                                    ("rule", Json::str(item.rule.clone())),
                                    ("path", Json::str(item.path.clone())),
                                    ("entity", Json::opt_uint(item.entity)),
                                    (
                                        "attribute_index",
                                        Json::opt_uint(item.attribute_index.map(|i| i as u64)),
                                    ),
                                    ("attribute_name", Json::opt_str(item.attribute_name.clone())),
                                    ("line", Json::opt_uint(item.line.map(|l| l as u64))),
                                    ("message", Json::str(item.message.clone())),
                                ])
                            })
                            .collect(),
                    ),
                ),
            ]),
            Err(error) => Json::object([
                ("file", Json::str(file.clone())),
                (
                    "error",
                    Json::object([
                        ("kind", Json::str(error.kind())),
                        ("message", Json::str(error.to_string())),
                    ]),
                ),
            ]),
        })
        .collect();
    let document = Json::object([
        ("tool", Json::str("openbim-ifc")),
        ("version", Json::str(env!("CARGO_PKG_VERSION"))),
        ("passed", Json::Bool(verdict(results) == Outcome::Clean)),
        ("files", Json::Array(reports)),
    ]);
    out.write_all(document.pretty().as_bytes())
}

fn sarif(
    files: &[String],
    results: &[CliResult<Checked>],
    out: &mut impl Write,
) -> std::io::Result<()> {
    let mut run = sarif::Run {
        files: files.to_vec(),
        ..sarif::Run::default()
    };
    for (index, result) in results.iter().enumerate() {
        match result {
            Ok(checked) => {
                run.findings
                    .extend(checked.items.iter().map(|item| sarif::Finding {
                        rule: item.rule.clone(),
                        level: match item.severity {
                            Severity::Error | Severity::EvaluationError => Level::Error,
                            Severity::Warning => Level::Warning,
                            _ => Level::Note,
                        },
                        message: item.message.clone(),
                        file: index,
                        line: item.line,
                        entity: item.entity.map(|_| item.path.clone()),
                        is_attribute: item.attribute_index.is_some(),
                    }));
            }
            Err(error) => run.failures.push((index, error.to_string())),
        }
    }
    out.write_all(run.to_json().pretty().as_bytes())
}
