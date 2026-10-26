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

/// One member entity of `type_name` with `attributes`, alone in `Pset_Test`.
fn member_model(type_name: &str, attributes: Vec<Value>) -> Model {
    let mut model = Model::new();
    let member = model.push(Entity::new(type_name, attributes));
    set(&mut model, "IFCPROPERTYSET", "Pset_Test", &[member]);
    model
}

fn member_types(report: &Report) -> Vec<&Finding> {
    rules(report, MEMBER_TYPE)
}

/// The attribute name a finding points at.
fn slot_of(finding: &Finding) -> &str {
    match &finding.path {
        Path::Attribute { name, .. } => name.as_deref().unwrap_or(""),
        other => panic!("not an attribute path: {other}"),
    }
}

fn labels(values: &[Value]) -> Value {
    Value::List(values.to_vec())
}

fn form(name: &str, form: CatalogMemberForm) -> Fixed {
    props(vec![CatalogMember::new(name, form)])
}

#[test]
fn a_declared_select_takes_every_member_of_its_closure() {
    let catalog = form(
        "Params",
        CatalogMemberForm::ListValue {
            data_type: Some("IfcValue".into()),
        },
    );
    let good = member_model(
        "IFCPROPERTYLISTVALUE",
        vec![
            text("Params"),
            Value::Null,
            labels(&[
                typed("IFCLENGTHMEASURE", Value::Real(1.0)),
                typed("IFCPLANEANGLEMEASURE", Value::Real(1.0)),
                typed("IFCLABEL", text("a")),
            ]),
            Value::Null,
        ],
    );
    let report = run(&good, &catalog, ifc_schema::ifc4());
    assert!(report.findings().is_empty(), "{:?}", report.findings());

    let bad = member_model(
        "IFCPROPERTYLISTVALUE",
        vec![
            text("Params"),
            Value::Null,
            labels(&[typed("IFCNOSUCHTYPE", Value::Real(1.0))]),
            Value::Null,
        ],
    );
    let report = run(&bad, &catalog, ifc_schema::ifc4());
    assert_eq!(member_types(&report).len(), 1, "{:?}", report.findings());
}

#[test]
fn each_bound_of_a_bounded_value_is_checked() {
    let catalog = form(
        "Range",
        CatalogMemberForm::BoundedValue {
            data_type: Some("IfcLengthMeasure".into()),
        },
    );
    let length = || typed("IFCLENGTHMEASURE", Value::Real(1.0));
    let label = || typed("IFCLABEL", text("x"));
    // Name, Description, UpperBoundValue, LowerBoundValue, Unit, SetPointValue.
    for (wrong, slot) in [
        (2, "UpperBoundValue"),
        (3, "LowerBoundValue"),
        (5, "SetPointValue"),
    ] {
        let mut attributes = vec![
            text("Range"),
            Value::Null,
            length(),
            length(),
            Value::Null,
            length(),
        ];
        let clean = member_model("IFCPROPERTYBOUNDEDVALUE", attributes.clone());
        let report = run(&clean, &catalog, ifc_schema::ifc4());
        assert!(report.findings().is_empty(), "{:?}", report.findings());
        attributes[wrong] = label();
        let model = member_model("IFCPROPERTYBOUNDEDVALUE", attributes);
        let report = run(&model, &catalog, ifc_schema::ifc4());
        let found = member_types(&report);
        assert_eq!(found.len(), 1, "{slot}: {:?}", report.findings());
        assert_eq!(slot_of(found[0]), slot);
    }
}

#[test]
fn an_enumerated_value_is_checked_against_its_declared_type() {
    let catalog = form(
        "Status",
        CatalogMemberForm::EnumeratedValue {
            data_type: Some("IfcLabel".into()),
        },
    );
    let model = |value: Value| {
        member_model(
            "IFCPROPERTYENUMERATEDVALUE",
            vec![text("Status"), Value::Null, labels(&[value]), Value::Null],
        )
    };
    let report = run(
        &model(typed("IFCLABEL", text("NEW"))),
        &catalog,
        ifc_schema::ifc4(),
    );
    assert!(report.findings().is_empty(), "{:?}", report.findings());
    let report = run(
        &model(typed("IFCINTEGER", Value::Integer(1))),
        &catalog,
        ifc_schema::ifc4(),
    );
    let found = member_types(&report);
    assert_eq!(found.len(), 1, "{:?}", report.findings());
    assert_eq!(slot_of(found[0]), "EnumerationValues");
}

#[test]
fn every_member_of_a_list_value_is_checked() {
    let catalog = form(
        "Items",
        CatalogMemberForm::ListValue {
            data_type: Some("IfcLabel".into()),
        },
    );
    let model = |second: Value| {
        member_model(
            "IFCPROPERTYLISTVALUE",
            vec![
                text("Items"),
                Value::Null,
                labels(&[typed("IFCLABEL", text("a")), second]),
                Value::Null,
            ],
        )
    };
    let report = run(
        &model(typed("IFCLABEL", text("b"))),
        &catalog,
        ifc_schema::ifc4(),
    );
    assert!(report.findings().is_empty(), "{:?}", report.findings());
    let report = run(
        &model(typed("IFCBOOLEAN", Value::Bool(true))),
        &catalog,
        ifc_schema::ifc4(),
    );
    let found = member_types(&report);
    assert_eq!(found.len(), 1, "{:?}", report.findings());
    assert_eq!(slot_of(found[0]), "ListValues");
}

#[test]
fn a_table_value_swapping_its_columns_fails() {
    let catalog = form(
        "Curve",
        CatalogMemberForm::TableValue {
            defining_type: Some("IfcLengthMeasure".into()),
            defined_type: Some("IfcLabel".into()),
        },
    );
    let length = || typed("IFCLENGTHMEASURE", Value::Real(1.0));
    let label = || typed("IFCLABEL", text("x"));
    // Name, Description, DefiningValues, DefinedValues, Expression,
    // DefiningUnit, DefinedUnit, CurveInterpolation.
    let table = |defining: Value, defined: Value| {
        member_model(
            "IFCPROPERTYTABLEVALUE",
            vec![
                text("Curve"),
                Value::Null,
                labels(&[defining]),
                labels(&[defined]),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
            ],
        )
    };
    let report = run(&table(length(), label()), &catalog, ifc_schema::ifc4());
    assert!(report.findings().is_empty(), "{:?}", report.findings());
    let report = run(&table(label(), length()), &catalog, ifc_schema::ifc4());
    let mut slots: Vec<_> = member_types(&report).into_iter().map(slot_of).collect();
    slots.sort_unstable();
    assert_eq!(slots, ["DefinedValues", "DefiningValues"]);
}

#[test]
fn a_reference_value_must_point_at_the_declared_entity() {
    let catalog = form(
        "Ref",
        CatalogMemberForm::ReferenceValue {
            reference_type: "IfcWall".into(),
        },
    );
    let model = |target: &str| {
        let mut model = Model::new();
        let target = model.push(Entity::new(target, vec![]));
        let reference = model.push(Entity::new(
            "IFCPROPERTYREFERENCEVALUE",
            vec![text("Ref"), Value::Null, Value::Null, Value::Ref(target)],
        ));
        set(&mut model, "IFCPROPERTYSET", "Pset_Test", &[reference]);
        model
    };
    let report = run(&model("IFCWALL"), &catalog, ifc_schema::ifc4());
    assert!(report.findings().is_empty(), "{:?}", report.findings());
    let report = run(&model("IFCDOOR"), &catalog, ifc_schema::ifc4());
    let found = member_types(&report);
    assert_eq!(found.len(), 1, "{:?}", report.findings());
    assert_eq!(slot_of(found[0]), "PropertyReference");
    assert!(matches!(
        &found[0].detail,
        Some(FindingDetail::CatalogMember(CatalogMemberDetail {
            mismatch: CatalogMismatch::DataType { declared, found },
            ..
        })) if declared == "IfcWall" && found == "IfcDoor"
    ));

    let unknown = form(
        "Ref",
        CatalogMemberForm::ReferenceValue {
            reference_type: "IfcNoSuchEntity".into(),
        },
    );
    let report = run(&model("IFCWALL"), &unknown, ifc_schema::ifc4());
    assert_eq!(report.findings().len(), 1, "{:?}", report.findings());
    assert_eq!(report.findings()[0].severity, Severity::Unsupported);
    assert!(report.findings()[0].message.contains("IfcNoSuchEntity"));
}

#[test]
fn two_sets_sharing_a_member_give_one_finding() {
    let catalog = props(vec![single("IsExternal", Some("IfcBoolean"))]);
    let mut model = Model::new();
    let shared = prop(&mut model, "IsExternal", typed("IFCLABEL", text("yes")));
    set(&mut model, "IFCPROPERTYSET", "Pset_Test", &[shared]);
    set(&mut model, "IFCPROPERTYSET", "Pset_Test", &[shared]);
    let report = run(&model, &catalog, ifc_schema::ifc4());
    assert_eq!(member_types(&report).len(), 1, "{:?}", report.findings());
}

#[cfg(feature = "ifc2x3")]
mod history {
    use super::*;

    fn history_catalog() -> Fixed {
        Fixed {
            templates: vec![(
                "Pset_AirTerminalBoxPHistory",
                CatalogSet::property(vec![single("AirflowCurve", Some("IfcTimeSeries"))]),
            )],
            known: vec!["Pset_AirTerminalBoxPHistory"],
        }
    }

    fn history(member: &str, attributes: Vec<Value>, mut model: Model) -> Model {
        let member = model.push(Entity::new(member, attributes));
        set(
            &mut model,
            "IFCPROPERTYSET",
            "Pset_AirTerminalBoxPHistory",
            &[member],
        );
        model
    }

    fn reference_to(target: &str) -> Model {
        let mut model = Model::new();
        let target = model.push(Entity::new(target, vec![]));
        history(
            "IFCPROPERTYREFERENCEVALUE",
            vec![
                text("AirflowCurve"),
                Value::Null,
                Value::Null,
                Value::Ref(target),
            ],
            model,
        )
    }

    #[test]
    fn a_reference_to_the_declared_time_series_is_the_legal_encoding() {
        let model = reference_to("IFCREGULARTIMESERIES");
        let report = run(&model, &history_catalog(), ifc_schema::ifc2x3());
        assert!(report.findings().is_empty(), "{:?}", report.findings());
    }

    #[test]
    fn a_reference_to_the_declared_time_series_value_is_the_legal_encoding() {
        let catalog = Fixed {
            templates: vec![(
                "Pset_AirTerminalBoxPHistory",
                CatalogSet::property(vec![single("AirflowCurve", Some("IfcTimeSeriesValue"))]),
            )],
            known: vec!["Pset_AirTerminalBoxPHistory"],
        };
        let report = run(
            &reference_to("IFCTIMESERIESVALUE"),
            &catalog,
            ifc_schema::ifc2x3(),
        );
        assert!(report.findings().is_empty(), "{:?}", report.findings());
        let report = run(&reference_to("IFCWALL"), &catalog, ifc_schema::ifc2x3());
        assert_eq!(member_types(&report).len(), 1, "{:?}", report.findings());
    }

    #[test]
    fn a_reference_to_another_entity_is_a_data_type_mismatch() {
        let model = reference_to("IFCWALL");
        let report = run(&model, &history_catalog(), ifc_schema::ifc2x3());
        assert_eq!(member_types(&report).len(), 1, "{:?}", report.findings());
    }

    #[test]
    fn any_other_form_is_still_a_form_mismatch() {
        let model = history(
            "IFCPROPERTYLISTVALUE",
            vec![
                text("AirflowCurve"),
                Value::Null,
                Value::List(vec![]),
                Value::Null,
            ],
            Model::new(),
        );
        let report = run(&model, &history_catalog(), ifc_schema::ifc2x3());
        let found = member_types(&report);
        assert_eq!(found.len(), 1, "{:?}", report.findings());
        assert!(matches!(
            &found[0].detail,
            Some(FindingDetail::CatalogMember(CatalogMemberDetail {
                mismatch: CatalogMismatch::Form { .. },
                ..
            }))
        ));
    }
}
