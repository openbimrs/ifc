//! `convert`: STEP to ifcXML and back, in either ifcXML layout.
//!
//! The native layout carries any model losslessly; with the declared
//! release's tables it names attributes after the schema. The XSD layout is
//! the buildingSMART configuration of IFC4 ADD2 TC1 and IFC4X3 ADD2: a
//! model it cannot carry exactly is refused by the codec with a typed
//! error, never written differently, and nothing is written.

use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use ifc::{Codec, Model, SchemaVersion, StepCodec, XmlCodec};

use crate::cli::{ConvertArgs, ModelFormat, OutputLayout};
use crate::error::{CliError, CliResult, Outcome};
use crate::input::{self, Source};

/// Run `convert`.
pub(crate) fn run(args: &ConvertArgs, out: &mut impl Write) -> CliResult<Outcome> {
    let to_stdout = args.output.as_os_str() == "-";
    let format = match args.to {
        Some(format) => format,
        None if to_stdout => {
            return Err(CliError::Usage(
                "writing to standard output needs --to step or --to ifcxml".to_owned(),
            ))
        }
        None => format_of(&args.output)?,
    };
    if format == ModelFormat::Step && args.layout.is_some() {
        return Err(CliError::Usage(
            "--layout applies to ifcXML output only".to_owned(),
        ));
    }
    if !to_stdout && !args.force && args.output.exists() {
        return Err(CliError::Usage(format!(
            "{} exists; pass --force to replace it",
            args.output.display()
        )));
    }

    let loaded = input::read(&args.input, args.input_options.input_layout)?;
    let layout = args.layout.unwrap_or(OutputLayout::Native);
    let bytes = match format {
        ModelFormat::Step => StepCodec.write_bytes(&loaded.model),
        ModelFormat::Ifcxml => xml_writer(&loaded.model, layout)?.write_bytes(&loaded.model),
    }
    .map_err(|error| CliError::Write(error.to_string()))?;

    let written = if to_stdout {
        out.write_all(&bytes)
            .map_err(|error| CliError::stdout(&error))?;
        "standard output".to_owned()
    } else {
        std::fs::write(&args.output, &bytes).map_err(|error| CliError::Io {
            path: args.output.display().to_string(),
            detail: error.to_string(),
        })?;
        args.output.display().to_string()
    };
    let from = match loaded.source {
        Source::Step => "STEP".to_owned(),
        Source::IfcXml(layout) => format!("ifcXML ({})", layout_name(layout)),
    };
    let to = match format {
        ModelFormat::Step => "STEP".to_owned(),
        ModelFormat::Ifcxml => format!(
            "ifcXML ({})",
            match layout {
                OutputLayout::Native => "native layout",
                OutputLayout::Xsd => "XSD layout",
            }
        ),
    };
    eprintln!(
        "openbim-ifc: {} entities: {} ({from}) -> {written} ({to})",
        loaded.model.len(),
        loaded.path
    );
    Ok(Outcome::Clean)
}

fn layout_name(layout: ifc::XmlLayout) -> &'static str {
    match layout {
        ifc::XmlLayout::Xsd => "XSD layout",
        _ => "native layout",
    }
}

/// The output format an extension implies.
fn format_of(path: &Path) -> CliResult<ModelFormat> {
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
    } else {
        Err(CliError::Usage(format!(
            "cannot tell the output format from {}; name it with --to step or --to ifcxml",
            path.display()
        )))
    }
}

/// The ifcXML codec that writes `model` in `layout`.
fn xml_writer(model: &Model, layout: OutputLayout) -> CliResult<XmlCodec> {
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
