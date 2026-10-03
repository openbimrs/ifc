//! The equality proof for the format change of #317 (temporary).
//!
//! Decodes the format-2 (bincode) snapshots this change replaces with an
//! inline copy of their decoder, and asserts that the format-3 container
//! and the per-edition files decode to exactly the same catalogs: every
//! edition, in the official and in the corrected profile, compared as
//! values, as their full `Debug` rendering (which includes the name index,
//! the profile, the applied-patch ledger and the advisories), and by the
//! content fingerprint `tests/snapshot.rs` pins permanently.

#![cfg(feature = "embedded")]

#[path = "support/content.rs"]
mod content;

use bincode::{Decode, Encode};
use ifc_template_catalog::catalog::{Catalog, CatalogProfile};
use ifc_template_catalog::definition::{CatalogEdition, SetTemplate, SourceManifest};
use ifc_template_catalog::embedded::{corrected_catalog, official_catalog};
use ifc_template_catalog::overlay::corrected_patches;
use ifc_template_catalog::snapshot::{decode_all, decode_edition, encode, EDITIONS};

#[derive(Encode, Decode)]
struct ArchivePayload {
    manifest: SourceManifest,
    templates: Vec<SetTemplate>,
}

/// The format-2 decoder as it stood on main (c171a3a): magic, a bincode
/// `u16` version 2, one bincode payload, no trailing bytes.
fn decode_v2(bytes: &[u8]) -> Catalog {
    assert!(bytes.starts_with(b"NEHPSDQ\0"));
    let standard = bincode::config::standard();
    let (version, used): (u16, usize) = bincode::decode_from_slice(&bytes[8..], standard).unwrap();
    assert_eq!(version, 2);
    let payload = &bytes[8 + used..];
    let (archive, consumed): (ArchivePayload, usize) =
        bincode::decode_from_slice(payload, standard).unwrap();
    assert_eq!(consumed, payload.len());
    Catalog::try_new(
        archive.manifest,
        CatalogProfile::Official,
        archive.templates,
    )
    .unwrap()
}

fn legacy(edition: CatalogEdition) -> Catalog {
    decode_v2(match edition {
        CatalogEdition::Ifc2x3Tc1 => include_bytes!("legacy/ifc2x3-tc1.bin"),
        CatalogEdition::Ifc4Add2Tc1 => include_bytes!("legacy/ifc4-add2-tc1.bin"),
        CatalogEdition::Ifc4x3Add2 => include_bytes!("legacy/ifc4x3-add2.bin"),
        _ => unreachable!(),
    })
}

fn assert_same(old: &Catalog, new: &Catalog, what: &str) {
    assert_eq!(old.manifest(), new.manifest(), "{what}: manifest");
    assert_eq!(old.profile(), new.profile(), "{what}: profile");
    assert_eq!(old.len(), new.len(), "{what}: template count");
    for (a, b) in old.iter().zip(new.iter()) {
        assert_eq!(a, b, "{what}: template {}", a.name);
    }
    assert_eq!(
        old.applied_patches(),
        new.applied_patches(),
        "{what}: patches"
    );
    assert_eq!(format!("{old:?}"), format!("{new:?}"), "{what}: Debug");
    assert_eq!(
        content::content_digest(old),
        content::content_digest(new),
        "{what}: fingerprint"
    );
}

#[test]
fn format_3_decodes_exactly_what_format_2_did() {
    let container = include_bytes!("../data/catalog.bin");
    for edition in EDITIONS {
        let old = legacy(edition);
        assert_same(
            &old,
            &official_catalog(edition).unwrap(),
            &format!("{edition:?} embedded"),
        );
        assert_same(
            &old,
            &decode_edition(container, edition).unwrap(),
            &format!("{edition:?} container"),
        );
        let file = encode(&[&old]).unwrap();
        assert_same(
            &old,
            &decode_edition(&file, edition).unwrap(),
            &format!("{edition:?} file"),
        );
        println!("{edition:?} official {}", content::content_digest(&old));

        if edition != CatalogEdition::Ifc2x3Tc1 {
            let old = old
                .with_patches(CatalogProfile::Corrected, &corrected_patches(edition))
                .unwrap();
            assert_same(
                &old,
                &corrected_catalog(edition).unwrap(),
                &format!("{edition:?} corrected"),
            );
            println!("{edition:?} corrected {}", content::content_digest(&old));
        }
    }
    // The container is exactly what encoding the format-2 content writes.
    let old: Vec<Catalog> = EDITIONS.iter().map(|e| legacy(*e)).collect();
    let refs: Vec<&Catalog> = old.iter().collect();
    assert_eq!(encode(&refs).unwrap(), container.as_slice());
    assert_eq!(decode_all(container).unwrap().len(), 3);
}
