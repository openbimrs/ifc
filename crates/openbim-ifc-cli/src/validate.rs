//! `validate`: every file against the schema its own header declares.
//!
//! The verdict is the validator's: a file fails on an error or an
//! evaluation error (a rule that could not be decided is not a rule that
//! passed), on a truncated report (its counts are lower bounds), and with
//! `--deny-warnings` on a warning. Rules the validator does not evaluate
//! are counted, never a failure. A file that cannot be read or declares no
//! bundled schema is reported and makes the run exit 2, after every other
//! file has been checked.

use std::io::Write;

use ifc::validate::{validate_with, Budget, Path, Report, Severity};

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
    unsupported: usize,
    items: Vec<Item>,
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
    let results: Vec<CliResult<Checked>> = args
        .files
        .iter()
        .map(|file| {
            let loaded = input::read(file, args.input.input_layout)?;
            check(&loaded, budget, args)
        })
        .collect();

    match args.format {
        ReportFormat::Human => human(&files, &results, out),
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

fn check(loaded: &Loaded, budget: Budget, args: &ValidateArgs) -> CliResult<Checked> {
    let schema = input::declared_schema(&loaded.model)?;
    let report: Report = validate_with(&loaded.model, schema, budget);
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
        writeln!(
            out,
            "{file}: {}: {verdict}: {}, {}, {}; {} not evaluated{truncated}",
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
