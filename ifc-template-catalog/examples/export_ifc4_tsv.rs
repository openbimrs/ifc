use std::env;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;

use ifc_template_catalog::definition::CatalogEdition;
use ifc_template_catalog::embedded::official_catalog;
use ifc_template_catalog::export::write_applicability_tsv;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = env::args_os().skip(1);
    let edition = match arguments.next().as_deref() {
        Some(value) if value == "ifc2x3-tc1" => CatalogEdition::Ifc2x3Tc1,
        Some(value) if value == "ifc4-add2-tc1" => CatalogEdition::Ifc4Add2Tc1,
        Some(value) if value == "ifc4x3-add2" => CatalogEdition::Ifc4x3Add2,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "usage: cargo run -p ifc-template-catalog --example export_ifc4_tsv -- <ifc2x3-tc1|ifc4-add2-tc1|ifc4x3-add2> <output.tsv>",
            )
            .into())
        }
    };
    let output = arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing TSV output path"))?;
    if arguments.next().is_some() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "too many arguments").into());
    }
    let parent = output.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "output must have a parent directory",
        )
    })?;
    fs::create_dir_all(parent)?;
    let mut temporary = output.clone();
    temporary.set_extension("tsv.tmp");

    let catalog = official_catalog(edition)?;
    let file = File::create(&temporary)?;
    let mut writer = BufWriter::new(file);
    let summary = write_applicability_tsv(&catalog, &mut writer)?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    fs::rename(&temporary, &output)?;
    println!(
        "wrote {} rows from {} sets for {:?} to {}",
        summary.row_count,
        summary.set_count,
        edition,
        output.display()
    );
    Ok(())
}
