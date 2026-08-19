use std::hint::black_box;
use std::time::Instant;

use ifc_template_catalog::definition::CatalogEdition;
use ifc_template_catalog::embedded::{corrected_catalog, official_catalog};

fn main() {
    let started = Instant::now();
    let official = official_catalog(CatalogEdition::Ifc4Add2Tc1).unwrap();
    let first_load = started.elapsed();

    let started = Instant::now();
    for _ in 0..100_000 {
        black_box(official.get(black_box("Qto_WallBaseQuantities")));
    }
    let lookups = started.elapsed();

    let started = Instant::now();
    black_box(corrected_catalog(CatalogEdition::Ifc4Add2Tc1).unwrap());
    let corrected = started.elapsed();

    println!("official_first_load={first_load:?}");
    println!("exact_name_100k={lookups:?}");
    println!("corrected_first_load={corrected:?}");
}
