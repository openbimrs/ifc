//! `ifc-validate` against the IfcOpenShell pass/fail corpus.
//!
//! The corpus is external ground truth: each file is named for what
//! IfcOpenShell's own validator concludes about it. A `pass-*` file that this
//! crate rejects is a false positive; a `fail-*` file it accepts is a miss.

use std::path::PathBuf;

use ifc_model::{Codec, Model};
use ifc_step::StepCodec;
use ifc_validate::{validate, Budget, Severity};

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures/ifcopenshell-validate")
}

fn load(name: &str) -> Model {
    StepCodec
        .read_path(&corpus().join(name))
        .unwrap_or_else(|error| panic!("{name} should parse: {error}"))
}

/// A clean IFC4 file produces no errors.
#[test]
fn a_conformant_file_is_reported_conformant() {
    let model = load("pass-duplicated-guids-ifc4.ifc");
    let report = validate(&model, ifc_schema::ifc4());
    let errors: Vec<_> = report
        .findings()
        .iter()
        .filter(|finding| finding.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "clean file produced errors: {errors:#?}");
    assert!(report.is_conformant());
}

/// Two entities sharing a GlobalId breaks `IfcRoot.UR1`.
///
/// The corpus pairs this with a near-identical file whose GUIDs differ, so a
/// validator that reported *every* file as broken would fail the test above.
#[test]
fn duplicate_global_ids_are_reported() {
    let model = load("fail-duplicated-guids-ifc4.ifc");
    let report = validate(&model, ifc_schema::ifc4());
    let found: Vec<_> = report
        .findings()
        .iter()
        .filter(|finding| finding.rule == "global.UniqueGlobalId")
        .collect();
    assert_eq!(found.len(), 1, "one repeat, one finding: {found:#?}");
    assert!(!report.is_conformant());
    // The finding must name the *second* occurrence: the first is innocent.
    assert_eq!(found[0].path.to_string(), "#2.GlobalId");
}

/// `IFCPOSITIVELENGTHMEASURE('1')` states a string where a real belongs.
///
/// This is the defect the corpus name calls an "invalid selected simple
/// type": the value is syntactically fine, and only the schema says the
/// wrapper demands a number.
#[test]
fn a_measure_holding_a_string_is_reported() {
    let model = load("fail-invalid-selected-simple-type.ifc");
    let report = validate(&model, ifc_schema::ifc4());
    let found: Vec<_> = report
        .findings()
        .iter()
        .filter(|finding| finding.rule.starts_with("type."))
        .collect();
    assert!(
        !found.is_empty(),
        "the typed wrapper mismatch went unreported: {:#?}",
        report.findings()
    );
}

/// A complex-number property is legal IFC4 and must not be flagged.
#[test]
fn a_complex_number_is_accepted() {
    let model = load("pass-complex-number-ifc4.ifc");
    let report = validate(&model, ifc_schema::ifc4());
    let errors: Vec<_> = report
        .findings()
        .iter()
        .filter(|finding| finding.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "false positive: {errors:#?}");
}

/// A file declaring IFC2X3 is refused rather than checked against IFC4.
///
/// Validating one schema's file against another's tables produces confident
/// nonsense: entities that exist in both may have different slot layouts.
#[test]
fn a_file_from_another_schema_is_refused_not_guessed() {
    let model = load("pass-selected-simple-type.ifc");
    let error = ifc_validate::validate_declared(&model)
        .expect_err("an IFC2X3 file must not be validated against IFC4 tables");
    assert_eq!(
        error,
        ifc_validate::ValidateError::UnknownSchema("IFC2X3".into())
    );
}

/// The report distinguishes "no errors" from "fully checked".
///
/// This is the crate's central honesty claim: a conformant verdict must not
/// imply every schema rule was evaluated.
#[test]
fn a_conformant_report_still_admits_unchecked_rules() {
    let model = load("pass-duplicated-guids-ifc4.ifc");
    let report = validate(&model, ifc_schema::ifc4());
    assert!(report.is_conformant(), "the file is clean");
    let summary = report.summary();
    assert_eq!(summary.errors, 0);
    assert!(
        !ifc_validate::where_rule::unsupported()
            .collect::<Vec<_>>()
            .is_empty(),
        "the registry must name rules this validator cannot evaluate"
    );
}

/// A dangling reference is reported against the slot that holds it.
#[test]
fn a_dangling_reference_names_its_slot() {
    let mut model = Model::new();
    let wall = model.push(ifc_model::Entity::new(
        "IFCWALL",
        vec![
            ifc_model::Value::Text("0hMOPMBpTAoOL$IPqA1$xY".into()),
            ifc_model::Value::Null,
            ifc_model::Value::Null,
            ifc_model::Value::Null,
            ifc_model::Value::Null,
            ifc_model::Value::Ref(ifc_model::EntityId(999)),
            ifc_model::Value::Null,
            ifc_model::Value::Null,
            ifc_model::Value::Null,
        ],
    ));
    let report = validate(&model, ifc_schema::ifc4());
    let dangling: Vec<_> = report
        .findings()
        .iter()
        .filter(|finding| finding.rule == "structure.reference.dangling")
        .collect();
    assert_eq!(dangling.len(), 1, "{:#?}", report.findings());
    // The path must identify the offending slot on the offending entity.
    // Slot 5 of IfcWall is ObjectPlacement.
    let path = dangling[0].path.to_string();
    assert!(
        path.starts_with(&format!("#{}", wall.0)),
        "the finding must name the entity that holds the bad reference: {path}"
    );
    assert!(
        path.contains("ObjectPlacement") || path.contains("[5]"),
        "the finding must name the slot: {path}"
    );
}

/// The budget bounds the report and says that it did.
#[test]
fn a_truncated_report_admits_truncation() {
    let mut model = Model::new();
    for _ in 0..50 {
        model.push(ifc_model::Entity::new(
            "IFCWALL",
            vec![
                ifc_model::Value::Text("0hMOPMBpTAoOL$IPqA1$xY".into()),
                ifc_model::Value::Null,
                ifc_model::Value::Null,
                ifc_model::Value::Null,
                ifc_model::Value::Null,
                ifc_model::Value::Ref(ifc_model::EntityId(999)),
                ifc_model::Value::Null,
                ifc_model::Value::Null,
                ifc_model::Value::Null,
            ],
        ));
    }
    let budget = Budget {
        max_findings: 5,
        ..Budget::DEFAULT
    };
    let report = ifc_validate::validate_with(&model, ifc_schema::ifc4(), budget);
    assert!(
        report.is_truncated(),
        "a bounded report must say it is bounded"
    );
}

/// The defect class this crate exists to catch, rebuilt from the real bug.
///
/// The committed costing fixture gave `IfcCostValue` a GlobalId. It is an
/// `IfcAppliedValue`, not an `IfcRoot`: `Name` is slot 0. Every slot after it
/// shifted, and three separate readers were written against the wrong layout
/// before anyone noticed. `ifcopenshell.validate` did not report it either.
#[test]
fn a_root_style_guid_on_a_non_root_entity_is_reported() {
    let mut model = Model::new();
    // IfcCostValue with a GlobalId in slot 0, exactly as the fixture had it.
    model.push(ifc_model::Entity::new(
        "IFCCOSTVALUE",
        vec![
            ifc_model::Value::Text("3LDeXWMt2mwidHVh75Q1Cz".into()),
            ifc_model::Value::Null,
            ifc_model::Value::Typed {
                type_name: "IFCMONETARYMEASURE".into(),
                value: Box::new(ifc_model::Value::Real(12345.67)),
            },
            ifc_model::Value::Null,
            ifc_model::Value::Text("Unit rate".into()),
            ifc_model::Value::Null,
            ifc_model::Value::Null,
        ],
    ));
    let report = validate(&model, ifc_schema::ifc4());
    let typed: Vec<_> = report
        .findings()
        .iter()
        .filter(|finding| finding.severity == Severity::Error)
        .collect();
    assert!(
        !typed.is_empty(),
        "the shifted-slot fixture must not validate clean: {:#?}",
        report.findings()
    );
}

/// An entity type no schema declares is reported, not silently accepted.
#[test]
fn an_unknown_entity_type_is_reported() {
    let mut model = Model::new();
    model.push(ifc_model::Entity::new(
        "IFCFUTURESUSTAINABILITYMETRIC",
        vec![ifc_model::Value::Null],
    ));
    let report = validate(&model, ifc_schema::ifc4());
    assert!(
        report
            .findings()
            .iter()
            .any(|f| f.rule.starts_with("type.entity")),
        "{:#?}",
        report.findings()
    );
}

/// An abstract supertype cannot be instantiated.
#[test]
fn an_abstract_entity_instance_is_reported() {
    let mut model = Model::new();
    model.push(ifc_model::Entity::new(
        "IFCPRODUCT",
        vec![ifc_model::Value::Text("0hMOPMBpTAoOL$IPqA1$xY".into())],
    ));
    let report = validate(&model, ifc_schema::ifc4());
    assert!(
        report
            .findings()
            .iter()
            .any(|f| f.rule.contains("abstract")),
        "{:#?}",
        report.findings()
    );
}

/// Two `IfcProject` entities breaks the IFC4 global rule.
///
/// A file with two projects has two coordinate systems and two unit
/// assignments, and nothing states which is authoritative.
#[test]
fn a_second_project_is_reported() {
    let mut model = Model::new();
    for guid in ["0hMOPMBpTAoOL$IPqA1$xY", "1sEzC8v31DshmvW5t5P631"] {
        model.push(ifc_model::Entity::new(
            "IFCPROJECT",
            vec![ifc_model::Value::Text(guid.into())],
        ));
    }
    let report = validate(&model, ifc_schema::ifc4());
    let found: Vec<_> = report
        .findings()
        .iter()
        .filter(|f| f.rule == "global.IfcSingleProjectInstance")
        .collect();
    assert_eq!(found.len(), 1, "{:#?}", report.findings());
    // A global rule blames the file, not an arbitrary one of the two.
    assert_eq!(found[0].path.to_string(), "<file>");
}

/// A single project is not reported.
///
/// Guards the rule against firing on every well-formed file.
#[test]
fn one_project_is_accepted() {
    let mut model = Model::new();
    model.push(ifc_model::Entity::new(
        "IFCPROJECT",
        vec![ifc_model::Value::Text("0hMOPMBpTAoOL$IPqA1$xY".into())],
    ));
    let report = validate(&model, ifc_schema::ifc4());
    assert!(!report
        .findings()
        .iter()
        .any(|f| f.rule == "global.IfcSingleProjectInstance"));
}

/// `IfcRelDefinesByProperties` must not attach a property set to a type.
///
/// IFC4 forbids it with `NoRelatedTypeObject`. Doing it anyway puts the
/// properties on every occurrence of that type at once, silently.
#[test]
fn a_property_set_attached_to_a_type_is_reported() {
    let mut model = Model::new();
    let wall_type = model.push(ifc_model::Entity::new(
        "IFCWALLTYPE",
        vec![ifc_model::Value::Text("0hMOPMBpTAoOL$IPqA1$xY".into())],
    ));
    let pset = model.push(ifc_model::Entity::new(
        "IFCPROPERTYSET",
        vec![ifc_model::Value::Text("1sEzC8v31DshmvW5t5P631".into())],
    ));
    model.push(ifc_model::Entity::new(
        "IFCRELDEFINESBYPROPERTIES",
        vec![
            ifc_model::Value::Text("2tFaD9w42EtinwX6u6Q742".into()),
            ifc_model::Value::Null,
            ifc_model::Value::Null,
            ifc_model::Value::Null,
            ifc_model::Value::List(vec![ifc_model::Value::Ref(wall_type)]),
            ifc_model::Value::Ref(pset),
        ],
    ));
    let report = validate(&model, ifc_schema::ifc4());
    let found: Vec<_> = report
        .findings()
        .iter()
        .filter(|f| f.rule == "IfcRelDefinesByProperties.NoRelatedTypeObject")
        .collect();
    assert_eq!(found.len(), 1, "{:#?}", report.findings());
}

/// An occurrence-attached property set is legal and must not be reported.
#[test]
fn a_property_set_attached_to_an_occurrence_is_accepted() {
    let mut model = Model::new();
    let wall = model.push(ifc_model::Entity::new(
        "IFCWALL",
        vec![ifc_model::Value::Text("0hMOPMBpTAoOL$IPqA1$xY".into())],
    ));
    let pset = model.push(ifc_model::Entity::new(
        "IFCPROPERTYSET",
        vec![ifc_model::Value::Text("1sEzC8v31DshmvW5t5P631".into())],
    ));
    model.push(ifc_model::Entity::new(
        "IFCRELDEFINESBYPROPERTIES",
        vec![
            ifc_model::Value::Text("2tFaD9w42EtinwX6u6Q742".into()),
            ifc_model::Value::Null,
            ifc_model::Value::Null,
            ifc_model::Value::Null,
            ifc_model::Value::List(vec![ifc_model::Value::Ref(wall)]),
            ifc_model::Value::Ref(pset),
        ],
    ));
    let report = validate(&model, ifc_schema::ifc4());
    assert!(!report
        .findings()
        .iter()
        .any(|f| f.rule == "IfcRelDefinesByProperties.NoRelatedTypeObject"));
}

/// The registry must name rules this validator cannot evaluate.
///
/// If this count reaches zero, either every IFC4 rule is implemented -- it is
/// not -- or unsupported rules are being dropped instead of declared, which
/// is the exact dishonesty the registry exists to prevent.
#[test]
fn the_registry_declares_unevaluated_rules() {
    let gaps: Vec<_> = ifc_validate::where_rule::unsupported().collect();
    assert!(
        !gaps.is_empty(),
        "a validator claiming complete WHERE-rule coverage is lying"
    );
    for entry in gaps {
        assert!(
            matches!(
                entry.support,
                ifc_validate::where_rule::Support::Unsupported(reason) if !reason.is_empty()
            ),
            "{} must say why it is unsupported",
            entry.id
        );
    }
}

/// Every rule marked implemented must actually be evaluated.
///
/// The registry is a claim, and a claim can drift from the code: marking a
/// rule `Implemented` without wiring it into the engine produces exactly the
/// false assurance the registry was built to prevent. This pins the claim to
/// the dispatch list in `where_rule::engine`.
#[test]
fn every_implemented_rule_is_actually_dispatched() {
    // The rules `where_rule::evaluate` calls, by id. Adding a rule to the
    // registry as Implemented without adding it here fails this test.
    const DISPATCHED: &[&str] = &[
        "global.IfcSingleProjectInstance",
        "global.UniqueGlobalId",
        "IfcRelDefinesByProperties.NoRelatedTypeObject",
    ];
    let claimed: Vec<&str> = ifc_validate::where_rule::implemented()
        .map(|entry| entry.id)
        .collect();
    for id in &claimed {
        assert!(
            DISPATCHED.contains(id),
            "{id} is registered as implemented but the engine never runs it"
        );
    }
    assert_eq!(
        claimed.len(),
        DISPATCHED.len(),
        "dispatch list and registry disagree: {claimed:?}"
    );
}
