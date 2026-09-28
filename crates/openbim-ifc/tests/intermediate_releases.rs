//! IFC4X1 and IFC4X2 files resolve to their own releases (#33).
//!
//! A file's `FILE_SCHEMA` token must resolve to its own `SchemaVersion`,
//! round-trip through `release_id`, and select its own bundled table -- one
//! that is neither IFC4's nor IFC4X3's.
#![cfg(all(feature = "step", feature = "schema"))]

use ifc::schema::for_version;
use ifc::{Codec, SchemaVersion, StepCodec};

fn file(token: &str, data: &str) -> Vec<u8> {
    format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{token}'));\nENDSEC;\n\
         DATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
    )
    .into_bytes()
}

#[test]
fn intermediate_release_files_resolve_to_their_own_tables() {
    let cases = [
        (
            "IFC4X1",
            SchemaVersion::Ifc4x1,
            "IFC4X1_FINAL",
            "#1=IFCALIGNMENT('0uU9Lk0Sf0Lw5dDy9o$Mc4',$,'A',$,$,$,$,$,$);",
            "IFCALIGNMENT",
        ),
        (
            "IFC4X2",
            SchemaVersion::Ifc4x2,
            "IFC4X2_FINAL",
            "#1=IFCBRIDGE('0uU9Lk0Sf0Lw5dDy9o$Mc5',$,'B',$,$,$,$,$,$,$);",
            "IFCBRIDGE",
        ),
    ];
    for (token, version, release_id, data, entity) in cases {
        let model = StepCodec.read_bytes(&file(token, data)).expect("parses");
        let declared = model.header().schema_token().expect("FILE_SCHEMA");
        assert_eq!(SchemaVersion::from_header_token(declared), Some(version));
        assert_eq!(version.release_id(), release_id);
        assert_eq!(
            SchemaVersion::ALL
                .into_iter()
                .find(|candidate| candidate.release_id() == release_id),
            Some(version),
            "release_id round-trips"
        );

        let schema = for_version(version).expect("bundled in the default build");
        assert_eq!(schema.name(), token);
        assert_eq!(schema.version(), Some(version));
        for neighbour in [SchemaVersion::Ifc4, SchemaVersion::Ifc4x3] {
            let other = for_version(neighbour).expect("bundled");
            assert_ne!(schema.entity_count(), other.entity_count());
        }
        let (_, record) = model.iter().next().expect("one record");
        assert!(schema.is_a(&record.type_name, "IfcRoot"), "{token}");
        assert_eq!(record.type_name.as_ref(), entity);
        assert_eq!(
            record.attributes.len(),
            schema.attribute_names(entity).len(),
            "the record matches its release's own slot count"
        );
    }
}
