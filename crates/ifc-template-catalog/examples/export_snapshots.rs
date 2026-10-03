//! Write each edition's per-edition snapshot file from the embedded
//! container, checking every file against its pinned SHA-256 first.
//!
//! ```text
//! cargo run -p ifc-template-catalog --features runtime --example export_snapshots -- <dir>
//! ```
//!
//! The npm package ships these files as `catalog/<edition>.bin` (#318); any
//! host that installs catalogs at runtime (`runtime::install`) can use them.

use std::fs;
use std::path::PathBuf;

use ifc_template_catalog::embedded::official_catalog;
use ifc_template_catalog::runtime::decode_verified;
use ifc_template_catalog::snapshot::{encode, file_name, EDITIONS};

fn main() {
    let Some(directory) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: export_snapshots <output directory>");
        std::process::exit(2);
    };
    fs::create_dir_all(&directory).expect("create the output directory");
    for edition in EDITIONS {
        let catalog = official_catalog(edition).expect("decode the embedded edition");
        let bytes = encode(&[&catalog]).expect("encode the edition");
        // The pin check: a file that would be refused at runtime is never
        // written.
        decode_verified(edition, &bytes).expect("the encoded file matches its pin");
        let path = directory.join(file_name(edition));
        fs::write(&path, &bytes).expect("write the snapshot");
        println!("{} {} bytes", path.display(), bytes.len());
    }
}
