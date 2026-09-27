//! Compile an EXPRESS schema into the committed binary artifact.
//!
//! ```text
//! ifc-schema-generate ifc2x3 references/specs/ifc2x3-tc1/IFC2X3_TC1.exp
//! ifc-schema-generate ifc4   references/specs/ifc4-add2-tc1/IFC4.exp
//! ifc-schema-generate ifc4x3 references/specs/ifc4x3-add2/IFC4X3_ADD2.exp
//! ```
//!
//! The expected entity/type counts are asserted per schema. They are the
//! cheapest possible guard against pointing this at the wrong `.exp`, which
//! would otherwise produce a plausible-looking artifact describing the wrong
//! schema -- the kind of defect that surfaces months later as an
//! inexplicable slot offset.

use std::env;
use std::fs;
use std::path::PathBuf;

use ifc_schema::artifact_encode_schema as encode_schema;

/// A schema this tool can compile: selector, output file, and the counts a
/// correct source must produce.
struct Target {
    selector: &'static str,
    output: &'static str,
    entities: usize,
    types: usize,
    label: &'static str,
}

/// Counts are from the normative EXPRESS files, recomputed with
/// `grep -cE '^ENTITY ' ` and `grep -cE '^TYPE ' `. The trailing spaces
/// avoid counting expressions such as IFC4X3's line-leading `TYPEOF(...)`.
const TARGETS: &[Target] = &[
    Target {
        selector: "ifc2x3",
        output: "data/ifc2x3-tc1.bin",
        entities: 653,
        types: 327,
        label: "IFC2x3 TC1",
    },
    Target {
        selector: "ifc4",
        output: "data/ifc4-add2-tc1.bin",
        entities: 776,
        types: 397,
        label: "IFC4 ADD2 TC1",
    },
    Target {
        selector: "ifc4x3",
        output: "data/ifc4x3-add2.bin",
        entities: 876,
        types: 436,
        label: "IFC4X3 ADD2",
    },
];

fn main() {
    if let Err(error) = run() {
        eprintln!("ifc-schema-generate: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut arguments = env::args_os().skip(1);
    let selector = arguments
        .next()
        .and_then(|value| value.into_string().ok())
        .ok_or_else(usage)?;
    let source = arguments.next().map(PathBuf::from).ok_or_else(usage)?;
    if arguments.next().is_some() {
        return Err(usage());
    }

    let target = TARGETS
        .iter()
        .find(|target| target.selector.eq_ignore_ascii_case(&selector))
        .ok_or_else(|| format!("unknown schema `{selector}`\n{}", usage()))?;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = manifest_dir.join(target.output);

    let bytes = fs::read(&source).map_err(|error| format!("read {}: {error}", source.display()))?;
    // EXPRESS files are Latin-1 and may use CRLF; both are handled by mapping
    // each byte to its code point and letting the tokenizer treat `\r` as
    // whitespace.
    let text: String = bytes.iter().map(|&b| b as char).collect();
    let parsed = openbim_step::express::parse(&text);

    let entity_count = parsed.entities.len();
    let type_count = parsed.types.len();
    if entity_count != target.entities {
        return Err(format!(
            "{} declares {} entities, got {entity_count} -- wrong source file?",
            target.label, target.entities
        ));
    }
    if type_count != target.types {
        return Err(format!(
            "{} declares {} types, got {type_count} -- wrong source file?",
            target.label, target.types
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
    let mut text = String::from("usage: ifc-schema-generate <schema> <source.exp>\nschemas:\n");
    for target in TARGETS {
        text.push_str(&format!(
            "  {:<8} {} -> {}\n",
            target.selector, target.label, target.output
        ));
    }
    text
}
