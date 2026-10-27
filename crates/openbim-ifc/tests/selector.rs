//! The selector language (#464) over one model in IFC2X3, IFC4 and IFC4X3:
//! filtering, reading by path, and writing by path.
//!
//! `test/fixtures/synthetic-relations/selector_*.ifc`, identical in the
//! three releases but for each release's own arities:
//!
//! ```text
//! #13 Level 0       holds #17 Wall A, #18 Wall B, #28 Door 1, #31 Window 1
//! #37 Level 1       holds #19 Wall C; aggregates #38 Room 101
//! #38 Room 101      holds #40 Slab 1
//! #16 WT-200        types #17, #18; own Pset_WallCommon (FireRating F90,
//!                   AcousticRating R45); layer set #54 of #52 Concrete;
//!                   classified 04 22 00
//! #17 Wall A        Pset_WallCommon.IsExternal TRUE, Qto Length 5,
//!                   layer set usage #55
//! #18 Wall B        Pset_WallCommon.IsExternal FALSE, layer set usage #55
//! #27 D-90          types #28; material #58 Oak (category Timber after IFC2X3)
//! #28 Door 1        classified 08 14 00 Wood Doors; in group #65 Fire doors
//! ```

#![cfg(all(
    feature = "step",
    feature = "selector",
    feature = "properties",
    feature = "ifc2x3",
    feature = "ifc4",
    feature = "ifc4x3"
))]

use ifc::selector::{
    get_value, select, set_value, ParseErrorKind, SelectedValue, Selector, SelectorError,
    SelectorIndex, SetTarget, ValuePath,
};
use ifc::{Codec, EntityId, Model, NamedAttributeError, Schema, SchemaVersion, StepCodec, Value};

const FIXTURES: [(&str, SchemaVersion); 3] = [
    ("selector_ifc2x3.ifc", SchemaVersion::Ifc2x3),
    ("selector_ifc4.ifc", SchemaVersion::Ifc4),
    ("selector_ifc4x3.ifc", SchemaVersion::Ifc4x3),
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

fn each_release(mut check: impl FnMut(&str, SchemaVersion, &Model, &'static Schema)) {
    for (name, version) in FIXTURES {
        let model = read(name);
        let schema = ifc::schema::for_version(version).expect("bundled");
        check(name, version, &model, schema);
    }
}

fn ids(raw: &[u64]) -> Vec<EntityId> {
    raw.iter().copied().map(EntityId).collect()
}

fn run(model: &Model, schema: &Schema, text: &str) -> Result<Vec<EntityId>, SelectorError> {
    let selector = Selector::parse(text).unwrap_or_else(|error| panic!("{text:?}: {error}"));
    select(model, schema, &selector)
}

fn value(model: &Model, schema: &Schema, id: u64, path: &str) -> SelectedValue {
    let path = ValuePath::parse(path).unwrap();
    get_value(model, schema, EntityId(id), &path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

fn text(value: &str) -> SelectedValue {
    SelectedValue::Text(value.to_owned())
}

#[test]
fn facets_select_the_same_entities_in_every_release() {
    let cases: &[(&str, &[u64])] = &[
        ("IfcWall", &[17, 18, 19]),
        ("IFCWALL", &[17, 18, 19]),
        ("IfcElement", &[17, 18, 19, 21, 22, 23, 28, 31, 40]),
        ("IfcDoor + IfcWindow", &[28, 31]),
        ("IfcWall, IfcSlab", &[17, 18, 19, 40]),
        ("IfcWall, type=WT-200", &[17, 18]),
        ("IfcWall, type=NULL", &[19]),
        ("IfcWall, type!=WT-200", &[19]),
        ("IfcWall, Name=/Wall [AB]/", &[17, 18]),
        ("IfcWall, Name*=\"all C\"", &[19]),
        ("IfcBuildingStorey, Name=\"Level 1\"", &[37]),
        ("Pset_WallCommon.IsExternal=TRUE", &[17]),
        ("IfcWall, Pset_WallCommon.FireRating=F90", &[17, 18]),
        ("IfcWall, Pset_WallCommon.FireRating=NULL", &[19]),
        ("/Pset_.*Common/.IsExternal=FALSE", &[18]),
        ("Pset_WallCommon./.*Rating/=R45", &[16, 17, 18]),
        ("Qto_WallBaseQuantities.Length>4.5", &[17]),
        ("Qto_WallBaseQuantities.Length>=5", &[17]),
        ("Qto_WallBaseQuantities.Length>5", &[]),
        ("material=Concrete", &[16, 17, 18]),
        ("material=Oak", &[27, 28]),
        ("IfcWall, material=NULL", &[19]),
        ("classification=\"04 22 00\"", &[16, 17, 18]),
        ("classification=\"Wood Doors\"", &[28]),
        ("classification=/0[48] /", &[16, 17, 18, 28]),
        ("location=\"Level 0\"", &[17, 18, 28, 31]),
        ("container=\"Level 1\"", &[19, 38, 40]),
        ("location=Building", &[13, 17, 18, 19, 28, 31, 37, 38, 40]),
        ("IfcElement, location!=Building", &[21, 22, 23]),
        ("group=\"Fire doors\"", &[28]),
        ("query:type.Name=WT-200", &[16, 17, 18]),
        ("query:materials.Name=Oak", &[27, 28]),
        ("query:storey.Name=\"Level 1\"", &[19, 38, 40]),
        ("IfcWallType, query:occurrences.count=2", &[16]),
        ("IfcWall, !IfcWall", &[]),
        ("IfcWall /* not C */, Name!=\"Wall C\"", &[17, 18]),
    ];
    each_release(|name, _, model, schema| {
        let index = SelectorIndex::new(model, schema);
        for (text, expected) in cases {
            let selector = Selector::parse(text).unwrap();
            let found = index
                .select(&selector)
                .unwrap_or_else(|error| panic!("{name} {text:?}: {error}"));
            assert_eq!(found, ids(expected), "{name} {text:?}");
        }
    });
}

#[test]
fn instance_facets_read_the_global_id_index() {
    each_release(|name, _, model, schema| {
        let SelectedValue::Text(global_id) = value(model, schema, 17, "GlobalId") else {
            panic!("{name}: Wall A has a GlobalId");
        };
        assert_eq!(
            run(model, schema, &global_id).unwrap(),
            ids(&[17]),
            "{name}"
        );
        let text = format!("IfcWall, !{global_id}");
        assert_eq!(run(model, schema, &text).unwrap(), ids(&[18, 19]), "{name}");
        // A negated facet first starts from every product and type product.
        let rest = run(model, schema, &format!("!{global_id}")).unwrap();
        assert!(
            rest.contains(&EntityId(18)) && !rest.contains(&EntityId(17)),
            "{name}"
        );
        let rest = run(model, schema, "!IfcWall").unwrap();
        assert!(
            rest.contains(&EntityId(28)) && !rest.contains(&EntityId(17)),
            "{name}"
        );
        assert!(
            !rest.contains(&EntityId(11)),
            "{name}: the project is no product"
        );
    });
}

#[test]
fn predefined_types_follow_the_type_object_where_it_states_one() {
    each_release(|name, version, model, schema| {
        let (query, expected): (&str, &[u64]) = if version == SchemaVersion::Ifc2x3 {
            // IFC2X3 walls have no PredefinedType; their type's is read.
            ("IfcWall, PredefinedType=STANDARD", &[17, 18])
        } else {
            ("IfcWall, PredefinedType=SOLIDWALL", &[17, 18, 19])
        };
        assert_eq!(run(model, schema, query).unwrap(), ids(expected), "{name}");
        let category = run(model, schema, "material=Timber").unwrap();
        let expected: &[u64] = if version == SchemaVersion::Ifc2x3 {
            &[]
        } else {
            &[27, 28]
        };
        assert_eq!(
            category,
            ids(expected),
            "{name}: IFC2X3 materials have no Category"
        );
    });
}

#[test]
fn a_base_set_bounds_every_facet() {
    each_release(|name, _, model, schema| {
        let index = SelectorIndex::new(model, schema);
        let base = ids(&[17, 28, 40]);
        let selector = Selector::parse("IfcWall + Name=/Door/").unwrap();
        assert_eq!(
            index.select_from(&selector, &base).unwrap(),
            ids(&[17, 28]),
            "{name}"
        );
        let selector = Selector::parse("!IfcWall").unwrap();
        assert_eq!(
            index.select_from(&selector, &base).unwrap(),
            ids(&[28, 40]),
            "{name}"
        );
        let missing = index.select_from(&selector, &ids(&[999]));
        assert_eq!(missing, Err(SelectorError::MissingEntity(EntityId(999))));
    });
}

#[test]
fn names_the_release_does_not_declare_are_refused_with_their_span() {
    each_release(|name, _, model, schema| {
        match run(model, schema, "IfcWall + IfcWal") {
            Err(SelectorError::UnknownClass { name: class, span }) => {
                assert_eq!((class.as_str(), span), ("IfcWal", 10..16), "{name}");
            }
            other => panic!("{name}: {other:?}"),
        }
        // Checked before any entity is read: no facet reaches one here.
        match run(model, schema, "IfcSlab, Name=x, Flavour=red") {
            Err(SelectorError::UnknownAttribute {
                name: attribute,
                span,
            }) => {
                assert_eq!((attribute.as_str(), span), ("Flavour", 17..28), "{name}");
            }
            other => panic!("{name}: {other:?}"),
        }
    });
    let error = Selector::parse("IfcWall, colour=red").unwrap_err();
    assert_eq!(error.span, 9..15);
    assert!(matches!(error.kind, ParseErrorKind::UnknownKeyword(_)));
}

#[test]
fn paths_read_attributes_relations_and_properties() {
    each_release(|name, _, model, schema| {
        let cases: &[(u64, &str, SelectedValue)] = &[
            (17, "Name", text("Wall A")),
            (17, "type.Name", text("WT-200")),
            (17, "class", text("IfcWall")),
            (17, "id", SelectedValue::Integer(17)),
            (17, "container.Name", text("Level 0")),
            (17, "storey.Name", text("Level 0")),
            (17, "building.Name", text("Building")),
            (40, "space.Name", text("Room 101")),
            (40, "storey.Name", text("Level 1")),
            (38, "parent.Name", text("Level 1")),
            (17, "material.Name", text("Concrete 200")),
            (
                17,
                "materials.Name",
                SelectedValue::List(vec![text("Concrete")]),
            ),
            (17, "materials.count", SelectedValue::Integer(1)),
            (28, "mat.Name", text("Oak")),
            (
                28,
                "classification.Name",
                SelectedValue::List(vec![text("Wood Doors")]),
            ),
            (
                28,
                "group.Name",
                SelectedValue::List(vec![text("Fire doors")]),
            ),
            (
                16,
                "occurrences.Name",
                SelectedValue::List(vec![text("Wall A"), text("Wall B")]),
            ),
            (16, "types.1.Name", text("Wall B")),
            (17, "Pset_WallCommon.IsExternal", SelectedValue::Bool(true)),
            (17, "Pset_WallCommon.FireRating", text("F90")),
            (16, "Pset_WallCommon.FireRating", text("F90")),
            (
                17,
                "Qto_WallBaseQuantities.Length",
                SelectedValue::Real(5.0),
            ),
            (
                17,
                "Pset_WallCommon./.*Rating/",
                SelectedValue::List(vec![text("F90"), text("R45")]),
            ),
            (19, "type", SelectedValue::Null),
            (19, "type.Name", SelectedValue::Null),
            (19, "Pset_WallCommon", SelectedValue::Null),
            (17, "Name.anything", SelectedValue::Null),
        ];
        for (id, path, expected) in cases {
            assert_eq!(
                &value(model, schema, *id, path),
                expected,
                "{name} #{id} {path}"
            );
        }
        let SelectedValue::Properties(properties) = value(model, schema, 17, "Pset_WallCommon")
        else {
            panic!("{name}: a property set reads as its properties");
        };
        let names: Vec<&str> = properties.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            ["IsExternal", "FireRating", "AcousticRating"],
            "{name}"
        );
        let missing = get_value(
            model,
            schema,
            EntityId(999),
            &ValuePath::parse("Name").unwrap(),
        );
        assert_eq!(missing, Err(SelectorError::MissingEntity(EntityId(999))));
    });
}

fn path(text: &str) -> ValuePath {
    ValuePath::parse(text).unwrap()
}

#[test]
fn writes_by_path_go_through_one_checked_transaction() {
    each_release(|name, _, model, schema| {
        let mut model = model.clone();
        let label = Value::Text("Wall A2".into());
        set_value(
            &mut model,
            schema,
            EntityId(17),
            &path("Name"),
            &label,
            ifc::fresh_seed().unwrap(),
        )
        .unwrap();
        assert_eq!(value(&model, schema, 17, "Name"), text("Wall A2"), "{name}");

        // Through a relation: the type object's attribute.
        let renamed = Value::Text("WT-300".into());
        set_value(
            &mut model,
            schema,
            EntityId(17),
            &path("type.Name"),
            &renamed,
            ifc::fresh_seed().unwrap(),
        )
        .unwrap();
        assert_eq!(value(&model, schema, 16, "Name"), text("WT-300"), "{name}");
        assert_eq!(
            value(&model, schema, 18, "type.Name"),
            text("WT-300"),
            "{name}"
        );

        // Every entity a list reaches.
        let concrete = Value::Text("Beton".into());
        set_value(
            &mut model,
            schema,
            EntityId(17),
            &path("materials.Name"),
            &concrete,
            ifc::fresh_seed().unwrap(),
        )
        .unwrap();
        assert_eq!(value(&model, schema, 52, "Name"), text("Beton"), "{name}");

        // An optional attribute takes `$`; a required one does not.
        set_value(
            &mut model,
            schema,
            EntityId(17),
            &path("Description"),
            &Value::Null,
            ifc::fresh_seed().unwrap(),
        )
        .unwrap();
        let before = model.revision();
        let refused = set_value(
            &mut model,
            schema,
            EntityId(17),
            &path("GlobalId"),
            &Value::Null,
            ifc::fresh_seed().unwrap(),
        );
        assert!(
            matches!(refused, Err(SelectorError::TypeMismatch { .. })),
            "{name}: {refused:?}"
        );
        let refused = set_value(
            &mut model,
            schema,
            EntityId(17),
            &path("Name"),
            &Value::Integer(3),
            ifc::fresh_seed().unwrap(),
        );
        assert!(
            matches!(refused, Err(SelectorError::TypeMismatch { .. })),
            "{name}: {refused:?}"
        );
        let refused = set_value(
            &mut model,
            schema,
            EntityId(17),
            &path("Flavour"),
            &label,
            ifc::fresh_seed().unwrap(),
        );
        assert!(
            matches!(
                refused,
                Err(SelectorError::Attribute(
                    NamedAttributeError::UnknownAttribute { .. }
                ))
            ),
            "{name}: {refused:?}"
        );
        assert_eq!(
            model.revision(),
            before,
            "{name}: a refused write changes nothing"
        );
    });
}

#[test]
fn enumerations_and_selects_are_checked_against_the_release() {
    let model = read("selector_ifc4.ifc");
    let schema = ifc::schema::ifc4();
    let mut model = model.clone();
    let item = Value::Enum("PARTITIONING".into());
    set_value(
        &mut model,
        schema,
        EntityId(17),
        &path("PredefinedType"),
        &item,
        ifc::fresh_seed().unwrap(),
    )
    .unwrap();
    assert_eq!(
        value(&model, schema, 17, "PredefinedType"),
        SelectedValue::Enum("PARTITIONING".into())
    );
    let refused = set_value(
        &mut model,
        schema,
        EntityId(17),
        &path("PredefinedType"),
        &Value::Enum("NOPE".into()),
        ifc::fresh_seed().unwrap(),
    );
    assert!(
        matches!(refused, Err(SelectorError::TypeMismatch { .. })),
        "{refused:?}"
    );
    // A reference must name an entity of the declared type.
    let wrong = Value::Ref(EntityId(52));
    let refused = set_value(
        &mut model,
        schema,
        EntityId(17),
        &path("OwnerHistory"),
        &wrong,
        ifc::fresh_seed().unwrap(),
    );
    assert!(
        matches!(refused, Err(SelectorError::TypeMismatch { .. })),
        "{refused:?}"
    );
    set_value(
        &mut model,
        schema,
        EntityId(17),
        &path("OwnerHistory"),
        &Value::Ref(EntityId(5)),
        ifc::fresh_seed().unwrap(),
    )
    .unwrap();
}

#[test]
fn paths_that_name_no_value_are_not_settable() {
    each_release(|name, _, model, schema| {
        let index = SelectorIndex::new(model, schema);
        let label = Value::Text("x".into());
        for text in ["count", "type", "/Na.*/", "type.count", "type.Name.x"] {
            let refused = index.stage_set_value(
                EntityId(19),
                &path(text),
                &label,
                ifc::fresh_seed().unwrap(),
            );
            assert!(
                matches!(refused, Err(SelectorError::NotSettable { .. })),
                "{name} {text:?}: {refused:?}"
            );
        }
        assert_eq!(
            index
                .set_targets(EntityId(17), &path("type.Pset_WallCommon.FireRating"))
                .unwrap(),
            [SetTarget::Property {
                object: EntityId(16),
                set: "Pset_WallCommon".into(),
                property: "FireRating".into(),
            }],
            "{name}"
        );
        assert_eq!(
            index.set_targets(EntityId(17), &path("type.Name")).unwrap(),
            [SetTarget::Attribute {
                entity: EntityId(16),
                name: "Name".into(),
            }],
            "{name}"
        );
    });
}

#[test]
fn property_writes_take_the_stated_type_or_a_typed_value() {
    each_release(|name, _, model, schema| {
        let mut model = model.clone();
        // A set no catalog describes: a typed value creates it.
        let note = Value::Typed {
            type_name: "IFCLABEL".into(),
            value: Box::new(Value::Text("checked".into())),
        };
        set_value(
            &mut model,
            schema,
            EntityId(19),
            &path("Custom_Notes.Note"),
            &note,
            ifc::fresh_seed().unwrap(),
        )
        .unwrap();
        assert_eq!(
            value(&model, schema, 19, "Custom_Notes.Note"),
            text("checked"),
            "{name}"
        );
        // Then a bare value takes the type the property states.
        let bare = Value::Text("rechecked".into());
        set_value(
            &mut model,
            schema,
            EntityId(19),
            &path("Custom_Notes.Note"),
            &bare,
            ifc::fresh_seed().unwrap(),
        )
        .unwrap();
        assert_eq!(
            value(&model, schema, 19, "Custom_Notes.Note"),
            text("rechecked"),
            "{name}"
        );
        // A bare value for a property nothing states is refused.
        let refused = set_value(
            &mut model,
            schema,
            EntityId(19),
            &path("Custom_Notes.Other"),
            &bare,
            ifc::fresh_seed().unwrap(),
        );
        assert!(
            matches!(refused, Err(SelectorError::UntypedValue { .. })),
            "{name}: {refused:?}"
        );
    });
}

#[cfg(feature = "property-catalog")]
#[test]
fn catalog_property_writes_override_the_type_s_value() {
    each_release(|name, _, model, schema| {
        let mut model = model.clone();
        let rating = Value::Text("F120".into());
        let at = path("Pset_WallCommon.FireRating");
        set_value(
            &mut model,
            schema,
            EntityId(17),
            &at,
            &rating,
            ifc::fresh_seed().unwrap(),
        )
        .unwrap();
        assert_eq!(
            value(&model, schema, 17, "Pset_WallCommon.FireRating"),
            text("F120"),
            "{name}"
        );
        assert_eq!(
            value(&model, schema, 16, "Pset_WallCommon.FireRating"),
            text("F90"),
            "{name}"
        );
        let external = path("Pset_WallCommon.IsExternal");
        set_value(
            &mut model,
            schema,
            EntityId(17),
            &external,
            &Value::Bool(false),
            ifc::fresh_seed().unwrap(),
        )
        .unwrap();
        assert_eq!(
            run(&model, schema, "Pset_WallCommon.IsExternal=TRUE").unwrap(),
            []
        );
    });
}

#[test]
fn the_guide_example_runs() -> Result<(), Box<dyn std::error::Error>> {
    let model = read("selector_ifc4.ifc");
    let schema = ifc::schema::ifc4();
    let mut printed = Vec::new();
    // docs:snippet selector-rust
    use ifc::selector::{set_value, Selector, SelectorIndex, ValuePath};

    // One index per model: each relationship and property scan runs once,
    // on first use, for every query and path after it.
    let index = SelectorIndex::new(&model, schema);
    let query = Selector::parse("IfcWall, Pset_WallCommon.FireRating=F90 + IfcDoor")?;
    let type_name = ValuePath::parse("type.Name")?;
    for id in index.select(&query)? {
        printed.push(format!("#{} {:?}", id.0, index.value(id, &type_name)?));
    }
    // #17 Text("WT-200"), #18 Text("WT-200"), #28 Text("D-90")

    // A refusal names the byte span of what it refuses.
    let error = Selector::parse("IfcWall, colour=red").unwrap_err();
    assert_eq!(error.span, 9..15);

    // Writes are checked against the release and staged on one transaction.
    let mut model = model.clone();
    let renamed = ifc::Value::Text("WT-300".into());
    set_value(
        &mut model,
        schema,
        EntityId(17),
        &type_name,
        &renamed,
        ifc::fresh_seed()?,
    )?;
    // docs:end
    assert_eq!(
        printed,
        [
            "#17 Text(\"WT-200\")",
            "#18 Text(\"WT-200\")",
            "#28 Text(\"D-90\")"
        ]
    );
    assert_eq!(value(&model, schema, 16, "Name"), text("WT-300"));
    Ok(())
}
