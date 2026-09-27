//! Exact enumeration of an object's properties (#78).
//!
//! `exact_properties` and `exact_properties_where` must answer what
//! `exact_property` answers, for every property at once: the same provenance,
//! the same occurrence-over-type override, the same refusals, and an empty
//! result only as a proven absence. Models are IFC4 ADD2 TC1 STEP text.

mod common;

use std::sync::Arc;

use ifc_model::{Codec, EntityId, Model};
use ifc_properties::{
    exact_properties, exact_properties_where, exact_property, property_sets_by_object,
    quantity_sets, ExactPropertyEntry, ExactPropertyError, ExactResolution, ExactSource,
    ExactValue,
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

const WALL: &str = "#1=IFCWALL('1xS3BCk291UvhgP2dvNsgp',$,'Wall',$,$,$,$,$,$);";
const WALL_TYPE: &str =
    "#2=IFCWALLTYPE('0000000000000000000002',$,'WT',$,$,(#40,#43),$,$,$,.STANDARD.);";
const TYPED: &str = "#3=IFCRELDEFINESBYTYPE('0000000000000000000003',$,$,$,(#1),#2);";

fn single(id: u64, name: &str, value: &str) -> String {
    format!("#{id}=IFCPROPERTYSINGLEVALUE('{name}',$,{value},$);")
}

fn pset(id: u64, name: &str, members: &[u64]) -> String {
    let refs: Vec<String> = members.iter().map(|m| format!("#{m}")).collect();
    format!(
        "#{id}=IFCPROPERTYSET('{id:0>22}',$,'{name}',$,({}));",
        refs.join(",")
    )
}

fn qset(id: u64, name: &str, members: &[u64]) -> String {
    let refs: Vec<String> = members.iter().map(|m| format!("#{m}")).collect();
    format!(
        "#{id}=IFCELEMENTQUANTITY('{id:0>22}',$,'{name}',$,$,({}));",
        refs.join(",")
    )
}

fn defines(id: u64, objects: &[u64], definition: u64) -> String {
    let refs: Vec<String> = objects.iter().map(|o| format!("#{o}")).collect();
    format!(
        "#{id}=IFCRELDEFINESBYPROPERTIES('{id:0>22}',$,$,$,({}),#{definition});",
        refs.join(",")
    )
}

/// A wall with occurrence and type properties and quantities:
///
/// ```text
/// occurrence  Pset_WallCommon #20   FireRating='F30' #21, IsExternal=T #22
///             Qto_WallBase    #25   Length=4.5 #26
/// type #2     Pset_WallCommon #40   FireRating='F90' #41, LoadBearing=F #42
///             Pset_Acoustic   #43   FireRating='n/a' #44
/// ```
fn wall_model(extra: &[&str]) -> Model {
    let mut records = vec![
        WALL.to_owned(),
        WALL_TYPE.to_owned(),
        TYPED.to_owned(),
        single(21, "FireRating", "IFCLABEL('F30')"),
        single(22, "IsExternal", "IFCBOOLEAN(.T.)"),
        pset(20, "Pset_WallCommon", &[21, 22]),
        defines(23, &[1], 20),
        "#26=IFCQUANTITYLENGTH('Length',$,$,4.5,$);".to_owned(),
        qset(25, "Qto_WallBase", &[26]),
        defines(27, &[1], 25),
        single(41, "FireRating", "IFCLABEL('F90')"),
        single(42, "LoadBearing", "IFCBOOLEAN(.F.)"),
        pset(40, "Pset_WallCommon", &[41, 42]),
        single(44, "FireRating", "IFCLABEL('n/a')"),
        pset(43, "Pset_Acoustic", &[44]),
    ];
    records.extend(extra.iter().map(|r| (*r).to_owned()));
    let refs: Vec<&str> = records.iter().map(String::as_str).collect();
    ifc4(&refs)
}

/// `(set, name, source, property id)` of each entry, in result order.
fn summary(entries: &[ExactPropertyEntry]) -> Vec<(String, String, ExactSource, EntityId)> {
    entries
        .iter()
        .map(|entry| {
            (
                entry.property.property_set.to_string(),
                entry.name.to_string(),
                entry.property.source,
                entry.property.property_id,
            )
        })
        .collect()
}

#[test]
fn every_property_is_listed_with_provenance_and_occurrence_override() {
    let entries = exact_properties(&wall_model(&[]), EntityId(1)).expect("enumerates");
    let occurrence = ExactSource::Occurrence;
    let typed = ExactSource::Type(EntityId(2));
    let row = |set: &str, name: &str, source, id| (set.into(), name.into(), source, EntityId(id));
    assert_eq!(
        summary(&entries),
        [
            row("Pset_WallCommon", "FireRating", occurrence, 21),
            row("Pset_WallCommon", "IsExternal", occurrence, 22),
            row("Qto_WallBase", "Length", occurrence, 26),
            // #41 is overridden by #21: same set name, same property name.
            row("Pset_WallCommon", "LoadBearing", typed, 42),
            // Another set's FireRating is a different property.
            row("Pset_Acoustic", "FireRating", typed, 44),
        ]
    );
    let length = &entries[2].property;
    assert_eq!(length.value_type.as_deref(), Some("IFCLENGTHMEASURE"));
    assert_eq!(length.value, ExactValue::Real(4.5));
    assert_eq!(entries[3].property.value, ExactValue::Bool(false));
}

#[test]
fn a_single_selected_name_answers_exactly_what_exact_property_answers() {
    let mut models = vec![wall_model(&[]), common::fixture()];
    // A fixture with refusals in it, so errors are compared too.
    models.push(wall_model(&[
        "#50=IFCPROPERTYENUMERATEDVALUE('Status',$,(IFCLABEL('NEW')),$);",
        &pset(51, "Pset_Status", &[50, 52, 52]),
        &single(52, "Grade", "IFCLABEL('A')"),
        &defines(53, &[1], 51),
    ]));
    let mut compared = 0;
    for model in &models {
        let (by_object, _) = property_sets_by_object(model);
        let (quantity_sets, _) = quantity_sets(model);
        let mut names: Vec<(String, String)> = by_object
            .values()
            .flatten()
            .flat_map(|(_, set)| {
                let set_name = set.name.as_deref().unwrap_or_default().to_owned();
                set.properties.iter().filter_map(move |p| {
                    p.name.as_deref().map(|n| (set_name.clone(), n.to_owned()))
                })
            })
            .collect();
        for set in &quantity_sets {
            for quantity in &set.quantities {
                if let (Some(set_name), Some(name)) = (set.name.as_deref(), quantity.name()) {
                    names.push((set_name.to_owned(), name.to_owned()));
                }
            }
        }
        names.push(("Pset_WallCommon".into(), "NoSuchProperty".into()));
        names.push(("Pset_NoSuchSet".into(), "FireRating".into()));
        for object in model.ids_of_type("IFCWALL") {
            for (set_name, name) in &names {
                let single = exact_property(model, *object, Some(set_name), name);
                let listed = exact_properties_where(
                    model,
                    *object,
                    |set| set == set_name,
                    |property| property == name,
                );
                match (single, listed) {
                    (Ok(ExactResolution::Present(one)), Ok(entries)) => {
                        assert_eq!(entries.len(), 1, "{set_name}.{name}");
                        assert_eq!(entries[0].property, one, "{set_name}.{name}");
                        assert_eq!(entries[0].name.as_ref(), name.as_str());
                    }
                    (Ok(ExactResolution::Absent), Ok(entries)) => {
                        assert!(entries.is_empty(), "{set_name}.{name}: {entries:?}");
                    }
                    (Err(single), Err(listed)) => assert_eq!(single, listed, "{set_name}.{name}"),
                    (single, listed) => {
                        panic!("{set_name}.{name}: {single:?} but enumerated {listed:?}")
                    }
                }
                compared += 1;
            }
        }
    }
    assert!(compared >= 20, "only {compared} comparisons ran");
}

#[test]
fn an_empty_result_is_a_proven_absence() {
    let bare = ifc4(&[WALL]);
    assert_eq!(exact_properties(&bare, EntityId(1)), Ok(Vec::new()));
    let none = exact_properties_where(
        &wall_model(&[]),
        EntityId(1),
        |_| true,
        |name| name == "NoSuchProperty",
    );
    assert_eq!(none, Ok(Vec::new()));
    let no_set = exact_properties_where(&wall_model(&[]), EntityId(1), |_| false, |_| true);
    assert_eq!(no_set, Ok(Vec::new()));
}

#[test]
fn an_unsupported_member_refuses_only_when_it_is_selected() {
    let m = wall_model(&[
        "#50=IFCPROPERTYENUMERATEDVALUE('Status',$,(IFCLABEL('NEW')),$);",
        &pset(51, "Pset_Status", &[50]),
        &defines(52, &[1], 51),
    ]);
    let refused = Err(ExactPropertyError::UnsupportedProperty {
        entity: EntityId(50),
        type_name: "IFCPROPERTYENUMERATEDVALUE".into(),
    });
    assert_eq!(exact_properties(&m, EntityId(1)), refused);
    let entries = exact_properties_where(&m, EntityId(1), |set| set != "Pset_Status", |_| true)
        .expect("the set of the enumerated value is not selected");
    assert_eq!(entries.len(), 5);
    let entries = exact_properties_where(&m, EntityId(1), |_| true, |name| name != "Status")
        .expect("the enumerated value is not selected");
    assert_eq!(entries.len(), 5);
}

#[test]
fn a_complex_quantity_refuses_only_when_it_is_selected() {
    let m = wall_model(&[
        "#60=IFCPHYSICALCOMPLEXQUANTITY('Layer',$,(#61),'layer',$,$);",
        "#61=IFCQUANTITYLENGTH('Width',$,$,0.2,$);",
        &qset(62, "Qto_Layers", &[60]),
        &defines(63, &[1], 62),
    ]);
    assert_eq!(
        exact_properties(&m, EntityId(1)),
        Err(ExactPropertyError::UnsupportedProperty {
            entity: EntityId(60),
            type_name: "IFCPHYSICALCOMPLEXQUANTITY".into(),
        })
    );
    assert!(exact_properties_where(&m, EntityId(1), |_| true, |name| name != "Layer").is_ok());
}

#[test]
fn every_member_of_a_selected_set_is_validated() {
    // A dangling member refuses even when only another member is selected:
    // it could have carried any name. A set whose name is not selected is
    // skipped unread, as `exact_property` skips a set of another name.
    let m = wall_model(&[
        &single(72, "Grade", "IFCLABEL('A')"),
        &pset(70, "Pset_Other", &[72, 99]),
        &defines(71, &[1], 70),
    ]);
    let refused = Err(ExactPropertyError::MissingReference {
        from: EntityId(70),
        to: EntityId(99),
    });
    assert_eq!(
        exact_properties_where(&m, EntityId(1), |_| true, |name| name == "Grade"),
        refused
    );
    assert_eq!(
        exact_property(&m, EntityId(1), None, "Grade").map(|_| ()),
        refused.map(|_: Vec<ExactPropertyEntry>| ())
    );
    let skipped = exact_properties_where(&m, EntityId(1), |set| set != "Pset_Other", |_| true)
        .expect("Pset_Other is not selected");
    assert_eq!(skipped.len(), 5);
}

#[test]
fn a_predefined_set_refuses_when_one_of_its_own_attributes_is_selected() {
    // IfcDoorLiningProperties (IFC4): IfcRoot's four attributes, then
    // LiningDepth, LiningThickness, ... (17 in all), carried by the wall.
    let lining = "#80=IFCDOORLININGPROPERTIES('0000000000000000000080',$,$,$,\
                  0.1,$,$,$,$,$,$,$,$,$,$,$,$);";
    let m = wall_model(&[lining, &defines(81, &[1], 80)]);
    let refused = Err(ExactPropertyError::UnsupportedDefinition {
        entity: EntityId(80),
        type_name: "IFCDOORLININGPROPERTIES".into(),
    });
    assert_eq!(exact_properties(&m, EntityId(1)), refused);
    // Only the set's own attributes are offered, not those of `IfcRoot`.
    let mut offered = Vec::new();
    let result = exact_properties_where(
        &m,
        EntityId(1),
        |_| true,
        |name| {
            offered.push(name.to_owned());
            name == "FireRating"
        },
    );
    assert_eq!(result.map(|entries| entries.len()), Ok(2));
    assert!(offered.iter().any(|name| name == "LiningDepth"));
    assert!(!offered
        .iter()
        .any(|name| name == "Name" || name == "GlobalId"));
    // The set states no Name, so a set selector cannot rule it out.
    assert_eq!(
        exact_properties_where(
            &m,
            EntityId(1),
            |set| set == "Pset_WallCommon",
            |name| name == "LiningDepth"
        ),
        refused
    );
    // A named predefined set outside the set selection is skipped.
    let named = "#80=IFCDOORLININGPROPERTIES('0000000000000000000080',$,'Lining',$,\
                 0.1,$,$,$,$,$,$,$,$,$,$,$,$);";
    let m = wall_model(&[named, &defines(81, &[1], 80)]);
    assert_eq!(
        exact_properties_where(
            &m,
            EntityId(1),
            |set| set == "Pset_WallCommon",
            |name| name == "LiningDepth"
        ),
        Ok(Vec::new())
    );
    assert_eq!(
        exact_properties_where(&m, EntityId(1), |_| true, |name| name == "LiningDepth"),
        refused
    );
}

#[test]
fn ambiguity_among_selected_sets_and_members_is_refused_per_source() {
    // A second occurrence set named Pset_WallCommon.
    let m = wall_model(&[
        &single(90, "Colour", "IFCLABEL('red')"),
        &pset(91, "Pset_WallCommon", &[90]),
        &defines(92, &[1], 91),
    ]);
    assert_eq!(
        exact_properties(&m, EntityId(1)),
        Err(ExactPropertyError::DuplicateMatchingSets {
            source: ExactSource::Occurrence,
            first: EntityId(20),
            second: EntityId(91),
        })
    );
    // Neither same-named set is selected: nothing is ambiguous.
    let entries = exact_properties_where(&m, EntityId(1), |set| set == "Qto_WallBase", |_| true)
        .expect("Pset_WallCommon is not selected");
    assert_eq!(entries.len(), 1);
    // The type's own Pset_WallCommon is another source, never ambiguous
    // with the occurrence's: the base fixture has one on each.
    assert!(exact_properties(&wall_model(&[]), EntityId(1)).is_ok());

    // A property set and a quantity set of one name are ambiguous too.
    let mixed = wall_model(&[
        "#93=IFCQUANTITYAREA('Area',$,$,9.,$);",
        &qset(94, "Pset_WallCommon", &[93]),
        &defines(95, &[1], 94),
    ]);
    assert!(matches!(
        exact_properties(&mixed, EntityId(1)),
        Err(ExactPropertyError::DuplicateMatchingSets { .. })
    ));

    // Two selected members of one name in one set.
    let doubled = wall_model(&[
        &single(96, "Grade", "IFCLABEL('A')"),
        &single(97, "Grade", "IFCLABEL('B')"),
        &pset(98, "Pset_Grades", &[96, 97]),
        &defines(99, &[1], 98),
    ]);
    assert_eq!(
        exact_properties(&doubled, EntityId(1)),
        Err(ExactPropertyError::DuplicateMatchingProperties {
            set: EntityId(98),
            first: EntityId(96),
            second: EntityId(97),
        })
    );
    assert!(
        exact_properties_where(&doubled, EntityId(1), |_| true, |name| name != "Grade").is_ok()
    );
}

#[test]
fn model_level_refusals_are_those_of_exact_property() {
    let mut unsupported = ifc4(&[WALL]);
    unsupported.header_mut().schema = vec!["IFC2X2_FINAL".into()];
    assert_eq!(
        exact_properties(&unsupported, EntityId(1)),
        Err(ExactPropertyError::UnsupportedSchema {
            schema: "IFC2X2_FINAL".into(),
        })
    );
    let m = wall_model(&[]);
    assert_eq!(
        exact_properties(&m, EntityId(2)),
        Err(ExactPropertyError::InvalidQueryObject {
            object: EntityId(2),
            type_name: Arc::from("IFCWALLTYPE"),
        })
    );
}
