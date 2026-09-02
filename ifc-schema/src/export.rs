//! Deterministic, schema-derived structural catalog export.
//!
//! The export contains only release identity, entity names, inheritance, and
//! ordered attribute-slot names. It deliberately excludes normative prose and
//! never substitutes one IFC release for another.

use std::io::{self, Write};

use crate::{for_version, SchemaVersion};

/// Counts written to one structural catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructuralCatalogSummary {
    /// Number of entity rows written.
    pub entity_rows: usize,
    /// Number of defined types declared by the source schema.
    pub type_count: usize,
}

/// Write a deterministic tab-separated structural catalog for a bundled IFC release.
///
/// Rows have four fields: `entity`, canonical entity name, nearest-first
/// comma-separated supertypes, and inherited-first comma-separated Part 21
/// attribute names. A `-` is the empty-sequence marker; IFC declaration names
/// cannot contain the delimiters used by this format.
pub fn write_structural_catalog(
    version: SchemaVersion,
    mut output: impl Write,
) -> io::Result<StructuralCatalogSummary> {
    let schema = for_version(version).expect("every SchemaVersion has a bundled schema");
    let entity_count = schema.entity_count();
    let type_count = schema.type_count();

    assert_eq!(entity_count, version.expected_entity_count());
    assert_eq!(type_count, version.expected_type_count());

    writeln!(output, "# openbim.ifc structural-catalog v1")?;
    writeln!(
        output,
        "schema\t{}\t{}\t{}",
        version.release_id(),
        entity_count,
        type_count
    )?;

    let mut names = schema.entity_names().collect::<Vec<_>>();
    names.sort_unstable();
    for name in &names {
        assert_catalog_token(name);
        let supertypes = schema.supertypes(name);
        let attributes = schema.attribute_names(name);
        for value in supertypes.iter().chain(attributes.iter()) {
            assert_catalog_token(value);
        }
        writeln!(
            output,
            "entity\t{}\t{}\t{}",
            name,
            joined_or_dash(&supertypes),
            joined_or_dash(&attributes)
        )?;
    }

    Ok(StructuralCatalogSummary {
        entity_rows: names.len(),
        type_count,
    })
}

fn joined_or_dash(values: &[&str]) -> String {
    if values.is_empty() {
        "-".to_owned()
    } else {
        values.join(",")
    }
}

fn assert_catalog_token(value: &str) {
    assert!(
        !value.contains(['\t', '\n', '\r', ',']),
        "IFC declaration token contains a structural-catalog delimiter: {value:?}"
    );
}
