//! #202: predefined property sets in the declared release.
//!
//! From the EXPRESS sources: `IfcRoot.OwnerHistory` is `IfcOwnerHistory` in
//! IFC2X3 TC1 and `OPTIONAL IfcOwnerHistory` in IFC4 ADD2 TC1 and IFC4X3
//! ADD2. IFC2X3 `IfcDoorLiningProperties` has no `LiningToPanelOffsetX/Y`,
//! IFC2X3 `IfcWindowLiningProperties` no `LiningOffset` either, and IFC2X3
//! types their thicknesses `IfcPositiveLengthMeasure`. Each release is
//! authored through the `*_with_owner_history` writers, written as STEP,
//! read back with `ifc-step` and re-read through `exact_predefined_sets`.
//! Templates are in `template_release.rs`; `openbim-ifc`'s
//! `release_bound_owner_history_authoring` runs both through
//! `ifc-validate`.

mod predefined_support;

use ifc_model::{Codec, Edit, Entity, EntityId, Model, Transaction, Value};
use ifc_properties::{
    add_door_lining_properties, add_door_lining_properties_with_owner_history,
    add_door_panel_properties, add_door_panel_properties_with_owner_history,
    add_permeable_covering_properties, add_permeable_covering_properties_with_owner_history,
    add_reinforcement_bar_properties, add_reinforcement_definition_properties,
    add_reinforcement_definition_properties_with_owner_history, add_section_properties,
    add_section_reinforcement_properties, add_window_lining_properties,
    add_window_lining_properties_with_owner_history, add_window_panel_properties,
    add_window_panel_properties_with_owner_history, exact_predefined_sets, DoorLiningDraft,
    ExactValue, PropertyError, ReinforcementBarDraft, SchemaVersion, SectionReinforcementDraft,
    WindowLiningDraft,
};
use ifc_step::StepCodec;
use predefined_support::{door_model, table, DOOR_TYPE, RELEASES};

const OWNER: EntityId = EntityId(44);
const PROFILE: EntityId = EntityId(52);
const G: [&str; 4] = [
    "0YvctVUKr0kugbFTf53O08",
    "0YvctVUKr0kugbFTf53O09",
    "0YvctVUKr0kugbFTf53O0A",
    "0YvctVUKr0kugbFTf53O0B",
];

/// The door and door type of `door_model`, an owner history `#44` and a
/// rectangle profile `#52`, in `version`.
fn model(version: SchemaVersion) -> Model {
    door_model(
        version,
        &[],
        &[],
        &[
            "#40=IFCPERSON($,'Doe','Jane',$,$,$,$,$);".to_owned(),
            "#41=IFCORGANIZATION($,'Acme',$,$,$);".to_owned(),
            "#42=IFCPERSONANDORGANIZATION(#40,#41,$);".to_owned(),
            "#43=IFCAPPLICATION(#41,'1.0','Test','test');".to_owned(),
            "#44=IFCOWNERHISTORY(#42,#43,$,.NOCHANGE.,$,$,$,1700000000);".to_owned(),
            "#50=IFCCARTESIANPOINT((0.,0.));".to_owned(),
            "#51=IFCAXIS2PLACEMENT2D(#50,$);".to_owned(),
            "#52=IFCRECTANGLEPROFILEDEF(.AREA.,$,#51,100.,200.);".to_owned(),
        ],
    )
}

fn door_lining(ifc4: bool) -> DoorLiningDraft<'static> {
    let mut draft = DoorLiningDraft::new()
        .name("Lining")
        .lining_depth(120.0)
        .lining_thickness(40.0)
        .casing_thickness(10.0)
        .casing_depth(60.0);
    draft.lining_to_panel_offset_x = ifc4.then_some(5.0);
    draft
}

fn window_lining(ifc4: bool) -> WindowLiningDraft<'static> {
    let mut draft = WindowLiningDraft::new()
        .name("Window lining")
        .lining_depth(100.0)
        .lining_thickness(50.0)
        .first_transom_offset(0.5);
    draft.lining_offset = ifc4.then_some(-5.0);
    draft
}

/// Every predefined set, with an owner history, in `version`'s layout.
fn author_predefined(tx: &mut Transaction, model: &Model, version: SchemaVersion) -> Vec<EntityId> {
    let ifc4 = version != SchemaVersion::Ifc2x3;
    let bar = add_reinforcement_bar_properties(
        tx,
        ReinforcementBarDraft::new(314.0, "B500B")
            .bar_surface("TEXTURED")
            .nominal_bar_diameter(20.0)
            .bar_count(4),
    )
    .expect("bar");
    let section = add_section_properties(tx, "UNIFORM", PROFILE, None).expect("section");
    let reinforcement = add_section_reinforcement_properties(
        tx,
        SectionReinforcementDraft::new(0.0, 3000.0, "MAIN", section, &[bar]),
    )
    .expect("section reinforcement");
    let panel = (Some(40.0), Some(0.5));
    let frame = (Some(60.0), Some(50.0));
    vec![
        add_door_lining_properties_with_owner_history(tx, model, G[0], door_lining(ifc4), OWNER),
        add_window_lining_properties_with_owner_history(
            tx,
            model,
            G[1],
            window_lining(ifc4),
            OWNER,
        ),
        add_door_panel_properties_with_owner_history(
            tx,
            model,
            G[2],
            Some("Leaf"),
            "SWINGING",
            "LEFT",
            panel,
            OWNER,
        ),
        add_window_panel_properties_with_owner_history(
            tx,
            model,
            G[3],
            Some("Casement"),
            "SIDEHUNGLEFTHAND",
            "MIDDLE",
            frame,
            OWNER,
        ),
        add_permeable_covering_properties_with_owner_history(
            tx,
            model,
            "1YvctVUKr0kugbFTf53O08",
            Some("Louvre"),
            "LOUVER",
            "TOP",
            frame,
            OWNER,
        ),
        add_reinforcement_definition_properties_with_owner_history(
            tx,
            model,
            "1YvctVUKr0kugbFTf53O09",
            Some("Rebar"),
            None,
            Some("MAIN"),
            &[reinforcement],
            OWNER,
        ),
    ]
    .into_iter()
    .map(|written| written.expect("predefined set"))
    .collect()
}

fn slot(version: SchemaVersion, entity: &str, attribute: &str) -> usize {
    table(version)
        .attribute_names(entity)
        .iter()
        .position(|name| name.eq_ignore_ascii_case(attribute))
        .unwrap_or_else(|| panic!("{entity}.{attribute}"))
}

/// Refused before anything is staged.
fn refused(
    model: &Model,
    author: impl Fn(&mut Transaction) -> Result<EntityId, PropertyError>,
) -> PropertyError {
    let mut tx = Transaction::new(model);
    let error = author(&mut tx).expect_err("refused");
    assert!(tx.is_empty(), "a refusal staged {:?}", tx.edits());
    error
}

/// Each release: every predefined set with the release's arity and the
/// owner history, held by the door type, written as STEP, read back, and
/// re-read through `exact_predefined_sets`.
#[test]
fn predefined_sets_round_trip_in_every_release() {
    for (schema, version) in RELEASES {
        let mut model = model(version);
        let mut tx = Transaction::new(&model);
        let sets = author_predefined(&mut tx, &model, version);
        let held = Value::List(sets[..5].iter().copied().map(Value::Ref).collect());
        let has_sets = slot(
            version,
            &model.get(DOOR_TYPE).unwrap().type_name,
            "HasPropertySets",
        );
        tx.set_attribute(DOOR_TYPE, has_sets, held);
        tx.commit(&mut model).expect("commit");

        let bytes = StepCodec.write_bytes(&model).expect("written");
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        for id in &sets {
            let record = back.get(*id).expect("read back");
            let entity = record.type_name.as_ref();
            assert_eq!(
                record.attributes.len(),
                table(version).attributes(entity).len(),
                "{schema}: {entity}"
            );
            assert_eq!(
                record.attributes[slot(version, entity, "OwnerHistory")],
                Value::Ref(OWNER)
            );
        }
        let read = |entity| {
            let found = exact_predefined_sets(&back, DOOR_TYPE, entity).expect(entity);
            assert_eq!(found.len(), 1, "{schema}: {entity}");
            found.into_iter().next().unwrap()
        };
        let lining = read("IfcDoorLiningProperties");
        assert_eq!(lining.set_id, sets[0]);
        assert_eq!(
            lining.attribute("LiningThickness").unwrap().value,
            ExactValue::Real(40.0)
        );
        let offset = lining.attribute("LiningToPanelOffsetX");
        if version == SchemaVersion::Ifc2x3 {
            assert!(offset.is_none(), "IFC2X3 declares no LiningToPanelOffsetX");
        } else {
            assert_eq!(offset.unwrap().value, ExactValue::Real(5.0));
        }
        let window = read("IfcWindowLiningProperties");
        assert_eq!(
            window.attribute("FirstTransomOffset").unwrap().value,
            ExactValue::Real(0.5)
        );
        let panel = read("IfcDoorPanelProperties");
        assert_eq!(
            panel.attribute("PanelOperation").unwrap().value,
            ExactValue::Enum("SWINGING".into())
        );
        read("IfcWindowPanelProperties");
        read("IfcPermeableCoveringProperties");
    }
}

/// IFC4 and IFC4X3: each `*_with_owner_history` record is the one the
/// writer that takes no model stages, with the owner history in its
/// optional slot; that writer's output is unchanged.
#[test]
fn ifc4_and_ifc4x3_output_is_unchanged() {
    for version in [SchemaVersion::Ifc4, SchemaVersion::Ifc4x3] {
        let model = model(version);
        let mut owned = Transaction::new(&model);
        author_predefined(&mut owned, &model, version);
        let mut plain = Transaction::new(&model);
        let bar = add_reinforcement_bar_properties(
            &mut plain,
            ReinforcementBarDraft::new(314.0, "B500B")
                .bar_surface("TEXTURED")
                .nominal_bar_diameter(20.0)
                .bar_count(4),
        )
        .unwrap();
        let section = add_section_properties(&mut plain, "UNIFORM", PROFILE, None).unwrap();
        let reinforcement = add_section_reinforcement_properties(
            &mut plain,
            SectionReinforcementDraft::new(0.0, 3000.0, "MAIN", section, &[bar]),
        )
        .unwrap();
        let (panel, frame) = ((Some(40.0), Some(0.5)), (Some(60.0), Some(50.0)));
        add_door_lining_properties(&mut plain, G[0], door_lining(true)).unwrap();
        add_window_lining_properties(&mut plain, G[1], window_lining(true)).unwrap();
        add_door_panel_properties(&mut plain, G[2], Some("Leaf"), "SWINGING", "LEFT", panel)
            .unwrap();
        add_window_panel_properties(
            &mut plain,
            G[3],
            Some("Casement"),
            "SIDEHUNGLEFTHAND",
            "MIDDLE",
            frame,
        )
        .unwrap();
        add_permeable_covering_properties(
            &mut plain,
            "1YvctVUKr0kugbFTf53O08",
            Some("Louvre"),
            "LOUVER",
            "TOP",
            frame,
        )
        .unwrap();
        add_reinforcement_definition_properties(
            &mut plain,
            "1YvctVUKr0kugbFTf53O09",
            Some("Rebar"),
            None,
            Some("MAIN"),
            &[reinforcement],
        )
        .unwrap();
        let records = |tx: &Transaction| -> Vec<Entity> {
            tx.edits()
                .iter()
                .map(|edit| match edit {
                    Edit::Create { entity, .. } => entity.clone(),
                    other => panic!("{other:?}"),
                })
                .collect()
        };
        let (owned, plain) = (records(&owned), records(&plain));
        assert_eq!(owned.len(), plain.len());
        for (owned, mut plain) in owned.into_iter().zip(plain) {
            if plain
                .attributes
                .first()
                .is_some_and(|v| matches!(v, Value::Text(_)))
                && owned.attributes.get(1) == Some(&Value::Ref(OWNER))
            {
                assert_eq!(plain.attributes[1], Value::Null, "{version:?}");
                plain.attributes[1] = Value::Ref(OWNER);
            }
            assert_eq!(owned, plain, "{version:?}");
        }
    }
}

/// What IFC2X3 does not declare is refused, never dropped or written in
/// the IFC4 layout: two attributes, and a zero thickness where IFC2X3
/// wants a positive length.
#[test]
fn ifc2x3_refuses_what_it_does_not_declare() {
    let model = model(SchemaVersion::Ifc2x3);
    assert_eq!(
        refused(&model, |tx| add_door_lining_properties_with_owner_history(
            tx,
            &model,
            G[0],
            door_lining(true),
            OWNER
        )),
        PropertyError::AuthoringNotInSchema {
            entity: "IFCDOORLININGPROPERTIES",
            attribute: "LiningToPanelOffsetX",
            schema: SchemaVersion::Ifc2x3,
        }
    );
    assert_eq!(
        refused(
            &model,
            |tx| add_window_lining_properties_with_owner_history(
                tx,
                &model,
                G[0],
                window_lining(true),
                OWNER
            )
        ),
        PropertyError::AuthoringNotInSchema {
            entity: "IFCWINDOWLININGPROPERTIES",
            attribute: "LiningOffset",
            schema: SchemaVersion::Ifc2x3,
        }
    );
    let zero = door_lining(false).lining_thickness(0.0);
    let error = refused(&model, |tx| {
        add_door_lining_properties_with_owner_history(tx, &model, G[0], zero, OWNER)
    });
    assert!(
        matches!(
            &error,
            PropertyError::AuthoringInvalid {
                attribute: "LiningThickness",
                value,
                ..
            } if value.contains("IfcPositiveLengthMeasure")
        ),
        "{error:?}"
    );
    let ifc4 = self::model(SchemaVersion::Ifc4);
    let mut tx = Transaction::new(&ifc4);
    add_door_lining_properties_with_owner_history(&mut tx, &ifc4, G[0], zero, OWNER)
        .expect("IFC4 types LiningThickness as IfcNonNegativeLengthMeasure");
}

#[test]
fn a_wrong_type_or_missing_owner_history_is_refused() {
    for (schema, version) in RELEASES {
        let model = model(version);
        for owner in [DOOR_TYPE, EntityId(999)] {
            let error = refused(&model, |tx| {
                add_door_panel_properties_with_owner_history(
                    tx,
                    &model,
                    G[0],
                    None,
                    "SWINGING",
                    "LEFT",
                    (None, None),
                    owner,
                )
            });
            assert!(
                matches!(
                    error,
                    PropertyError::AuthoringInvalid {
                        attribute: "OwnerHistory",
                        ..
                    } | PropertyError::MissingEntity { .. }
                ),
                "{schema}: {error:?}"
            );
        }
    }
}

#[test]
fn multiple_or_unknown_schemas_are_refused() {
    for (header, expected) in [
        (
            vec!["IFC4".to_owned(), "IFC2X3".to_owned()],
            PropertyError::MultipleSchemas { schemas: 2 },
        ),
        (
            vec!["IFC9".to_owned()],
            PropertyError::UnsupportedSchema {
                schema: "IFC9".into(),
            },
        ),
    ] {
        let mut model = model(SchemaVersion::Ifc4);
        model.header_mut().schema = header;
        let error = refused(&model, |tx| {
            add_reinforcement_definition_properties_with_owner_history(
                tx,
                &model,
                G[0],
                None,
                None,
                None,
                &[EntityId(1)],
                OWNER,
            )
        });
        assert_eq!(error, expected);
    }
}
