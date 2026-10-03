//! The committed snapshot container and the per-edition files (#317).
//!
//! The anchor is independent of the encoding: each edition and profile
//! decodes to content whose fingerprint (`support/content.rs`) was taken
//! from the format-2 snapshots this format replaced, after a test decoded
//! both and found them equal field for field. The TSV indexes
//! (`tests/export.rs`) and the source counts and digests
//! (`tests/embedded.rs`) anchor the same content from other sides.

#![cfg(feature = "embedded")]

#[path = "support/content.rs"]
mod content;

use ifc_template_catalog::catalog::Catalog;
use ifc_template_catalog::definition::CatalogEdition;
use ifc_template_catalog::embedded::{corrected_catalog, official_catalog};
use ifc_template_catalog::snapshot::{
    decode_all, decode_edition, editions, encode, file_name, pinned_sha256, CONTAINER_SHA256,
    EDITIONS,
};
use sha2::{Digest, Sha256};

const CONTAINER: &[u8] = include_bytes!("../data/catalog.bin");

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn decoded_content_matches_the_format_2_fingerprints() {
    for (edition, official, corrected) in [
        (
            CatalogEdition::Ifc2x3Tc1,
            "5466e7cafb1ac29575235dde2887ef5c938174c6971140a161baa1d186250af1",
            None,
        ),
        (
            CatalogEdition::Ifc4Add2Tc1,
            "60308fa72b5db82a8e40ac76e47a2e9c4277b073deecab9b26876c4f485fea69",
            Some("a7defbaf56b488e5ceceee5ee4106fb33efe9d4111c56aff6cbb63e7e32eeb69"),
        ),
        (
            CatalogEdition::Ifc4x3Add2,
            "ecdef98d4b42c0e4ab1eefdcc0500ac7ca1702573719ca929013b85fad631a42",
            Some("c13b9e726ffbe50b8e9ae195d000c99f3cfc9f67bfd3a3133cdd37951651575f"),
        ),
    ] {
        let catalog = official_catalog(edition).unwrap();
        assert_eq!(content::content_digest(&catalog), official, "{edition:?}");
        if let Some(corrected) = corrected {
            let catalog = corrected_catalog(edition).unwrap();
            assert_eq!(content::content_digest(&catalog), corrected, "{edition:?}");
        }
    }
}

#[test]
fn the_container_and_each_file_match_their_pins() {
    assert_eq!(sha256(CONTAINER), CONTAINER_SHA256);
    assert_eq!(editions(CONTAINER).unwrap(), EDITIONS);
    for edition in EDITIONS {
        let catalog = official_catalog(edition).unwrap();
        let file = encode(&[&catalog]).unwrap();
        assert_eq!(sha256(&file), pinned_sha256(edition), "{edition:?}");
        assert!(file_name(edition).ends_with(".bin"));
        // A file decodes to the container's edition, template for template.
        let from_file = decode_edition(&file, edition).unwrap();
        assert_eq!(from_file.manifest(), catalog.manifest());
        assert!(from_file.iter().eq(catalog.iter()));
    }
}

#[test]
fn encoding_is_canonical() {
    // Decoding and re-encoding reproduces the committed bytes exactly, in
    // whatever order the editions are given.
    let mut catalogs = decode_all(CONTAINER).unwrap();
    catalogs.reverse();
    let refs: Vec<&Catalog> = catalogs.iter().collect();
    assert_eq!(encode(&refs).unwrap(), CONTAINER);
}

#[test]
fn encoding_refuses_a_derived_profile_and_a_repeated_edition() {
    let official = official_catalog(CatalogEdition::Ifc4Add2Tc1).unwrap();
    let corrected = corrected_catalog(CatalogEdition::Ifc4Add2Tc1).unwrap();
    assert!(encode(&[&corrected]).is_err());
    assert!(encode(&[&official, &official]).is_err());
}

#[test]
fn shared_content_is_stored_once() {
    // The container is smaller than its editions' files together: text and
    // templates two editions publish identically are stored once.
    let separate: usize = EDITIONS
        .iter()
        .map(|edition| {
            encode(&[&official_catalog(*edition).unwrap()])
                .unwrap()
                .len()
        })
        .sum();
    assert!(
        CONTAINER.len() * 10 < separate * 7,
        "{} vs {separate}",
        CONTAINER.len()
    );
}
