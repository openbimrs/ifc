use std::env;
use std::io::{self, BufWriter, Write};

use ifc_schema::{write_structural_catalog, SchemaVersion};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let requested = env::args().nth(1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: cargo run -p ifc-schema --example export_structural_catalog -- <ifc2x3|ifc4|ifc4x3>",
        )
    })?;
    let version = match requested.to_ascii_lowercase().as_str() {
        "ifc2x3" | "ifc2x3_tc1" => SchemaVersion::Ifc2x3,
        "ifc4" | "ifc4_add2_tc1" => SchemaVersion::Ifc4,
        "ifc4x3" | "ifc4x3_add2" => SchemaVersion::Ifc4x3,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unsupported bundled IFC release: {requested}"),
            )
            .into());
        }
    };

    let stdout = io::stdout();
    let mut output = BufWriter::new(stdout.lock());
    let summary = write_structural_catalog(version, &mut output)?;
    output.flush()?;
    eprintln!(
        "exported {} entities and {} types for {}",
        summary.entity_rows,
        summary.type_count,
        version.release_id()
    );
    Ok(())
}
