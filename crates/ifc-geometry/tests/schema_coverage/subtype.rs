//! The compiled subtype tables, per release, against the normative EXPRESS.
//!
//! `select::subtype` carries IFC4 chains and an IFC4X3 delta (#293). Every
//! chain it answers for a verified release must equal that release's `.exp`
//! chain, whole and in order, and the release-neutral `is_a` must answer
//! IFC4 for every entity IFC4 declares. The bundled-table twin of this check
//! is the `select::subtype` unit test, which runs without `references/`.

use ifc_geometry::select::{known_entities_in, supertypes_of, supertypes_of_in};
use ifc_schema::{Schema, SchemaVersion};

use super::ifc4x3::spec_root;
use crate::IFC4X3_ADDITIONS;

/// The normative schemas, or `None` when `references/ifc-spec` is absent.
fn normative() -> Option<(Schema, Schema)> {
    let Some(root) = spec_root() else {
        assert!(
            std::env::var_os("IFC_SPEC_REQUIRED").is_none(),
            "IFC_SPEC_REQUIRED is set but references/ifc-spec was not found"
        );
        eprintln!("skipped: references/ifc-spec not present");
        return None;
    };
    let read = |rel: &str| {
        let bytes = std::fs::read(root.join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"));
        Schema::from_express_bytes(&bytes)
    };
    Some((
        read("ifc4-add2-tc1/IFC4.exp"),
        read("ifc4x3-add2/IFC4X3_ADD2.exp"),
    ))
}

fn chain(schema: &Schema, entity: &str) -> Vec<String> {
    schema
        .supertypes(entity)
        .iter()
        .map(|s| s.to_ascii_uppercase())
        .collect()
}

/// Every compiled chain equals `IFC4.exp` and `IFC4X3_ADD2.exp`, per release.
///
/// Comparing whole chains, not spot `is_a` facts, is what catches a chain
/// that one release re-parents: IFC4X3 inserts `IfcOffsetCurve` and
/// `IfcDirectrixCurveSweptAreaSolid` above four IFC4 entities, so a single
/// merged chain fails one release or the other here.
#[test]
fn the_compiled_subtype_tables_match_the_normative_express_per_release() {
    let Some((ifc4, ifc4x3)) = normative() else {
        return;
    };
    for (version, schema, file, expected_rows) in [
        (SchemaVersion::Ifc4, &ifc4, "IFC4.exp", 104),
        (SchemaVersion::Ifc4x3, &ifc4x3, "IFC4X3_ADD2.exp", 128),
    ] {
        let mut wrong = Vec::new();
        let mut rows = 0;
        for entity in known_entities_in(version) {
            rows += 1;
            if schema.entity(entity).is_none() {
                wrong.push(format!("  {entity}: not declared"));
                continue;
            }
            let expected = chain(schema, entity);
            let compiled = supertypes_of_in(version, entity);
            if compiled != expected {
                wrong.push(format!("  {entity}: {compiled:?} != {expected:?}"));
            }
        }
        assert!(
            wrong.is_empty(),
            "{} compiled chains disagree with {file}:\n{}",
            wrong.len(),
            wrong.join("\n")
        );
        // Pinned so a selector that silently matches less cannot pass.
        assert_eq!(rows, expected_rows, "{version:?} rows checked");
    }
}

/// The IFC4X3 table carries every concrete geometry entity IFC4X3 adds.
#[test]
fn the_ifc4x3_table_carries_every_ifc4x3_addition() {
    let known: Vec<&str> = known_entities_in(SchemaVersion::Ifc4x3).collect();
    let missing: Vec<&&str> = IFC4X3_ADDITIONS
        .iter()
        .filter(|e| !known.iter().any(|k| k.eq_ignore_ascii_case(e)))
        .collect();
    assert!(
        missing.is_empty(),
        "IFC4X3 additions without a compiled chain: {missing:?}"
    );
    assert_eq!(IFC4X3_ADDITIONS.len(), 19);
}

/// The release-neutral chain is IFC4's where IFC4 declares the entity and
/// IFC4X3's where only IFC4X3 does.
#[test]
fn the_release_neutral_chain_is_the_earliest_declaring_release() {
    let Some((ifc4, ifc4x3)) = normative() else {
        return;
    };
    for entity in known_entities_in(SchemaVersion::Ifc4x3) {
        let (schema, file) = if ifc4.entity(entity).is_some() {
            (&ifc4, "IFC4.exp")
        } else {
            (&ifc4x3, "IFC4X3_ADD2.exp")
        };
        assert_eq!(
            supertypes_of(entity),
            chain(schema, entity),
            "{entity}: release-neutral chain must be {file}'s"
        );
    }
}
