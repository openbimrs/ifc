//! `lint`: products a model viewer will not draw.
//!
//! The facade's `unreachable_products` answers "is this valid but blank":
//! a product with geometry outside the spatial structure, or whose
//! geometry lives only in contexts a model viewer skips. Openings, parts,
//! containers and products without a shape are legitimately outside the
//! containment tree and are not reported. Any finding fails the run.

use std::io::Write;

use ifc::{unreachable_products, EntityId, Unreachable};

use crate::cli::{LintArgs, ReportFormat};
use crate::error::{CliError, CliResult, Outcome};
use crate::input::{self, Loaded};
use crate::json::Json;
use crate::sarif::{self, Level};

/// One finding.
struct Item {
    rule: &'static str,
    entity: u64,
    label: String,
    line: Option<usize>,
    message: String,
}

/// Run `lint`.
pub(crate) fn run(args: &LintArgs, out: &mut impl Write) -> CliResult<Outcome> {
    let files: Vec<String> = args
        .files
        .iter()
        .map(|file| file.display().to_string())
        .collect();
    let results: Vec<CliResult<Vec<Item>>> = args
        .files
        .iter()
        .map(|file| input::read(file, args.input.input_layout).map(|loaded| check(&loaded)))
        .collect();

    match args.format {
        ReportFormat::Human => human(&files, &results, out),
        ReportFormat::Json => json(&files, &results, out),
        ReportFormat::Sarif => sarif(&files, &results, out),
    }
    .map_err(|error| CliError::stdout(&error))?;

    Ok(verdict(&results))
}

fn verdict(results: &[CliResult<Vec<Item>>]) -> Outcome {
    if results.iter().any(Result::is_err) {
        Outcome::Incomplete
    } else if results.iter().flatten().any(|items| !items.is_empty()) {
        Outcome::Findings
    } else {
        Outcome::Clean
    }
}

fn check(loaded: &Loaded) -> Vec<Item> {
    let model = &loaded.model;
    // Names label the findings; a file whose release this build lacks is
    // still linted, its products labelled by type and id.
    let schema = input::declared_schema(model).ok();
    unreachable_products(model)
        .into_iter()
        .map(|(id, why)| {
            let rule = match why {
                Unreachable::NotContainedInSpatialStructure => "unreachable.not-contained",
                Unreachable::NoRepresentationInModelContext { .. } => {
                    "unreachable.no-model-context"
                }
                Unreachable::RepresentationWithoutContext => "unreachable.no-context",
                _ => "unreachable.other",
            };
            let EntityId(raw) = id;
            let type_name = model
                .get(id)
                .map(|entity| entity.type_name.to_string())
                .unwrap_or_default();
            let name = schema
                .and_then(|schema| ifc::root_identity(model, schema, id))
                .and_then(|identity| identity.name);
            let label = match name {
                Some(name) => format!("{type_name} #{raw} '{name}'"),
                None => format!("{type_name} #{raw}"),
            };
            Item {
                rule,
                entity: raw,
                label,
                line: loaded.lines.as_ref().and_then(|lines| lines.line(raw)),
                message: why.message(),
            }
        })
        .collect()
}

fn human(
    files: &[String],
    results: &[CliResult<Vec<Item>>],
    out: &mut impl Write,
) -> std::io::Result<()> {
    for (file, result) in files.iter().zip(results) {
        let items = match result {
            Ok(items) => items,
            Err(error) => {
                eprintln!("openbim-ifc: {file}: not linted: {error}");
                continue;
            }
        };
        for item in items {
            let at = match item.line {
                Some(line) => format!("{file}:{line}"),
                None => file.clone(),
            };
            writeln!(
                out,
                "{at}: warning [{}] {}: {}",
                item.rule, item.label, item.message
            )?;
        }
        writeln!(
            out,
            "{file}: {}",
            match items.len() {
                0 => "every product with geometry is reachable".to_owned(),
                1 => "1 product will not be drawn".to_owned(),
                n => format!("{n} products will not be drawn"),
            }
        )?;
    }
    Ok(())
}

fn json(
    files: &[String],
    results: &[CliResult<Vec<Item>>],
    out: &mut impl Write,
) -> std::io::Result<()> {
    let reports = files
        .iter()
        .zip(results)
        .map(|(file, result)| match result {
            Ok(items) => Json::object([
                ("file", Json::str(file.clone())),
                (
                    "findings",
                    Json::Array(
                        items
                            .iter()
                            .map(|item| {
                                Json::object([
                                    ("rule", Json::str(item.rule)),
                                    ("entity", Json::uint(item.entity)),
                                    ("label", Json::str(item.label.clone())),
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
    results: &[CliResult<Vec<Item>>],
    out: &mut impl Write,
) -> std::io::Result<()> {
    let mut run = sarif::Run {
        files: files.to_vec(),
        ..sarif::Run::default()
    };
    for (index, result) in results.iter().enumerate() {
        match result {
            Ok(items) => run.findings.extend(items.iter().map(|item| sarif::Finding {
                rule: item.rule.to_owned(),
                level: Level::Warning,
                message: format!("{}: {}", item.label, item.message),
                file: index,
                line: item.line,
                entity: Some(format!("#{}", item.entity)),
                is_attribute: false,
            })),
            Err(error) => run.failures.push((index, error.to_string())),
        }
    }
    out.write_all(run.to_json().pretty().as_bytes())
}
