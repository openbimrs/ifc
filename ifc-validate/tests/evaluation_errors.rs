//! A native rule that cannot decide an instance says so.
//!
//! Before evaluation errors existed, each of these inputs made its rule
//! `continue` without a finding, so the instance read as checked and passed.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::Schema;
use ifc_validate::{validate, Finding, Report, Severity};

fn entity(schema: &Schema, type_name: &str, values: &[(&str, Value)]) -> Entity {
    let names = schema.attribute_names(type_name);
    let mut attributes = vec![Value::Null; names.len()];
    for (name, value) in values {
        let index = names
            .iter()
            .position(|candidate| candidate.eq_ignore_ascii_case(name))
            .unwrap_or_else(|| panic!("{type_name} has no {name} in {}", schema.name()));
        attributes[index] = value.clone();
    }
    Entity::new(type_name, attributes)
}

/// Findings for `rule`, by severity.
fn verdicts(report: &Report, rule: &str) -> Vec<Severity> {
    report
        .findings()
        .iter()
        .filter(|finding| finding.rule == rule)
        .map(|finding| finding.severity)
        .collect()
}

/// A model whose header declares `token`, so the header check is quiet.
fn declared(token: &str) -> Model {
    let mut model = Model::new();
    model.header_mut().schema = vec![token.to_string()];
    model.header_mut().implementation_level = "2;1".to_string();
    model
}

/// A schema named IFC4, so the IFC4-gated rules run, whose
/// `IfcMaterialLayer` lacks the `Priority` attribute the rule reads.
///
/// Tables that disagree with a rule are a validator defect, not a file
/// defect; that is exactly the case that must not read as a pass.
const TABLES_WITHOUT_PRIORITY: &str = "
SCHEMA IFC4;
ENTITY IfcMaterialLayer;
    Name : OPTIONAL STRING;
END_ENTITY;
END_SCHEMA;
";

/// The one finding the issue asks for: a model whose only defect is an
/// undecidable rule is not reported conformant.
#[test]
fn a_rule_that_cannot_resolve_its_operand_makes_the_model_non_conformant() {
    let schema = Schema::from_express(TABLES_WITHOUT_PRIORITY);
    assert_eq!(schema.version(), Some(ifc_schema::SchemaVersion::Ifc4));
    let mut model = declared("IFC4");
    model.push(Entity::new(
        "IFCMATERIALLAYER",
        vec![Value::Text("layer".into())],
    ));

    let report = validate(&model, &schema);

    let decisive: Vec<&Finding> = report
        .findings()
        .iter()
        .filter(|finding| finding.severity != Severity::Unsupported)
        .collect();
    assert_eq!(
        decisive.len(),
        1,
        "exactly the evaluation error, nothing else: {decisive:#?}"
    );
    let finding = decisive[0];
    assert_eq!(finding.severity, Severity::EvaluationError);
    assert_eq!(finding.rule, "IfcMaterialLayer.NormalizedPriority");
    assert!(finding.message.contains("Priority"), "{}", finding.message);
    assert_eq!(report.summary().evaluation_errors, 1);
    assert_eq!(report.summary().errors, 0);
    assert!(
        !report.is_conformant(),
        "a rule nobody could decide is not a rule that passed"
    );
}

/// The same instance under the real tables is decided, and passes.
#[test]
fn the_same_record_under_complete_tables_is_conformant() {
    let schema = ifc_schema::ifc4();
    let mut model = declared("IFC4");
    model.push(entity(
        schema,
        "IFCMATERIALLAYER",
        &[("LayerThickness", Value::Real(1.0))],
    ));
    let report = validate(&model, schema);
    assert!(report.is_conformant(), "{:#?}", report.findings());
}

/// Every native rule, fed an operand shape it cannot reason about.
///
/// Each input is paired with the operand shape the rule can read, so a
/// rule that reports evaluation errors for everything fails too.
#[test]
fn every_unreadable_operand_is_an_evaluation_error() {
    let schema = ifc_schema::ifc4();
    type Build = fn(&Schema, Value) -> Model;
    let cases: &[(&str, Build, Value, Value)] = &[
        (
            "IfcMaterialLayer.NormalizedPriority",
            |schema, priority| {
                let mut model = Model::new();
                model.push(entity(
                    schema,
                    "IFCMATERIALLAYER",
                    &[("LayerThickness", Value::Real(1.0)), ("Priority", priority)],
                ));
                model
            },
            Value::Text("high".into()),
            Value::Integer(10),
        ),
        (
            "IfcRelConnectsPathElements.NormalizedRelatingPriorities",
            |schema, priorities| {
                let mut model = Model::new();
                model.push(entity(
                    schema,
                    "IFCRELCONNECTSPATHELEMENTS",
                    &[("RelatingPriorities", priorities)],
                ));
                model
            },
            Value::List(vec![Value::Text("first".into())]),
            Value::List(vec![Value::Integer(1)]),
        ),
        (
            "IfcRelConnectsPathElements.NormalizedRelatedPriorities",
            |schema, priorities| {
                let mut model = Model::new();
                model.push(entity(
                    schema,
                    "IFCRELCONNECTSPATHELEMENTS",
                    &[("RelatedPriorities", priorities)],
                ));
                model
            },
            Value::Integer(5),
            Value::List(vec![Value::Integer(5)]),
        ),
        (
            "IfcRelSequence.AvoidInconsistentSequence",
            |schema, relating| {
                let mut model = Model::new();
                model.insert(EntityId(1), entity(schema, "IFCTASK", &[]));
                model.push(entity(
                    schema,
                    "IFCRELSEQUENCE",
                    &[
                        ("RelatingProcess", relating),
                        ("RelatedProcess", Value::Ref(EntityId(1))),
                    ],
                ));
                model
            },
            Value::Text("task".into()),
            Value::Ref(EntityId(2)),
        ),
        (
            "IfcRelAggregates.NoSelfReference",
            |schema, relating| {
                let mut model = Model::new();
                model.insert(EntityId(1), entity(schema, "IFCPROJECT", &[]));
                model.push(entity(
                    schema,
                    "IFCRELAGGREGATES",
                    &[
                        ("RelatingObject", relating),
                        ("RelatedObjects", Value::List(vec![Value::Ref(EntityId(1))])),
                    ],
                ));
                model
            },
            Value::Integer(1),
            Value::Ref(EntityId(2)),
        ),
        (
            "IfcRelAssignsToGroupByFactor.NoSelfReference",
            |schema, relating| {
                let mut model = Model::new();
                model.insert(EntityId(1), entity(schema, "IFCWALL", &[]));
                model.push(entity(
                    schema,
                    "IFCRELASSIGNSTOGROUPBYFACTOR",
                    &[
                        ("RelatingGroup", relating),
                        ("RelatedObjects", Value::List(vec![Value::Ref(EntityId(1))])),
                    ],
                ));
                model
            },
            Value::Text("group".into()),
            Value::Ref(EntityId(2)),
        ),
        (
            "IfcRelDefinesByProperties.NoRelatedTypeObject",
            |schema, related| {
                let mut model = Model::new();
                model.insert(EntityId(1), entity(schema, "IFCWALL", &[]));
                model.push(entity(
                    schema,
                    "IFCRELDEFINESBYPROPERTIES",
                    &[("RelatedObjects", related)],
                ));
                model
            },
            // #99 does not exist: whether it is a type object is unknowable.
            Value::List(vec![Value::Ref(EntityId(99))]),
            Value::List(vec![Value::Ref(EntityId(1))]),
        ),
        (
            "IfcRelDefinesByProperties.NoRelatedTypeObject",
            |schema, related| {
                let mut model = Model::new();
                model.insert(EntityId(1), entity(schema, "IFCWALL", &[]));
                model.push(entity(
                    schema,
                    "IFCRELDEFINESBYPROPERTIES",
                    &[("RelatedObjects", related)],
                ));
                model
            },
            Value::List(vec![Value::Text("wall".into())]),
            Value::List(vec![Value::Ref(EntityId(1))]),
        ),
        (
            "IfcRelSpaceBoundary.CorrectPhysOrVirt",
            |schema, element| {
                let mut model = Model::new();
                model.insert(EntityId(1), entity(schema, "IFCWALL", &[]));
                model.push(entity(
                    schema,
                    "IFCRELSPACEBOUNDARY2NDLEVEL",
                    &[
                        ("RelatedBuildingElement", element),
                        ("PhysicalOrVirtualBoundary", Value::Enum("PHYSICAL".into())),
                    ],
                ));
                model
            },
            Value::Ref(EntityId(99)),
            Value::Ref(EntityId(1)),
        ),
        (
            "IfcRelSpaceBoundary.CorrectPhysOrVirt",
            |schema, physicality| {
                let mut model = Model::new();
                model.insert(EntityId(1), entity(schema, "IFCWALL", &[]));
                model.push(entity(
                    schema,
                    "IFCRELSPACEBOUNDARY",
                    &[
                        ("RelatedBuildingElement", Value::Ref(EntityId(1))),
                        ("PhysicalOrVirtualBoundary", physicality),
                    ],
                ));
                model
            },
            Value::Enum("SOMETIMES".into()),
            Value::Enum("PHYSICAL".into()),
        ),
        (
            "global.UniqueGlobalId",
            |schema, guid| {
                let mut model = Model::new();
                model.push(entity(schema, "IFCWALL", &[("GlobalId", guid)]));
                model
            },
            Value::Integer(7),
            Value::Text("0hMOPMBpTAoOL$IPqA1$xY".into()),
        ),
    ];
    let mut failures = Vec::new();
    for (rule, build, unreadable, readable) in cases {
        let undecided = verdicts(&validate(&build(schema, unreadable.clone()), schema), rule);
        if undecided != [Severity::EvaluationError] {
            failures.push(format!("{rule} with {unreadable:?}: {undecided:?}"));
        }
        let decided = verdicts(&validate(&build(schema, readable.clone()), schema), rule);
        if !decided.is_empty() {
            failures.push(format!("{rule} with {readable:?}: {decided:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// An unset operand is not an evaluation failure.
///
/// `NormalizedPriority` is `NOT(EXISTS(Priority)) OR ...` in the bundled
/// IFC4 EXPRESS, so `$` satisfies it; and presence of a mandatory operand
/// is `structure::required`'s verdict, not an undecided rule.
#[test]
fn an_unset_operand_is_not_an_evaluation_error() {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.push(entity(
        schema,
        "IFCMATERIALLAYER",
        &[("LayerThickness", Value::Real(1.0))],
    ));
    model.push(entity(schema, "IFCRELSEQUENCE", &[]));
    let report = validate(&model, schema);
    assert!(
        !report
            .findings()
            .iter()
            .any(|finding| finding.severity == Severity::EvaluationError),
        "{:#?}",
        report.findings()
    );
}

/// `IfcExternalReference.WR1` is a disjunction: one resolvable, present
/// operand decides it even when the tables lack another.
#[test]
fn a_satisfied_disjunction_is_decided_despite_an_unresolvable_operand() {
    let schema = Schema::from_express(
        "
SCHEMA IFC4;
ENTITY IfcExternalReference;
    Location : OPTIONAL STRING;
    Name : OPTIONAL STRING;
END_ENTITY;
END_SCHEMA;
",
    );
    let record = |location: Value| {
        let mut model = declared("IFC4");
        model.push(Entity::new(
            "IFCEXTERNALREFERENCE",
            vec![location, Value::Null],
        ));
        validate(&model, &schema)
    };
    let rule = "IfcExternalReference.WR1";

    // Identification is missing from the tables and nothing else is set:
    // the rule cannot be decided, and must not be reported as violated.
    let empty = record(Value::Null);
    assert_eq!(verdicts(&empty, rule), [Severity::EvaluationError]);

    // Location is set: the rule holds whatever Identification would say,
    // so it is decided, and a decided rule that holds says nothing.
    let located = record(Value::Text("urn:x".into()));
    assert_eq!(verdicts(&located, rule), []);
    assert!(located.is_conformant(), "{:#?}", located.findings());
}
