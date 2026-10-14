//! The grammar: what parses into what, the canonical text, and the span
//! of every refusal.

use super::*;

fn canonical(text: &str) -> String {
    Selector::parse(text)
        .unwrap_or_else(|error| panic!("{text:?}: {error}"))
        .to_string()
}

fn refused(text: &str) -> ParseError {
    match Selector::parse(text) {
        Ok(parsed) => panic!("{text:?} parsed as {parsed}"),
        Err(error) => error,
    }
}

/// The facets with their spans dropped, for comparing two parses.
fn kinds(selector: &Selector) -> Vec<Vec<FacetKind>> {
    selector
        .lists
        .iter()
        .map(|list| list.facets.iter().map(|facet| facet.kind.clone()).collect())
        .collect()
}

#[test]
fn every_facet_parses_into_its_kind() {
    let parsed = Selector::parse(
        "IfcWall, !IfcWallStandardCase, 2O2Fr$t4X7Zf8NOew3FLOH, Name=Wall, \
         Pset_WallCommon.IsExternal=TRUE, type!=WT, material*=Concrete, \
         classification=/04 .*/, container=\"Level 1\", group=NULL, \
         query:type.Name>=3 + IfcSlab",
    )
    .unwrap();
    assert_eq!(parsed.lists.len(), 2);
    let facets = &parsed.lists[0].facets;
    assert_eq!(facets.len(), 11);
    assert!(
        matches!(&facets[0].kind, FacetKind::Class { negated: false, name } if name == "IfcWall")
    );
    assert!(matches!(
        &facets[1].kind,
        FacetKind::Class { negated: true, .. }
    ));
    assert!(matches!(
        &facets[2].kind,
        FacetKind::Instance { negated: false, global_id } if global_id.to_string() == "2O2Fr$t4X7Zf8NOew3FLOH"
    ));
    assert!(matches!(
        &facets[3].kind,
        FacetKind::Attribute { name, comparison, value: Literal::Text(text) }
            if name == "Name" && *comparison == Comparison::new(false, Operator::Equal) && text == "Wall"
    ));
    assert!(matches!(
        &facets[4].kind,
        FacetKind::Property { set: Key::Text(set), property: Key::Text(property), value: Literal::True, .. }
            if set == "Pset_WallCommon" && property == "IsExternal"
    ));
    assert!(matches!(
        &facets[5].kind,
        FacetKind::Keyword { keyword: Keyword::Type, comparison, .. } if comparison.negated
    ));
    assert!(matches!(
        &facets[6].kind,
        FacetKind::Keyword { keyword: Keyword::Material, comparison, .. }
            if comparison.operator == Operator::Contains
    ));
    assert!(matches!(
        &facets[7].kind,
        FacetKind::Keyword { keyword: Keyword::Classification, value: Literal::Regex(pattern), .. }
            if pattern.source() == "04 .*"
    ));
    assert!(matches!(
        &facets[8].kind,
        FacetKind::Keyword { keyword: Keyword::Location, value: Literal::Text(text), .. } if text == "Level 1"
    ));
    assert!(matches!(
        &facets[9].kind,
        FacetKind::Keyword {
            keyword: Keyword::Group,
            value: Literal::Null,
            ..
        }
    ));
    assert!(matches!(
        &facets[10].kind,
        FacetKind::Query { path, comparison, .. }
            if path.keys.len() == 2 && comparison.operator == Operator::GreaterOrEqual
    ));
}

#[test]
fn the_canonical_text_parses_back_to_the_same_facets() {
    for text in [
        "IfcWall",
        "IfcWall,IfcSlab+IfcDoor+",
        "  IfcWall /* the walls */ , ! IfcWallStandardCase",
        "IfcWall, Name = \"Wall \\\"A\\\"\"",
        "Name=\"NULL\", Description=\"\", Tag=\"a,b\"",
        "\"Pset with space\".\"Fire.Rating\"=F90",
        "/Pset_.*Common/./Is.*/ != TRUE",
        "Qto_WallBaseQuantities.Length>4.5",
        "Qto_WallBaseQuantities.Length<=-1e3",
        "container=\"Level 1\"",
        "query:materials.Name*=\"Con\"",
        "query:\"query:odd\".x=1",
        "PredefinedType=SOLIDWALL, Name=/Wall [AB]/",
        "Name=C:\\\\path",
        "Name=a/b.c",
    ] {
        let first = Selector::parse(text).unwrap_or_else(|error| panic!("{text:?}: {error}"));
        let written = first.to_string();
        let second = Selector::parse(&written)
            .unwrap_or_else(|error| panic!("{written:?} (from {text:?}): {error}"));
        assert_eq!(kinds(&first), kinds(&second), "{text:?} -> {written:?}");
        assert_eq!(
            second.to_string(),
            written,
            "the canonical text is a fixed point"
        );
    }
}

#[test]
fn the_canonical_text_normalises_spacing_aliases_and_quotes() {
    assert_eq!(
        canonical(" IfcWall ,IfcSlab+ IfcDoor + "),
        "IfcWall, IfcSlab + IfcDoor"
    );
    assert_eq!(canonical("container = \"Level0\""), "location=Level0");
    assert_eq!(canonical("Name=\"TRUE\""), "Name=\"TRUE\"");
    assert_eq!(canonical("Name=TRUE"), "Name=TRUE");
    assert_eq!(canonical("Name ! = x"), "Name!=x");
}

#[test]
fn quoted_text_escapes_only_quotes_and_backslashes() {
    let parsed = Selector::parse(r#"Name="a\"b\\c\d""#).unwrap();
    let FacetKind::Attribute {
        value: Literal::Text(text),
        ..
    } = &parsed.lists[0].facets[0].kind
    else {
        panic!("{parsed:?}");
    };
    assert_eq!(text, r#"a"b\c\d"#);
}

#[test]
fn a_facet_span_covers_its_text() {
    let text = "IfcWall, Name=Wall + type=WT";
    let parsed = Selector::parse(text).unwrap();
    let spans: Vec<&str> = parsed
        .lists
        .iter()
        .flat_map(|list| &list.facets)
        .map(|facet| &text[facet.span.clone()])
        .collect();
    assert_eq!(spans, ["IfcWall", "Name=Wall", "type=WT"]);
}

/// A refused text, the span its error names, and a check of its kind.
type Refusal = (
    &'static str,
    std::ops::Range<usize>,
    fn(&ParseErrorKind) -> bool,
);

#[test]
fn every_refusal_names_its_span() {
    let cases: &[Refusal] = &[
        ("", 0..0, |kind| *kind == ParseErrorKind::Empty),
        ("  /* nothing */ ", 0..16, |kind| {
            *kind == ParseErrorKind::Empty
        }),
        ("IfcWall,", 8..8, |kind| {
            matches!(kind, ParseErrorKind::UnexpectedEnd { .. })
        }),
        ("IfcWall,,IfcSlab", 8..9, |kind| {
            matches!(kind, ParseErrorKind::UnexpectedChar { found: ',', .. })
        }),
        ("IfcWall IfcSlab", 8..9, |kind| {
            matches!(kind, ParseErrorKind::UnexpectedChar { found: 'I', .. })
        }),
        ("Name=\"open", 5..10, |kind| {
            *kind == ParseErrorKind::UnterminatedString
        }),
        ("Name=/open", 5..10, |kind| {
            *kind == ParseErrorKind::UnterminatedPattern
        }),
        ("IfcWall /* open", 8..15, |kind| {
            *kind == ParseErrorKind::UnterminatedComment
        }),
        ("Name=/(/", 5..8, |kind| {
            matches!(kind, ParseErrorKind::InvalidPattern(_))
        }),
        ("Name=//", 5..7, |kind| {
            matches!(kind, ParseErrorKind::InvalidPattern(_))
        }),
        (
            "colour=red",
            0..6,
            |kind| matches!(kind, ParseErrorKind::UnknownKeyword(word) if word == "colour"),
        ),
        ("Na-me=x", 0..5, |kind| {
            matches!(kind, ParseErrorKind::InvalidAttributeName(_))
        }),
        ("Wall", 0..4, |kind| {
            matches!(kind, ParseErrorKind::ExpectedClassOrGlobalId(_))
        }),
        ("!Name=x", 1..5, |kind| {
            matches!(kind, ParseErrorKind::ExpectedClassOrGlobalId(_))
        }),
        ("Name>NULL", 4..9, |kind| {
            matches!(kind, ParseErrorKind::OperatorNotApplicable { .. })
        }),
        ("Name*=/x/", 4..9, |kind| {
            matches!(kind, ParseErrorKind::OperatorNotApplicable { .. })
        }),
        ("Name!>3", 5..6, |kind| {
            matches!(kind, ParseErrorKind::UnexpectedChar { found: '>', .. })
        }),
        ("Name", 0..4, |kind| {
            matches!(kind, ParseErrorKind::ExpectedClassOrGlobalId(_))
        }),
        ("Name=", 5..5, |kind| {
            matches!(kind, ParseErrorKind::UnexpectedEnd { .. })
        }),
        ("Pset.=1", 5..6, |kind| {
            matches!(kind, ParseErrorKind::UnexpectedChar { found: '=', .. })
        }),
        ("\"Pset\"=1", 6..7, |kind| {
            matches!(kind, ParseErrorKind::UnexpectedChar { found: '=', .. })
        }),
        ("Name=x y", 7..8, |kind| {
            matches!(kind, ParseErrorKind::UnexpectedChar { found: 'y', .. })
        }),
    ];
    for (text, span, kind) in cases {
        let error = refused(text);
        assert_eq!(error.span, *span, "{text:?}: {error}");
        assert!(kind(&error.kind), "{text:?}: {error:?}");
    }
}

#[test]
fn value_paths_parse_and_print_back() {
    let path = ValuePath::parse(" type . \"Pset A\" . /Fire.*/ ").unwrap();
    assert_eq!(path.keys.len(), 3);
    assert_eq!(path.to_string(), "type.\"Pset A\"./Fire.*/");
    assert_eq!(path.keys[1].span, 8..16);
    assert_eq!(
        ValuePath::parse(&path.to_string()).unwrap().to_string(),
        path.to_string()
    );
    let error = ValuePath::parse("type.").unwrap_err();
    assert!(matches!(error.kind, ParseErrorKind::UnexpectedEnd { .. }));
    assert_eq!(error.span, 5..5);
    let error = ValuePath::parse("type Name").unwrap_err();
    assert_eq!(error.span, 5..6);
    assert!(matches!(
        ValuePath::parse(" ").unwrap_err().kind,
        ParseErrorKind::Empty
    ));
}

#[test]
fn patterns_match_from_the_start_as_python_re_match_does() {
    let pattern = Pattern::new("Pset_").unwrap();
    assert!(pattern.is_match("Pset_WallCommon"));
    assert!(!pattern.is_match("MyPset_WallCommon"));
    let whole = Pattern::new("Pset_.*Common$").unwrap();
    assert!(whole.is_match("Pset_WallCommon"));
    assert!(!whole.is_match("Pset_WallCommonX"));
}

#[test]
fn comparisons_follow_the_documented_rules() {
    let eq = Comparison::new(false, Operator::Equal);
    let ne = Comparison::new(true, Operator::Equal);
    let gt = Comparison::new(false, Operator::Greater);
    let has = Comparison::new(false, Operator::Contains);
    let text = |s: &str| Literal::Text(s.to_owned());
    let wall = SelectedValue::Text("Wall A".into());
    assert!(compare(&wall, eq, &text("Wall A")));
    assert!(compare(&wall, has, &text("all")));
    assert!(!compare(&wall, gt, &text("A")), "text has no order");
    assert!(compare(&SelectedValue::Real(5.0), eq, &text("5")));
    assert!(compare(&SelectedValue::Integer(5), gt, &text("4.5")));
    assert!(!compare(&SelectedValue::Real(5.0), eq, &text("five")));
    assert!(compare(&SelectedValue::Null, eq, &Literal::Null));
    assert!(compare(&SelectedValue::Null, ne, &text("x")));
    assert!(compare(&SelectedValue::Bool(true), eq, &Literal::True));
    assert!(!compare(&SelectedValue::Bool(true), eq, &text("TRUE")));
    assert!(compare(
        &SelectedValue::Enum("SOLIDWALL".into()),
        eq,
        &text("SOLIDWALL")
    ));
    // A list holds when any item does; its negation when none does.
    let list = SelectedValue::List(vec![text_value("a"), text_value("b")]);
    assert!(compare(&list, eq, &text("b")));
    assert!(!compare(&list, ne, &text("b")));
    assert!(compare(&list, ne, &text("c")));
    let regex = Literal::Regex(Pattern::new("W").unwrap());
    assert!(compare(&wall, eq, &regex));
    assert!(!compare(&SelectedValue::Integer(1), eq, &regex));
}

fn text_value(text: &str) -> SelectedValue {
    SelectedValue::Text(text.to_owned())
}

#[test]
fn the_grammar_is_versioned() {
    let (major, minor) = GRAMMAR_VERSION.split_once('.').unwrap();
    assert_eq!((major, minor), ("1", "0"));
}

#[cfg(all(feature = "ifc4", not(feature = "properties")))]
#[test]
fn property_facets_need_the_properties_feature() {
    let model = ifc_model::Model::new();
    let index = SelectorIndex::new(&model, ifc_schema::ifc4());
    let selector = Selector::parse("IfcWall, Pset_WallCommon.IsExternal=TRUE").unwrap();
    assert_eq!(
        index.select(&selector),
        Err(SelectorError::FeatureDisabled("properties"))
    );
}
