//! Writing a model: the format an output path implies, the codec that
//! writes it, and the edited model written next to or over its input.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ifc::{Codec, Model, SchemaVersion, StepCodec, XmlCodec, XmlLayout};

use crate::cli::{ModelFormat, OutputArgs, OutputLayout};
use crate::error::{CliError, CliResult};
use crate::input::{self, Loaded, Source};

/// The output format an extension implies.
pub(crate) fn format_of(path: &Path) -> CliResult<ModelFormat> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if StepCodec.extensions().contains(&extension.as_str()) || extension == "step" {
        Ok(ModelFormat::Step)
    } else if XmlCodec::default()
        .extensions()
        .contains(&extension.as_str())
    {
        Ok(ModelFormat::Ifcxml)
    } else if extension == "ifczip" {
        Ok(ModelFormat::Ifczip)
    } else {
        Err(CliError::Usage(format!(
            "cannot tell the output format from {}; name it with --to step, ifcxml or ifczip",
            path.display()
        )))
    }
}

/// The ifcZIP entry for `document`, named after the output file:
/// `out.ifczip` holds `out.ifc` or `out.ifcXML`, standard output `model.*`.
pub(crate) fn entry_name(output: &Path, document: ModelFormat) -> String {
    let stem = output
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty() && *stem != "-")
        .unwrap_or("model");
    match document {
        ModelFormat::Ifcxml => format!("{stem}.ifcXML"),
        _ => format!("{stem}.ifc"),
    }
}

/// The ifcXML codec that writes `model` in `layout`.
pub(crate) fn xml_writer(model: &Model, layout: OutputLayout) -> CliResult<XmlCodec> {
    match layout {
        OutputLayout::Native => {
            // Schema-named attributes when this build bundles the declared
            // release; marked positional names, equally lossless, otherwise.
            Ok(input::declared_schema(model)
                .ok()
                .map_or_else(XmlCodec::default, |schema| {
                    XmlCodec::with_schema(Arc::new(schema.clone()))
                }))
        }
        OutputLayout::Xsd => {
            let schema = input::declared_schema(model)?;
            let token = model.header().schema_token().unwrap_or_default();
            let profile = SchemaVersion::from_header_token(token)
                .and_then(input::xsd_profile_for)
                .ok_or_else(|| {
                    CliError::Unsupported(format!(
                        "the XSD layout exists for IFC4 and IFC4X3_ADD2; this file declares {token}"
                    ))
                })?;
            Ok(XmlCodec::xsd(Arc::new(schema.clone()), profile))
        }
    }
}

/// `model` as `document` (STEP or ifcXML in `layout`), in an ifcZIP
/// archive holding `entry` when one is named.
pub(crate) fn encode(
    model: &Model,
    document: ModelFormat,
    layout: OutputLayout,
    entry: Option<&str>,
) -> CliResult<Vec<u8>> {
    let bytes = match document {
        ModelFormat::Ifcxml => xml_writer(model, layout)?.write_bytes(model),
        _ => StepCodec.write_bytes(model),
    }
    .map_err(|error| CliError::Write(error.to_string()))?;
    match entry {
        Some(entry) => ifc::ifczip::pack(entry, &bytes)
            .map_err(|error| CliError::Write(format!("ifcZIP: {error}"))),
        None => Ok(bytes),
    }
}

/// Where an edit command writes, checked before anything is read: a new
/// file (refused when it exists without `--force`), the input itself with
/// `--in-place`, or nothing for a dry run.
pub(crate) fn destination(args: &OutputArgs, input: &Path) -> CliResult<Option<PathBuf>> {
    if args.dry_run {
        return Ok(None);
    }
    if args.in_place {
        return Ok(Some(input.to_path_buf()));
    }
    let Some(output) = &args.output else {
        return Err(CliError::Usage(
            "name the file to write with --output, or pass --in-place to replace the input \
             (or --dry-run to write nothing)"
                .to_owned(),
        ));
    };
    format_of(output)?;
    if !args.force && output.exists() {
        return Err(CliError::Usage(format!(
            "{} exists; pass --force to replace it",
            output.display()
        )));
    }
    Ok(Some(output.clone()))
}

/// Write the edited `model` to `path`: in the input's own format when it
/// is the input (`--in-place`), else in the format the extension implies,
/// an ifcXML output in the input's layout when the input was ifcXML. The
/// file is written beside its target and renamed over it, so a failed
/// write leaves the target as it was.
pub(crate) fn write_edited(
    model: &Model,
    loaded: &Loaded,
    path: &Path,
    in_place: bool,
) -> CliResult<()> {
    let input_layout = match loaded.source {
        Source::IfcXml(XmlLayout::Xsd) => OutputLayout::Xsd,
        _ => OutputLayout::Native,
    };
    let (document, entry) = if in_place {
        let document = match loaded.source {
            Source::Step => ModelFormat::Step,
            Source::IfcXml(_) => ModelFormat::Ifcxml,
        };
        (document, loaded.archive_entry.clone())
    } else {
        match format_of(path)? {
            ModelFormat::Ifczip => (ModelFormat::Step, Some(entry_name(path, ModelFormat::Step))),
            other => (other, None),
        }
    };
    let bytes = encode(model, document, input_layout, entry.as_deref())?;
    let io = |error: std::io::Error| CliError::Io {
        path: path.display().to_string(),
        detail: error.to_string(),
    };
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temporary = path.with_file_name(format!(".{name}.openbim-ifc-{}", std::process::id()));
    std::fs::write(&temporary, &bytes).map_err(io)?;
    std::fs::rename(&temporary, path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        io(error)
    })
}
