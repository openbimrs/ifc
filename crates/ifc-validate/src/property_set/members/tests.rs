use super::super::super::CatalogQuantityKind;
use super::*;
use crate::Severity;
use ifc_model::Entity;
use ifc_schema::SchemaVersion;

/// A catalog giving `Pset_Test` (properties) and `Qto_Test`
/// (quantities), and nothing for any other name.
struct Fixed {
    templates: Vec<(&'static str, CatalogSet)>,
    known: Vec<&'static str>,
}

impl PropertySetCatalog for Fixed {
    fn defines(&self, _: SchemaVersion, name: &str) -> Option<bool> {
        Some(self.known.contains(&name))
    }

    fn template(&self, _: SchemaVersion, name: &str) -> Option<CatalogSet> {
        self.templates
            .iter()
            .find(|(candidate, _)| *candidate == name)
            .map(|(_, set)| set.clone())
    }
}

fn single(name: &str, data_type: Option<&str>) -> CatalogMember {
    CatalogMember::new(
        name,
        CatalogMemberForm::SingleValue {
            data_type: data_type.map(str::to_owned),
        },
    )
}

fn props(members: Vec<CatalogMember>) -> Fixed {
    Fixed {
        templates: vec![("Pset_Test", CatalogSet::property(members))],
        known: vec!["Pset_Test"],
    }
}

fn text(value: &str) -> Value {
    Value::Text(value.into())
}

fn typed(type_name: &str, value: Value) -> Value {
    Value::Typed {
        type_name: type_name.into(),
        value: Box::new(value),
    }
}

fn refs(ids: &[EntityId]) -> Value {
    Value::List(ids.iter().map(|id| Value::Ref(*id)).collect())
}

/// A single-value property `name` holding `value`.
fn prop(model: &mut Model, name: &str, value: Value) -> EntityId {
    model.push(Entity::new(
        "IFCPROPERTYSINGLEVALUE",
        vec![text(name), Value::Null, value, Value::Null],
    ))
}

fn set(model: &mut Model, type_name: &str, name: &str, members: &[EntityId]) -> EntityId {
    model.push(Entity::new(
        type_name,
        vec![
            text("0abcdefghijklmnopqrstu"),
            Value::Null,
            text(name),
            Value::Null,
            refs(members),
        ],
    ))
}

fn run(model: &Model, catalog: &Fixed, schema: &Schema) -> Report {
    let mut report = Report::new();
    super::super::check(model, schema, Some(catalog), &mut report);
    report
}

fn rules<'a>(report: &'a Report, rule: &str) -> Vec<&'a Finding> {
    report
        .findings()
        .iter()
        .filter(|finding| finding.rule == rule)
        .collect()
}

fn one_prop(name: &str, value: Value) -> Model {
    let mut model = Model::new();
    let property = prop(&mut model, name, value);
    set(&mut model, "IFCPROPERTYSET", "Pset_Test", &[property]);
    model
}

#[test]
fn an_undeclared_member_is_a_warning() {
    let catalog = props(vec![single("IsExternal", Some("IfcBoolean"))]);
    let model = one_prop("MyOwnProp", typed("IFCBOOLEAN", Value::Bool(true)));
    let report = run(&model, &catalog, ifc_schema::ifc4());
    let found = rules(&report, UNDECLARED);
    assert_eq!(found.len(), 1, "{:?}", report.findings());
    assert_eq!(found[0].severity, Severity::Warning);
    assert!(found[0].message.contains("'MyOwnProp'"));
    assert!(matches!(
        found[0].path,
        Path::Attribute { entity: EntityId(1), index: 0, ref name } if name.as_deref() == Some("Name")
    ));
    assert!(matches!(
        found[0].detail,
        Some(FindingDetail::CatalogMember(CatalogMemberDetail {
            mismatch: CatalogMismatch::UndeclaredMember,
            ..
        }))
    ));
}

#[test]
fn a_case_only_difference_is_undeclared_with_a_hint() {
    let catalog = props(vec![single("IsExternal", Some("IfcBoolean"))]);
    let model = one_prop("isexternal", typed("IFCBOOLEAN", Value::Bool(true)));
    let report = run(&model, &catalog, ifc_schema::ifc4());
    let found = rules(&report, UNDECLARED);
    assert_eq!(found.len(), 1);
    assert!(
        found[0]
            .message
            .contains("the template declares 'IsExternal'"),
        "{}",
        found[0].message
    );
}

#[test]
fn a_declared_member_of_the_declared_type_is_silent() {
    let catalog = props(vec![single("IsExternal", Some("IfcBoolean"))]);
    let model = one_prop("IsExternal", typed("IFCBOOLEAN", Value::Bool(true)));
    let report = run(&model, &catalog, ifc_schema::ifc4());
    assert!(report.findings().is_empty(), "{:?}", report.findings());
}

#[test]
fn another_value_type_is_a_warning_naming_both_in_schema_spelling() {
    let catalog = props(vec![single("IsExternal", Some("IfcBoolean"))]);
    let model = one_prop("IsExternal", typed("IFCLABEL", text("yes")));
    let report = run(&model, &catalog, ifc_schema::ifc4());
    let found = rules(&report, MEMBER_TYPE);
    assert_eq!(found.len(), 1, "{:?}", report.findings());
    assert_eq!(found[0].severity, Severity::Warning);
    assert!(matches!(
        found[0].path,
        Path::Attribute { index: 2, ref name, .. } if name.as_deref() == Some("NominalValue")
    ));
    let Some(FindingDetail::CatalogMember(detail)) = &found[0].detail else {
        panic!("{:?}", found[0].detail);
    };
    assert_eq!(detail.member.as_deref(), Some("IsExternal"));
    assert_eq!(
        detail.mismatch,
        CatalogMismatch::DataType {
            declared: "IfcBoolean".into(),
            found: "IfcLabel".into()
        }
    );
}

#[test]
fn a_defined_type_aliasing_the_declared_one_passes() {
    let catalog = props(vec![single("Width", Some("IfcLengthMeasure"))]);
    let model = one_prop("Width", typed("IFCPOSITIVELENGTHMEASURE", Value::Real(1.0)));
    let report = run(&model, &catalog, ifc_schema::ifc4());
    assert!(report.findings().is_empty(), "{:?}", report.findings());
}

#[test]
fn a_single_value_where_an_enumeration_is_declared_is_a_form_mismatch() {
    let catalog = props(vec![CatalogMember::new(
        "Status",
        CatalogMemberForm::EnumeratedValue {
            data_type: Some("IfcLabel".into()),
        },
    )]);
    let model = one_prop("Status", typed("IFCLABEL", text("NEW")));
    let report = run(&model, &catalog, ifc_schema::ifc4());
    let found = rules(&report, MEMBER_TYPE);
    assert_eq!(found.len(), 1, "{:?}", report.findings());
    assert_eq!(found[0].path, Path::Entity(EntityId(1)));
    assert!(matches!(
        found[0].detail,
        Some(FindingDetail::CatalogMember(CatalogMemberDetail {
            mismatch: CatalogMismatch::Form { ref declared, ref found },
            ..
        })) if declared == "IfcPropertyEnumeratedValue" && found == "IfcPropertySingleValue"
    ));
}

#[test]
fn a_quantity_set_where_a_property_set_is_declared_is_one_finding() {
    let catalog = props(vec![single("IsExternal", Some("IfcBoolean"))]);
    // A set of quantities under a property-set name.
    let model = quantity_model(
        "IFCQUANTITYLENGTH",
        typed("IFCLENGTHMEASURE", Value::Real(1.0)),
        "Pset_Test",
    );
    let report = run(&model, &catalog, ifc_schema::ifc4());
    assert_eq!(report.findings().len(), 1, "{:?}", report.findings());
    let finding = &report.findings()[0];
    assert_eq!(finding.rule, MEMBER_TYPE);
    assert_eq!(finding.path, Path::Entity(EntityId(2)));
    assert!(matches!(
        finding.detail,
        Some(FindingDetail::CatalogMember(CatalogMemberDetail {
            member: None,
            mismatch: CatalogMismatch::SetKind { ref declared, ref found },
            ..
        })) if declared == "IfcPropertySet" && found == "IfcElementQuantity"
    ));
}

fn quantity_model(entity: &str, value: Value, set_name: &str) -> Model {
    let mut model = Model::new();
    let mut attributes = vec![text("Length"), Value::Null, Value::Null];
    attributes.push(value);
    attributes.push(Value::Null);
    let quantity = model.push(Entity::new(entity, attributes));
    model.push(Entity::new(
        "IFCELEMENTQUANTITY",
        vec![
            text("0abcdefghijklmnopqrstu"),
            Value::Null,
            text(set_name),
            Value::Null,
            Value::Null,
            refs(&[quantity]),
        ],
    ));
    model
}

fn quantities() -> Fixed {
    Fixed {
        templates: vec![(
            "Qto_Test",
            CatalogSet::quantity(vec![CatalogMember::new(
                "Length",
                CatalogMemberForm::Quantity(CatalogQuantityKind::Length),
            )]),
        )],
        known: vec!["Qto_Test"],
    }
}

#[test]
fn an_area_quantity_where_a_length_is_declared_is_a_form_mismatch() {
    let area = quantity_model(
        "IFCQUANTITYAREA",
        typed("IFCAREAMEASURE", Value::Real(1.0)),
        "Qto_Test",
    );
    let report = run(&area, &quantities(), ifc_schema::ifc4());
    let found = rules(&report, MEMBER_TYPE);
    assert_eq!(found.len(), 1, "{:?}", report.findings());
    assert!(matches!(
        found[0].detail,
        Some(FindingDetail::CatalogMember(CatalogMemberDetail {
            mismatch: CatalogMismatch::Form { ref declared, ref found },
            ..
        })) if declared == "IfcQuantityLength" && found == "IfcQuantityArea"
    ));
    let length = quantity_model(
        "IFCQUANTITYLENGTH",
        typed("IFCLENGTHMEASURE", Value::Real(1.0)),
        "Qto_Test",
    );
    let report = run(&length, &quantities(), ifc_schema::ifc4());
    assert!(report.findings().is_empty(), "{:?}", report.findings());
}

#[test]
fn a_nested_property_is_reported_by_its_dotted_path() {
    let catalog = props(vec![CatalogMember::new(
        "Outer",
        CatalogMemberForm::Complex {
            members: vec![single("Inner", Some("IfcBoolean"))],
        },
    )]);
    let mut model = Model::new();
    let inner = prop(&mut model, "Stray", typed("IFCBOOLEAN", Value::Bool(true)));
    let complex = model.push(Entity::new(
        "IFCCOMPLEXPROPERTY",
        vec![text("Outer"), Value::Null, text("Usage"), refs(&[inner])],
    ));
    set(&mut model, "IFCPROPERTYSET", "Pset_Test", &[complex]);
    let report = run(&model, &catalog, ifc_schema::ifc4());
    let found = rules(&report, UNDECLARED);
    assert_eq!(found.len(), 1, "{:?}", report.findings());
    assert!(found[0].message.contains("'Outer.Stray'"));
    assert!(matches!(
        found[0].detail,
        Some(FindingDetail::CatalogMember(CatalogMemberDetail { member: Some(ref m), .. }))
            if m == "Outer.Stray"
    ));
}

#[test]
fn a_complex_property_containing_itself_terminates() {
    let catalog = props(vec![CatalogMember::new(
        "Outer",
        CatalogMemberForm::Complex {
            members: vec![single("Inner", None)],
        },
    )]);
    let mut model = Model::new();
    let complex = model.push(Entity::new(
        "IFCCOMPLEXPROPERTY",
        vec![
            text("Outer"),
            Value::Null,
            text("Usage"),
            refs(&[EntityId(1)]),
        ],
    ));
    assert_eq!(complex, EntityId(1));
    set(&mut model, "IFCPROPERTYSET", "Pset_Test", &[complex]);
    let report = run(&model, &catalog, ifc_schema::ifc4());
    assert!(report.findings().is_empty(), "{:?}", report.findings());
}

#[test]
fn a_dangling_member_reference_is_ignored() {
    let catalog = props(vec![single("IsExternal", Some("IfcBoolean"))]);
    let mut model = Model::new();
    set(&mut model, "IFCPROPERTYSET", "Pset_Test", &[EntityId(99)]);
    let report = run(&model, &catalog, ifc_schema::ifc4());
    assert!(report.findings().is_empty(), "{:?}", report.findings());
}

#[test]
fn a_set_without_a_template_is_admitted_once_per_rule() {
    let catalog = Fixed {
        templates: Vec::new(),
        known: vec!["Pset_Test"],
    };
    let model = one_prop("Anything", typed("IFCLABEL", text("x")));
    let report = run(&model, &catalog, ifc_schema::ifc4());
    assert_eq!(report.findings().len(), 2, "{:?}", report.findings());
    for finding in report.findings() {
        assert_eq!(finding.severity, Severity::Unsupported);
        assert_eq!(finding.path, Path::File);
        assert!(finding.message.contains("members of 1 catalog sets"));
    }
    let ids: BTreeSet<_> = report.findings().iter().map(|f| f.rule.as_str()).collect();
    assert_eq!(ids, BTreeSet::from([UNDECLARED, MEMBER_TYPE]));
}

#[test]
fn a_declared_type_the_release_lacks_is_admitted_not_failed() {
    let catalog = props(vec![single("Odd", Some("IfcNoSuchType"))]);
    let model = one_prop("Odd", typed("IFCLABEL", text("x")));
    let report = run(&model, &catalog, ifc_schema::ifc4());
    assert_eq!(report.findings().len(), 1, "{:?}", report.findings());
    let finding = &report.findings()[0];
    assert_eq!(finding.severity, Severity::Unsupported);
    assert_eq!(finding.rule, MEMBER_TYPE);
    assert!(finding.message.contains("Pset_Test.Odd: IfcNoSuchType"));
}

#[test]
fn the_same_checks_run_against_the_ifc4x3_tables() {
    let catalog = props(vec![single("IsExternal", Some("IfcBoolean"))]);
    let model = one_prop("IsExternal", typed("IFCLABEL", text("yes")));
    let report = run(&model, &catalog, ifc_schema::ifc4x3());
    assert_eq!(rules(&report, MEMBER_TYPE).len(), 1);
}
