//! An object's type, a type's occurrences, openings and fillings, and
//! entities by `GlobalId` (#463), over one model in IFC2X3, IFC4 and IFC4X3.
//!
//! `test/fixtures/synthetic-relations/relations_*.ifc`, identical in the
//! three releases but for each release's own arities and its door type
//! (`IfcDoorStyle` in IFC2X3, `IfcDoorType` after):
//!
//! ```text
//! #16 wall type   types #17 Wall A, #18 Wall B; #19 Wall C is untyped
//! #27 door type   types #28 Door 1
//! #17 Wall A      voided by #21 (door opening), #23 (recess)
//! #18 Wall B      voided by #22 (window opening)
//! #21             filled by #28 Door 1
//! #22             filled by #31 Window 1
//! #36             gives Wall A a property set: in IFC2X3 it shares the
//!                 `IsDefinedBy` inverse with the type relationship
//! ```

#![cfg(all(
    feature = "step",
    feature = "ifc2x3",
    feature = "ifc4",
    feature = "ifc4x3"
))]

use ifc::{
    by_guid, fillings_of, occurrences_of, openings_of, root_identity, type_of, voided_element_of,
    Codec, EntityId, GlobalIdIndex, Guid, Model, Schema, SchemaVersion, StepCodec,
};

const FIXTURES: [(&str, SchemaVersion); 3] = [
    ("relations_ifc2x3.ifc", SchemaVersion::Ifc2x3),
    ("relations_ifc4.ifc", SchemaVersion::Ifc4),
    ("relations_ifc4x3.ifc", SchemaVersion::Ifc4x3),
];

fn read(name: &str) -> Model {
    let path = format!(
        "{}/../../test/fixtures/synthetic-relations/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    StepCodec
        .read_path(std::path::Path::new(&path))
        .unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn each_release(mut check: impl FnMut(&str, &Model, &Schema)) {
    for (name, version) in FIXTURES {
        let model = read(name);
        let token = model.header().schema_token().expect("a FILE_SCHEMA");
        assert_eq!(SchemaVersion::from_header_token(token), Some(version));
        let schema = ifc::schema::for_version(version).expect("bundled");
        check(name, &model, schema);
    }
}

fn ids(raw: &[u64]) -> Vec<EntityId> {
    raw.iter().copied().map(EntityId).collect()
}

#[test]
fn an_occurrence_finds_its_type_and_a_type_its_occurrences() {
    each_release(|name, model, schema| {
        assert_eq!(
            type_of(model, schema, EntityId(17)),
            Ok(Some(EntityId(16))),
            "{name}"
        );
        assert_eq!(
            type_of(model, schema, EntityId(18)),
            Ok(Some(EntityId(16))),
            "{name}"
        );
        assert_eq!(type_of(model, schema, EntityId(19)), Ok(None), "{name}");
        assert_eq!(
            type_of(model, schema, EntityId(28)),
            Ok(Some(EntityId(27))),
            "{name}"
        );
        assert_eq!(
            occurrences_of(model, schema, EntityId(16)),
            Ok(ids(&[17, 18])),
            "{name}"
        );
        assert_eq!(
            occurrences_of(model, schema, EntityId(27)),
            Ok(ids(&[28])),
            "{name}"
        );
    });
}

#[test]
fn an_element_finds_its_openings_and_an_opening_its_host_and_fillings() {
    each_release(|name, model, schema| {
        assert_eq!(
            openings_of(model, schema, EntityId(17)),
            Ok(ids(&[21, 23])),
            "{name}"
        );
        assert_eq!(
            openings_of(model, schema, EntityId(18)),
            Ok(ids(&[22])),
            "{name}"
        );
        assert_eq!(
            openings_of(model, schema, EntityId(19)),
            Ok(vec![]),
            "{name}"
        );
        assert_eq!(
            voided_element_of(model, schema, EntityId(21)),
            Ok(Some(EntityId(17))),
            "{name}"
        );
        assert_eq!(
            voided_element_of(model, schema, EntityId(22)),
            Ok(Some(EntityId(18))),
            "{name}"
        );
        assert_eq!(
            fillings_of(model, schema, EntityId(21)),
            Ok(ids(&[28])),
            "{name}"
        );
        assert_eq!(
            fillings_of(model, schema, EntityId(22)),
            Ok(ids(&[31])),
            "{name}"
        );
        assert_eq!(
            fillings_of(model, schema, EntityId(23)),
            Ok(vec![]),
            "{name}"
        );
    });
}

/// Every `IfcRoot` is found by its own `GlobalId`, through the index and
/// the one-shot scan, and nothing else is indexed.
#[test]
fn every_root_is_found_by_its_global_id() {
    each_release(|name, model, schema| {
        let index = GlobalIdIndex::build(model, schema);
        let mut roots = 0;
        for id in model.ids() {
            let Some(identity) = root_identity(model, schema, id) else {
                continue;
            };
            roots += 1;
            let guid = Guid::parse(identity.global_id.expect("set")).expect("valid");
            assert_eq!(index.get(&guid), Ok(Some(id)), "{name} #{}", id.0);
            assert_eq!(
                by_guid(model, schema, &guid),
                Ok(Some(id)),
                "{name} #{}",
                id.0
            );
        }
        assert_eq!(roots, 25, "{name}");
        assert_eq!(index.len(), roots, "{name}");
        assert_eq!(index.duplicates().count(), 0, "{name}");
        assert!(index.invalid().is_empty(), "{name}");
    });
}

/// A copied element keeps its source's `GlobalId`, the most common way a
/// file breaks uniqueness: both are reported.
#[test]
fn a_copied_global_id_is_reported_with_both_holders() {
    each_release(|name, model, schema| {
        let mut model = model.clone();
        let wall_a = model.get(EntityId(17)).expect("Wall A").clone();
        let copy = model.push(wall_a);
        let guid = Guid::parse(
            root_identity(&model, schema, EntityId(17))
                .and_then(|identity| identity.global_id)
                .expect("Wall A's GlobalId"),
        )
        .expect("valid");
        let index = GlobalIdIndex::build(&model, schema);
        let error = index.get(&guid).unwrap_err();
        assert_eq!(error.entities, [EntityId(17), copy], "{name}");
        assert_eq!(by_guid(&model, schema, &guid), Err(error), "{name}");
    });
}
