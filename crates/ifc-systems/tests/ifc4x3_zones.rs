//! #194: `zones().unwrap()` reads an IFC4X3 model against the IFC4X3 ADD2 table.
//!
//! Before, every header other than IFC2X3 and IFC4 was read against IFC4.
//! The zone readers now bind IFC4X3 and read every slot by attribute name.
//! The IFC4X3 differences they meet, verified against `IFC4X3_ADD2.exp`:
//!
//! - `IfcRelAssigns.RelatedObjectsType` is `IfcStrippedOptional` (was
//!   `IfcObjectTypeEnum`) at the same position, so `RelatingGroup` stays
//!   the seventh attribute;
//! - `IfcSystem` gains `IfcBuiltSystem`, which is no WR1 zone member;
//! - `IfcZone` itself (with `LongName` and WR1 over `IfcZone`, `IfcSpace`,
//!   `IfcSpatialZone`) is unchanged from IFC4.

use ifc_model::{Codec, EntityId, Model};
use ifc_schema::for_version;
use ifc_systems::{long_name_of, zones, SchemaResolutionError, SchemaVersion, SystemAnomaly};

/// A STEP record for `entity` in IFC4X3: `head` then `$` up to its arity.
fn record(id: u64, entity: &str, head: &[&str]) -> String {
    let arity = for_version(SchemaVersion::Ifc4x3)
        .unwrap()
        .attributes(entity)
        .len();
    let mut values: Vec<&str> = head.to_vec();
    values.resize(arity, "$");
    format!("#{id}={entity}({});", values.join(","))
}

fn step(schema: &str) -> Model {
    let records = [
        record(
            12,
            "IFCZONE",
            &["'z'", "$", "'Zone A'", "$", "$", "'Long A'"],
        ),
        record(13, "IFCZONE", &["'y'", "$", "'Zone B'"]),
        record(20, "IFCSPACE", &["'r'", "$", "'R1'"]),
        record(21, "IFCSPATIALZONE", &["'s'", "$", "'S1'"]),
        record(22, "IFCPUMP", &["'p'", "$", "'P1'"]),
        record(23, "IFCBUILTSYSTEM", &["'b'", "$", "'B1'"]),
        // RelatedObjectsType is IfcStrippedOptional in IFC4X3: always `$`.
        record(
            30,
            "IFCRELASSIGNSTOGROUP",
            &["'a'", "$", "$", "$", "(#20,#21,#13,#22,#23)", "$", "#12"],
        ),
    ];
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('t','2026-09-28T00:00:00',(''),(''),'','','');\n\
         FILE_SCHEMA(({schema}));\nENDSEC;\nDATA;\n{}\nENDSEC;\nEND-ISO-10303-21;\n",
        records.join("\n")
    );
    let model = ifc_step::StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

fn not_spatial(member: u64, type_name: &str) -> SystemAnomaly {
    SystemAnomaly::ZoneMemberNotSpatial {
        relation: EntityId(30),
        zone: EntityId(12),
        member: EntityId(member),
        type_name: type_name.into(),
    }
}

#[test]
fn zones_read_an_ifc4x3_model() {
    let model = step("'IFC4X3_ADD2'");
    let (found, anomalies) = zones(&model).unwrap();
    assert_eq!(found.len(), 2);
    let zone = &found[0];
    assert_eq!(zone.id, EntityId(12));
    assert_eq!(zone.name.as_deref(), Some("Zone A"));
    assert_eq!(zone.long_name.as_deref(), Some("Long A"));
    // A space, a spatial zone and a zone are WR1 members in IFC4X3.
    assert_eq!(zone.members, [EntityId(13), EntityId(20), EntityId(21)]);
    assert_eq!(found[1].long_name, None);
    // A pump, and the IFC4X3-only IfcBuiltSystem, are not.
    assert_eq!(
        anomalies,
        [
            not_spatial(22, "IFCPUMP"),
            not_spatial(23, "IFCBUILTSYSTEM")
        ]
    );

    assert_eq!(zones(&model), Ok((found, anomalies)));
    assert_eq!(
        long_name_of(&model, EntityId(12)),
        Ok(Some("Long A".to_owned()))
    );
    assert_eq!(long_name_of(&model, EntityId(13)), Ok(None));
}

#[test]
fn zones_refuses_unbound_or_unverified_schemas() {
    assert_eq!(
        zones(&step("'IFC4','IFC4X3'")),
        Err(SchemaResolutionError::MultipleSchemas { schemas: 2 })
    );
    for token in ["IFC4X1", "IFC4X2"] {
        assert_eq!(
            zones(&step(&format!("'{token}'"))),
            Err(SchemaResolutionError::UnsupportedSchema {
                schema: token.to_owned()
            })
        );
    }
    // A header with no schema (an in-memory model) binds nothing and is
    // refused, never read as IFC4.
    assert_eq!(
        zones(&Model::new()),
        Err(SchemaResolutionError::MissingSchema)
    );
}

/// Every slot the zone readers use, by name in each release.
#[test]
fn zone_slots_resolve_by_name_in_each_release() {
    for version in [
        SchemaVersion::Ifc2x3,
        SchemaVersion::Ifc4,
        SchemaVersion::Ifc4x3,
    ] {
        let schema = for_version(version).unwrap();
        let names = schema.attribute_names("IFCRELASSIGNSTOGROUP");
        assert_eq!(names[4], "RelatedObjects", "{version:?}");
        assert_eq!(names[5], "RelatedObjectsType", "{version:?}");
        assert_eq!(names[6], "RelatingGroup", "{version:?}");
        let zone = schema.attribute_names("IFCZONE");
        assert_eq!(zone[2], "Name", "{version:?}");
        let long_name = zone.iter().position(|name| *name == "LongName");
        let related_type = &schema.attributes("IFCRELASSIGNSTOGROUP")[5].type_name;
        match version {
            SchemaVersion::Ifc2x3 => {
                assert_eq!(long_name, None);
                assert!(!schema.is_a("IFCZONE", "IFCSYSTEM"));
                assert!(schema.entity("IFCSPATIALZONE").is_none());
                assert_eq!(related_type, "IfcObjectTypeEnum");
            }
            SchemaVersion::Ifc4 => {
                assert_eq!(long_name, Some(5));
                assert!(schema.is_a("IFCZONE", "IFCSYSTEM"));
                assert_eq!(related_type, "IfcObjectTypeEnum");
            }
            _ => {
                assert_eq!(long_name, Some(5));
                assert!(schema.is_a("IFCZONE", "IFCSYSTEM"));
                assert!(schema.is_a("IFCBUILTSYSTEM", "IFCSYSTEM"));
                assert!(!schema.is_a("IFCBUILTSYSTEM", "IFCZONE"));
                assert_eq!(related_type, "IfcStrippedOptional");
            }
        }
    }
}

/// The bundled zone layouts and ancestry equal the normative EXPRESS.
/// Skips when `references/ifc-spec` is absent, unless `IFC_SPEC_REQUIRED`
/// is set (as `scripts/gate.sh` does).
#[test]
fn bundled_zone_layouts_match_the_reference_express() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let Some(spec) = [
        "../../../../references/ifc-spec",
        "../../references/ifc-spec",
    ]
    .into_iter()
    .map(|rel| root.join(rel))
    .find(|path| path.is_dir()) else {
        assert!(
            std::env::var_os("IFC_SPEC_REQUIRED").is_none(),
            "IFC_SPEC_REQUIRED is set but references/ifc-spec was not found"
        );
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };
    for (version, path) in [
        (SchemaVersion::Ifc2x3, "ifc2x3-tc1/IFC2X3_TC1.exp"),
        (SchemaVersion::Ifc4, "ifc4-add2-tc1/IFC4.exp"),
        (SchemaVersion::Ifc4x3, "ifc4x3-add2/IFC4X3_ADD2.exp"),
    ] {
        let bytes = std::fs::read(spec.join(path)).expect("read the reference schema");
        let reference = ifc_schema::Schema::from_express_bytes(&bytes);
        let bundled = for_version(version).unwrap();
        for entity in [
            "IFCZONE",
            "IFCRELASSIGNSTOGROUP",
            "IFCSPACE",
            "IFCSPATIALZONE",
        ] {
            let layout = |schema: &ifc_schema::Schema| -> Vec<(String, String)> {
                schema
                    .attributes(entity)
                    .iter()
                    .map(|a| (a.name.clone(), a.type_name.clone()))
                    .collect()
            };
            assert_eq!(layout(bundled), layout(&reference), "{version:?} {entity}");
            assert_eq!(
                bundled.supertypes(entity),
                reference.supertypes(entity),
                "{version:?} {entity} ancestry"
            );
        }
    }
}
