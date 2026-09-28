//! Exact enumeration of an object's property sets, empty ones included (#186).
//!
//! An IDS property facet must fail on a matching set that holds no
//! properties, so the set's existence has to be observable on its own:
//! `exact_properties_where` lists properties and has nothing to say about a
//! set without any. `exact_property_sets_where` lists the sets, with the same
//! traversal and refusals. Models are IFC4 ADD2 TC1 STEP text.

use ifc_model::{Codec, EntityId, Model};
use ifc_properties::{
    exact_properties_where, exact_property_sets_where, ExactPropertyError, ExactPropertySetEntry,
    ExactSource,
};
use ifc_step::StepCodec;

fn ifc4(records: &[&str]) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\n\
         DATA;\n{}\nENDSEC;\nEND-ISO-10303-21;\n",
        records.join("\n")
    );
    let model = StepCodec
        .read_bytes(text.as_bytes())
        .unwrap_or_else(|e| panic!("fixture must parse: {e:?}"));
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

const WALL: EntityId = EntityId(1);
const WALL_RECORD: &str = "#1=IFCWALL('1xS3BCk291UvhgP2dvNsgp',$,'Wall',$,$,$,$,$,$);";

/// Wall #1 with, on the occurrence, `Pset_WallCommon` #20 given as
/// `members` and `Qto_WallBase` #25 holding one quantity.
fn wall(members: &str, extra: &[&str]) -> Model {
    let mut records = vec![
        WALL_RECORD.to_owned(),
        format!("#20=IFCPROPERTYSET('0000000000000000000020',$,'Pset_WallCommon',$,{members});"),
        "#21=IFCPROPERTYSINGLEVALUE('FireRating',$,IFCLABEL('F30'),$);".to_owned(),
        "#22=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$);".to_owned(),
        "#26=IFCQUANTITYLENGTH('Length',$,$,4.5,$);".to_owned(),
        "#25=IFCELEMENTQUANTITY('0000000000000000000025',$,'Qto_WallBase',$,$,(#26));".to_owned(),
        "#30=IFCRELDEFINESBYPROPERTIES('0000000000000000000030',$,$,$,(#1),#20);".to_owned(),
        "#31=IFCRELDEFINESBYPROPERTIES('0000000000000000000031',$,$,$,(#1),#25);".to_owned(),
    ];
    records.extend(extra.iter().map(|record| (*record).to_owned()));
    let refs: Vec<&str> = records.iter().map(String::as_str).collect();
    ifc4(&refs)
}

fn names(entries: &[ExactPropertySetEntry]) -> Vec<(&str, usize, ExactSource)> {
    entries
        .iter()
        .map(|entry| (entry.name.as_ref(), entry.members, entry.source))
        .collect()
}

/// The issue's acceptance case: an empty `Pset_WallCommon` is listed by
/// name with no members, where the property enumeration refuses it.
#[test]
fn an_empty_matching_set_is_listed_with_no_members() {
    let model = wall("()", &[]);
    let sets = exact_property_sets_where(&model, WALL, |s| s == "Pset_WallCommon").unwrap();
    assert_eq!(
        names(&sets),
        [("Pset_WallCommon", 0, ExactSource::Occurrence)]
    );
    assert_eq!(sets[0].set_id, EntityId(20));
    // `HasProperties` is SET [1:?]: the property view refuses what the set
    // view reports as existing but empty.
    assert!(matches!(
        exact_properties_where(&model, WALL, |s| s == "Pset_WallCommon", |_| true),
        Err(ExactPropertyError::MalformedAggregate {
            attribute: "HasProperties",
            ..
        })
    ));
}

/// An unset member list is as empty as `()`.
#[test]
fn an_unset_member_list_is_an_empty_set() {
    let model = wall("$", &[]);
    let sets = exact_property_sets_where(&model, WALL, |s| s == "Pset_WallCommon").unwrap();
    assert_eq!(
        names(&sets),
        [("Pset_WallCommon", 0, ExactSource::Occurrence)]
    );
}

/// Property sets and quantity sets both, with their member counts, and a
/// selector that picks nothing is a proven absence.
#[test]
fn populated_sets_report_their_member_counts() {
    let model = wall("(#21,#22)", &[]);
    let all = exact_property_sets_where(&model, WALL, |_| true).unwrap();
    assert_eq!(
        names(&all),
        [
            ("Pset_WallCommon", 2, ExactSource::Occurrence),
            ("Qto_WallBase", 1, ExactSource::Occurrence),
        ]
    );
    let none = exact_property_sets_where(&model, WALL, |s| s == "Pset_Missing").unwrap();
    assert!(none.is_empty());
}

/// Type sets follow the occurrence's, and a type set sharing an occurrence
/// set's name is still listed: overriding happens per property.
#[test]
fn type_sets_follow_and_are_not_hidden_by_a_same_named_occurrence_set() {
    let model = wall(
        "(#21)",
        &[
            "#2=IFCWALLTYPE('0000000000000000000002',$,'WT',$,$,(#40,#43),$,$,$,.STANDARD.);",
            "#3=IFCRELDEFINESBYTYPE('0000000000000000000003',$,$,$,(#1),#2);",
            "#41=IFCPROPERTYSINGLEVALUE('LoadBearing',$,IFCBOOLEAN(.F.),$);",
            "#40=IFCPROPERTYSET('0000000000000000000040',$,'Pset_WallCommon',$,(#41));",
            "#43=IFCPROPERTYSET('0000000000000000000043',$,'Pset_Acoustic',$,());",
        ],
    );
    let sets = exact_property_sets_where(&model, WALL, |s| s.starts_with("Pset_")).unwrap();
    let t = ExactSource::Type(EntityId(2));
    assert_eq!(
        names(&sets),
        [
            ("Pset_WallCommon", 1, ExactSource::Occurrence),
            ("Pset_WallCommon", 1, t),
            ("Pset_Acoustic", 0, t),
        ]
    );
}

/// Two selected sets of one name on one source are ambiguous, as for the
/// property enumeration.
#[test]
fn two_selected_sets_of_one_name_are_refused() {
    let model = wall(
        "(#21)",
        &[
            "#50=IFCPROPERTYSET('0000000000000000000050',$,'Pset_WallCommon',$,(#22));",
            "#51=IFCRELDEFINESBYPROPERTIES('0000000000000000000051',$,$,$,(#1),#50);",
        ],
    );
    assert!(matches!(
        exact_property_sets_where(&model, WALL, |s| s == "Pset_WallCommon"),
        Err(ExactPropertyError::DuplicateMatchingSets { .. })
    ));
}

/// A selected set's members are validated; an unselected set is not read.
#[test]
fn a_broken_member_refuses_only_when_its_set_is_selected() {
    let model = wall("(#21,#99)", &[]);
    let refused = exact_property_sets_where(&model, WALL, |s| s == "Pset_WallCommon");
    assert!(refused.is_err(), "{refused:?}");
    let other = exact_property_sets_where(&model, WALL, |s| s == "Qto_WallBase").unwrap();
    assert_eq!(
        names(&other),
        [("Qto_WallBase", 1, ExactSource::Occurrence)]
    );
}
