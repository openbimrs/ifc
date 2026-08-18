//! Print a summary of every fixture: entity count and top types.
//! Run with: cargo run -p ifc-step --example corpus_summary

use ifc_model::Codec;
use ifc_step::StepCodec;
use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../test/fixtures");
    let mut files = Vec::new();
    fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "ifc") {
                out.push(p);
            }
        }
    }
    walk(&dir, &mut files);
    files.sort();

    let codec = StepCodec;
    let mut total = 0usize;
    for path in &files {
        match codec.read_path(path) {
            Ok(model) => {
                total += model.len();
                let top: Vec<String> = model
                    .type_histogram()
                    .into_iter()
                    .take(3)
                    .map(|(t, n)| format!("{t}x{n}"))
                    .collect();
                println!(
                    "{:>6} entities  {:<12} {:<52} {}",
                    model.len(),
                    model.header().schema_token().unwrap_or("?"),
                    path.file_name().unwrap().to_string_lossy(),
                    top.join(" ")
                );
            }
            Err(e) => println!("  FAILED  {}: {e}", path.display()),
        }
    }
    println!("\n{} files, {total} entities total", files.len());
}
