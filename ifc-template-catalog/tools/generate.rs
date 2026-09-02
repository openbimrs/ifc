#[path = "corpus.rs"]
mod corpus;

use std::env;
use std::fs;
use std::path::PathBuf;

use ifc_template_catalog::generation::{decode_catalog, encode_catalog};

fn main() {
    if let Err(error) = run() {
        eprintln!("ifc-template-catalog-generate: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut arguments = env::args_os().skip(1);
    let edition = arguments
        .next()
        .and_then(|value| value.into_string().ok())
        .ok_or_else(usage)
        .and_then(|value| corpus::parse_edition(&value))?;
    let source = arguments.next().map(PathBuf::from).ok_or_else(usage)?;
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = match arguments.next() {
        Some(output) => PathBuf::from(output),
        None => corpus::default_output(&manifest_dir, edition)?,
    };
    if arguments.next().is_some() {
        return Err(usage());
    }

    let imported = corpus::import(edition, &source)?;
    let digest = imported.manifest.sha256.clone();
    let bytes = encode_catalog(imported.manifest, imported.templates)
        .map_err(|error| format!("encode artifact: {error}"))?;
    let decoded = decode_catalog(&bytes).map_err(|error| format!("verify artifact: {error}"))?;
    if decoded.manifest().edition != edition {
        return Err(format!(
            "decoded artifact has wrong edition: {:?}",
            decoded.manifest().edition
        ));
    }

    let temporary = output.with_extension("bin.tmp");
    fs::write(&temporary, &bytes)
        .map_err(|error| format!("write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, &output)
        .map_err(|error| format!("replace {}: {error}", output.display()))?;
    println!(
        "wrote {} bytes, {} templates, sha256 {} to {}",
        bytes.len(),
        decoded.len(),
        digest,
        output.display()
    );
    Ok(())
}

fn usage() -> String {
    "usage: ifc-template-catalog-generate <ifc2x3-tc1|ifc4-add2-tc1|ifc4x3-add2> <source directory> [output.bin]".into()
}
