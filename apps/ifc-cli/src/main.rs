//! `ifc` — command line tool for inspecting IFC files.
//!
//! An application, so this is the right place to bind concrete
//! implementations: a codec and a geometry backend. Library crates must not.

use ifc_model::Codec;
use ifc_step::StepCodec;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str);

    match command {
        Some("capabilities") => capabilities(),
        Some("info") => match args.get(1) {
            Some(path) => info(PathBuf::from(path)),
            None => {
                eprintln!("usage: ifc info <file.ifc>");
                std::process::exit(2);
            }
        },
        Some("types") => match args.get(1) {
            Some(path) => types(PathBuf::from(path)),
            None => {
                eprintln!("usage: ifc types <file.ifc>");
                std::process::exit(2);
            }
        },
        Some("--version") | Some("-V") => println!("ifc {}", env!("CARGO_PKG_VERSION")),
        _ => {
            println!("usage: ifc <command>");
            println!();
            println!("  info <file>      header, entity count, dangling references");
            println!("  types <file>     entity type histogram");
            println!("  capabilities     geometry backends compiled into this build");
            println!("  --version");
        }
    }
}

/// Report the geometry backends this build can use.
fn capabilities() {
    use geom_kernel::{Backend, Operation, Precision};

    let cpu = geom_backend_cpu::CpuBackend::detect();
    let descriptor = cpu.descriptor();
    println!(
        "{:<18} {:<16} {:<10} MESH_BOOLEAN",
        "BACKEND", "TARGET", "AVAILABLE"
    );
    println!(
        "{:<18} {:<16?} {:<10} {}",
        descriptor.id,
        descriptor.target,
        descriptor.available,
        descriptor.supports(Operation::MeshBoolean, Precision::F64)
    );
    println!();
    println!("CPU features: {:?}", cpu.features());
    println!("selected for mesh boolean: none (not implemented yet)");
}

/// Summarize one file.
fn info(path: PathBuf) {
    let codec = StepCodec;
    let model = match codec.read_path(&path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    let header = model.header();
    println!("file:        {}", path.display());
    println!("schema:      {}", header.schema_token().unwrap_or("(none)"));
    println!("name:        {}", header.name);
    println!("timestamp:   {}", header.time_stamp);
    println!("application: {}", header.originating_system);
    println!("entities:    {}", model.len());

    let dangling = model.dangling_references();
    if dangling.is_empty() {
        println!("references:  all resolve");
    } else {
        println!("references:  {} DANGLING", dangling.len());
        for (from, to) in dangling.iter().take(5) {
            println!("               {from} -> {to} (missing)");
        }
    }
}

/// Print the entity type histogram.
fn types(path: PathBuf) {
    let codec = StepCodec;
    let model = match codec.read_path(&path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    for (type_name, count) in model.type_histogram().into_iter().take(30) {
        println!("{count:>7}  {type_name}");
    }
}
