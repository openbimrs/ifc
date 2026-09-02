use std::collections::BTreeMap;

use ifc_schema::{write_direct_structural_catalog, write_structural_catalog, SchemaVersion};

fn export_direct(version: SchemaVersion) -> String {
    let mut bytes = Vec::new();
    let summary =
        write_direct_structural_catalog(version, &mut bytes).expect("direct catalog export");
    assert_eq!(summary.entity_rows, version.expected_entity_count());
    String::from_utf8(bytes).expect("UTF-8 catalog")
}

fn parse_full(output: &str) -> BTreeMap<String, (Vec<String>, Vec<String>)> {
    output
        .lines()
        .skip(2)
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(fields.len(), 4);
            let sequence = |value: &str| {
                if value == "-" {
                    Vec::new()
                } else {
                    value.split(',').map(str::to_owned).collect()
                }
            };
            (
                fields[1].to_owned(),
                (sequence(fields[2]), sequence(fields[3])),
            )
        })
        .collect()
}

#[test]
fn direct_catalogs_reconstruct_every_expanded_schema_exactly() {
    for version in [
        SchemaVersion::Ifc2x3,
        SchemaVersion::Ifc4,
        SchemaVersion::Ifc4x3,
    ] {
        let direct = parse_full(&export_direct(version));
        let mut expanded_bytes = Vec::new();
        write_structural_catalog(version, &mut expanded_bytes).expect("expanded catalog");
        let expanded = parse_full(&String::from_utf8(expanded_bytes).expect("UTF-8"));

        for (name, (expected_supertypes, expected_attributes)) in &expanded {
            let mut supertypes = Vec::new();
            let mut attributes = Vec::new();
            let mut current = name.as_str();
            let mut chain = Vec::new();
            loop {
                let (parent, declared) = &direct[current];
                chain.push(declared);
                let Some(parent) = parent.first() else { break };
                supertypes.push(parent.clone());
                current = parent;
            }
            for declared in chain.into_iter().rev() {
                attributes.extend(declared.iter().cloned());
            }
            assert_eq!(&supertypes, expected_supertypes, "{name} ancestry");
            assert_eq!(&attributes, expected_attributes, "{name} attributes");
        }
    }
}

#[test]
fn direct_rows_are_four_field_sorted_and_release_specific() {
    let ifc4 = export_direct(SchemaVersion::Ifc4);
    let rows = ifc4.lines().skip(2).collect::<Vec<_>>();
    assert_eq!(rows.len(), 776);
    assert!(rows
        .iter()
        .all(|line| line.split('\t').count() == 4 && !line.ends_with('\t')));
    assert!(rows
        .windows(2)
        .all(|pair| pair[0].split('\t').nth(1) < pair[1].split('\t').nth(1)));
    assert!(ifc4.contains("entity\tIfcWall\tIfcBuildingElement\tPredefinedType"));
    assert!(export_direct(SchemaVersion::Ifc4x3)
        .contains("entity\tIfcWall\tIfcBuiltElement\tPredefinedType"));
}
