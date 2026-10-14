//! `INVERSE` attributes by name (#462) over one model in IFC2X3, IFC4 and
//! IFC4X3, and the bundled `INVERSE` declarations against the normative
//! EXPRESS.
//!
//! `test/fixtures/synthetic-relations/relations_*.ifc` (see
//! `tests/object_relations.rs` for the full map):
//!
//! ```text
//! #12 building    decomposes #11 project through #14, is decomposed by #15
//! #13 storey      contains #17 #18 #19 #28 #31 through #33
//! #16 wall type   types #17 Wall A, #18 Wall B through #20
//! #17 Wall A      typed by #20, property set by #36, voided by #24 #25
//! #21 opening     voids #17 through #24, filled by #28 through #30
//! #28 Door 1      typed by #29, fills #21 through #30
//! ```
//!
//! The declarations each assertion relies on, as the releases state them
//! (`references/ifc-spec`): IFC2X3 `IfcObject.IsDefinedBy : SET [0:?] OF
//! IfcRelDefines FOR RelatedObjects` holds type and property relationships
//! alike, and `IfcTypeObject.ObjectTypeOf : SET [0:1] OF
//! IfcRelDefinesByType FOR RelatingType`; IFC4 and IFC4X3 split
//! `IsTypedBy : SET [0:1] OF IfcRelDefinesByType` from `IsDefinedBy : SET
//! [0:?] OF IfcRelDefinesByProperties`, and rename the type's inverse
//! `Types`; `IfcObjectDefinition.Decomposes : SET [0:1] OF IfcRelAggregates`
//! (IFC2X3: `IfcRelDecomposes`); `ReferencedInStructures` is `IfcElement`'s
//! in IFC2X3 and IFC4 and `IfcProduct`'s in IFC4X3;
//! `IfcFeatureElementSubtraction.VoidsElements : IfcRelVoidsElement FOR
//! RelatedOpeningElement` is not an aggregate in any of them.

#![cfg(all(
    feature = "step",
    feature = "ifc2x3",
    feature = "ifc4",
    feature = "ifc4x3"
))]

use std::path::PathBuf;

use ifc::{
    attribute_by_name, attribute_value_by_name, inverse_slot, inverse_slots, total_inverses,
    AttributeValue, Codec, EntityId, InverseValue, Model, NamedAttributeError, ReverseIndex,
    Schema, SchemaVersion, StepCodec,
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

fn release(version: SchemaVersion) -> (Model, &'static Schema) {
    let (name, _) = FIXTURES
        .iter()
        .find(|(_, fixture)| *fixture == version)
        .expect("a fixture");
    let model = read(name);
    let token = model.header().schema_token().expect("a FILE_SCHEMA");
    assert_eq!(SchemaVersion::from_header_token(token), Some(version));
    (model, ifc::schema::for_version(version).expect("bundled"))
}

/// `name` of `#id`: the referrers, or the refusal.
fn inverse(
    model: &Model,
    schema: &Schema,
    id: u64,
    name: &str,
) -> Result<InverseValue, NamedAttributeError> {
    let reverse = ReverseIndex::build(model);
    match attribute_value_by_name(model, schema, &reverse, EntityId(id), name)? {
        AttributeValue::Inverse(value) => Ok(value),
        other => panic!("{name} of #{id} read as {other:?}"),
    }
}

fn set(raw: &[u64]) -> Result<InverseValue, NamedAttributeError> {
    Ok(InverseValue::Aggregate(
        raw.iter().copied().map(EntityId).collect(),
    ))
}

fn unknown(schema: &Schema, id: u64, model: &Model, name: &str) {
    let reverse = ReverseIndex::build(model);
    let refused = attribute_value_by_name(model, schema, &reverse, EntityId(id), name);
    assert!(
        matches!(refused, Err(NamedAttributeError::UnknownAttribute { .. })),
        "{} #{id}.{name}: {refused:?}",
        schema.name()
    );
}

#[test]
fn ifc2x3_is_defined_by_holds_the_type_and_the_property_set() {
    let (model, schema) = release(SchemaVersion::Ifc2x3);
    assert_eq!(inverse(&model, schema, 17, "IsDefinedBy"), set(&[20, 36]));
    unknown(schema, 17, &model, "IsTypedBy");
    assert_eq!(inverse(&model, schema, 16, "ObjectTypeOf"), set(&[20]));
    unknown(schema, 16, &model, "Types");
    // The door style is an IfcTypeProduct too.
    assert_eq!(inverse(&model, schema, 27, "ObjectTypeOf"), set(&[29]));
}

#[test]
fn ifc4_and_ifc4x3_split_is_typed_by_from_is_defined_by() {
    for version in [SchemaVersion::Ifc4, SchemaVersion::Ifc4x3] {
        let (model, schema) = release(version);
        assert_eq!(inverse(&model, schema, 17, "IsDefinedBy"), set(&[36]));
        assert_eq!(inverse(&model, schema, 17, "IsTypedBy"), set(&[20]));
        assert_eq!(inverse(&model, schema, 18, "IsDefinedBy"), set(&[]));
        assert_eq!(inverse(&model, schema, 19, "IsTypedBy"), set(&[]));
        assert_eq!(inverse(&model, schema, 16, "Types"), set(&[20]));
        unknown(schema, 16, &model, "ObjectTypeOf");
    }
}

#[test]
fn containment_decomposition_and_openings_in_every_release() {
    for (_, version) in FIXTURES {
        let (model, schema) = release(version);
        let name = schema.name();
        for element in [17, 18, 19, 28, 31] {
            assert_eq!(
                inverse(&model, schema, element, "ContainedInStructure"),
                set(&[33]),
                "{name} #{element}"
            );
        }
        assert_eq!(
            inverse(&model, schema, 13, "ContainsElements"),
            set(&[33]),
            "{name}"
        );
        assert_eq!(
            inverse(&model, schema, 12, "Decomposes"),
            set(&[14]),
            "{name}"
        );
        assert_eq!(
            inverse(&model, schema, 12, "IsDecomposedBy"),
            set(&[15]),
            "{name}"
        );
        assert_eq!(
            inverse(&model, schema, 11, "Decomposes"),
            set(&[]),
            "{name}"
        );
        assert_eq!(
            inverse(&model, schema, 17, "HasOpenings"),
            set(&[24, 25]),
            "{name}"
        );
        assert_eq!(
            inverse(&model, schema, 17, "HasAssociations"),
            set(&[]),
            "{name}"
        );
        assert_eq!(
            inverse(&model, schema, 21, "HasFillings"),
            set(&[30]),
            "{name}"
        );
        assert_eq!(
            inverse(&model, schema, 28, "FillsVoids"),
            set(&[30]),
            "{name}"
        );
        // Not an aggregate: the one relationship, unwrapped.
        assert_eq!(
            inverse(&model, schema, 21, "voidselements"),
            Ok(InverseValue::Single(Some(EntityId(24)))),
            "{name}"
        );
        let reverse = ReverseIndex::build(&model);
        // #20 (type), #24 #25 (voids), #33 (containment), and in IFC2X3's
        // IsDefinedBy as elsewhere #36 (properties).
        assert_eq!(total_inverses(&reverse, EntityId(17)), 5, "{name}");
        assert_eq!(total_inverses(&reverse, EntityId(36)), 0, "{name}");
    }
}

#[test]
fn ifc4x3_moves_referenced_in_structures_up_to_ifc_product() {
    let declared_by = |version| {
        let schema = ifc::schema::for_version(version).unwrap();
        inverse_slot(schema, "IfcWall", "ReferencedInStructures")
            .unwrap()
            .map(|slot| slot.declared_by)
    };
    assert_eq!(declared_by(SchemaVersion::Ifc2x3), Some("IfcElement"));
    assert_eq!(declared_by(SchemaVersion::Ifc4), Some("IfcElement"));
    assert_eq!(declared_by(SchemaVersion::Ifc4x3), Some("IfcProduct"));
    // So a storey has it in IFC4X3 only.
    let (model, schema) = release(SchemaVersion::Ifc4);
    unknown(schema, 13, &model, "ReferencedInStructures");
    let (model, schema) = release(SchemaVersion::Ifc4x3);
    assert_eq!(
        inverse(&model, schema, 13, "ReferencedInStructures"),
        set(&[])
    );
}

#[test]
fn the_slot_reads_refuse_an_inverse_name_in_every_release() {
    for (_, version) in FIXTURES {
        let (model, schema) = release(version);
        assert!(matches!(
            attribute_by_name(&model, schema, EntityId(17), "IsDefinedBy"),
            Err(NamedAttributeError::InverseAttribute { ref name, .. }) if name == "IsDefinedBy"
        ));
    }
}

/// Locate `references/ifc-spec` in either checkout layout.
fn spec_root() -> Option<PathBuf> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        "../../../../references/ifc-spec",
        "../../references/ifc-spec",
    ]
    .into_iter()
    .map(|rel| crate_dir.join(rel))
    .find(|path| path.is_dir())
}

/// Every entity's `INVERSE` attributes, inherited ones included, with their
/// `FOR` clause, bounds and declaring entity, equal the ones parsed from
/// the normative EXPRESS of each release.
#[test]
fn bundled_inverse_slots_match_the_normative_express() {
    let Some(root) = spec_root() else {
        assert!(
            std::env::var_os("IFC_SPEC_REQUIRED").is_none(),
            "IFC_SPEC_REQUIRED is set but references/ifc-spec was not found; \
             run scripts/fetch-ifc-schemas.sh"
        );
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };
    for (version, file) in [
        (SchemaVersion::Ifc2x3, "ifc2x3-tc1/IFC2X3_TC1.exp"),
        (SchemaVersion::Ifc4, "ifc4-add2-tc1/IFC4.exp"),
        (SchemaVersion::Ifc4x3, "ifc4x3-add2/IFC4X3_ADD2.exp"),
    ] {
        let bytes = std::fs::read(root.join(file)).expect("read the reference schema");
        let normative = Schema::from_express_bytes(&bytes);
        let bundled = ifc::schema::for_version(version).unwrap();
        let mut compared = 0;
        for entity in normative.entities() {
            let expected = inverse_slots(&normative, &entity.name).unwrap();
            let actual = inverse_slots(bundled, &entity.name).unwrap();
            assert_eq!(actual, expected, "{} {}", bundled.name(), entity.name);
            compared += actual.len();
        }
        // Inherited inverses count once per entity that has them.
        assert!(compared > 1000, "{} compared {compared}", bundled.name());
    }
}
