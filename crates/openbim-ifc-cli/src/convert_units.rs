//! `convert-units`: the facade's `convert_length_unit` (#469) on a file.
//!
//! Every length, area and volume value is rescaled to the new project
//! length unit as one checked transaction, and the model is written as
//! STEP. Values the conversion cannot carry (a parameter value on a curve
//! whose parameter space IFC leaves undefined, polynomial coefficients, a
//! p-curve on a curved surface) refuse the whole run with each one named;
//! nothing is written. The report goes to standard error, or to standard
//! output for `--dry-run`.

use std::io::Write;

use ifc::properties::ConversionUnit;
use ifc::{Codec, LengthUnitError, LengthUnitOptions, LengthUnitReport, StepCodec};

use crate::cli::{ConvertUnitsArgs, TextFormat};
use crate::error::{CliError, CliResult, Outcome};
use crate::input;
use crate::json::Json;

/// Run `convert-units`.
pub(crate) fn run(args: &ConvertUnitsArgs, out: &mut impl Write) -> CliResult<Outcome> {
    let to_stdout = args.output.as_os_str() == "-";
    if !args.dry_run && !to_stdout && !args.force && args.output.exists() {
        return Err(CliError::Usage(format!(
            "{} exists; pass --force to replace it",
            args.output.display()
        )));
    }
    let target = ConversionUnit::parse(&args.unit)
        .map_err(|error| CliError::Usage(format!("--unit: {error}")))?;
    let options = LengthUnitOptions::new()
        .area_and_volume(!args.keep_area_volume)
        .remove_replaced_units(!args.keep_replaced_units);
    let mut loaded = input::read(&args.input, args.input_options.input_layout)?;
    let report = if args.dry_run {
        ifc::stage_length_unit_conversion(&loaded.model, &target, &options)
            .map_err(refusal)?
            .report
    } else {
        ifc::convert_length_unit(&mut loaded.model, &target, &options).map_err(refusal)?
    };
    let text = match args.format {
        TextFormat::Human => human(&report),
        TextFormat::Json => json(&report).pretty() + "\n",
    };
    if args.dry_run {
        out.write_all(text.as_bytes())
            .map_err(|error| CliError::stdout(&error))?;
        return Ok(Outcome::Clean);
    }
    eprint!("{text}");
    let bytes = StepCodec
        .write_bytes(&loaded.model)
        .map_err(|error| CliError::Write(error.to_string()))?;
    if to_stdout {
        out.write_all(&bytes)
            .map_err(|error| CliError::stdout(&error))?;
    } else {
        std::fs::write(&args.output, &bytes).map_err(|error| CliError::Io {
            path: args.output.display().to_string(),
            detail: error.to_string(),
        })?;
    }
    Ok(Outcome::Clean)
}

fn refusal(error: LengthUnitError) -> CliError {
    match &error {
        LengthUnitError::Target { .. } => CliError::Usage(format!("--unit: {error}")),
        LengthUnitError::Schema(_) => CliError::UnsupportedSchema(error.to_string()),
        LengthUnitError::Refused(refusals) => CliError::Unsupported(format!(
            "{} value(s) cannot be converted:\n{}",
            refusals.len(),
            refusals
                .iter()
                .map(|refusal| {
                    let attribute = refusal
                        .attribute
                        .as_deref()
                        .map(|attribute| format!(".{attribute}"))
                        .unwrap_or_default();
                    format!(
                        "  #{} {}{attribute}: {:?}",
                        refusal.entity.0, refusal.entity_type, refusal.reason
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        )),
        _ => CliError::InvalidModel(error.to_string()),
    }
}

fn human(report: &LengthUnitReport) -> String {
    let mut text = format!(
        "length unit: {} -> {} (x {})\n",
        report.from.name,
        report.to.name,
        report.length.factor()
    );
    if let Some(area) = report.area {
        text += &format!("areas: x {}\n", area.factor());
    }
    if let Some(volume) = report.volume {
        text += &format!("volumes: x {}\n", volume.factor());
    }
    text += &format!(
        "{} values rescaled in {} entities{}\n",
        report.values_converted,
        report.entities_changed,
        if report.applied { "" } else { " (dry run)" }
    );
    for converted in &report.converted {
        text += &format!(
            "  {}.{}: {} values in {} entities ({:?})\n",
            converted.entity_type,
            converted.attribute,
            converted.values,
            converted.entities,
            converted.kind
        );
    }
    for kept in &report.kept {
        text += &format!(
            "  kept {}.{}: {} values ({:?})\n",
            kept.entity_type, kept.attribute, kept.values, kept.reason
        );
    }
    text
}

fn json(report: &LengthUnitReport) -> Json {
    Json::object([
        ("from", Json::str(report.from.name.as_ref())),
        ("to", Json::str(report.to.name.as_ref())),
        ("length_factor", Json::Real(report.length.factor())),
        (
            "area_factor",
            report
                .area
                .map_or(Json::Null, |area| Json::Real(area.factor())),
        ),
        (
            "volume_factor",
            report
                .volume
                .map_or(Json::Null, |volume| Json::Real(volume.factor())),
        ),
        (
            "values_converted",
            Json::uint(report.values_converted as u64),
        ),
        (
            "entities_changed",
            Json::uint(report.entities_changed as u64),
        ),
        (
            "converted",
            Json::Array(
                report
                    .converted
                    .iter()
                    .map(|c| {
                        Json::object([
                            ("entity_type", Json::str(c.entity_type.as_ref())),
                            ("attribute", Json::str(c.attribute.as_ref())),
                            ("kind", Json::str(format!("{:?}", c.kind))),
                            ("entities", Json::uint(c.entities as u64)),
                            ("values", Json::uint(c.values as u64)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "kept",
            Json::Array(
                report
                    .kept
                    .iter()
                    .map(|k| {
                        Json::object([
                            ("entity_type", Json::str(k.entity_type.as_ref())),
                            ("attribute", Json::str(k.attribute.as_ref())),
                            ("reason", Json::str(format!("{:?}", k.reason))),
                            ("values", Json::uint(k.values as u64)),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("applied", Json::Bool(report.applied)),
    ])
}
