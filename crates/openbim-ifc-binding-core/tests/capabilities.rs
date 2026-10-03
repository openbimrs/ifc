//! Native tests of the non-record facade surface every host binds (#244):
//! lenient reads, the STEP header, validation, ifcXML and the reachability
//! lint. The hosts' own suites only check their conversion of these records.

use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{header, BindingError, IfcModel, OnMalformed, ParseOptions};

const FILE: &str = "ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('t.ifc','2026-10-03T00:00:00',('Ann','Bo'),('Org'),'pre','sys','auth');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'Project',$,$,$,$,$,$);
#5=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,$,$,$,.STANDARD.);
#6=IFCRELDEFINESBYPROPERTIES('2YvctVUKr0kugbFTf53O9L',$,$,$,(#5),#7);
#7=IFCPROPERTYSET('3YvctVUKr0kugbFTf53O9L',$,'Pset',$,(#8));
#8=IFCPROPERTYSINGLEVALUE('Length',$,IFCLENGTHMEASURE(2.5),$);
ENDSEC;
END-ISO-10303-21;
";

/// `FILE` with one unreadable record and one real without its point.
fn damaged() -> String {
    FILE.replace(
        "#8=IFCPROPERTYSINGLEVALUE('Length',$,IFCLENGTHMEASURE(2.5),$);",
        "#8=IFCPROPERTYSINGLEVALUE('Length',$,IFCLENGTHMEASURE(1E-05),$);\n#9=IFCWALL('x',,;",
    )
}

fn model() -> IfcModel {
    IfcModel::parse(FILE.as_bytes()).expect("fixture parses")
}

// --- lenient reads ---------------------------------------------------------

#[test]
fn a_strict_read_refuses_a_damaged_file_and_a_lenient_one_reports_it() {
    let damaged = damaged();
    assert!(matches!(
        IfcModel::parse(damaged.as_bytes()),
        Err(BindingError::Parse(_))
    ));
    assert!(matches!(
        IfcModel::parse_with(damaged.as_bytes(), ParseOptions::strict()),
        Err(BindingError::Parse(_))
    ));

    let model = IfcModel::parse_with(damaged.as_bytes(), ParseOptions::lenient()).unwrap();
    assert_eq!(model.ids(), vec![1, 5, 6, 7, 8], "#9 was skipped");
    assert_eq!(
        model.diagnostics().len(),
        2,
        "one per recovery: {:?}",
        model.diagnostics()
    );
    assert_eq!(
        model.attribute(8, 2).unwrap(),
        Tagged::Typed {
            type_name: "IFCLENGTHMEASURE".into(),
            value: Box::new(Tagged::Real(1e-5)),
        }
    );
}

#[test]
fn each_read_option_is_applied_on_its_own() {
    let damaged = damaged();
    let skip_only = ParseOptions {
        on_malformed: OnMalformed::Skip,
        ..ParseOptions::strict()
    };
    let points_only = ParseOptions {
        accept_real_without_point: true,
        ..ParseOptions::strict()
    };
    let only_points = FILE.replace("IFCLENGTHMEASURE(2.5)", "IFCLENGTHMEASURE(1E-05)");
    assert!(IfcModel::parse_with(only_points.as_bytes(), ParseOptions::strict()).is_err());
    let model = IfcModel::parse_with(only_points.as_bytes(), points_only).unwrap();
    assert_eq!(model.diagnostics().len(), 1);
    assert!(IfcModel::parse_with(damaged.as_bytes(), points_only).is_err());

    // Without the point option, the record holding `1E-05` is malformed
    // too, so skipping drops it alongside #9.
    let skipped = IfcModel::parse_owned_with(damaged.into_bytes(), skip_only).unwrap();
    assert_eq!(skipped.ids(), vec![1, 5, 6, 7]);
    assert_eq!(
        skipped.diagnostics().len(),
        2,
        "{:?}",
        skipped.diagnostics()
    );

    let dangling = FILE.replace("#7);", "#70);");
    let checked = IfcModel::parse_with(
        dangling.as_bytes(),
        ParseOptions {
            check_references: true,
            ..ParseOptions::strict()
        },
    )
    .unwrap();
    assert_eq!(
        checked.diagnostics().len(),
        1,
        "{:?}",
        checked.diagnostics()
    );
    let unchecked = IfcModel::parse(dangling.as_bytes()).unwrap();
    assert!(unchecked.diagnostics().is_empty(), "off by default");
}

#[test]
fn a_lenient_open_reads_from_disk() {
    let dir = std::env::temp_dir().join(format!("binding-core-244-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("damaged.ifc");
    std::fs::write(&path, damaged()).unwrap();
    assert!(IfcModel::open(&path).is_err());
    let model = IfcModel::open_with(&path, ParseOptions::lenient()).unwrap();
    assert_eq!(model.len(), 5);
    // SAFETY: the file is private to this test and unchanged while mapped.
    let mapped = unsafe { IfcModel::open_mapped_with(&path, ParseOptions::lenient()) }.unwrap();
    assert_eq!(mapped.ids(), model.ids());
    drop(mapped);
    std::fs::remove_dir_all(&dir).unwrap();
}

// --- header ----------------------------------------------------------------

#[test]
fn the_header_reads_every_field_and_a_replacement_is_written() {
    let mut model = model();
    let mut header = model.header();
    assert_eq!(header.description, ["ViewDefinition [CoordinationView]"]);
    assert_eq!(header.implementation_level, "2;1");
    assert_eq!(header.name, "t.ifc");
    assert_eq!(header.time_stamp, "2026-10-03T00:00:00");
    assert_eq!(header.author, ["Ann", "Bo"]);
    assert_eq!(header.organization, ["Org"]);
    assert_eq!(header.preprocessor_version, "pre");
    assert_eq!(header.originating_system, "sys");
    assert_eq!(header.authorization, "auth");
    assert_eq!(header.schema, ["IFC4"]);

    header.author = vec!["Zoë 'Q'".into()];
    header.name = "renamed.ifc".into();
    model.set_header(header.clone());
    let reparsed = IfcModel::parse(&model.write().unwrap()).unwrap();
    assert_eq!(reparsed.header(), header, "escaped and read back exactly");
}

#[test]
fn the_tagged_header_round_trips_and_refuses_other_shapes() {
    let header = model().header();
    let tagged = header::to_tagged(&header);
    let Tagged::List(fields) = &tagged else {
        panic!("a list");
    };
    assert_eq!(fields.len(), header::FIELDS);
    assert_eq!(fields[2], Tagged::Text("t.ifc".into()));
    assert_eq!(header::from_tagged(tagged.clone()).unwrap(), header);

    let mut short = fields.clone();
    short.pop();
    let mut wrong_kind = fields.clone();
    wrong_kind[2] = Tagged::Integer(1);
    let mut wrong_item = fields.clone();
    wrong_item[4] = Tagged::List(vec![Tagged::Null]);
    for bad in [
        Tagged::Text("x".into()),
        Tagged::List(short),
        Tagged::List(wrong_kind),
        Tagged::List(wrong_item),
    ] {
        assert!(
            matches!(header::from_tagged(bad), Err(BindingError::InvalidValue(_))),
            "refused"
        );
    }
}

// --- validation ------------------------------------------------------------

#[cfg(feature = "validate")]
#[test]
fn validation_reports_structured_findings_in_a_stable_order() {
    let mut model = model();
    model.remove(5).unwrap();
    let report = model.validate(None).unwrap();
    assert!(!report.conformant, "{report:#?}");
    assert!(!report.truncated);
    let dangling = report
        .findings
        .iter()
        .find(|f| f.severity == "error" && f.entity == Some(6))
        .unwrap_or_else(|| panic!("a finding on #6: {report:#?}"));
    assert!(!dangling.rule.is_empty());
    assert!(dangling.path.starts_with("#6"), "{}", dangling.path);
    assert!(!dangling.message.is_empty());
    assert_eq!(
        report.summary.errors,
        report
            .findings
            .iter()
            .filter(|f| f.severity == "error")
            .count()
    );
    assert_eq!(model.validate(None).unwrap(), report, "deterministic");

    let tagged = openbim_ifc_binding_core::validation::findings_to_tagged(&report.findings);
    let Tagged::List(rows) = tagged else {
        panic!("a list")
    };
    assert_eq!(rows.len(), report.findings.len());
}

#[cfg(feature = "validate")]
#[test]
fn a_finding_budget_truncates_the_report() {
    let mut model = model();
    model.remove(5).unwrap();
    model.remove(7).unwrap();
    let report = model.validate(Some(1)).unwrap();
    assert_eq!(report.findings.len(), 1);
    assert!(report.truncated);
}

#[cfg(feature = "validate")]
#[test]
fn validation_needs_a_bundled_declared_schema() {
    let model = IfcModel::parse(FILE.replace("'IFC4'", "'IFC9'").as_bytes()).unwrap();
    assert_eq!(
        model.validate(None),
        Err(BindingError::UnsupportedSchema("IFC9".into()))
    );
}

#[cfg(not(feature = "validate"))]
#[test]
fn validation_left_out_of_the_build_is_refused() {
    assert_eq!(
        model().validate(None),
        Err(BindingError::FeatureDisabled("validate"))
    );
}

// --- ifcXML ----------------------------------------------------------------

#[cfg(feature = "ifcxml")]
#[test]
fn the_native_layout_round_trips_the_model() {
    let model = model();
    let xml = model.write_ifcxml(None).unwrap();
    let back = IfcModel::parse_ifcxml(&xml, None).unwrap();
    assert_eq!(back.ids(), model.ids());
    for id in model.ids() {
        assert_eq!(back.attributes(id).unwrap(), model.attributes(id).unwrap());
    }
    assert_eq!(back.schema(), Some("IFC4"));
}

#[cfg(all(feature = "ifcxml", feature = "ifc4"))]
#[test]
fn the_xsd_layout_round_trips_an_ifc4_model() {
    let mut model = model();
    let Err(BindingError::Write(detail)) = model.write_ifcxml(Some("IFC4")) else {
        panic!("the XSD header holds one author");
    };
    assert!(detail.contains("author"), "{detail}");
    let mut header = model.header();
    header.author.truncate(1);
    model.set_header(header);
    let xml = model.write_ifcxml(Some("ifc4")).unwrap();
    let text = String::from_utf8(xml.clone()).unwrap();
    assert!(
        text.contains("https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD2_TC1/XML"),
        "{text}"
    );
    let back = IfcModel::parse_ifcxml(&xml, Some("IFC4")).unwrap();
    assert_eq!(back.len(), model.len());
    // The XSD writer numbers entities in model order, so the read-back
    // model differs in ids only; it writes the same document again.
    let types = |m: &IfcModel| -> Vec<String> {
        m.ids()
            .into_iter()
            .map(|id| m.type_of(id).unwrap().to_owned())
            .collect()
    };
    assert_eq!(types(&back), types(&model));
    let renumbered = back.write_ifcxml(Some("IFC4")).unwrap();
    let again = IfcModel::parse_ifcxml(&renumbered, Some("IFC4"))
        .unwrap()
        .write_ifcxml(Some("IFC4"))
        .unwrap();
    assert!(
        again == renumbered,
        "the XSD layout is a fixed point:\n{}\n---\n{}",
        String::from_utf8_lossy(&renumbered),
        String::from_utf8_lossy(&again)
    );
}

#[cfg(feature = "ifcxml")]
#[test]
fn ifcxml_refusals_have_their_own_codes() {
    let model = model();
    assert_eq!(
        model.write_ifcxml(Some("IFC2X3")).unwrap_err().code(),
        "unsupported-profile"
    );
    assert_eq!(
        IfcModel::parse_ifcxml(b"<x/>", Some("IFC5"))
            .err()
            .map(|e| e.code()),
        Some("unsupported-profile")
    );
    let refused = IfcModel::parse_ifcxml(b"<ifcXML><unclosed></ifcXML>", None);
    let Err(BindingError::Parse(detail)) = refused else {
        panic!("a parse error: {refused:?}");
    };
    assert!(detail.starts_with("ifcXML: "), "{detail}");
    #[cfg(feature = "ifc4x3")]
    assert!(
        matches!(
            model.write_ifcxml(Some("IFC4X3_ADD2")),
            Err(BindingError::Write(_))
        ),
        "the header must declare the profile's schema"
    );
}

/// The gate runs this crate with `--no-default-features --features ifc4`.
#[cfg(all(feature = "ifcxml", feature = "ifc4", not(feature = "ifc4x3")))]
#[test]
fn an_xsd_profile_whose_release_is_not_bundled_is_refused() {
    assert!(matches!(
        model().write_ifcxml(Some("IFC4X3_ADD2")),
        Err(BindingError::UnsupportedSchema(_))
    ));
}

#[cfg(not(feature = "ifcxml"))]
#[test]
fn ifcxml_left_out_of_the_build_is_refused() {
    assert_eq!(
        model().write_ifcxml(None),
        Err(BindingError::FeatureDisabled("ifcxml"))
    );
    assert_eq!(
        IfcModel::parse_ifcxml(b"<x/>", None).err(),
        Some(BindingError::FeatureDisabled("ifcxml"))
    );
}

// --- unreachable products --------------------------------------------------

/// A wall with Body geometry that no spatial structure contains, and a
/// door whose only geometry is in a plan context.
#[cfg(feature = "unreachable")]
const UNREACHABLE: &str = "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('u.ifc','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'Project',$,$,$,$,(#10),$);
#2=IFCBUILDINGSTOREY('4YvctVUKr0kugbFTf53O9L',$,'Storey',$,$,$,$,$,.ELEMENT.,$);
#3=IFCRELAGGREGATES('5YvctVUKr0kugbFTf53O9L',$,$,$,#1,(#2));
#10=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,$,$);
#11=IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#10,$,.PLAN_VIEW.,$);
#20=IFCSHAPEREPRESENTATION(#10,'Body','SweptSolid',());
#21=IFCPRODUCTDEFINITIONSHAPE($,$,(#20));
#22=IFCSHAPEREPRESENTATION(#11,'Body','SweptSolid',());
#23=IFCPRODUCTDEFINITIONSHAPE($,$,(#22));
#30=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,$,#21,$,.STANDARD.);
#31=IFCDOOR('6YvctVUKr0kugbFTf53O9L',$,'Door',$,$,$,#23,$,$,$,$,$,$);
#40=IFCRELCONTAINEDINSPATIALSTRUCTURE('7YvctVUKr0kugbFTf53O9L',$,$,$,(#31),#2);
ENDSEC;
END-ISO-10303-21;
";

#[cfg(feature = "unreachable")]
#[test]
fn unreachable_products_carry_a_stable_reason() {
    let model = IfcModel::parse(UNREACHABLE.as_bytes()).unwrap();
    let products = model.unreachable_products().unwrap();
    assert_eq!(products.len(), 2, "{products:#?}");
    assert_eq!(products[0].id, 30);
    assert_eq!(products[0].reason, "not-contained-in-spatial-structure");
    assert!(products[0].found_views.is_empty());
    assert!(products[0].message.contains("spatial structure"));
    assert_eq!(products[1].id, 31);
    assert_eq!(products[1].reason, "no-representation-in-model-context");
    assert_eq!(products[1].found_views, ["PlanView"]);

    let tagged = openbim_ifc_binding_core::unreachable::products_to_tagged(&products);
    let Tagged::List(rows) = tagged else {
        panic!("a list")
    };
    assert_eq!(
        rows[1],
        Tagged::List(vec![
            Tagged::Ref(31),
            Tagged::Text("no-representation-in-model-context".into()),
            Tagged::List(vec![Tagged::Text("PlanView".into())]),
            Tagged::Text(products[1].message.clone()),
        ])
    );
}

#[cfg(not(feature = "unreachable"))]
#[test]
fn the_reachability_lint_left_out_of_the_build_is_refused() {
    assert_eq!(
        model().unreachable_products(),
        Err(BindingError::FeatureDisabled("unreachable"))
    );
}
