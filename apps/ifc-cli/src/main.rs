//! `ifc` — command line tool for inspecting IFC files.
//!
//! An application, so this is the right place to bind concrete
//! implementations: a codec and a geometry backend. Library crates must not.

mod mesh;

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
        Some("mesh") => match args.get(1) {
            Some(path) => mesh_command(PathBuf::from(path), args.iter().any(|a| a == "-v")),
            None => {
                eprintln!("usage: ifc mesh <file.ifc> [-v]");
                std::process::exit(2);
            }
        },
        Some("differential") => {
            let paths: Vec<PathBuf> = args[1..].iter().map(PathBuf::from).collect();
            if paths.is_empty() {
                eprintln!("usage: ifc differential <file.ifc>...");
                std::process::exit(2);
            }
            differential_command(paths);
        }
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
            println!("  mesh <file> [-v] compile geometry to triangle meshes");
            println!("  capabilities     geometry backends compiled into this build");
            println!("  --version");
        }
    }
}

/// Report execution contexts and implemented geometry providers in this build.
fn capabilities() {
    let cpu = axiolid_backend_cpu::CpuExecution::detect();
    println!("CPU execution context");
    println!("  instruction set: {:?}", cpu.instruction_set());
    println!("  worker bound: {}", cpu.thread_count());
    println!("  detected features: {:?}", cpu.features());
    println!("operation providers");
    println!(
        "  {:<24} {:?}",
        axiolid_compile::BACKEND_ID.as_str(),
        axiolid_kernel::Operation::GraphCompilation
    );
    println!(
        "  {:<24} {:?}",
        axiolid_boolmesh::BoolmeshBoolean::ID.as_str(),
        axiolid_kernel::Operation::MeshBoolean
    );
    println!(
        "  {:<24} {:?}",
        axiolid_compile::BACKEND_ID.as_str(),
        axiolid_kernel::Operation::ProfileTriangulation
    );
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

/// Compile a file's geometry and report what was produced.
fn mesh_command(path: PathBuf, verbose: bool) {
    let model = match StepCodec.read_path(&path) {
        Ok(model) => model,
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    };
    println!("file:        {}", path.display());
    let summary = mesh::compile_model(&model, verbose);
    println!("meshed:      {} items", summary.meshed);
    println!("triangles:   {}", summary.triangles);
    println!("not lowered: {} items", summary.not_lowered);
    println!("not compiled:{} items", summary.not_compiled);

    let products = mesh::compile_products(&model);
    if !products.is_empty() {
        println!("products with openings:");
        for product in &products {
            println!(
                "  {} {:<22} {:>6} tris  {} void(s) applied",
                product.id,
                product.type_name,
                product.mesh.triangle_count(),
                product.voids_applied
            );
        }
    }
    if summary.meshed == 0 {
        // Nothing produced is a failure for a mesh command: a caller
        // scripting this needs a non-zero status to react to.
        std::process::exit(1);
    }
}

/// Emit one JSON record per compiled product, matching the reference schema.
fn differential_command(paths: Vec<PathBuf>) {
    for path in paths {
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let Ok(model) = StepCodec.read_path(&path) else {
            println!("{{\"file\":\"{name}\",\"error\":\"open\"}}");
            continue;
        };
        let start = std::time::Instant::now();
        let products = mesh::compile_products(&model);
        // One timing for the whole file, divided per product: compile_products
        // shares a lowering session and a boolean provider across products, so
        // per-product timing would misattribute that shared setup.
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        let per = if products.is_empty() {
            0.0
        } else {
            ms / products.len() as f64
        };
        for product in products {
            let volume = mesh::signed_volume(&product.mesh);
            let manifold = mesh::edge_manifold(&product.mesh);
            println!(
                "{{\"file\":\"{}\",\"id\":{},\"type\":\"{}\",\"ms\":{},\"triangles\":{},\"volume\":{},\"manifold\":{},\"voids\":{}}}",
                name,
                product.id.0,
                product.type_name,
                per,
                product.mesh.triangle_count(),
                volume,
                manifold,
                product.voids_applied
            );
        }
    }
}
