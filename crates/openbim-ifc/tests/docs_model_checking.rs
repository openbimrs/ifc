//! Compile-and-run proof for `docs/use-cases/model-checking.md`.
//!
//! The code between `// docs:snippet <name>` and `// docs:end` is copied onto
//! the page by `cargo run -p xtask -- docs`; the assertions around it are
//! what make the page's claims true rather than asserted. Kept apart from
//! `docs_examples.rs` so neither file approaches the size gate.
#![cfg(all(
    feature = "step",
    feature = "schema",
    feature = "validate",
    feature = "properties"
))]

use ifc::{Codec, EntityId, StepCodec};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// One wall with an occurrence property set and a quantity set.
const CHECKED: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('wall.ifc','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n\
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O00',$,'Demo',$,$,$,$,$,$);\n\
#7=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,$,$,$,$);\n\
#8=IFCPROPERTYSET('0YvctVUKr0kugbFTf53O08',$,'Pset_WallCommon',$,(#10));\n\
#9=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O09',$,$,$,(#7),#8);\n\
#10=IFCPROPERTYSINGLEVALUE('FireRating',$,IFCLABEL('F90'),$);\n\
#11=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O11',$,'Qto_WallBaseQuantities',$,$,(#13));\n\
#12=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O12',$,$,$,(#7),#11);\n\
#13=IFCQUANTITYLENGTH('Length',$,$,5000.,$);\n\
ENDSEC;\nEND-ISO-10303-21;\n";

const WALL: EntityId = EntityId(7);

/// The same wall, broken: its property relationship points at nothing and
/// the second relationship reuses a GlobalId.
const BROKEN: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('broken.ifc','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n\
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O00',$,'Demo',$,$,$,$,$,$);\n\
#7=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,$,$,$,$);\n\
#9=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O09',$,$,$,(#7),#99);\n\
#12=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O09',$,$,$,(#7),#98);\n\
ENDSEC;\nEND-ISO-10303-21;\n";

/// Declared-schema validation: the file's own `FILE_SCHEMA` picks the tables.
#[test]
fn declared_schema_validation_reports_a_clean_but_partial_verdict() -> TestResult {
    let bytes = CHECKED.as_bytes();
    // docs:snippet model-checking-validate
    use ifc::validate::{validate_declared, Severity};
    use ifc::{Codec, StepCodec};

    let model = StepCodec.read_bytes(bytes)?;
    // IFC2X3, IFC4 or IFC4X3 tables, chosen by the file's FILE_SCHEMA.
    // A file that declares none, or an unknown token, is refused.
    let report = validate_declared(&model)?;

    println!("{}", report.summary()); // "0 errors, 0 evaluation errors, 0 warnings, 2 unsupported"
    for finding in report.sorted() {
        if finding.severity == Severity::Error {
            println!("{} at {}: {}", finding.rule, finding.path, finding.message);
        }
    }
    let clean = report.is_conformant() && !report.is_truncated();
    // docs:end
    let summary = report.summary();
    assert!(clean, "{:?}", report.findings());
    assert_eq!((summary.errors, summary.warnings), (0, 0));
    // The comment in the snippet quotes this exact line.
    assert_eq!(
        summary.to_string(),
        "0 errors, 0 evaluation errors, 0 warnings, 2 unsupported"
    );
    Ok(())
}

/// A file that declares no schema is refused, never validated against a
/// guessed release.
#[test]
fn a_file_without_a_declared_schema_is_refused() -> TestResult {
    let text = CHECKED.replace("FILE_SCHEMA(('IFC4'));", "FILE_SCHEMA(());");
    let model = StepCodec.read_bytes(text.as_bytes())?;
    assert!(ifc::validate::validate_declared(&model).is_err());
    Ok(())
}

/// Errors for the broken file, with rule ids and paths.
#[test]
fn a_broken_file_is_not_conformant() -> TestResult {
    let model = StepCodec.read_bytes(BROKEN.as_bytes())?;
    let report = ifc::validate::validate_declared(&model)?;
    assert!(!report.is_conformant());
    let errors: Vec<_> = report
        .findings()
        .iter()
        .filter(|finding| finding.severity == ifc::validate::Severity::Error)
        .map(|finding| finding.rule.as_str())
        .collect();
    assert!(errors.contains(&"global.UniqueGlobalId"), "{errors:?}");
    assert!(
        errors.len() >= 3,
        "two dangling refs and a duplicate: {errors:?}"
    );
    Ok(())
}

/// The findings cap: storage is bounded and hitting it is reported.
#[test]
fn the_findings_cap_marks_the_report_truncated() -> TestResult {
    let model = StepCodec.read_bytes(BROKEN.as_bytes())?;
    // docs:snippet model-checking-budget
    use ifc::schema::ifc4;
    use ifc::validate::{validate_with, Budget};

    // Budget::DEFAULT caps stored findings; `validate` and
    // `validate_declared` use it. Tighten it for a quick CI verdict.
    let budget = Budget { max_findings: 1 };
    let report = validate_with(&model, ifc4(), budget);

    if report.is_truncated() {
        // Counts are now lower bounds: "1 error" means "at least 1".
        println!("stopped early: {} (at least)", report.summary());
    }
    // docs:end
    assert!(report.is_truncated());
    assert_eq!(report.findings().len(), 1);
    assert!(!report.is_conformant());

    // Mutation guard: the default budget holds every finding untruncated.
    let full = ifc::validate::validate(&model, ifc4());
    assert!(!full.is_truncated());
    assert!(full.findings().len() > 1);
    Ok(())
}

/// Unsupported rules are findings of their own severity, and never count
/// against conformance.
#[test]
fn unsupported_rules_are_reported_not_passed() -> TestResult {
    let model = StepCodec.read_bytes(CHECKED.as_bytes())?;
    let report = ifc::validate::validate_declared(&model)?;
    // docs:snippet model-checking-unsupported
    use ifc::validate::where_rule::{Support, RULES};
    use ifc::validate::Severity;

    // What this run could not check, because the file uses what it constrains.
    for finding in report.findings() {
        if finding.severity == Severity::Unsupported {
            println!("unchecked {}: {}", finding.rule, finding.message);
        }
    }

    // The whole registry, independent of any file.
    for rule in RULES {
        if let Support::Unsupported(reason) = rule.support {
            println!("{} is never evaluated: {reason}", rule.id);
        }
    }
    // docs:end
    let unchecked: Vec<&str> = report
        .findings()
        .iter()
        .filter(|finding| finding.severity == Severity::Unsupported)
        .map(|finding| finding.rule.as_str())
        .collect();
    // The quantity length carries an expression-only WHERE rule; the global
    // same-WCS rule needs geometry. Both are reported, neither is an error.
    assert!(unchecked.contains(&"IfcQuantityLength.WR21"));
    assert!(unchecked.contains(&"IfcRepresentationContextSameWCS"));
    // Rules for entity types the file never uses are not reported.
    assert!(!unchecked.contains(&"IfcPolyLoop.AllPointsSameDim"));
    assert!(report.is_conformant());

    // The documented categories are all present in the registry.
    let reasons: Vec<&str> = RULES
        .iter()
        .filter_map(|rule| match rule.support {
            Support::Unsupported(reason) => Some(reason),
            _ => None,
        })
        .collect();
    for category in ["INVERSE", "aggregate bounds", "EXPRESS expression"] {
        assert!(
            reasons.iter().any(|reason| reason.contains(category)),
            "{category}: {reasons:?}"
        );
    }
    Ok(())
}

/// Exact property and quantity lookup, fail-closed.
#[test]
fn exact_property_and_quantity_lookup() -> TestResult {
    let model = StepCodec.read_bytes(CHECKED.as_bytes())?;
    let wall = WALL;
    // docs:snippet model-checking-exact
    use ifc::properties::{exact_property, ExactResolution, ExactValue};

    // Pset_WallCommon.FireRating on this wall (occurrence, else its type).
    match exact_property(&model, wall, Some("Pset_WallCommon"), "FireRating")? {
        ExactResolution::Present(property) => {
            println!(
                "{} = {:?} ({:?})",
                property.property_set, property.value, property.source
            );
        }
        ExactResolution::Absent => println!("proven absent"),
        _ => {}
    }

    // Quantities resolve the same way: a quantity is a property to a checker.
    if let ExactResolution::Present(length) =
        exact_property(&model, wall, Some("Qto_WallBaseQuantities"), "Length")?
    {
        if let ExactValue::Real(value) = length.value {
            println!(
                "{value} {:?} in unit {:?}",
                length.value_type, length.unit_id
            );
        }
    }
    // docs:end
    let ExactResolution::Present(fire) =
        exact_property(&model, wall, Some("Pset_WallCommon"), "FireRating")?
    else {
        panic!("FireRating must resolve");
    };
    assert_eq!(fire.value, ExactValue::Text("F90".into()));
    assert_eq!(fire.source, ifc::properties::ExactSource::Occurrence);

    let ExactResolution::Present(length) =
        exact_property(&model, wall, Some("Qto_WallBaseQuantities"), "Length")?
    else {
        panic!("Length must resolve");
    };
    assert_eq!(length.value, ExactValue::Real(5000.0));
    assert_eq!(length.value_type.as_deref(), Some("IFCLENGTHMEASURE"));
    assert_eq!(
        length.unit_id, None,
        "no explicit unit: the project unit applies"
    );

    assert_eq!(
        exact_property(&model, wall, Some("Pset_WallCommon"), "IsExternal")?,
        ExactResolution::Absent
    );
    Ok(())
}

/// Refusal rather than a guess: a dangling property definition means the
/// resolver cannot prove absence.
#[test]
fn exact_lookup_refuses_incomplete_evidence() -> TestResult {
    let model = StepCodec.read_bytes(BROKEN.as_bytes())?;
    let result = ifc::properties::exact_property(&model, WALL, None, "FireRating");
    assert!(
        matches!(
            result,
            Err(ifc::properties::ExactPropertyError::MissingReference { .. })
        ),
        "{result:?}"
    );
    Ok(())
}
