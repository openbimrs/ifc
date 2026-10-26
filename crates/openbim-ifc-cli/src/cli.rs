//! The argument grammar.
//!
//! Every command takes its input as a path; a file whose content is not
//! STEP or ifcXML is refused before anything is read into a model. The
//! output format of a report is `--format`; `convert` takes its output
//! format from the output file's extension unless `--to` names it.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Validate, convert, inspect and query IFC files (STEP, ifcXML and ifcZIP).
///
/// Exit codes: 0 success, 1 findings (validate, lint), 2 usage error,
/// unreadable input or a refusal, 3 an edit refused (edit, import-table;
/// nothing written).
#[derive(Debug, Parser)]
#[command(
    name = "openbim-ifc",
    version,
    about,
    long_about = None,
    after_help = "Exit codes: 0 success, 1 findings (validate, lint), 2 usage error, \
                  unreadable input or a refusal, 3 an edit refused (edit, import-table; \
                  nothing written).\nGuide: https://openbimrs.github.io/ifc/guide/cli"
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
    /// Convert between STEP, ifcXML (native or XSD layout) and ifcZIP.
    Convert(ConvertArgs),
    /// Show the header, the declared schema and entity counts.
    Info(InfoArgs),
    /// Show the property and quantity sets of one entity or type object.
    Psets(PsetsArgs),
    /// Show the spatial structure: project, site, building, storey, space.
    Tree(TreeArgs),
    /// Report products no model viewer will draw (exit 1 on findings).
    Lint(LintArgs),
    /// List the entities a selector selects, with values read by path.
    Select(SelectArgs),
    /// Convert the model's length unit, rescaling every length, area and
    /// volume, and write it as STEP.
    ConvertUnits(ConvertUnitsArgs),
    /// Set values by path on the entities a selector selects, as one
    /// checked transaction, into a new file (exit 3 on a refusal).
    Edit(EditArgs),
    /// Write the entities a selector selects as a table: one column per
    /// value path, keyed by GlobalId or id (CSV, TSV or XLSX).
    ExportTable(ExportTableArgs),
    /// Write a table's edited cells back by key, as one checked
    /// transaction, into a new file (exit 3 on a refusal).
    ImportTable(ImportTableArgs),
    /// Show what one entity is related to: container, parent, type,
    /// openings, materials, classifications, groups, and every
    /// relationship naming it.
    Relations(RelationsArgs),
    /// Show the material definition of each selected entity, with its
    /// layers, profiles or constituents.
    Materials(MaterialsArgs),
    /// Show every cost schedule with its tree of cost items, values and
    /// rolled-up totals.
    Cost(CostArgs),
    /// Show every work schedule and work plan with its tree of tasks and
    /// their scheduled times.
    Schedule(CostArgs),
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
    /// Also evaluate every WHERE, type and global rule of the release from
    /// its EXPRESS schema text (`IFC4.exp` and the like), which you supply:
    /// the schemas are CC BY-ND and not bundled. Repeat it for several
    /// releases, one file each; a file is checked with the first one that
    /// declares its release, and refused with `express-mismatch` when none
    /// does.
    #[arg(long = "express", value_name = "FILE.exp")]
    pub(crate) express: Vec<PathBuf>,
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
    /// ifcZIP (`.ifczip`): one STEP or ifcXML document (`--inner`) in a
    /// ZIP archive.
    Ifczip,
}

/// The document an ifcZIP archive holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum InnerFormat {
    /// ISO 10303-21, as `<name>.ifc`.
    Step,
    /// ISO 10303-28, as `<name>.ifcXML`, in `--layout`.
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
    /// The file to read (STEP, ifcXML or ifcZIP).
    pub(crate) input: PathBuf,
    /// The file to write, or `-` for standard output (needs `--to`).
    pub(crate) output: PathBuf,
    /// The output format; by default from the output extension (`.ifc`,
    /// `.step`, `.stp`, `.ifcxml`, `.xml` or `.ifczip`).
    #[arg(long, value_enum)]
    pub(crate) to: Option<ModelFormat>,
    /// The document an ifcZIP output holds [default: step].
    #[arg(long, value_enum)]
    pub(crate) inner: Option<InnerFormat>,
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

/// `select`.
#[derive(Debug, Args)]
pub(crate) struct SelectArgs {
    /// The file to read.
    pub(crate) file: PathBuf,
    /// The selector, e.g. `IfcWall, Pset_WallCommon.IsExternal=TRUE +
    /// IfcSlab` (grammar 1.0; see the guide's selector page).
    pub(crate) query: String,
    /// A value path to show for each entity, e.g. `type.Name` or
    /// `Pset_WallCommon.FireRating`; repeat for several columns.
    #[arg(long = "value", value_name = "PATH")]
    pub(crate) values: Vec<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = TableFormat::Table)]
    pub(crate) format: TableFormat,
    #[command(flatten)]
    pub(crate) input: InputArgs,
}

/// `convert-units`.
#[derive(Debug, Args)]
pub(crate) struct ConvertUnitsArgs {
    /// The file to read.
    pub(crate) input: PathBuf,
    /// The STEP file to write, or `-` for standard output.
    pub(crate) output: PathBuf,
    /// The new length unit: `METRE`, `MILLIMETRE`, `CENTIMETRE` (any SI
    /// prefix), `inch`, `foot`, `yard`, `mile` or `US survey foot`.
    #[arg(long, value_name = "UNIT")]
    pub(crate) unit: String,
    /// Keep the area and volume units (and their values) instead of
    /// replacing them by the square and cube of the new length unit.
    #[arg(long)]
    pub(crate) keep_area_volume: bool,
    /// Keep replaced unit entities nothing else uses.
    #[arg(long)]
    pub(crate) keep_replaced_units: bool,
    /// Report what would change and write nothing.
    #[arg(long)]
    pub(crate) dry_run: bool,
    /// Report format, on standard error (standard output for a dry run).
    #[arg(long, value_enum, default_value_t = TextFormat::Human)]
    pub(crate) format: TextFormat,
    /// Replace an existing output file.
    #[arg(long)]
    pub(crate) force: bool,
    #[command(flatten)]
    pub(crate) input_options: InputArgs,
}

/// Where an edited model is written.
#[derive(Debug, Clone, Args)]
pub(crate) struct OutputArgs {
    /// The file to write (`.ifc`, `.ifcxml` or `.ifczip`); the input is
    /// never changed unless `--in-place` asks for it.
    #[arg(short, long, value_name = "FILE", conflicts_with = "in_place")]
    pub(crate) output: Option<PathBuf>,
    /// Replace the input file itself, in its own format.
    #[arg(long)]
    pub(crate) in_place: bool,
    /// Plan and report the changes; write nothing.
    #[arg(long)]
    pub(crate) dry_run: bool,
    /// Replace the output file if it exists.
    #[arg(long)]
    pub(crate) force: bool,
}

/// `edit`.
#[derive(Debug, Args)]
pub(crate) struct EditArgs {
    /// The file to read.
    pub(crate) file: PathBuf,
    /// The selector choosing the entities to edit, e.g. `IfcWall,
    /// type=WT-200` (grammar 1.0).
    pub(crate) query: String,
    /// A value path and the value to write there, e.g. `--set Name "Wall
    /// A"` or `--set Pset_WallCommon.FireRating F90`; repeat for several.
    /// The value is read as the target's type; `IFCLABEL(F90)` names one.
    #[arg(long = "set", num_args = 2, value_names = ["PATH", "VALUE"], required = true)]
    pub(crate) set: Vec<String>,
    /// The value text that writes `$` (NULL).
    #[arg(long, value_name = "TEXT", default_value = "$")]
    pub(crate) null: String,
    /// Report format.
    #[arg(long, value_enum, default_value_t = TextFormat::Human)]
    pub(crate) format: TextFormat,
    #[command(flatten)]
    pub(crate) output: OutputArgs,
    #[command(flatten)]
    pub(crate) input: InputArgs,
}

/// Table file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum SheetFormatArg {
    /// RFC 4180 CSV, UTF-8.
    Csv,
    /// Tab-separated values, UTF-8, with `\t`, `\n` and `\\` escapes.
    Tsv,
    /// An Office Open XML workbook; every cell is text.
    Xlsx,
}

/// The column rows are matched by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum KeyArg {
    /// `GlobalId`, stable across tools.
    GlobalId,
    /// `id`, the entity's `#id` in this file.
    Id,
}

/// `export-table`.
#[derive(Debug, Args)]
pub(crate) struct ExportTableArgs {
    /// The file to read.
    pub(crate) file: PathBuf,
    /// The selector choosing the rows, e.g. `IfcWall + IfcSlab`.
    pub(crate) query: String,
    /// A value path per column, e.g. `Name`, `type.Name`,
    /// `Pset_WallCommon.FireRating`, `container.Name`; repeat for several
    /// [default: class, Name].
    #[arg(long = "column", short = 'c', value_name = "PATH")]
    pub(crate) columns: Vec<String>,
    /// The table to write, or `-` for standard output.
    #[arg(short, long, value_name = "FILE", default_value = "-")]
    pub(crate) output: PathBuf,
    /// The table format; by default from the output extension (`.csv`,
    /// `.tsv`, `.xlsx`), else CSV.
    #[arg(long, value_enum)]
    pub(crate) to: Option<SheetFormatArg>,
    /// The key column, first in the table.
    #[arg(long, value_enum, default_value_t = KeyArg::GlobalId)]
    pub(crate) key: KeyArg,
    /// The cell text of `$` (NULL).
    #[arg(long, value_name = "TEXT", default_value = "")]
    pub(crate) null: String,
    /// Replace the output file if it exists.
    #[arg(long)]
    pub(crate) force: bool,
    #[command(flatten)]
    pub(crate) input: InputArgs,
}

/// `import-table`.
#[derive(Debug, Args)]
pub(crate) struct ImportTableArgs {
    /// The model to read.
    pub(crate) file: PathBuf,
    /// The table to apply (CSV, TSV or XLSX).
    pub(crate) table: PathBuf,
    /// The table format; by default from its extension.
    #[arg(long, value_enum)]
    pub(crate) from: Option<SheetFormatArg>,
    /// The key column [default: GlobalId when the table has one, else id].
    #[arg(long, value_enum)]
    pub(crate) key: Option<KeyArg>,
    /// The cell text of `$` (NULL).
    #[arg(long, value_name = "TEXT", default_value = "")]
    pub(crate) null: String,
    /// Report format.
    #[arg(long, value_enum, default_value_t = TextFormat::Human)]
    pub(crate) format: TextFormat,
    #[command(flatten)]
    pub(crate) output: OutputArgs,
    #[command(flatten)]
    pub(crate) input: InputArgs,
}

/// `relations`.
#[derive(Debug, Args)]
pub(crate) struct RelationsArgs {
    /// The file to read.
    pub(crate) file: PathBuf,
    /// The entity: `#12`, `12`, or its 22-character `GlobalId`.
    pub(crate) entity: String,
    /// List the chain of parents up to the project instead.
    #[arg(long)]
    pub(crate) traverse_up: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = TextFormat::Human)]
    pub(crate) format: TextFormat,
    #[command(flatten)]
    pub(crate) input: InputArgs,
}

/// `materials`.
#[derive(Debug, Args)]
pub(crate) struct MaterialsArgs {
    /// The file to read.
    pub(crate) file: PathBuf,
    /// The selector choosing the entities.
    #[arg(default_value = "IfcElement + IfcElementType")]
    pub(crate) query: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = TextFormat::Human)]
    pub(crate) format: TextFormat,
    #[command(flatten)]
    pub(crate) input: InputArgs,
}

/// `cost` and `schedule`.
#[derive(Debug, Args)]
pub(crate) struct CostArgs {
    /// The file to read.
    pub(crate) file: PathBuf,
    /// Expand at most this many levels of nested items or sub-tasks.
    #[arg(long, value_name = "N")]
    pub(crate) depth: Option<usize>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = TextFormat::Human)]
    pub(crate) format: TextFormat,
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
