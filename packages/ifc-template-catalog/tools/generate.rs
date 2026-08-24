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
    let source = arguments.next().map(PathBuf::from).ok_or_else(usage)?;
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| corpus::default_output(&manifest_dir));
    if arguments.next().is_some() {
        return Err(usage());
    }

    let imported = corpus::import(&source)?;
    let digest = imported.manifest.sha256.clone();
    let bytes = encode_catalog(imported.manifest, imported.templates)
        .map_err(|error| format!("encode artifact: {error}"))?;
    let decoded = decode_catalog(&bytes).map_err(|error| format!("verify artifact: {error}"))?;
    if decoded.len() != 513 {
        return Err(format!("decoded artifact has {} templates", decoded.len()));
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
    "usage: ifc-template-catalog-generate <IFC4 HTML directory> [output.bin]".into()
}
