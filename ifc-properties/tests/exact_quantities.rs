//! `exact_property` over quantity sets and predefined property sets (#66).
//!
//! A quantity in a same-named `IfcElementQuantity` was reported `Absent`,
//! a confident wrong answer for an IDS checker, which treats quantities as
//! properties. Quantities now resolve exactly; a predefined set that could
//! hold the name is refused instead of skipped.

use ifc_model::{Codec, EntityId, Model};
use ifc_properties::{
    exact_property, ExactPropertyError, ExactResolution, ExactSource, ExactValue,
};

const WALL: EntityId = EntityId(7);

fn parse(schema: &str, data: &str) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\n\
         ENDSEC;\nDATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
    );
    let model = ifc_step::StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

/// The issue's fixture: a wall with a `Foo_Bar` quantity set holding `Foo`.
fn ifc4(quantities: &str) -> Model {
    parse(
        "IFC4",
        &format!(
            "#7=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,$,$,$,$,$,$,$);
#8=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O08',$,'Foo_Bar',$,$,(#10));
#9=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O09',$,$,$,(#7),#8);
{quantities}"
        ),
    )
}

fn present(result: Result<ExactResolution, ExactPropertyError>) -> ifc_properties::ExactProperty {
    match result {
        Ok(ExactResolution::Present(property)) => property,
        other => panic!("expected a value, got {other:?}"),
    }
}

#[test]
fn a_quantity_in_a_quantity_set_is_resolved_not_absent() {
    let model = ifc4("#10=IFCQUANTITYLENGTH('Foo',$,$,42.,$);");
    for set in [Some("Foo_Bar"), None] {
        let found = present(exact_property(&model, WALL, set, "Foo"));
        assert_eq!(found.source, ExactSource::Occurrence);
        assert_eq!(&*found.property_set, "Foo_Bar");
        assert_eq!(found.set_id, EntityId(8));
        assert_eq!(found.property_id, EntityId(10));
        assert_eq!(found.value_type.as_deref(), Some("IFCLENGTHMEASURE"));
        assert_eq!(found.unit_id, None);
        assert_eq!(found.value, ExactValue::Real(42.0));
    }
    // Asking for a name the set does not hold is still a proven absence.
    assert_eq!(
        exact_property(&model, WALL, Some("Foo_Bar"), "Bar"),
        Ok(ExactResolution::Absent)
    );
}

#[test]
fn a_quantity_keeps_its_unit_and_declared_measure() {
    let model = ifc4(
        "#10=IFCQUANTITYAREA('Foo',$,#11,2.5,$);
#11=IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.);",
    );
    let found = present(exact_property(&model, WALL, Some("Foo_Bar"), "Foo"));
    assert_eq!(found.value_type.as_deref(), Some("IFCAREAMEASURE"));
    assert_eq!(found.unit_id, Some(EntityId(11)));
    assert_eq!(found.value, ExactValue::Real(2.5));
}

#[test]
fn a_count_quantity_resolves() {
    let model = ifc4("#10=IFCQUANTITYCOUNT('Foo',$,$,3.,$);");
    let found = present(exact_property(&model, WALL, None, "Foo"));
    assert_eq!(found.value_type.as_deref(), Some("IFCCOUNTMEASURE"));
    assert_eq!(found.value, ExactValue::Real(3.0));
}

#[test]
fn malformed_quantities_are_refused() {
    // A unit that is not an IfcNamedUnit.
    let derived = ifc4(
        "#10=IFCQUANTITYLENGTH('Foo',$,#11,1.,$);
#11=IFCDERIVEDUNIT((#12),.LINEARFORCEUNIT.,$);
#12=IFCDERIVEDUNITELEMENT(#13,2);
#13=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);",
    );
    assert_eq!(
        exact_property(&derived, WALL, None, "Foo"),
        Err(ExactPropertyError::UnsupportedUnit {
            property: EntityId(10)
        })
    );
    // A missing value.
    let missing = ifc4("#10=IFCQUANTITYLENGTH('Foo',$,$,$,$);");
    assert_eq!(
        exact_property(&missing, WALL, None, "Foo"),
        Err(ExactPropertyError::MissingValueSlot {
            property: EntityId(10)
        })
    );
}

#[test]
fn a_complex_or_duplicated_quantity_is_refused() {
    let complex = ifc4(
        "#10=IFCPHYSICALCOMPLEXQUANTITY('Foo',$,(#11),'Layer',$,$);
#11=IFCQUANTITYLENGTH('Width',$,$,0.2,$);",
    );
    assert!(matches!(
        exact_property(&complex, WALL, None, "Foo"),
        Err(ExactPropertyError::UnsupportedProperty { entity, .. }) if entity == EntityId(10)
    ));

    let twice = parse(
        "IFC4",
        "#7=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,$,$,$,$,$,$,$);
#8=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O08',$,'Foo_Bar',$,$,(#10,#11));
#9=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O09',$,$,$,(#7),#8);
#10=IFCQUANTITYLENGTH('Foo',$,$,1.,$);
#11=IFCQUANTITYLENGTH('Foo',$,$,2.,$);",
    );
    assert_eq!(
        exact_property(&twice, WALL, None, "Foo"),
        Err(ExactPropertyError::DuplicateMatchingProperties {
            set: EntityId(8),
            first: EntityId(10),
            second: EntityId(11),
        })
    );
}

/// A quantity set of another name does not disturb a proven absence; one
/// with no `Name` could be the set asked for, so it is refused, not skipped.
#[test]
fn another_quantity_set_keeps_absence_but_a_nameless_one_is_refused() {
    let model = ifc4("#10=IFCQUANTITYLENGTH('Length',$,$,1.,$);");
    assert_eq!(
        exact_property(&model, WALL, Some("Pset_Test"), "Missing"),
        Ok(ExactResolution::Absent)
    );

    let nameless = parse(
        "IFC4",
        "#7=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,$,$,$,$,$,$,$);
#8=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O08',$,$,$,$,(#10));
#9=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O09',$,$,$,(#7),#8);
#10=IFCQUANTITYLENGTH('Length',$,$,1.,$);",
    );
    assert!(matches!(
        exact_property(&nameless, WALL, Some("Pset_Test"), "Missing"),
        Err(ExactPropertyError::MalformedName { entity, .. }) if entity == EntityId(8)
    ));
}

/// A property set and a quantity set of one name both matching: ambiguous.
#[test]
fn a_same_named_property_set_and_quantity_set_are_ambiguous() {
    let model = ifc4(
        "#10=IFCQUANTITYLENGTH('Foo',$,$,42.,$);
#20=IFCPROPERTYSINGLEVALUE('Foo',$,IFCLABEL('x'),$);
#21=IFCPROPERTYSET('0YvctVUKr0kugbFTf53O21',$,'Foo_Bar',$,(#20));
#22=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O22',$,$,$,(#7),#21);",
    );
    assert!(matches!(
        exact_property(&model, WALL, Some("Foo_Bar"), "Foo"),
        Err(ExactPropertyError::DuplicateMatchingSets { .. })
    ));
}

#[test]
fn an_ifc2x3_quantity_resolves_against_ifc2x3() {
    let model = parse(
        "IFC2X3",
        "#7=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,$,$,$,$,$,$);
#8=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O08',$,'Qto_WallBaseQuantities',$,$,(#10));
#9=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O09',$,$,$,(#7),#8);
#10=IFCQUANTITYVOLUME('NetVolume',$,$,1.25);",
    );
    let found = present(exact_property(
        &model,
        WALL,
        Some("Qto_WallBaseQuantities"),
        "NetVolume",
    ));
    assert_eq!(found.value_type.as_deref(), Some("IFCVOLUMEMEASURE"));
    assert_eq!(found.value, ExactValue::Real(1.25));
}

/// A predefined set holding the requested attribute cannot prove absence.
#[test]
fn a_predefined_set_that_may_hold_the_name_is_refused() {
    let slots = ifc_schema::ifc4()
        .attributes("IFCDOORLININGPROPERTIES")
        .len();
    // GlobalId, OwnerHistory, Name, Description, LiningDepth, then `$`s.
    let rest = vec!["$"; slots - 5].join(",");
    let model = parse(
        "IFC4",
        &format!(
            "#7=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,$,$,$,$,$,$,$);
#8=IFCDOORLININGPROPERTIES('0YvctVUKr0kugbFTf53O08',$,'Lining',$,0.1,{rest});
#9=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O09',$,$,$,(#7),#8);"
        ),
    );
    for set in [None, Some("Lining")] {
        assert!(matches!(
            exact_property(&model, WALL, set, "LiningDepth"),
            Err(ExactPropertyError::UnsupportedDefinition { entity, .. }) if entity == EntityId(8)
        ));
    }
    // A name none of its attributes carries, or another set: proven absent.
    assert_eq!(
        exact_property(&model, WALL, None, "FireRating"),
        Ok(ExactResolution::Absent)
    );
    assert_eq!(
        exact_property(&model, WALL, Some("Other"), "LiningDepth"),
        Ok(ExactResolution::Absent)
    );
    // An inherited IfcRoot attribute is not a property name it holds.
    assert_eq!(
        exact_property(&model, WALL, None, "Name"),
        Ok(ExactResolution::Absent)
    );
}
