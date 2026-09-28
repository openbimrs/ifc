//! IFC4 and IFC4X3 output is unchanged by #202.
//!
//! The writers that take no model keep their record; the
//! `*_with_owner_history` variant writes the same record with the owner
//! history in slot 1. Where a plain writer's arity disagrees with the
//! release, the variant writes the release's own arity and the difference
//! is pinned here: `assign_to_actor` and `assign_to_process` stop at seven
//! of eight attributes, `connect_with_realizing_elements` at eight of nine,
//! and `interfere_elements` writes ten where IFC4 declares nine.

mod release_fixture;

use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_schema::SchemaVersion;
use ifc_spatial::authoring::{
    assign_to_actor, assign_to_actor_with_owner_history, assign_to_group_by_factor,
    assign_to_group_by_factor_with_owner_history, assign_to_process,
    assign_to_process_with_owner_history, assign_to_product, assign_to_product_with_owner_history,
    connect_elements, connect_elements_with_owner_history, connect_with_realizing_elements,
    connect_with_realizing_elements_with_owner_history, cover_spaces,
    cover_spaces_with_owner_history, declare, declare_with_owner_history, fill_element,
    fill_element_with_owner_history, interfere_elements, interfere_elements_with_owner_history,
    void_element, void_element_with_owner_history,
};
use ifc_spatial::facility::IFCBRIDGE;
use ifc_spatial::{
    aggregate, aggregate_with_owner_history, connect_path_elements,
    connect_path_elements_with_owner_history, contain, contain_with_owner_history,
    create_external_spatial_element, create_external_spatial_element_with_owner_history,
    create_facility, create_facility_with_owner_history, create_project, create_project_library,
    create_project_library_with_owner_history, create_project_with_owner_history,
    create_space_boundary, create_space_boundary_with_owner_history, create_spatial_element,
    create_spatial_element_with_owner_history, BoundaryDraft, BoundaryLevel, ExternalSpatialDraft,
    FacilityDraft, ProjectLibraryDraft, SpatialDraft, SpatialKind,
};
use release_fixture::{
    base, guid, ACTOR, CONTEXT, COVERING, DOOR, GROUP, OPENING, OWNER, PROXY, SPACE, TASK, UNITS,
    WALL, WALL2, WALL_TYPE,
};

/// The record `variant` wrote equals `plain`'s with its own GlobalId and
/// the owner history in slot 1, `plain` first padded with `$` or truncated
/// to `arity`.
fn owned_copy(model: &Model, plain: EntityId, variant: EntityId, arity: usize) {
    let mut expected: Entity = model.get(plain).expect("plain").clone();
    let variant = model.get(variant).expect("variant");
    expected.attributes.resize(arity, Value::Null);
    expected.attributes[0] = variant.attributes[0].clone();
    expected.attributes[1] = Value::Ref(OWNER);
    assert_eq!(variant, &expected);
}

#[test]
#[allow(clippy::too_many_lines)]
fn variants_write_the_plain_record_with_an_owner_history() {
    for (schema, version) in [
        ("IFC4", SchemaVersion::Ifc4),
        ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
    ] {
        let mut model = base(schema, version);
        let m = model.clone();
        let mut tx = Transaction::new(&model);
        let t = &mut tx;
        let mut n = 0;
        let mut g = || {
            n += 1;
            guid(n)
        };
        let mut pairs: Vec<(EntityId, EntityId, usize)> = Vec::new();
        let draft = SpatialDraft {
            name: Some("Site"),
            description: Some("d"),
            long_name: Some("Long"),
            composition: Some("ELEMENT"),
            placement: None,
        };
        for (kind, arity) in [
            (SpatialKind::Site, 14),
            (SpatialKind::Building, 12),
            (SpatialKind::Storey, 10),
            (SpatialKind::Space, 11),
        ] {
            pairs.push((
                create_spatial_element(t, kind, &g(), draft).unwrap(),
                create_spatial_element_with_owner_history(t, &m, kind, &g(), draft, OWNER).unwrap(),
                arity,
            ));
        }
        pairs.push((
            create_project(t, &g(), Some("P"), Some(UNITS)).unwrap(),
            create_project_with_owner_history(t, &m, &g(), Some("P"), Some(UNITS), OWNER).unwrap(),
            9,
        ));
        pairs.push((
            aggregate(t, &g(), WALL, &[WALL2]).unwrap(),
            aggregate_with_owner_history(t, &m, &g(), WALL, &[WALL2], OWNER).unwrap(),
            6,
        ));
        pairs.push((
            contain(t, &g(), SPACE, &[WALL]).unwrap(),
            contain_with_owner_history(t, &m, &g(), SPACE, &[WALL], OWNER).unwrap(),
            6,
        ));
        pairs.push((
            cover_spaces(t, &g(), SPACE, &[COVERING]).unwrap(),
            cover_spaces_with_owner_history(t, &m, &g(), SPACE, &[COVERING], OWNER).unwrap(),
            6,
        ));
        pairs.push((
            declare(t, &g(), WALL, &[WALL_TYPE]).unwrap(),
            declare_with_owner_history(t, &m, &g(), WALL, &[WALL_TYPE], OWNER).unwrap(),
            6,
        ));
        pairs.push((
            assign_to_product(t, &g(), PROXY, &[WALL]).unwrap(),
            assign_to_product_with_owner_history(t, &m, &g(), PROXY, &[WALL], OWNER).unwrap(),
            7,
        ));
        // Plain stops at seven: ActingRole / QuantityInProcess are unset.
        pairs.push((
            assign_to_actor(t, &g(), ACTOR, &[WALL]).unwrap(),
            assign_to_actor_with_owner_history(t, &m, &g(), ACTOR, &[WALL], OWNER).unwrap(),
            8,
        ));
        pairs.push((
            assign_to_process(t, &g(), TASK, &[WALL]).unwrap(),
            assign_to_process_with_owner_history(t, &m, &g(), TASK, &[WALL], OWNER).unwrap(),
            8,
        ));
        pairs.push((
            assign_to_group_by_factor(t, &g(), GROUP, &[WALL], 0.5).unwrap(),
            assign_to_group_by_factor_with_owner_history(t, &m, &g(), GROUP, &[WALL], 0.5, OWNER)
                .unwrap(),
            8,
        ));
        pairs.push((
            connect_elements(t, &g(), WALL, WALL2).unwrap(),
            connect_elements_with_owner_history(t, &m, &g(), WALL, WALL2, OWNER).unwrap(),
            7,
        ));
        pairs.push((
            connect_with_realizing_elements(t, &g(), WALL, WALL2, &[PROXY]).unwrap(),
            connect_with_realizing_elements_with_owner_history(
                t,
                &m,
                &g(),
                WALL,
                WALL2,
                &[PROXY],
                OWNER,
            )
            .unwrap(),
            9,
        ));
        let interferes = if version == SchemaVersion::Ifc4 {
            9
        } else {
            10
        };
        pairs.push((
            interfere_elements(t, &g(), WALL, WALL2, Some(true)).unwrap(),
            interfere_elements_with_owner_history(t, &m, &g(), WALL, WALL2, Some(true), OWNER)
                .unwrap(),
            interferes,
        ));
        pairs.push((
            void_element(t, &g(), WALL, OPENING).unwrap(),
            void_element_with_owner_history(t, &m, &g(), WALL, OPENING, OWNER).unwrap(),
            6,
        ));
        pairs.push((
            fill_element(t, &g(), OPENING, DOOR).unwrap(),
            fill_element_with_owner_history(t, &m, &g(), OPENING, DOOR, OWNER).unwrap(),
            6,
        ));
        let priorities: (&[i64], &[i64]) = (&[10], &[20]);
        pairs.push((
            connect_path_elements(t, &g(), WALL, WALL2, priorities, ("atstart", "ATEND")).unwrap(),
            connect_path_elements_with_owner_history(
                t,
                &m,
                &g(),
                WALL,
                WALL2,
                priorities,
                ("atstart", "ATEND"),
                OWNER,
            )
            .unwrap(),
            11,
        ));
        let external = ExternalSpatialDraft {
            name: Some("Outside"),
            object_type: Some("Yard"),
            long_name: Some("L"),
            predefined_type: Some("USERDEFINED"),
            ..ExternalSpatialDraft::default()
        };
        pairs.push((
            create_external_spatial_element(t, &g(), external).unwrap(),
            create_external_spatial_element_with_owner_history(t, &m, &g(), external, OWNER)
                .unwrap(),
            9,
        ));
        let library = ProjectLibraryDraft {
            name: Some("Lib"),
            phase: Some("Design"),
            units: Some(UNITS),
            ..ProjectLibraryDraft::default()
        };
        pairs.push((
            create_project_library(t, &g(), library, &[CONTEXT]).unwrap(),
            create_project_library_with_owner_history(t, &m, &g(), library, &[CONTEXT], OWNER)
                .unwrap(),
            9,
        ));
        if version == SchemaVersion::Ifc4x3 {
            let bridge = FacilityDraft {
                name: Some("Bridge"),
                composition: Some("ELEMENT"),
                ..FacilityDraft::default()
            };
            pairs.push((
                create_facility(t, IFCBRIDGE, &g(), Some("GIRDER"), bridge).unwrap(),
                create_facility_with_owner_history(
                    t,
                    &m,
                    IFCBRIDGE,
                    &g(),
                    Some("GIRDER"),
                    bridge,
                    OWNER,
                )
                .unwrap(),
                10,
            ));
        }
        tx.commit(&mut model).expect("commit");
        for (plain, variant, arity) in pairs {
            owned_copy(&model, plain, variant, arity);
        }
    }
}

/// The plain arity of the four writers whose record disagrees with the
/// release, pinned so a fix is a deliberate, changelogged change.
#[test]
fn known_plain_arity_disagreements_are_pinned() {
    let mut model = base("IFC4", SchemaVersion::Ifc4);
    let mut tx = Transaction::new(&model);
    let actor = assign_to_actor(&mut tx, &guid(1), ACTOR, &[WALL]).unwrap();
    let process = assign_to_process(&mut tx, &guid(2), TASK, &[WALL]).unwrap();
    let realizing =
        connect_with_realizing_elements(&mut tx, &guid(3), WALL, WALL2, &[PROXY]).unwrap();
    let interferes = interfere_elements(&mut tx, &guid(4), WALL, WALL2, None).unwrap();
    tx.commit(&mut model).expect("commit");
    for (id, arity) in [(actor, 7), (process, 7), (realizing, 8), (interferes, 10)] {
        assert_eq!(model.get(id).unwrap().attributes.len(), arity);
    }
}

/// `create_space_boundary` now binds the release; its IFC4 and IFC4X3
/// records are the pre-#202 positional ones, slot for slot.
#[test]
fn space_boundary_records_are_unchanged() {
    for (schema, version) in [
        ("IFC4", SchemaVersion::Ifc4),
        ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
    ] {
        let model = base(schema, version);
        let mut tx = Transaction::new(&model);
        let text = |s: &str| Value::Text(s.into());
        let draft = BoundaryDraft {
            name: Some("B"),
            description: Some("D"),
            space: SPACE,
            element: WALL,
            connection_geometry: None,
            physical_or_virtual: "physical",
            internal_or_external: "EXTERNAL_EARTH",
            parent: None,
            corresponding: None,
        };
        let parent =
            create_space_boundary(&mut tx, &model, BoundaryLevel::First, &guid(1), draft).unwrap();
        let second = BoundaryDraft {
            parent: Some(parent),
            corresponding: Some(parent),
            ..draft
        };
        let plain = create_space_boundary(&mut tx, &model, BoundaryLevel::Second, &guid(2), second)
            .unwrap();
        let owned = create_space_boundary_with_owner_history(
            &mut tx,
            &model,
            BoundaryLevel::Second,
            &guid(2),
            second,
            OWNER,
        )
        .unwrap();
        let mut expected = vec![
            text(&guid(2)),
            Value::Null,
            text("B"),
            text("D"),
            Value::Ref(SPACE),
            Value::Ref(WALL),
            Value::Null,
            Value::Enum("PHYSICAL".into()),
            Value::Enum("EXTERNAL_EARTH".into()),
            Value::Ref(parent),
            Value::Ref(parent),
        ];
        let staged = |id| {
            tx.edits()
                .iter()
                .find_map(|edit| match edit {
                    ifc_model::Edit::Create { id: e, entity } if *e == id => {
                        Some(entity.attributes.clone())
                    }
                    _ => None,
                })
                .unwrap()
        };
        assert_eq!(staged(plain), expected, "{schema}");
        expected[1] = Value::Ref(OWNER);
        assert_eq!(staged(owned), expected, "{schema}");
    }
}
