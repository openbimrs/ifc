//! `info`: the header, the declared release and what the file holds.
//!
//! Needs no schema tables: a file declaring a release this build does not
//! bundle still reports its header and counts, and says that the release
//! is not bundled rather than failing.

use std::io::Write;

use ifc::{Header, SchemaVersion};

use crate::cli::{InfoArgs, TextFormat};
use crate::error::{CliError, CliResult, Outcome};
use crate::input::{self, Source};
use crate::json::Json;

/// How many entity types the default listing shows.
const TOP_TYPES: usize = 20;

/// Run `info`.
pub(crate) fn run(args: &InfoArgs, out: &mut impl Write) -> CliResult<Outcome> {
    let loaded = input::read(&args.file, args.input.input_layout)?;
    let model = &loaded.model;
    let header = model.header();
    let token = header.schema_token().unwrap_or_default().to_owned();
    let release = SchemaVersion::from_header_token(&token);
    let bundled = input::declared_schema(model).is_ok();
    let histogram = model.type_histogram();
    let shown = if args.all_types {
        histogram.len()
    } else {
        histogram.len().min(TOP_TYPES)
    };
    let format = match loaded.source {
        Source::Step => "STEP",
        Source::IfcXml(ifc::XmlLayout::Xsd) => "ifcXML (XSD layout)",
        Source::IfcXml(_) => "ifcXML (native layout)",
    };
    let diagnostics: Vec<String> = model
        .diagnostics()
        .iter()
        .map(ToString::to_string)
        .collect();

    let written = match args.format {
        TextFormat::Human => {
            let mut text = String::new();
            let mut line = |label: &str, value: &str| {
                if !value.is_empty() {
                    text.push_str(&format!("{label:<22}{value}\n"));
                }
            };
            line("File", &loaded.path);
            line("Format", format);
            line(
                "Schema",
                &match (release, bundled) {
                    (Some(release), true) => format!("{token} ({}, bundled)", release.release_id()),
                    (Some(release), false) => {
                        format!(
                            "{token} ({}, not bundled in this build)",
                            release.release_id()
                        )
                    }
                    (None, _) if token.is_empty() => "(none declared)".to_owned(),
                    (None, _) => format!("{token} (unrecognised)"),
                },
            );
            header_lines(header, &mut line);
            line("Entities", &model.len().to_string());
            line("Entity types", &histogram.len().to_string());
            line(
                "Read diagnostics",
                &if diagnostics.is_empty() {
                    "none".to_owned()
                } else {
                    diagnostics.len().to_string()
                },
            );
            if !histogram.is_empty() {
                text.push('\n');
                let width = histogram[..shown]
                    .iter()
                    .map(|(_, count)| count.to_string().len())
                    .max()
                    .unwrap_or(1);
                for (name, count) in &histogram[..shown] {
                    text.push_str(&format!("  {count:>width$}  {name}\n"));
                }
                if shown < histogram.len() {
                    text.push_str(&format!(
                        "  ... {} more types (--all-types lists them)\n",
                        histogram.len() - shown
                    ));
                }
            }
            out.write_all(text.as_bytes())
        }
        TextFormat::Json => {
            let document = Json::object([
                ("file", Json::str(loaded.path.clone())),
                ("format", Json::str(format)),
                (
                    "schema",
                    Json::object([
                        ("token", Json::opt_str(header.schema_token())),
                        ("release", Json::opt_str(release.map(|r| r.release_id()))),
                        ("bundled", Json::Bool(bundled)),
                    ]),
                ),
                ("header", header_json(header)),
                ("entities", Json::uint(model.len() as u64)),
                ("entity_types", Json::uint(histogram.len() as u64)),
                (
                    "types",
                    Json::Array(
                        histogram[..shown]
                            .iter()
                            .map(|(name, count)| {
                                Json::object([
                                    ("type", Json::str(*name)),
                                    ("count", Json::uint(*count as u64)),
                                ])
                            })
                            .collect(),
                    ),
                ),
                (
                    "diagnostics",
                    Json::Array(diagnostics.into_iter().map(Json::Str).collect()),
                ),
            ]);
            out.write_all(document.pretty().as_bytes())
        }
    };
    written.map_err(|error| CliError::stdout(&error))?;
    Ok(Outcome::Clean)
}

fn header_lines(header: &Header, line: &mut impl FnMut(&str, &str)) {
    line("Description", &header.description.join("; "));
    line("Implementation level", &header.implementation_level);
    line("Name", &header.name);
    line("Time stamp", &header.time_stamp);
    line("Author", &header.author.join("; "));
    line("Organization", &header.organization.join("; "));
    line("Preprocessor", &header.preprocessor_version);
    line("Originating system", &header.originating_system);
    line("Authorization", &header.authorization);
}

fn header_json(header: &Header) -> Json {
    let texts = |values: &[String]| Json::Array(values.iter().cloned().map(Json::Str).collect());
    Json::object([
        ("description", texts(&header.description)),
        (
            "implementation_level",
            Json::str(header.implementation_level.clone()),
        ),
        ("name", Json::str(header.name.clone())),
        ("time_stamp", Json::str(header.time_stamp.clone())),
        ("author", texts(&header.author)),
        ("organization", texts(&header.organization)),
        (
            "preprocessor_version",
            Json::str(header.preprocessor_version.clone()),
        ),
        (
            "originating_system",
            Json::str(header.originating_system.clone()),
        ),
        ("authorization", Json::str(header.authorization.clone())),
        ("schema", texts(&header.schema)),
    ])
}
