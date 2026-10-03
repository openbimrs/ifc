//! Ad hoc timing sample for the first catalog lookup per edition, the
//! corrected profile and exact-name lookups; not an automated benchmark.
//!
//! The first lookup of an edition decodes it from the embedded container
//! (`data/catalog.bin`). Run in release on a quiet machine:
//!
//! ```text
//! cargo run --release -p ifc-template-catalog --features runtime --example catalog_metrics
//! ```

use std::hint::black_box;
use std::time::Instant;

use ifc_template_catalog::embedded::{corrected_catalog, official_catalog};
use ifc_template_catalog::snapshot::{decode_edition, encode, EDITIONS};

fn main() {
    for edition in EDITIONS {
        let started = Instant::now();
        let official = official_catalog(edition).unwrap();
        black_box(official.get(black_box("Pset_WallCommon")));
        let first = started.elapsed();
        let file = encode(&[&official]).unwrap();
        let started = Instant::now();
        black_box(decode_edition(&file, edition).unwrap());
        let from_file = started.elapsed();
        println!(
            "{edition:?}: first_lookup_from_container={first:?} decode_from_file={from_file:?}"
        );
    }

    let official = official_catalog(EDITIONS[1]).unwrap();
    let started = Instant::now();
    for _ in 0..100_000 {
        black_box(official.get(black_box("Qto_WallBaseQuantities")));
    }
    println!("exact_name_100k={:?}", started.elapsed());

    let started = Instant::now();
    black_box(corrected_catalog(EDITIONS[1]).unwrap());
    println!("corrected_first_load={:?}", started.elapsed());
}
