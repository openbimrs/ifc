use ifc_schema::{write_structural_catalog, SchemaVersion};

fn export(version: SchemaVersion) -> String {
    let mut bytes = Vec::new();
    let summary = write_structural_catalog(version, &mut bytes).expect("catalog export");
    assert_eq!(summary.entity_rows, version.expected_entity_count());
    assert_eq!(summary.type_count, version.expected_type_count());
    String::from_utf8(bytes).expect("UTF-8 catalog")
}

#[test]
fn exports_each_bundled_schema_with_exact_release_identity_and_counts() {
    let cases = [
        (SchemaVersion::Ifc2x3, "IFC2X3_TC1", 653, 327),
        (SchemaVersion::Ifc4, "IFC4_ADD2_TC1", 776, 397),
        (SchemaVersion::Ifc4x3, "IFC4X3_ADD2", 876, 436),
    ];

    for (version, release, entities, types) in cases {
        let output = export(version);
        let mut lines = output.lines();
        assert_eq!(lines.next(), Some("# openbim.ifc structural-catalog v1"));
        assert_eq!(
            lines.next(),
            Some(format!("schema\t{release}\t{entities}\t{types}").as_str())
        );
        assert_eq!(lines.count(), entities);
    }
}

#[test]
fn entity_rows_are_sorted_and_preserve_version_specific_structure() {
    let ifc4 = export(SchemaVersion::Ifc4);
    let ifc4x3 = export(SchemaVersion::Ifc4x3);

    let names = ifc4
        .lines()
        .skip(2)
        .map(|line| {
            assert_eq!(line.split('\t').count(), 4);
            assert!(!line.ends_with('\t'));
            line.split('\t').nth(1).expect("entity name")
        })
        .collect::<Vec<_>>();
    assert!(names.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(ifc4.contains("entity\tIfcRepresentationItem\t-\t-"));

    let ifc4_wall = ifc4
        .lines()
        .find(|line| line.starts_with("entity\tIfcWall\t"))
        .expect("IFC4 IfcWall");
    assert!(ifc4_wall.contains("IfcBuildingElement"));
    assert!(!ifc4_wall.contains("IfcBuiltElement"));
    assert!(ifc4_wall.ends_with("\tGlobalId,OwnerHistory,Name,Description,ObjectType,ObjectPlacement,Representation,Tag,PredefinedType"));

    let ifc4x3_wall = ifc4x3
        .lines()
        .find(|line| line.starts_with("entity\tIfcWall\t"))
        .expect("IFC4X3 IfcWall");
    assert!(ifc4x3_wall.contains("IfcBuiltElement"));
    assert!(!ifc4x3_wall.contains("IfcBuildingElement"));
}
