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

/// Write direct IFC entity declarations without repeated inherited structure.
///
/// Rows have four fields: `entity`, canonical name, immediate supertype, and
/// directly declared Part 21 attribute names. `-` marks an absent parent or
/// empty declaration list. Consumers reconstruct release-specific ancestry and
/// inherited slots by following immediate parents in the same catalog.
pub fn write_direct_structural_catalog(
    version: SchemaVersion,
    mut output: impl Write,
) -> io::Result<StructuralCatalogSummary> {
    let schema = for_version(version).expect("every SchemaVersion has a bundled schema");
    let entity_count = schema.entity_count();
    let type_count = schema.type_count();
    assert_eq!(entity_count, version.expected_entity_count());
    assert_eq!(type_count, version.expected_type_count());

    writeln!(output, "# openbim.ifc direct-structural-catalog v1")?;
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
        let parent = supertypes.first().copied();
        let inherited_count = if let Some(parent) = parent {
            let parent_supertypes = schema.supertypes(parent);
            assert_eq!(supertypes[1..], parent_supertypes);
            let parent_attributes = schema.attribute_names(parent);
            assert!(
                attributes.starts_with(&parent_attributes),
                "{name} attributes must begin with all inherited {parent} attributes"
            );
            parent_attributes.len()
        } else {
            0
        };
        let declared = &attributes[inherited_count..];
        for value in parent.iter().copied().chain(declared.iter().copied()) {
            assert_catalog_token(value);
        }
        writeln!(
            output,
            "entity\t{}\t{}\t{}",
            name,
            parent.unwrap_or("-"),
            joined_or_dash(declared)
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
