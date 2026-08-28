use std::env;
use std::fs;
use std::path::PathBuf;

use ifc_schema::artifact_encode_schema as encode_schema;

fn main() {
    if let Err(error) = run() {
        eprintln!("ifc-schema-generate: {error}");
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
        .unwrap_or_else(|| manifest_dir.join("data/ifc4-add2-tc1.bin"));
    if arguments.next().is_some() {
        return Err(usage());
    }

    let bytes = fs::read(&source).map_err(|error| format!("read {}: {error}", source.display()))?;
    let text: String = bytes.iter().map(|&b| b as char).collect();
    let parsed = openbim_step::express::parse(&text);

    let entity_count = parsed.entities.len();
    let type_count = parsed.types.len();
    if entity_count != 776 {
        return Err(format!(
            "IFC4 ADD2 TC1 declares 776 entities, got {entity_count} -- wrong source file?"
        ));
    }
    if type_count != 397 {
        return Err(format!(
            "IFC4 ADD2 TC1 declares 397 types, got {type_count} -- wrong source file?"
        ));
    }

    let encoded = encode_schema(&parsed).map_err(|error| format!("encode artifact: {error}"))?;
    let decoded = ifc_schema::artifact_decode_schema(&encoded)
        .map_err(|error| format!("verify artifact: {error}"))?;
    if decoded != parsed {
        return Err("decoded artifact does not match the parsed schema".into());
    }

    let temporary = output.with_extension("bin.tmp");
    fs::write(&temporary, &encoded)
        .map_err(|error| format!("write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, &output)
        .map_err(|error| format!("replace {}: {error}", output.display()))?;
    println!(
        "wrote {} bytes, {entity_count} entities, {type_count} types to {}",
        encoded.len(),
        output.display()
    );
    Ok(())
}

fn usage() -> String {
    "usage: ifc-schema-generate <IFC4.exp path> [output.bin]".into()
}
