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
    assert_eq!(
        report
            .findings()
            .iter()
            .filter(|finding| finding.severity == Severity::Error)
            .count(),
        1,
        "one duplicated GUID must not be reported by two validation phases"
    );
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

/// A valid IFC4X3 file is checked against the bundled IFC4X3 tables.
#[test]
fn an_ifc4x3_file_is_validated_against_ifc4x3_tables() {
    let model = load("../ifclite-geometry/bath_csg_solid.ifc");
    let report = ifc_validate::validate_declared(&model)
        .expect("IFC4X3 ADD2 tables are bundled, so this must validate");
    let errors: Vec<_> = report
        .findings()
        .iter()
        .filter(|finding| finding.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "valid IFC4X3 fixture produced errors: {errors:#?}"
    );
    assert_eq!(
        ifc_schema::for_version(ifc_schema::SchemaVersion::Ifc4x3)
            .expect("IFC4X3 must be bundled")
            .name(),
        "IFC4X3_ADD2"
    );
}

/// The existing malformed IFC4X3 mapped item is validated, not skipped.
#[test]
fn an_invalid_ifc4x3_file_reports_its_missing_mapping_target() {
    let model = load("../ifclite-geometry/nested_mapped_item_cycle.ifc");
    let report =
        ifc_validate::validate_declared(&model).expect("known IFC4X3 files must reach validation");
    assert!(
        report.findings().iter().any(|finding| {
            finding.severity == Severity::Error
                && finding.path.to_string().contains("MappingTarget")
        }),
        "invalid IFC4X3 fixture was not rejected: {:#?}",
        report.findings()
    );
}

/// An IFC2x3 file is now validated against IFC2x3 tables, not refused.
///
/// The schema differences are load-bearing: `IfcWallStandardCase` has 8
/// attributes in IFC2x3 and 9 in IFC4, and `IfcRoot.OwnerHistory` is
/// mandatory in IFC2x3 but OPTIONAL in IFC4. Checking one against the other
/// invents errors in both directions.
#[test]
fn an_ifc2x3_file_is_validated_against_ifc2x3_tables() {
    let model = load("pass-selected-simple-type.ifc");
    let report = ifc_validate::validate_declared(&model)
        .expect("IFC2x3 tables are bundled, so this must validate");
    let slot_errors: Vec<_> = report
        .findings()
        .iter()
        .filter(|finding| finding.rule.starts_with("structure.required.slot_count"))
        .collect();
    assert!(
        slot_errors.is_empty(),
        "IFC4 tables would invent slot-count errors here: {slot_errors:#?}"
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
    assert_eq!(
        report.findings().len(),
        budget.max_findings,
        "the budget must cap storage, not merely set a flag"
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
        "IfcExternalReference.WR1",
        "IfcRelSequence.WR1",
        "IfcRelSequence.AvoidInconsistentSequence",
        "IfcRelAggregates.NoSelfReference",
        "IfcRelNests.NoSelfReference",
        "IfcMaterialLayer.NormalizedPriority",
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

/// Every committed IFC4 fixture that is meant to be well-formed validates clean.
///
/// This is the regression that would have caught the SELECT walk bug: a
/// membership query bounded by loop iterations rather than visited types gave
/// up part-way through IFC4's wide value selects and reported legal monetary
/// and flow-rate measures as non-members. Eight findings across three
/// fixtures, all false.
///
/// Fixtures under `ifcopenshell-validate/fail-*` are excluded: they are
/// deliberately broken. So is `nested_mapped_item_cycle.ifc`, which
/// `ifc-geometry` documents as malformed in two ways on purpose.
/// Fixtures that violate the schema on purpose, each with its reason.
///
/// An explicit list, not a name pattern: a new malformed fixture must fail
/// the corpus test and be justified here, rather than slipping through
/// because its filename happened to match.
const DELIBERATELY_INVALID: &[&str] = &[
    // ifc-geometry asserts a typed cyclic report and a missing MappingTarget.
    "nested_mapped_item_cycle.ifc",
    // Abstract IfcBSplineCurve/Surface instances; lowering must refuse them.
    "invalid_abstract_base_splines.ifc",
    // Upstream ifc-lite fixtures (MPL-2.0) that omit IfcRoot.OwnerHistory.
    // It is mandatory in IFC2x3 and only became OPTIONAL in IFC4, so these
    // were invalid all along and simply unverifiable until this build gained
    // IFC2x3 tables. `ifcopenshell.validate` reports the same 7 and 2
    // instances. They are kept byte-identical to upstream because the fixture
    // AGENTS.md makes filename and content provenance a hard rule -- editing
    // them would silently fork a file we claim is upstream's.
    "issue_2019_wall_two_overlapping_openings.ifc",
    "swept_disk_composite_arc_crankbar.ifc",
];

#[test]
fn every_well_formed_bundled_schema_fixture_validates_clean() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures");
    let mut checked = 0usize;
    let mut dirty = Vec::new();

    for path in ifc_fixtures(&root) {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if DELIBERATELY_INVALID.contains(&name.as_str()) || name.starts_with("fail-") {
            continue;
        }
        let Ok(model) = StepCodec.read_path(&path) else {
            continue;
        };
        // Validate every recognised bundled version against its own tables;
        // cross-version substitution invents slot-count/type errors.
        let Ok(report) = ifc_validate::validate_declared(&model) else {
            continue;
        };
        checked += 1;
        let errors: Vec<String> = report
            .sorted()
            .iter()
            .filter(|finding| finding.severity == Severity::Error)
            .map(|finding| format!("{name}: {finding}"))
            .collect();
        if !errors.is_empty() {
            dirty.extend(errors);
        }
    }

    assert_eq!(
        checked, 29,
        "all intended-clean fixtures must run; raw-header fail fixtures stay excluded"
    );
    assert!(
        dirty.is_empty(),
        "well-formed fixtures produced {} errors:\n{}",
        dirty.len(),
        dirty.join("\n")
    );
}

/// Every `fail-*` corpus fixture this build can read is actually rejected.
///
/// Guards the other direction: a validator that reports nothing would pass
/// the test above trivially.
#[test]
fn every_ifc4_fail_fixture_is_rejected() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures/ifcopenshell-validate");
    let mut judged = 0usize;
    for path in ifc_fixtures(&root) {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        // Header-syntax fixtures are excluded: the codec normalizes the
        // header, so that evidence never reaches validation. Documented in
        // `ifc_validate::header`.
        if !name.starts_with("fail-") || name.contains("header") {
            continue;
        }
        let Ok(model) = StepCodec.read_path(&path) else {
            continue;
        };
        let Ok(report) = ifc_validate::validate_declared(&model) else {
            continue;
        };
        judged += 1;
        assert!(
            !report.is_conformant(),
            "{name} is a known-bad fixture but validated clean"
        );
    }
    assert!(judged >= 3, "expected fail fixtures, judged {judged}");
}

/// Collect `.ifc` files recursively, in a stable order.
fn ifc_fixtures(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "ifc") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Every deliberately-invalid fixture must actually be invalid.
///
/// The exclusion list is a claim, and an unexamined exclusion list is how a
/// real defect hides: adding a filename silences the corpus test forever.
/// This proves each entry still fails validation, so a fixture that gets
/// repaired must be removed from the list rather than left as a permanent
/// blind spot.
#[test]
fn every_excluded_fixture_is_genuinely_invalid() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures");
    for excluded in DELIBERATELY_INVALID {
        let path = ifc_fixtures(&root)
            .into_iter()
            .find(|path| path.file_name().is_some_and(|name| name == *excluded))
            .unwrap_or_else(|| panic!("{excluded} is excluded but does not exist"));
        let model = StepCodec
            .read_path(&path)
            .unwrap_or_else(|error| panic!("{excluded}: {error}"));
        let report = ifc_validate::validate_declared(&model)
            .unwrap_or_else(|error| panic!("{excluded}: {error}"));
        assert!(
            !report.is_conformant(),
            "{excluded} is on the deliberately-invalid list but validates \
             clean; remove it from the list instead of hiding the fixture"
        );
    }
}

/// A GUID shorter than 22 characters is reported.
///
/// `IfcGloballyUniqueId` is `STRING(22) FIXED` in both bundled schemas, so a
/// 21-character GUID is malformed regardless of uniqueness. IfcOpenShell
/// reports the same defect in `issue_2019_wall_two_overlapping_openings.ifc`;
/// this validator could not see it until IFC2x3 tables existed.
#[test]
fn a_guid_of_the_wrong_width_is_reported() {
    let mut model = Model::new();
    model.push(ifc_model::Entity::new(
        "IFCWALL",
        vec![ifc_model::Value::Text("0000000000000000000C1".into())],
    ));
    let report = validate(&model, ifc_schema::ifc4());
    let found: Vec<_> = report
        .findings()
        .iter()
        .filter(|f| f.rule == "type.scalar.fixed_width")
        .collect();
    assert_eq!(found.len(), 1, "{:#?}", report.findings());
    assert!(
        found[0].message.contains("22") && found[0].message.contains("21"),
        "the message must state both widths: {}",
        found[0].message
    );
}

/// A correctly sized GUID is not reported.
///
/// Guards against a width check that fires on every file.
#[test]
fn a_guid_of_the_right_width_is_accepted() {
    let mut model = Model::new();
    model.push(ifc_model::Entity::new(
        "IFCWALL",
        vec![ifc_model::Value::Text("0hMOPMBpTAoOL$IPqA1$xY".into())],
    ));
    let report = validate(&model, ifc_schema::ifc4());
    assert!(!report
        .findings()
        .iter()
        .any(|f| f.rule == "type.scalar.fixed_width"));
}

/// `OwnerHistory` is mandatory in IFC2x3 and optional in IFC4.
///
/// The single clearest proof that the two bundled tables are actually
/// consulted independently: the same record is conformant under one schema
/// and not the other.
#[test]
fn owner_history_is_mandatory_only_in_ifc2x3() {
    // IFC2x3 IfcWall has 8 slots, IFC4 has 9 (PredefinedType). Build each
    // record at its own width: a short record trips the slot-count guard and
    // never reaches the optionality check.
    let record = |slots: usize| {
        let mut attributes = vec![ifc_model::Value::Null; slots];
        attributes[0] = ifc_model::Value::Text("0hMOPMBpTAoOL$IPqA1$xY".into());
        let mut model = Model::new();
        model.push(ifc_model::Entity::new("IFCWALL", attributes));
        model
    };
    let missing = |model: &Model, schema| {
        validate(model, schema)
            .findings()
            .iter()
            .filter(|f| {
                f.rule == "structure.required.missing"
                    && f.path.to_string().contains("OwnerHistory")
            })
            .count()
    };
    assert_eq!(
        missing(&record(8), ifc_schema::ifc2x3()),
        1,
        "IFC2x3 requires OwnerHistory"
    );
    assert_eq!(
        missing(&record(9), ifc_schema::ifc4()),
        0,
        "IFC4 makes OwnerHistory OPTIONAL"
    );
}
