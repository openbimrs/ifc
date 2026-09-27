//! Probe: load the real damaged AC20-FZK-Haus.ifc through the IFC codec.

use ifc_model::Codec;
use ifc_step::StepCodec;

fn main() {
    let path = std::env::args().nth(1).expect("path");

    match StepCodec.read_path(std::path::Path::new(&path)) {
        Ok(_) => println!("strict: OK (unexpected)"),
        Err(error) => println!("strict: {error}"),
    }

    let model = StepCodec::lenient()
        .read_path(std::path::Path::new(&path))
        .expect("lenient read");
    println!(
        "lenient: {} entities, complete={}, {} diagnostics",
        model.len(),
        model.is_complete(),
        model.diagnostics().len()
    );
    for diagnostic in model.diagnostics() {
        println!("  {diagnostic}");
    }
    println!(
        "IfcRelSpaceBoundary: {}",
        model.ids_of_type("IfcRelSpaceBoundary").len()
    );
    println!("schema: {:?}", model.header().schema);
    println!("dangling refs: {}", model.dangling_references().len());
}
