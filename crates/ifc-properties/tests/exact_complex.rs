//! Complex properties and quantities as present composites (#208).
//!
//! `exact_property` refused a set as soon as the asked-for member was an
//! `IfcComplexProperty` or `IfcPhysicalComplexQuantity`, so a checker could
//! not tell "present, but no simple value" from "unreadable". The
//! buildingSMART IDS case `fail-complex_properties_are_not_supported_1_2`
//! needs the former: a complex quantity `Foo` that is present and of no
//! data type. Complex members now resolve as `ExactValue::Complex`, with
//! cycles, depth and budget still refused.

use ifc_model::{Codec, EntityId, Model};
use ifc_properties::{
    exact_properties, exact_property, ExactComplexValue, ExactPropertyError, ExactResolution,
    ExactValue,
};
use ifc_step::StepCodec;

const WALL: EntityId = EntityId(7);

fn parse(schema: &str, records: &[String]) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\n\
         DATA;\n{}\nENDSEC;\nEND-ISO-10303-21;\n",
        records.join("\n")
    );
    let model = StepCodec
        .read_bytes(text.as_bytes())
        .unwrap_or_else(|e| panic!("fixture must parse: {e:?}"));
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

/// A wall (#7) whose set #8 of `kind` (`PROPERTYSET` or `ELEMENTQUANTITY`)
/// holds `members`, with `records` defining them. IFC2X3 walls have one
/// attribute fewer.
fn wall(schema: &str, kind: &str, members: &[u64], records: &[&str]) -> Model {
    let wall = if schema == "IFC2X3" {
        "#7=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,$,$,$,$,$,$);"
    } else {
        "#7=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,$,$,$,$,$,$,$);"
    };
    let refs: Vec<String> = members.iter().map(|m| format!("#{m}")).collect();
    let set = match kind {
        "PROPERTYSET" => format!(
            "#8=IFCPROPERTYSET('0YvctVUKr0kugbFTf53O08',$,'Foo_Bar',$,({}));",
            refs.join(",")
        ),
        _ => format!(
            "#8=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O08',$,'Foo_Bar',$,$,({}));",
            refs.join(",")
        ),
    };
    let mut all = vec![
        wall.to_owned(),
        set,
        "#9=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O09',$,$,$,(#7),#8);".to_owned(),
    ];
    all.extend(records.iter().map(|record| (*record).to_owned()));
    parse(schema, &all)
}

fn complex(result: Result<ExactResolution, ExactPropertyError>) -> ExactComplexValue {
    match result {
        Ok(ExactResolution::Present(property)) => {
            assert_eq!(property.value_type, None, "a complex has no value type");
            assert_eq!(property.unit_id, None);
            match property.value {
                ExactValue::Complex(value) => value,
                other => panic!("expected a complex value, got {other:?}"),
            }
        }
        other => panic!("expected a present property, got {other:?}"),
    }
}

/// The IDS corpus case, in each release: a complex quantity `Foo` in the
/// quantity set `Foo_Bar` is present, untyped, and lists its members.
#[test]
fn the_ids_complex_quantity_is_present_in_every_release() {
    for (schema, length) in [
        ("IFC2X3", "#11=IFCQUANTITYLENGTH('Width',$,$,0.2);"),
        ("IFC4", "#11=IFCQUANTITYLENGTH('Width',$,$,0.2,$);"),
        ("IFC4X3_ADD2", "#11=IFCQUANTITYLENGTH('Width',$,$,0.2,$);"),
    ] {
        let model = wall(
            schema,
            "ELEMENTQUANTITY",
            &[10],
            &[
                "#10=IFCPHYSICALCOMPLEXQUANTITY('Foo',$,(#11),'Layer','Good',$);",
                length,
            ],
        );
        for set in [Some("Foo_Bar"), None] {
            let found = complex(exact_property(&model, WALL, set, "Foo"));
            assert_eq!(found.discrimination.as_deref(), Some("Layer"), "{schema}");
            assert_eq!(found.quality.as_deref(), Some("Good"));
            assert_eq!(found.usage, None);
            let [member] = found.members.as_slice() else {
                panic!("one member expected: {found:?}");
            };
            assert_eq!(&*member.name, "Width");
            assert_eq!(member.id, EntityId(11));
            assert_eq!(member.value_type.as_deref(), Some("IFCLENGTHMEASURE"));
            assert_eq!(member.value, ExactValue::Real(0.2));
        }
    }
}

#[test]
fn a_complex_property_is_present_in_every_release() {
    for schema in ["IFC2X3", "IFC4", "IFC4X3_ADD2"] {
        let model = wall(
            schema,
            "PROPERTYSET",
            &[10, 12],
            &[
                "#10=IFCCOMPLEXPROPERTY('Foo',$,'Grouping',(#11));",
                "#11=IFCPROPERTYSINGLEVALUE('Inner',$,IFCLABEL('NEW'),$);",
                "#12=IFCPROPERTYSINGLEVALUE('Plain',$,IFCINTEGER(3),$);",
            ],
        );
        let found = complex(exact_property(&model, WALL, Some("Foo_Bar"), "Foo"));
        assert_eq!(found.usage.as_deref(), Some("Grouping"), "{schema}");
        assert_eq!(found.discrimination, None);
        assert_eq!(found.members.len(), 1);
        assert_eq!(&*found.members[0].name, "Inner");
        assert_eq!(found.members[0].value_type.as_deref(), Some("IFCLABEL"));
        assert_eq!(found.members[0].value, ExactValue::Text("NEW".into()));

        // The simple sibling reads as before, and the whole set enumerates.
        let Ok(ExactResolution::Present(plain)) =
            exact_property(&model, WALL, Some("Foo_Bar"), "Plain")
        else {
            panic!("the simple member resolves");
        };
        assert_eq!(plain.value, ExactValue::Integer(3));
        assert_eq!(exact_properties(&model, WALL).map(|all| all.len()), Ok(2));
    }
}

#[test]
fn nested_complexes_resolve_recursively() {
    let model = wall(
        "IFC4",
        "PROPERTYSET",
        &[10],
        &[
            "#10=IFCCOMPLEXPROPERTY('Foo',$,'Outer',(#11,#12));",
            "#11=IFCCOMPLEXPROPERTY('Nested',$,'Inner',(#13));",
            "#12=IFCPROPERTYLISTVALUE('List',$,(IFCINTEGER(1),IFCINTEGER(2)),$);",
            "#13=IFCPROPERTYSINGLEVALUE('Leaf',$,IFCBOOLEAN(.T.),$);",
        ],
    );
    let found = complex(exact_property(&model, WALL, None, "Foo"));
    let [nested, list] = found.members.as_slice() else {
        panic!("two members expected: {found:?}");
    };
    assert_eq!(nested.value_type, None);
    let ExactValue::Complex(inner) = &nested.value else {
        panic!("a nested complex: {nested:?}");
    };
    assert_eq!(inner.usage.as_deref(), Some("Inner"));
    assert_eq!(inner.members[0].value, ExactValue::Bool(true));
    assert!(matches!(&list.value, ExactValue::List(values) if values.len() == 2));
}

#[test]
fn a_cycle_is_refused_not_followed() {
    let direct = wall(
        "IFC4",
        "PROPERTYSET",
        &[10],
        &["#10=IFCCOMPLEXPROPERTY('Foo',$,'Loop',(#10));"],
    );
    assert_eq!(
        exact_property(&direct, WALL, None, "Foo"),
        Err(ExactPropertyError::ComplexCycle {
            complex: EntityId(10),
            member: EntityId(10),
        })
    );
    let longer = wall(
        "IFC4",
        "ELEMENTQUANTITY",
        &[10],
        &[
            "#10=IFCPHYSICALCOMPLEXQUANTITY('Foo',$,(#11),'A',$,$);",
            "#11=IFCPHYSICALCOMPLEXQUANTITY('Bar',$,(#10),'B',$,$);",
        ],
    );
    assert_eq!(
        exact_property(&longer, WALL, None, "Foo"),
        Err(ExactPropertyError::ComplexCycle {
            complex: EntityId(11),
            member: EntityId(10),
        })
    );
}

#[test]
fn nesting_deeper_than_the_limit_is_refused() {
    // #10 holds #11 holds ... #26: seventeen complexes, the last one level
    // past the sixteen followed.
    let mut records: Vec<String> = (10..26)
        .map(|id| format!("#{id}=IFCCOMPLEXPROPERTY('P{id}',$,'U',(#{}));", id + 1))
        .collect();
    records.push("#26=IFCCOMPLEXPROPERTY('P26',$,'U',(#27));".to_owned());
    records.push("#27=IFCPROPERTYSINGLEVALUE('Leaf',$,IFCINTEGER(1),$);".to_owned());
    let refs: Vec<&str> = records.iter().map(String::as_str).collect();
    let model = wall("IFC4", "PROPERTYSET", &[10], &refs);
    assert_eq!(
        exact_property(&model, WALL, None, "P10"),
        Err(ExactPropertyError::ComplexTooDeep {
            complex: EntityId(26),
            limit: 16,
        })
    );
    // One level less resolves.
    records[15] = "#25=IFCCOMPLEXPROPERTY('P25',$,'U',(#27));".to_owned();
    let refs: Vec<&str> = records.iter().map(String::as_str).collect();
    let model = wall("IFC4", "PROPERTYSET", &[10], &refs);
    complex(exact_property(&model, WALL, None, "P10"));
}

#[test]
fn more_nested_members_than_the_budget_are_refused() {
    let count = 10_001;
    let ids: Vec<String> = (0..count).map(|i| format!("#{}", 100 + i)).collect();
    let mut records = vec![format!(
        "#10=IFCCOMPLEXPROPERTY('Foo',$,'Wide',({}));",
        ids.join(",")
    )];
    records
        .extend((0..count).map(|i| format!("#{}=IFCPROPERTYSINGLEVALUE('M{i}',$,$,$);", 100 + i)));
    let refs: Vec<&str> = records.iter().map(String::as_str).collect();
    let model = wall("IFC4", "PROPERTYSET", &[10], &refs);
    assert_eq!(
        exact_property(&model, WALL, None, "Foo"),
        Err(ExactPropertyError::ComplexBudgetExceeded {
            complex: EntityId(10),
            limit: 10_000,
        })
    );
}

#[test]
fn repeated_member_names_follow_the_release_rule() {
    let property = wall(
        "IFC4",
        "PROPERTYSET",
        &[10],
        &[
            "#10=IFCCOMPLEXPROPERTY('Foo',$,'U',(#11,#12));",
            "#11=IFCPROPERTYSINGLEVALUE('A',$,IFCINTEGER(1),$);",
            "#12=IFCPROPERTYSINGLEVALUE('A',$,IFCINTEGER(2),$);",
        ],
    );
    assert_eq!(
        exact_property(&property, WALL, None, "Foo"),
        Err(ExactPropertyError::InconsistentValues {
            entity: EntityId(10),
            rule: "WR22",
        })
    );
    let quantities = |schema: &str, length: &str| {
        wall(
            schema,
            "ELEMENTQUANTITY",
            &[10],
            &[
                "#10=IFCPHYSICALCOMPLEXQUANTITY('Foo',$,(#11,#12),'D',$,$);",
                &format!("#11=IFCQUANTITYLENGTH('A',$,$,1.{length});"),
                &format!("#12=IFCQUANTITYLENGTH('A',$,$,2.{length});"),
            ],
        )
    };
    assert_eq!(
        exact_property(&quantities("IFC4", ",$"), WALL, None, "Foo"),
        Err(ExactPropertyError::InconsistentValues {
            entity: EntityId(10),
            rule: "UniqueQuantityNames",
        })
    );
    // IFC2X3 states no such rule for a complex quantity.
    let found = complex(exact_property(&quantities("IFC2X3", ""), WALL, None, "Foo"));
    assert_eq!(found.members.len(), 2);
}

#[test]
fn malformed_members_are_refused() {
    // A property inside a complex quantity is of the wrong family.
    let foreign = wall(
        "IFC4",
        "ELEMENTQUANTITY",
        &[10],
        &[
            "#10=IFCPHYSICALCOMPLEXQUANTITY('Foo',$,(#11),'D',$,$);",
            "#11=IFCPROPERTYSINGLEVALUE('A',$,IFCINTEGER(1),$);",
        ],
    );
    assert_eq!(
        exact_property(&foreign, WALL, None, "Foo"),
        Err(ExactPropertyError::UnsupportedProperty {
            entity: EntityId(11),
            type_name: "IFCPROPERTYSINGLEVALUE".into(),
        })
    );
    let dangling = wall(
        "IFC4",
        "PROPERTYSET",
        &[10],
        &["#10=IFCCOMPLEXPROPERTY('Foo',$,'U',(#99));"],
    );
    assert_eq!(
        exact_property(&dangling, WALL, None, "Foo"),
        Err(ExactPropertyError::MissingReference {
            from: EntityId(10),
            to: EntityId(99),
        })
    );
    let unnamed_usage = wall(
        "IFC4",
        "PROPERTYSET",
        &[10],
        &[
            "#10=IFCCOMPLEXPROPERTY('Foo',$,$,(#11));",
            "#11=IFCPROPERTYSINGLEVALUE('A',$,IFCINTEGER(1),$);",
        ],
    );
    assert_eq!(
        exact_property(&unnamed_usage, WALL, None, "Foo"),
        Err(ExactPropertyError::MalformedName {
            entity: EntityId(10),
            attribute: "UsageName",
        })
    );
}
