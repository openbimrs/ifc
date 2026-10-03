//! CLI: import a source edition's PSD/QTO XML corpus and write it into the
//! committed snapshot container, `data/catalog.bin`.
//!
//! The container holds every edition; this replaces (or adds) the one
//! imported and re-encodes the others unchanged, so regenerating an edition
//! never touches another's data. The artifact must be reproducible on any
//! machine: never encode a local absolute path or a timestamp in it. Source
//! files are identified by their normalized relative path and content hash
//! only.

#[path = "corpus.rs"]
mod corpus;

use std::env;
use std::fs;
use std::path::PathBuf;

use ifc_template_catalog::catalog::{Catalog, CatalogProfile};
use ifc_template_catalog::snapshot::{decode_all, decode_edition, encode};

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
        None => corpus::default_output(&manifest_dir),
    };
    if arguments.next().is_some() {
        return Err(usage());
    }

    let imported = corpus::import(edition, &source)?;
    let digest = imported.manifest.sha256.clone();
    let imported = Catalog::try_new(
        imported.manifest,
        CatalogProfile::Official,
        imported.templates,
    )
    .map_err(|error| format!("imported catalog: {error}"))?;

    // Every other edition already in the container is kept as it is.
    let mut catalogs = match fs::read(&output) {
        Ok(bytes) => decode_all(&bytes)
            .map_err(|error| format!("read existing {}: {error}", output.display()))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(format!("read {}: {error}", output.display())),
    };
    catalogs.retain(|catalog| catalog.manifest().edition != edition);
    catalogs.push(imported);
    let refs: Vec<&Catalog> = catalogs.iter().collect();
    let bytes = encode(&refs).map_err(|error| format!("encode container: {error}"))?;

    let decoded =
        decode_edition(&bytes, edition).map_err(|error| format!("verify container: {error}"))?;
    let expected = catalogs
        .iter()
        .find(|catalog| catalog.manifest().edition == edition)
        .expect("the imported edition was just added");
    if decoded.manifest() != expected.manifest() || !decoded.iter().eq(expected.iter()) {
        return Err(format!("{edition:?} does not decode to what was imported"));
    }

    let temporary = output.with_extension("bin.tmp");
    fs::write(&temporary, &bytes)
        .map_err(|error| format!("write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, &output)
        .map_err(|error| format!("replace {}: {error}", output.display()))?;
    println!(
        "wrote {} bytes ({} editions); {edition:?}: {} templates, source sha256 {} to {}",
        bytes.len(),
        catalogs.len(),
        decoded.len(),
        digest,
        output.display()
    );
    Ok(())
}

fn usage() -> String {
    "usage: ifc-template-catalog-generate <ifc2x3-tc1|ifc4-add2-tc1|ifc4x3-add2> <source directory> [container.bin]".into()
}
