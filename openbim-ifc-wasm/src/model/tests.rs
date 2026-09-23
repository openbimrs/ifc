//! Native tests for the model handle and the value encoding.

use super::IfcModel;
use crate::value::{Tagged, MAX_NESTING};
use crate::BindingError;

/// Every value kind the model has, including the ones a naive binding would
/// fold together: `$` vs `*`, `.U.` vs `.F.`, integer vs real, a typed
/// wrapper vs its payload, and an integer beyond JS's 2^53.
const FILE: &str = "ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('t.ifc','2026-09-23T00:00:00',('a'),('o'),'p','s','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'Project',*,$,$,$,$,$);
#2=IFCPROPERTYSINGLEVALUE('Flags',$,IFCLOGICAL(.U.),$);
#3=IFCPROPERTYSINGLEVALUE('Count',$,IFCINTEGER(9007199254740993),$);
#4=IFCPROPERTYSINGLEVALUE('Length',$,IFCLENGTHMEASURE(2.5),$);
#5=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'W',$,$,$,$,$,.STANDARD.);
#6=IFCRELDEFINESBYPROPERTIES('2YvctVUKr0kugbFTf53O9L',$,$,$,(#5),#1);
#7=IFCCARTESIANPOINT((0.,1.,-2.5E-3));
#8=IFCPROPERTYSINGLEVALUE('Done',$,IFCBOOLEAN(.F.),$);
#9=IFCPIXELTEXTURE(.T.,.F.,$,$,$,1,1,1,(\"0A1\"));
ENDSEC;
END-ISO-10303-21;
";

fn model() -> IfcModel {
    IfcModel::parse(FILE.as_bytes()).expect("fixture parses")
}

#[test]
fn every_value_survives_a_round_trip_through_the_encoding() {
    let model = model();
    for id in model.ids() {
        for value in model.attributes(id).unwrap() {
            let decoded = value.clone().into_value().expect("decodes");
            assert_eq!(Tagged::from_value(&decoded), value, "#{id}");
        }
    }
}

#[test]
fn the_kinds_a_naive_binding_would_merge_stay_distinct() {
    let model = model();
    assert_eq!(model.attribute(1, 1).unwrap(), Tagged::Null);
    assert_eq!(model.attribute(1, 3).unwrap(), Tagged::Derived);
    let typed = |_id: u64, name: &str, inner| Tagged::Typed {
        type_name: name.into(),
        value: Box::new(inner),
    };
    assert_eq!(
        model.attribute(2, 2).unwrap(),
        typed(2, "IFCLOGICAL", Tagged::Unknown)
    );
    assert_eq!(
        model.attribute(8, 2).unwrap(),
        typed(8, "IFCBOOLEAN", Tagged::Bool(false))
    );
    assert_eq!(
        model.attribute(3, 2).unwrap(),
        typed(3, "IFCINTEGER", Tagged::Integer(9_007_199_254_740_993)),
        "an integer past 2^53 keeps every bit"
    );
    assert_eq!(
        model.attribute(4, 2).unwrap(),
        typed(4, "IFCLENGTHMEASURE", Tagged::Real(2.5))
    );
    assert_eq!(
        model.attribute(5, 8).unwrap(),
        Tagged::Enum("STANDARD".into())
    );
    assert_eq!(model.attribute(6, 5).unwrap(), Tagged::Ref(1));
    assert_eq!(
        model.attribute(9, 8).unwrap(),
        Tagged::List(vec![Tagged::Binary("0A1".into())])
    );
}

#[test]
fn a_file_written_back_unchanged_reparses_to_the_same_model() {
    let model = model();
    let reparsed = IfcModel::parse(&model.write().unwrap()).unwrap();
    assert_eq!(reparsed.ids(), model.ids());
    for id in model.ids() {
        assert_eq!(reparsed.type_of(id).unwrap(), model.type_of(id).unwrap());
        assert_eq!(
            reparsed.attributes(id).unwrap(),
            model.attributes(id).unwrap()
        );
    }
    assert_eq!(reparsed.schema(), Some("IFC4"));
}

#[test]
fn an_edit_and_an_added_entity_survive_writing() {
    let mut model = model();
    let previous = model
        .set_attribute(5, 2, Tagged::Text("Renamed".into()))
        .unwrap();
    assert_eq!(previous, Tagged::Text("W".into()));
    let point = model
        .add(
            "IfcCartesianPoint",
            vec![Tagged::List(vec![Tagged::Real(1.0), Tagged::Real(2.0)])],
        )
        .unwrap();
    assert_eq!(point, 10, "next id after #9");

    let reparsed = IfcModel::parse(&model.write().unwrap()).unwrap();
    assert_eq!(
        reparsed.attribute(5, 2).unwrap(),
        Tagged::Text("Renamed".into())
    );
    assert_eq!(reparsed.type_of(point).unwrap(), "IFCCARTESIANPOINT");
}

#[test]
fn removing_an_entity_reports_its_dangling_references() {
    let mut model = model();
    model.remove(5).unwrap();
    assert_eq!(model.dangling_references(), vec![(6, 5)]);
    assert_eq!(model.remove(5), Err(BindingError::MissingEntity(5)));
}

#[test]
fn a_missing_entity_is_an_error_not_a_default() {
    let model = model();
    assert_eq!(model.type_of(99), Err(BindingError::MissingEntity(99)));
    assert_eq!(model.attributes(99), Err(BindingError::MissingEntity(99)));
    assert_eq!(
        model.attribute(1, 50).unwrap(),
        Tagged::Null,
        "past the end is $"
    );
}

#[test]
fn ids_of_type_is_case_insensitive_and_exact() {
    let model = model();
    assert_eq!(
        model.ids_of_type("ifcPropertySingleValue"),
        vec![2, 3, 4, 8]
    );
    assert!(
        model.ids_of_type("IfcBuildingElement").is_empty(),
        "no subtypes"
    );
}

#[test]
fn values_the_writer_could_not_emit_are_refused() {
    let mut model = model();
    for bad in ["", "1ABC", "A-B", "A B", ".X."] {
        assert!(
            matches!(
                model.set_attribute(5, 8, Tagged::Enum(bad.into())),
                Err(BindingError::InvalidValue(_))
            ),
            "enum {bad:?}"
        );
        assert!(
            matches!(model.add(bad, vec![]), Err(BindingError::InvalidValue(_))),
            "type {bad:?}"
        );
    }
    assert_eq!(
        model.type_of(5).unwrap(),
        "IFCWALL",
        "a refused edit changes nothing"
    );
}

#[test]
fn nesting_beyond_the_parser_limit_is_refused() {
    let mut deep = Tagged::Null;
    for _ in 0..=MAX_NESTING + 1 {
        deep = Tagged::List(vec![deep]);
    }
    assert!(matches!(
        deep.into_value(),
        Err(BindingError::InvalidValue(_))
    ));

    let mut at_limit = Tagged::Null;
    for _ in 0..MAX_NESTING {
        at_limit = Tagged::List(vec![at_limit]);
    }
    assert!(at_limit.into_value().is_ok());
}

#[test]
fn invalid_step_is_a_parse_error() {
    assert!(matches!(
        IfcModel::parse(b"not a step file"),
        Err(BindingError::Parse(_))
    ));
}

#[test]
fn a_subtype_query_follows_the_declared_schema() {
    let model = model();
    // IFC4: IfcWall's subtypes include IfcWallStandardCase, but this file
    // has only a plain IFCWALL.
    assert_eq!(
        model.ids_of_type_including_subtypes("IfcWall").unwrap(),
        vec![5]
    );
    assert_eq!(
        model
            .ids_of_type_including_subtypes("IfcBuildingElement")
            .unwrap(),
        vec![5],
        "an abstract supertype finds its concrete subtypes"
    );
    assert!(
        model
            .ids_of_type_including_subtypes("IfcWal")
            .unwrap()
            .is_empty(),
        "an undeclared name finds nothing"
    );
}

#[test]
fn a_subtype_query_needs_a_known_schema() {
    let text = FILE.replace("FILE_SCHEMA(('IFC4'))", "FILE_SCHEMA(('IFC9'))");
    let model = IfcModel::parse(text.as_bytes()).unwrap();
    assert_eq!(
        model.ids_of_type_including_subtypes("IfcWall"),
        Err(BindingError::UnsupportedSchema("IFC9".into()))
    );
}
