//! Checked property and quantity edits (#123): what each edit writes, where
//! it writes it, and that a refused batch writes nothing.
//!
//! Most cases use the redistributable `synthetic-properties` fixture (IFC4):
//!
//! ```text
//! #29 IfcWallType  HasPropertySets (#36 Pset_WallCommon: IsExternal .T., FireRating 'F30')
//! #30 Wall A       typed #29; #38 Pset_WallCommon (IsExternal .F.) via #39;
//!                  #62 Qto_WallBaseQuantities (Width 200 mm, ...) via #63
//! #31 Wall B       typed #29; #51 Pset_Families (Colour from enumeration #40, ...) via #52
//! ```
//!
//! Every edit is read back through the exact resolver the read side uses,
//! and the release cases (IFC2X3, IFC4X3) through a STEP round trip. The
//! fixture's sets are `Pset_`/`Qto_` sets, which are checked against the
//! catalog, so the cases that edit them need `property-catalog`; the gate
//! also runs this file without it, where such a set is refused.

#![cfg(all(feature = "step", feature = "properties"))]

use ifc::properties::{exact_properties, ExactPropertyError, ExactSource, ExactValue};
use ifc::{
    apply_property_edits, Codec, EntityId, Model, PropertyEdit, PropertyEditFailure, SetType,
    StepCodec, Value,
};

#[cfg_attr(not(feature = "property-catalog"), allow(dead_code))]
fn fixture() -> Model {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test/fixtures/synthetic-properties/synthetic_properties.ifc"
    );
    StepCodec
        .read_path(std::path::Path::new(path))
        .expect("fixture reads")
}

fn typed(type_name: &str, value: Value) -> Value {
    Value::Typed {
        type_name: type_name.into(),
        value: Box::new(value),
    }
}

fn label(text: &str) -> Value {
    typed("IFCLABEL", Value::Text(text.into()))
}

fn bytes(model: &Model) -> Vec<u8> {
    StepCodec.write_bytes(model).expect("writes")
}

/// `(source, set id, value)` of `object`'s `set.name`, read exactly.
fn read(
    model: &Model,
    object: u64,
    set: &str,
    name: &str,
) -> Option<(ExactSource, EntityId, ExactValue)> {
    exact_properties(model, EntityId(object))
        .expect("resolves")
        .into_iter()
        .find(|entry| &*entry.property.property_set == set && &*entry.name == name)
        .map(|entry| {
            (
                entry.property.source,
                entry.property.set_id,
                entry.property.value,
            )
        })
}

fn failure(model: &mut Model, edits: &[PropertyEdit]) -> (Option<usize>, PropertyEditFailure) {
    let before = bytes(model);
    let error = apply_property_edits(model, edits).expect_err("refused");
    assert_eq!(bytes(model), before, "a refused batch changed the model");
    (error.edit, error.failure)
}

#[cfg(feature = "property-catalog")]
#[test]
fn an_own_value_changes_in_place() {
    let mut model = fixture();
    let outcome = apply_property_edits(
        &mut model,
        &[PropertyEdit::set(
            EntityId(30),
            "Pset_WallCommon",
            "IsExternal",
            typed("IFCBOOLEAN", Value::Bool(true)),
        )],
    )
    .expect("applies");
    assert_eq!(outcome.properties, vec![Some(EntityId(37))]);
    assert!(outcome.created.is_empty());
    let (source, set, value) = read(&model, 30, "Pset_WallCommon", "IsExternal").unwrap();
    assert_eq!(
        (source, set, value),
        (
            ExactSource::Occurrence,
            EntityId(38),
            ExactValue::Bool(true)
        )
    );
}

#[cfg(feature = "property-catalog")]
#[test]
fn an_inherited_value_is_overridden_on_the_occurrence_never_on_the_type() {
    let mut model = fixture();
    let type_set = model.get(EntityId(36)).cloned();
    let type_value = model.get(EntityId(35)).cloned();
    // Wall A has its own Pset_WallCommon: the override joins it. Wall B has
    // none: a set and its relationship are created.
    let outcome = apply_property_edits(
        &mut model,
        &[
            PropertyEdit::set(EntityId(30), "Pset_WallCommon", "FireRating", label("F90")),
            PropertyEdit::set(EntityId(31), "Pset_WallCommon", "FireRating", label("F60")),
        ],
    )
    .expect("applies");
    assert_eq!(
        model.get(EntityId(36)).cloned(),
        type_set,
        "the type's set changed"
    );
    assert_eq!(
        model.get(EntityId(35)).cloned(),
        type_value,
        "the type's value changed"
    );

    let (source, set, value) = read(&model, 30, "Pset_WallCommon", "FireRating").unwrap();
    assert_eq!((source, set), (ExactSource::Occurrence, EntityId(38)));
    assert_eq!(value, ExactValue::Text("F90".into()));
    let (source, set, value) = read(&model, 31, "Pset_WallCommon", "FireRating").unwrap();
    assert_eq!(source, ExactSource::Occurrence);
    assert!(outcome.created.contains(&set), "a new set for wall B");
    assert_eq!(value, ExactValue::Text("F60".into()));
    // The other occurrence values and the inherited IsExternal still read.
    let (source, _, value) = read(&model, 31, "Pset_WallCommon", "IsExternal").unwrap();
    assert_eq!(
        (source, value),
        (ExactSource::Type(EntityId(29)), ExactValue::Bool(true))
    );
    // The new set and relationship carry the wall's owner history and a
    // valid, fresh GlobalId.
    let new_set = model.get(set).unwrap();
    assert_eq!(new_set.attributes[1], Value::Ref(EntityId(5)));
    let guid = new_set.attributes[0].as_text().unwrap();
    assert!(ifc_model::guid::Guid::parse(guid).is_some(), "{guid}");
}

#[cfg(feature = "property-catalog")]
#[test]
fn addressing_the_type_object_edits_its_own_set() {
    let mut model = fixture();
    apply_property_edits(
        &mut model,
        &[PropertyEdit::set(
            EntityId(29),
            "Pset_WallCommon",
            "FireRating",
            label("F120"),
        )],
    )
    .expect("applies");
    assert_eq!(
        model.get(EntityId(35)).unwrap().attributes[2],
        label("F120")
    );
    let (source, _, value) = read(&model, 31, "Pset_WallCommon", "FireRating").unwrap();
    assert_eq!(
        (source, value),
        (
            ExactSource::Type(EntityId(29)),
            ExactValue::Text("F120".into())
        )
    );
}

#[cfg(feature = "property-catalog")]
#[test]
fn quantities_are_written_bare_in_their_declared_measure() {
    let mut model = fixture();
    let outcome = apply_property_edits(
        &mut model,
        &[
            PropertyEdit::set(
                EntityId(30),
                "Qto_WallBaseQuantities",
                "Width",
                typed("IFCLENGTHMEASURE", Value::Real(250.0)),
            ),
            PropertyEdit::set(
                EntityId(30),
                "Qto_WallBaseQuantities",
                "Length",
                typed("IFCLENGTHMEASURE", Value::Real(5.0)),
            ),
        ],
    )
    .expect("applies");
    assert_eq!(outcome.properties[0], Some(EntityId(53)));
    assert_eq!(
        model.get(EntityId(53)).unwrap().attributes[3],
        Value::Real(250.0)
    );
    let length = outcome.properties[1].unwrap();
    let entity = model.get(length).unwrap();
    assert_eq!(&*entity.type_name, "IFCQUANTITYLENGTH");
    assert_eq!(entity.attributes[3], Value::Real(5.0));
    let (_, set, value) = read(&model, 30, "Qto_WallBaseQuantities", "Length").unwrap();
    assert_eq!((set, value), (EntityId(62), ExactValue::Real(5.0)));

    let (edit, refused) = failure(
        &mut model,
        &[PropertyEdit::set(
            EntityId(30),
            "Qto_WallBaseQuantities",
            "Width",
            typed("IFCAREAMEASURE", Value::Real(1.0)),
        )],
    );
    assert_eq!(edit, Some(0));
    assert!(
        matches!(refused, PropertyEditFailure::InvalidValue(_)),
        "{refused:?}"
    );
}

#[cfg(feature = "property-catalog")]
#[test]
fn removing_the_last_property_removes_the_set_and_its_relationship() {
    let mut model = fixture();
    let outcome = apply_property_edits(
        &mut model,
        &[PropertyEdit::remove(
            EntityId(30),
            "Pset_WallCommon",
            "IsExternal",
        )],
    )
    .expect("applies");
    assert_eq!(outcome.properties, vec![None]);
    let mut removed = outcome.removed.clone();
    removed.sort();
    assert_eq!(removed, vec![EntityId(37), EntityId(38), EntityId(39)]);
    // The inherited value shows through again.
    let (source, _, value) = read(&model, 30, "Pset_WallCommon", "IsExternal").unwrap();
    assert_eq!(
        (source, value),
        (ExactSource::Type(EntityId(29)), ExactValue::Bool(true))
    );
}

#[cfg(feature = "property-catalog")]
#[test]
fn an_inherited_property_is_not_removed_through_the_occurrence() {
    let mut model = fixture();
    let (edit, refused) = failure(
        &mut model,
        &[PropertyEdit::remove(
            EntityId(31),
            "Pset_WallCommon",
            "FireRating",
        )],
    );
    assert_eq!(edit, Some(0));
    assert_eq!(
        refused,
        PropertyEditFailure::MissingProperty {
            object: EntityId(31),
            set: "Pset_WallCommon".into(),
            name: "FireRating".into(),
            inherited_from: Some(EntityId(29)),
        }
    );
}

#[cfg(feature = "property-catalog")]
#[test]
fn an_enumerated_value_stays_in_its_enumeration() {
    let mut model = fixture();
    let green = Value::List(vec![label("green")]);
    apply_property_edits(
        &mut model,
        &[PropertyEdit::set(
            EntityId(31),
            "Pset_Families",
            "Colour",
            green.clone(),
        )],
    )
    .expect("green is in #40");
    assert_eq!(model.get(EntityId(42)).unwrap().attributes[2], green);
    let (_, refused) = failure(
        &mut model,
        &[PropertyEdit::set(
            EntityId(31),
            "Pset_Families",
            "Colour",
            Value::List(vec![label("blue")]),
        )],
    );
    assert!(
        matches!(refused, PropertyEditFailure::Template(_)),
        "{refused:?}"
    );
}

#[cfg(feature = "property-catalog")]
#[test]
fn values_the_release_does_not_admit_are_refused() {
    let mut model = fixture();
    let cases = [
        // A bare literal: IfcValue is a SELECT and needs its wrapper.
        Value::Real(0.3),
        // No IfcValue member.
        typed("IFCGLOBALLYUNIQUEID", Value::Text("x".into())),
        // A member with the wrong payload.
        typed("IFCLENGTHMEASURE", Value::Text("thick".into())),
        typed("IFCINTEGER", Value::Real(1.5)),
    ];
    for value in cases {
        let (_, refused) = failure(
            &mut model,
            &[PropertyEdit::set(
                EntityId(31),
                "Pset_Families",
                "Thickness",
                value.clone(),
            )],
        );
        assert!(
            matches!(refused, PropertyEditFailure::InvalidValue(_)),
            "{value:?}: {refused:?}"
        );
    }
    // A stated unit fixes the measure.
    let mut model = StepCodec.read_bytes(WITH_UNIT.as_bytes()).expect("parses");
    let (_, refused) = failure(
        &mut model,
        &[PropertyEdit::set(
            EntityId(1),
            "Custom",
            "Flow",
            typed("IFCLENGTHMEASURE", Value::Real(1.0)),
        )],
    );
    assert!(
        matches!(refused, PropertyEditFailure::InvalidValue(_)),
        "{refused:?}"
    );
    apply_property_edits(
        &mut model,
        &[PropertyEdit::set(
            EntityId(1),
            "Custom",
            "Flow",
            typed("IFCVOLUMETRICFLOWRATEMEASURE", Value::Real(0.1)),
        )],
    )
    .expect("the stated unit's measure");
}

/// A wall with a value in a stated unit.
#[cfg_attr(not(feature = "property-catalog"), allow(dead_code))]
const WITH_UNIT: &str = "ISO-10303-21;
HEADER;FILE_DESCRIPTION((''),'2;1');FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;
DATA;
#1=IFCWALL('2YvctVUKr0kugbFTf53O9L',$,'W1',$,$,$,$,$,$);
#2=IFCSIUNIT(*,.VOLUMEUNIT.,$,.CUBIC_METRE.);
#3=IFCSIUNIT(*,.TIMEUNIT.,$,.SECOND.);
#4=IFCDERIVEDUNITELEMENT(#2,1);
#5=IFCDERIVEDUNITELEMENT(#3,-1);
#6=IFCDERIVEDUNIT((#4,#5),.VOLUMETRICFLOWRATEUNIT.,$);
#10=IFCPROPERTYSINGLEVALUE('Flow',$,IFCVOLUMETRICFLOWRATEMEASURE(0.05),#6);
#11=IFCPROPERTYSET('3YvctVUKr0kugbFTf53O9L',$,'Custom',$,(#10));
#12=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O9L',$,$,$,(#1),#11);
ENDSEC;
END-ISO-10303-21;
";

#[cfg(feature = "property-catalog")]
#[test]
fn forms_this_writer_does_not_write_are_refused() {
    let mut model = fixture();
    let (_, refused) = failure(
        &mut model,
        &[PropertyEdit::set(
            EntityId(31),
            "Pset_Families",
            "Range",
            typed("IFCLENGTHMEASURE", Value::Real(1.0)),
        )],
    );
    assert!(
        matches!(refused, PropertyEditFailure::Unsupported(_)),
        "{refused:?}"
    );
    let (_, refused) = failure(
        &mut model,
        &[PropertyEdit::set(
            EntityId(30),
            "Pset_WallCommon",
            "IsExternal",
            typed("IFCBOOLEAN", Value::Bool(true)),
        )
        .with_set_type(SetType::ElementQuantity)],
    );
    assert!(
        matches!(refused, PropertyEditFailure::WrongSetType(_)),
        "{refused:?}"
    );
}

#[cfg(feature = "property-catalog")]
#[test]
fn a_refused_edit_anywhere_in_a_batch_writes_nothing() {
    let mut model = fixture();
    let revision = model.revision();
    let (edit, _) = failure(
        &mut model,
        &[
            PropertyEdit::set(
                EntityId(30),
                "Pset_WallCommon",
                "IsExternal",
                typed("IFCBOOLEAN", Value::Bool(true)),
            ),
            PropertyEdit::set(EntityId(31), "Custom", "Note", label("kept")),
            PropertyEdit::remove(EntityId(31), "Custom", "Missing"),
        ],
    );
    assert_eq!(edit, Some(2));
    assert_eq!(model.revision(), revision);
}

#[cfg(feature = "property-catalog")]
#[test]
fn a_later_edit_sees_the_earlier_ones() {
    let mut model = fixture();
    let outcome = apply_property_edits(
        &mut model,
        &[
            PropertyEdit::set(EntityId(31), "Custom", "A", label("a")),
            PropertyEdit::set(EntityId(31), "Custom", "B", label("b")),
            PropertyEdit::set(EntityId(31), "Custom", "A", label("a2")),
            PropertyEdit::remove(EntityId(31), "Custom", "B"),
        ],
    )
    .expect("applies");
    assert!(outcome.properties[0].is_some());
    assert_eq!(outcome.properties[0], outcome.properties[2]);
    assert_eq!(
        outcome.properties[1], None,
        "B was removed later in the batch"
    );
    assert_eq!(
        read(&model, 31, "Custom", "A").unwrap().2,
        ExactValue::Text("a2".into())
    );
    assert!(read(&model, 31, "Custom", "B").is_none());
}

/// Two walls related to one set by one relationship, and one property
/// entity shared by two sets.
const SHARED: &str = "ISO-10303-21;
HEADER;FILE_DESCRIPTION((''),'2;1');FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;
DATA;
#1=IFCWALL('2YvctVUKr0kugbFTf53O9L',$,'W1',$,$,$,$,$,$);
#2=IFCWALL('2YvctVUKr0kugbFTf53O9M',$,'W2',$,$,$,$,$,$);
#3=IFCWALL('2YvctVUKr0kugbFTf53O9N',$,'W3',$,$,$,$,$,$);
#10=IFCPROPERTYSINGLEVALUE('Note',$,IFCLABEL('shared'),$);
#11=IFCPROPERTYSET('3YvctVUKr0kugbFTf53O9L',$,'Custom',$,(#10));
#12=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O9L',$,$,$,(#1,#2),#11);
#13=IFCPROPERTYSET('3YvctVUKr0kugbFTf53O9M',$,'Custom',$,(#10));
#14=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O9M',$,$,$,(#3),#13);
ENDSEC;
END-ISO-10303-21;
";

#[test]
fn a_shared_set_or_property_is_copied_before_it_changes() {
    let mut model = StepCodec.read_bytes(SHARED.as_bytes()).expect("parses");
    apply_property_edits(
        &mut model,
        &[
            PropertyEdit::set(EntityId(1), "Custom", "Note", label("one")),
            PropertyEdit::set(EntityId(3), "Custom", "Note", label("three")),
        ],
    )
    .expect("applies");
    assert_eq!(
        model.get(EntityId(10)).unwrap().attributes[2],
        label("shared")
    );
    assert_eq!(
        read(&model, 2, "Custom", "Note").unwrap().2,
        ExactValue::Text("shared".into())
    );
    let (_, set, value) = read(&model, 1, "Custom", "Note").unwrap();
    assert_ne!(set, EntityId(11));
    assert_eq!(value, ExactValue::Text("one".into()));
    // #13 is wall 3's alone, so it is kept, and only its member is copied.
    let (_, set, value) = read(&model, 3, "Custom", "Note").unwrap();
    assert_eq!(
        (set, value),
        (EntityId(13), ExactValue::Text("three".into()))
    );
    assert_eq!(
        model.get(EntityId(12)).unwrap().attributes[4],
        Value::List(vec![Value::Ref(EntityId(2))])
    );
}

/// The same edits in IFC2X3 and IFC4X3, read back from STEP.
fn release_model(token: &str, owner_history: &str) -> Model {
    let text = format!(
        "ISO-10303-21;
HEADER;FILE_DESCRIPTION((''),'2;1');FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('{token}'));ENDSEC;
DATA;
#1=IFCPERSON($,'P',$,$,$,$,$,$);
#2=IFCORGANIZATION($,'O',$,$,$);
#3=IFCPERSONANDORGANIZATION(#1,#2,$);
#4=IFCAPPLICATION(#2,'1','A','A');
#5=IFCOWNERHISTORY(#3,#4,$,.ADDED.,$,$,$,0);
#7=IFCBUILDINGELEMENTPROXY('2YvctVUKr0kugbFTf53O9L',{owner_history},'P',$,$,$,$,$,$);
ENDSEC;
END-ISO-10303-21;
"
    );
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

#[test]
fn ifc2x3_and_ifc4x3_round_trip_through_step() {
    for token in ["IFC2X3", "IFC4X3_ADD2"] {
        let mut model = release_model(token, "#5");
        let outcome = apply_property_edits(
            &mut model,
            &[
                PropertyEdit::set(EntityId(7), "Custom", "Note", label("x")),
                PropertyEdit::set(
                    EntityId(7),
                    "Custom_Quantities",
                    "Depth",
                    typed("IFCLENGTHMEASURE", Value::Real(0.2)),
                )
                .with_set_type(SetType::ElementQuantity),
            ],
        )
        .unwrap_or_else(|error| panic!("{token}: {error}"));
        let text = StepCodec.write_bytes(&model).unwrap();
        let back = StepCodec.read_bytes(&text).unwrap();
        assert_eq!(
            read(&back, 7, "Custom", "Note").unwrap().2,
            ExactValue::Text("x".into()),
            "{token}"
        );
        assert_eq!(
            read(&back, 7, "Custom_Quantities", "Depth").unwrap().2,
            ExactValue::Real(0.2),
            "{token}"
        );
        // New records take the object's owner history, which IFC2X3 needs.
        for id in &outcome.created {
            let entity = back.get(*id).unwrap();
            if entity.type_name.starts_with("IFCREL")
                || entity.type_name.ends_with("SET")
                || &*entity.type_name == "IFCELEMENTQUANTITY"
            {
                assert_eq!(
                    entity.attributes[1],
                    Value::Ref(EntityId(5)),
                    "{token} {}",
                    entity.type_name
                );
            }
        }
    }
}

#[test]
fn ifc2x3_refuses_a_new_set_for_an_object_without_owner_history() {
    let mut model = release_model("IFC2X3", "$");
    let (_, refused) = failure(
        &mut model,
        &[PropertyEdit::set(EntityId(7), "Custom", "Note", label("x"))],
    );
    assert!(
        matches!(refused, PropertyEditFailure::InvalidModel(_)),
        "{refused:?}"
    );
}

#[test]
fn a_release_the_resolver_does_not_read_is_refused() {
    let mut model = release_model("IFC4X1", "$");
    let (edit, refused) = failure(
        &mut model,
        &[PropertyEdit::set(EntityId(7), "Custom", "Note", label("x"))],
    );
    assert_eq!(edit, None);
    assert!(
        matches!(
            refused,
            PropertyEditFailure::Resolve(ExactPropertyError::UnsupportedSchema { .. })
        ),
        "{refused:?}"
    );
}

#[cfg(feature = "property-catalog")]
#[test]
fn a_catalog_set_is_checked_against_its_template() {
    let mut model = fixture();
    // FireRating is an IfcLabel in Pset_WallCommon.
    let (_, refused) = failure(
        &mut model,
        &[PropertyEdit::set(
            EntityId(30),
            "Pset_WallCommon",
            "FireRating",
            typed("IFCREAL", Value::Real(1.0)),
        )],
    );
    assert!(
        matches!(refused, PropertyEditFailure::Template(_)),
        "{refused:?}"
    );
    // A property the template does not declare is not added.
    let (_, refused) = failure(
        &mut model,
        &[PropertyEdit::set(
            EntityId(30),
            "Pset_WallCommon",
            "Colour",
            label("red"),
        )],
    );
    assert!(
        matches!(refused, PropertyEditFailure::Template(_)),
        "{refused:?}"
    );
    // Status is enumerated: the value must be one of the template's.
    let (_, refused) = failure(
        &mut model,
        &[PropertyEdit::set(
            EntityId(30),
            "Pset_WallCommon",
            "Status",
            Value::List(vec![label("SOMEDAY")]),
        )],
    );
    assert!(
        matches!(refused, PropertyEditFailure::Template(_)),
        "{refused:?}"
    );
    let outcome = apply_property_edits(
        &mut model,
        &[PropertyEdit::set(
            EntityId(30),
            "Pset_WallCommon",
            "Status",
            Value::List(vec![label("NEW")]),
        )],
    )
    .expect("NEW is an element status");
    let status = model.get(outcome.properties[0].unwrap()).unwrap();
    assert_eq!(&*status.type_name, "IFCPROPERTYENUMERATEDVALUE");
    // A catalog quantity set is an IfcElementQuantity of the template's kinds.
    let (_, refused) = failure(
        &mut model,
        &[PropertyEdit::set(
            EntityId(31),
            "Qto_WallBaseQuantities",
            "Height",
            typed("IFCAREAMEASURE", Value::Real(1.0)),
        )],
    );
    assert!(
        matches!(refused, PropertyEditFailure::Template(_)),
        "{refused:?}"
    );
}

#[cfg(not(feature = "property-catalog"))]
#[test]
fn a_catalog_set_is_refused_without_the_catalog() {
    let mut model = fixture();
    let (_, refused) = failure(
        &mut model,
        &[PropertyEdit::set(
            EntityId(30),
            "Pset_WallCommon",
            "FireRating",
            label("F90"),
        )],
    );
    assert_eq!(
        refused,
        PropertyEditFailure::CatalogUnavailable("Pset_WallCommon".into())
    );
    // The object is checked first.
    let (_, refused) = failure(
        &mut model,
        &[PropertyEdit::set(
            EntityId(999),
            "Pset_WallCommon",
            "FireRating",
            label("F90"),
        )],
    );
    assert_eq!(refused, PropertyEditFailure::MissingEntity(EntityId(999)));
    // Removal needs no template.
    apply_property_edits(
        &mut model,
        &[PropertyEdit::remove(
            EntityId(30),
            "Pset_WallCommon",
            "IsExternal",
        )],
    )
    .expect("removes");
}
