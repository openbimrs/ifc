use std::env;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;

use ifc_template_catalog::definition::CatalogEdition;
use ifc_template_catalog::embedded::official_catalog;
use ifc_template_catalog::export::write_applicability_tsv;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = env::args_os().nth(1).map(PathBuf::from).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: cargo run -p ifc-template-catalog --example export_ifc4_tsv -- <output.tsv>",
        )
    })?;
    let parent = output.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "output must have a parent directory",
        )
    })?;
    fs::create_dir_all(parent)?;
    let mut temporary = output.clone();
    temporary.set_extension("tsv.tmp");

    let catalog = official_catalog(CatalogEdition::Ifc4Add2Tc1)?;
    let file = File::create(&temporary)?;
    let mut writer = BufWriter::new(file);
    let summary = write_applicability_tsv(&catalog, &mut writer)?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    fs::rename(&temporary, &output)?;
    println!(
        "wrote {} rows from {} sets to {}",
        summary.row_count,
        summary.set_count,
        output.display()
    );
    Ok(())
}
