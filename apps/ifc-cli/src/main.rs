//! `ifc` — the reference consumer of this library.
//!
//! The CLI exists for three reasons beyond being useful:
//!
//! 1. **It proves the library is usable.** An API that is awkward to drive from
//!    a binary is awkward to drive from an application.
//! 2. **It is where backend selection legitimately happens.** Libraries take a
//!    kernel by injection; an application must choose one. This is that place.
//! 3. **It is the harness for the performance claims** in `docs/ROADMAP.md`.
//!    Wall-clock numbers come from here, not from assertions.

use geom_kernel::backend::Dispatcher;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("capabilities") => print_capabilities(),
        Some("--version") | Some("-V") => {
            println!("ifc {}", env!("CARGO_PKG_VERSION"));
        }
        _ => {
            eprintln!("usage: ifc <command>");
            eprintln!();
            eprintln!("commands:");
            eprintln!("  capabilities   show detected geometry backends");
            eprintln!("  --version      print version");
            std::process::exit(2);
        }
    }
}

/// Dump what the geometry kernel detected on this machine.
///
/// This is a real diagnostic, not a placeholder: when a performance number
/// looks wrong, the first question is which backend actually ran.
fn print_capabilities() {
    let d = Dispatcher::detect();
    println!("{:<8} {:<10} MESH_BOOLEAN", "BACKEND", "AVAILABLE");
    for c in d.capabilities() {
        // `{:<8?}` does not pad Debug output, so format to a String first.
        println!(
            "{:<8} {:<10} {}",
            format!("{:?}", c.backend),
            c.available,
            c.mesh_boolean
        );
    }
    match d.best_for_mesh_boolean() {
        Some(b) => println!("\nselected for mesh boolean: {b:?}"),
        None => println!("\nselected for mesh boolean: none (not implemented yet)"),
    }
}
