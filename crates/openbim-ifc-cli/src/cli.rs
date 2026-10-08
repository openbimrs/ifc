//! The argument grammar.
//!
//! Every command takes its input as a path; a file whose content is not
//! STEP or ifcXML is refused before anything is read into a model. The
//! output format of a report is `--format`; `convert` takes its output
//! format from the output file's extension unless `--to` names it.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Validate, convert and inspect IFC files (STEP and ifcXML).
///
/// Exit codes: 0 success, 1 findings (validate, lint), 2 usage error,
/// unreadable input or a refusal.
#[derive(Debug, Parser)]
#[command(
    name = "openbim-ifc",
    version,
    about,
    long_about = None,
    after_help = "Exit codes: 0 success, 1 findings (validate, lint), 2 usage error, \
                  unreadable input or a refusal.\nGuide: https://openbimrs.github.io/ifc/guide/cli"
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

/// The commands.
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Validate against the schema each file declares (exit 1 on findings).
    Validate(ValidateArgs),
    /// Convert between STEP and ifcXML (native or XSD layout).
    Convert(ConvertArgs),
    /// Show the header, the declared schema and entity counts.
    Info(InfoArgs),
    /// Show the property and quantity sets of one entity or type object.
    Psets(PsetsArgs),
    /// Show the spatial structure: project, site, building, storey, space.
    Tree(TreeArgs),
    /// Report products no model viewer will draw (exit 1 on findings).
    Lint(LintArgs),
}

/// How the input's ifcXML layout is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum InputLayout {
    /// The native layout when the root element carries a `schema`
    /// attribute, else the XSD layout of the release whose namespace the
    /// root declares; anything else is refused.
    Auto,
    /// This library's own lossless layout.
    Native,
    /// The buildingSMART XSD configuration (IFC4 ADD2 TC1, IFC4X3 ADD2).
    Xsd,
}

/// Options shared by every command that reads one model.
#[derive(Debug, Clone, Args)]
pub(crate) struct InputArgs {
    /// The ifcXML layout of the input (STEP input ignores it).
    #[arg(long, value_enum, default_value_t = InputLayout::Auto)]
    pub(crate) input_layout: InputLayout,
}

/// Report formats of `validate` and `lint`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum ReportFormat {
    /// One line per finding, then a summary.
    Human,
    /// One JSON document.
    Json,
    /// SARIF 2.1.0, for code-scanning tools.
    Sarif,
}

/// `validate`.
#[derive(Debug, Args)]
pub(crate) struct ValidateArgs {
    /// Files to validate; each against the schema its own header declares.
    #[arg(required = true)]
    pub(crate) files: Vec<PathBuf>,
    /// Report format.
    #[arg(long, value_enum, default_value_t = ReportFormat::Human)]
    pub(crate) format: ReportFormat,
    /// Record at most this many findings per file; a truncated report
    /// fails the run, since its verdict is not final.
    #[arg(long, value_name = "N")]
    pub(crate) max_findings: Option<usize>,
    /// Fail the run on warnings too, not only on errors.
    #[arg(long)]
    pub(crate) deny_warnings: bool,
    /// Also list the rules this validator does not evaluate; they are
    /// counted in the summary either way and never fail the run.
    #[arg(long)]
    pub(crate) include_unsupported: bool,
    #[command(flatten)]
    pub(crate) input: InputArgs,
}

/// Output formats of `convert`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum ModelFormat {
    /// ISO 10303-21 (`.ifc`).
    Step,
    /// ISO 10303-28 (`.ifcxml`).
    Ifcxml,
}

/// The ifcXML layout `convert` writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum OutputLayout {
    /// This library's own lossless layout, any release.
    Native,
    /// The buildingSMART XSD configuration; IFC4 and IFC4X3_ADD2 only.
    Xsd,
}

/// `convert`.
#[derive(Debug, Args)]
pub(crate) struct ConvertArgs {
    /// The file to read (STEP or ifcXML).
    pub(crate) input: PathBuf,
    /// The file to write, or `-` for standard output (needs `--to`).
    pub(crate) output: PathBuf,
    /// The output format; by default from the output extension (`.ifc`,
    /// `.step`, `.stp` or `.ifcxml`, `.xml`).
    #[arg(long, value_enum)]
    pub(crate) to: Option<ModelFormat>,
    /// The ifcXML layout to write [default: native].
    #[arg(long, value_enum)]
    pub(crate) layout: Option<OutputLayout>,
    /// Replace the output file if it exists.
    #[arg(long)]
    pub(crate) force: bool,
    #[command(flatten)]
    pub(crate) input_options: InputArgs,
}

/// Formats of the inspection commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum TextFormat {
    /// Readable text.
    Human,
    /// One JSON document.
    Json,
}

/// `info`.
#[derive(Debug, Args)]
pub(crate) struct InfoArgs {
    /// The file to read.
    pub(crate) file: PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = TextFormat::Human)]
    pub(crate) format: TextFormat,
    /// List the count of every entity type, not only the 20 most frequent.
    #[arg(long)]
    pub(crate) all_types: bool,
    #[command(flatten)]
    pub(crate) input: InputArgs,
}

/// Formats of `psets`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum TableFormat {
    /// An aligned text table.
    Table,
    /// One JSON document.
    Json,
    /// RFC 4180 CSV with a header row.
    Csv,
}

/// `psets`.
#[derive(Debug, Args)]
pub(crate) struct PsetsArgs {
    /// The file to read.
    pub(crate) file: PathBuf,
    /// The entity: `#12`, `12`, or its 22-character `GlobalId`.
    pub(crate) entity: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = TableFormat::Table)]
    pub(crate) format: TableFormat,
    #[command(flatten)]
    pub(crate) input: InputArgs,
}

/// `tree`.
#[derive(Debug, Args)]
pub(crate) struct TreeArgs {
    /// The file to read.
    pub(crate) file: PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = TextFormat::Human)]
    pub(crate) format: TextFormat,
    /// List each container's elements, not only their count.
    #[arg(long)]
    pub(crate) elements: bool,
    #[command(flatten)]
    pub(crate) input: InputArgs,
}

/// `lint`.
#[derive(Debug, Args)]
pub(crate) struct LintArgs {
    /// Files to check.
    #[arg(required = true)]
    pub(crate) files: Vec<PathBuf>,
    /// Report format.
    #[arg(long, value_enum, default_value_t = ReportFormat::Human)]
    pub(crate) format: ReportFormat,
    #[command(flatten)]
    pub(crate) input: InputArgs,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    #[test]
    fn the_grammar_is_consistent() {
        super::Cli::command().debug_assert();
    }
}
