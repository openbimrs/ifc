//! Simple and complex property templates read by attribute name (#108).
//!
//! `references/ifc-spec/ifc4-add2-tc1/IFC4.exp` and
//! `references/ifc-spec/ifc4x3-add2/IFC4X3_ADD2.exp` declare, after the four
//! `IfcRoot` attributes:
//!
//! ```text
//! IfcSimplePropertyTemplate   TemplateType PrimaryMeasureType SecondaryMeasureType
//!                             Enumerators PrimaryUnit SecondaryUnit Expression
//!                             AccessState
//! IfcComplexPropertyTemplate  UsageName TemplateType HasPropertyTemplates
//! IfcPropertySetTemplate      TemplateType ApplicableEntity HasPropertyTemplates
//! ```
//!
//! IFC2X3 TC1 declares none of them. Every value below is distinct, so a
//! reader that takes any attribute from another's slot fails.

use ifc_model::{Codec, Entity, EntityId, Model, Transaction, Value};
use ifc_properties::{
    add_complex_property_template, add_property_set_template, defines_by_template_slot,
    property_set_template, property_set_template_checked, property_template,
    property_template_checked, pset_template_slot, PropertyAnomaly, PropertyTemplateKind,
    SchemaVersion, TemplateError,
};
use ifc_schema::{ifc4, ifc4x3};
use ifc_step::StepCodec;

/// A model parsed from STEP `records` under `FILE_SCHEMA((schema))`.
fn parse(schema: &str, records: &[String]) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\n\
         DATA;\n{}\nENDSEC;\nEND-ISO-10303-21;\n",
        records.join("\n")
    );
    let model = StepCodec
        .read_bytes(text.as_bytes())
        .unwrap_or_else(|e| panic!("fixture must parse: {e:?}"));
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

fn guid(id: u64) -> String {
    format!("'{id:0>22}'")
}

/// A simple template with every slot set to a distinct value.
fn full_simple(id: u64) -> String {
    format!(
        "#{id}=IFCSIMPLEPROPERTYTEMPLATE({},$,'Flow','Rated flow',.P_TABLEVALUE.,\
         'IfcVolumetricFlowRateMeasure','IfcPressureMeasure',#90,#91,#92,'y = 2x',.READONLY.);",
        guid(id)
    )
}

/// Units and an enumeration the full template points at.
fn referenced() -> Vec<String> {
    vec![
        "#90=IFCPROPERTYENUMERATION('Grades',(IFCLABEL('A'),IFCLABEL('B')),$);".into(),
        "#91=IFCSIUNIT(*,.VOLUMEUNIT.,$,.CUBIC_METRE.);".into(),
        "#92=IFCSIUNIT(*,.PRESSUREUNIT.,$,.PASCAL.);".into(),
    ]
}

#[test]
fn every_simple_template_slot_reads_by_name() {
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let mut records = referenced();
        records.push(full_simple(10));
        let model = parse(schema, &records);
        let (template, anomalies) =
            property_template_checked(&model, EntityId(10)).expect("a simple template");
        assert!(anomalies.is_empty(), "{schema}: {anomalies:?}");
        assert_eq!(template.kind, PropertyTemplateKind::Simple);
        assert_eq!(template.name.as_deref(), Some("Flow"));
        assert_eq!(template.description.as_deref(), Some("Rated flow"));
        assert_eq!(template.template_type.as_deref(), Some("P_TABLEVALUE"));
        assert_eq!(
            template.primary_measure.as_deref(),
            Some("IfcVolumetricFlowRateMeasure")
        );
        assert_eq!(
            template.secondary_measure.as_deref(),
            Some("IfcPressureMeasure")
        );
        assert_eq!(template.enumerators, Some(EntityId(90)));
        assert_eq!(template.primary_unit, Some(EntityId(91)));
        assert_eq!(template.secondary_unit, Some(EntityId(92)));
        assert_eq!(template.expression.as_deref(), Some("y = 2x"));
        assert_eq!(template.access_state.as_deref(), Some("READONLY"));
        assert_eq!(template.usage_name, None);
        assert!(template.templates.is_empty());
        assert_eq!(property_template(&model, EntityId(10)), Some(template));
    }
}

/// A complex template no longer reads its `UsageName` as `TemplateType`.
#[test]
fn a_complex_template_reads_its_own_layout_and_nested_templates() {
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let mut records = referenced();
        records.push(full_simple(10));
        records.push(format!(
            "#11=IFCCOMPLEXPROPERTYTEMPLATE({},$,'Leaf','Door leaf','LeafUsage',.P_COMPLEX.,(#10,#12));",
            guid(11)
        ));
        records.push(format!(
            "#12=IFCCOMPLEXPROPERTYTEMPLATE({},$,'Frame',$,$,.Q_COMPLEX.,$);",
            guid(12)
        ));
        records.push(format!(
            "#13=IFCPROPERTYSETTEMPLATE({},$,'Pset_Door','Doors',.PSET_TYPEDRIVENOVERRIDE.,'IfcDoor',(#11));",
            guid(13)
        ));
        let model = parse(schema, &records);
        let (set, anomalies) =
            property_set_template_checked(&model, EntityId(13)).expect("a set template");
        assert!(anomalies.is_empty(), "{schema}: {anomalies:?}");
        assert_eq!(set.name.as_deref(), Some("Pset_Door"));
        assert_eq!(set.description.as_deref(), Some("Doors"));
        assert_eq!(
            set.template_type.as_deref(),
            Some("PSET_TYPEDRIVENOVERRIDE")
        );
        assert_eq!(set.applicable_entity.as_deref(), Some("IfcDoor"));
        let leaf = set.property("Leaf").expect("the complex template");
        assert_eq!(leaf.kind, PropertyTemplateKind::Complex);
        assert_eq!(leaf.description.as_deref(), Some("Door leaf"));
        assert_eq!(leaf.usage_name.as_deref(), Some("LeafUsage"));
        assert_eq!(leaf.template_type.as_deref(), Some("P_COMPLEX"));
        assert_eq!(leaf.primary_measure, None);
        assert_eq!(leaf.access_state, None);
        let names: Vec<_> = leaf.templates.iter().map(|t| t.name.clone()).collect();
        assert_eq!(names, [Some("Flow".into()), Some("Frame".into())]);
        let flow = leaf.template("Flow").expect("nested simple template");
        assert_eq!(flow.expression.as_deref(), Some("y = 2x"));
        let frame = leaf.template("Frame").expect("nested complex template");
        assert_eq!(frame.template_type.as_deref(), Some("Q_COMPLEX"));
        assert_eq!(frame.usage_name, None);
        assert!(frame.templates.is_empty(), "HasPropertyTemplates is $");
        assert_eq!(property_set_template(&model, EntityId(13)), Some(set));
    }
}

/// What `add_complex_property_template` writes survives STEP text and reads
/// back as a complex template.
#[test]
fn an_authored_complex_template_round_trips() {
    let mut model = Model::default();
    model.header_mut().schema = vec!["IFC4".into()];
    let mut tx = Transaction::new(&model);
    let child = tx.create(Entity::new("IFCSIMPLEPROPERTYTEMPLATE", {
        let mut a = vec![Value::Null; 12];
        a[0] = Value::Text("0aBcDeFgHiJkLmNoPqRsTu".into());
        a[2] = Value::Text("Width".into());
        a[4] = Value::Enum("P_SINGLEVALUE".into());
        a[5] = Value::Text("IfcLengthMeasure".into());
        a
    }));
    let complex = add_complex_property_template(
        &mut tx,
        "1aBcDeFgHiJkLmNoPqRsTu",
        Some("Assembly"),
        (Some("Leaf"), Some("P_COMPLEX")),
        &[("Width", child)],
    )
    .expect("complex template");
    let set = add_property_set_template(
        &mut tx,
        "2aBcDeFgHiJkLmNoPqRsTu",
        "Pset_Assembly",
        Some("IfcWall"),
        &[complex],
    )
    .expect("set template");
    tx.commit(&mut model).expect("commit");

    let bytes = StepCodec.write_bytes(&model).expect("written");
    let reparsed = StepCodec.read_bytes(&bytes).expect("reparsed");
    let (template, anomalies) =
        property_set_template_checked(&reparsed, set).expect("the set template");
    assert!(anomalies.is_empty(), "{anomalies:?}");
    let assembly = template.property("Assembly").expect("complex member");
    assert_eq!(assembly.kind, PropertyTemplateKind::Complex);
    assert_eq!(assembly.usage_name.as_deref(), Some("Leaf"));
    assert_eq!(assembly.template_type.as_deref(), Some("P_COMPLEX"));
    let width = assembly.template("Width").expect("nested template");
    assert_eq!(width.template_type.as_deref(), Some("P_SINGLEVALUE"));
    assert_eq!(width.primary_measure.as_deref(), Some("IfcLengthMeasure"));
}

/// Complex templates in a cycle of any length are cut on the path, and the
/// cut is reported, not silently truncated.
#[test]
fn a_nested_template_cycle_is_cut_and_reported() {
    let complex = |id: u64, members: &str| {
        format!(
            "#{id}=IFCCOMPLEXPROPERTYTEMPLATE({},$,'C{id}',$,$,.P_COMPLEX.,({members}));",
            guid(id)
        )
    };
    let records = vec![
        complex(10, "#11"),
        complex(11, "#12"),
        complex(12, "#10"),
        complex(20, "#20"),
    ];
    let model = parse("IFC4", &records);

    let (root, anomalies) = property_template_checked(&model, EntityId(10)).expect("template");
    assert_eq!(
        anomalies,
        [PropertyAnomaly::ComplexCycle {
            complex: EntityId(12),
            member: EntityId(10),
        }]
    );
    let depth = |mut t: &ifc_properties::PropertyTemplate| {
        let mut depth = 0;
        while let Some(next) = t.templates.first() {
            depth += 1;
            t = next;
        }
        depth
    };
    assert_eq!(depth(&root), 2, "#10 > #11 > #12, and #12's #10 is cut");

    // `NoSelfReference`, the direct case the schema itself forbids.
    let (own, anomalies) = property_template_checked(&model, EntityId(20)).expect("template");
    assert!(own.templates.is_empty());
    assert_eq!(
        anomalies,
        [PropertyAnomaly::ComplexCycle {
            complex: EntityId(20),
            member: EntityId(20),
        }]
    );
}

/// Depth and budget bound a legal but deep or heavily shared nesting.
#[test]
fn deep_and_shared_template_nesting_is_bounded() {
    // A chain 40 deep.
    let mut records: Vec<String> = (100..140)
        .map(|id| {
            format!(
                "#{id}=IFCCOMPLEXPROPERTYTEMPLATE({},$,'C{id}',$,$,$,(#{}));",
                guid(id),
                id + 1
            )
        })
        .collect();
    records.push(format!(
        "#140=IFCSIMPLEPROPERTYTEMPLATE({},$,'Leaf',$,$,$,$,$,$,$,$,$);",
        guid(140)
    ));
    // Two distinct members per level, both leading to the same next level:
    // 2^16 paths from 32 records.
    for level in 0..16u64 {
        let (a, b) = (200 + 2 * level, 201 + 2 * level);
        let next = if level == 15 {
            "#140".to_owned()
        } else {
            format!("#{},#{}", a + 2, b + 2)
        };
        for id in [a, b] {
            records.push(format!(
                "#{id}=IFCCOMPLEXPROPERTYTEMPLATE({},$,'D{id}',$,$,$,({next}));",
                guid(id)
            ));
        }
    }
    let model = parse("IFC4", &records);

    let (_, anomalies) = property_template_checked(&model, EntityId(100)).expect("template");
    assert!(
        matches!(
            anomalies.as_slice(),
            [PropertyAnomaly::ComplexTooDeep { limit: 16, .. }]
        ),
        "{anomalies:?}"
    );
    let (_, anomalies) = property_template_checked(&model, EntityId(200)).expect("template");
    assert!(
        anomalies
            .iter()
            .any(|a| matches!(a, PropertyAnomaly::ComplexBudgetExceeded { .. })),
        "{anomalies:?}"
    );
}

/// Malformed members and attributes are reported by the checked readers.
#[test]
fn malformed_template_facts_are_reported() {
    let records = vec![
        format!(
            "#10=IFCPROPERTYSETTEMPLATE({},$,'Pset_X',$,.NOTDEFINED.,$,(#11,#12,#13,#99));",
            guid(10)
        ),
        // Not a template at all.
        "#11=IFCPROPERTYSINGLEVALUE('Width',$,$,$);".into(),
        // `Q_NUMBER` is an IFC4X3 constant, not an IFC4 one.
        format!(
            "#12=IFCSIMPLEPROPERTYTEMPLATE({},$,'Count',$,.Q_NUMBER.,$,$,$,$,$,$,$);",
            guid(12)
        ),
        format!(
            "#13=IFCSIMPLEPROPERTYTEMPLATE({},$,'Count',$,.Q_COUNT.,$,$,$,$,$,$,$);",
            guid(13)
        ),
    ];
    let model = parse("IFC4", &records);
    let (template, anomalies) =
        property_set_template_checked(&model, EntityId(10)).expect("set template");
    assert_eq!(template.properties.len(), 2);
    assert_eq!(
        anomalies,
        [
            PropertyAnomaly::NotATemplate {
                container: EntityId(10),
                member: EntityId(11),
                type_name: "IFCPROPERTYSINGLEVALUE".into(),
            },
            PropertyAnomaly::MalformedAttribute {
                entity: EntityId(12),
                attribute: "TemplateType",
                found: "Enum(\"Q_NUMBER\")".into(),
            },
            PropertyAnomaly::MissingMember {
                container: EntityId(10),
                member: EntityId(99),
            },
            PropertyAnomaly::DuplicatePropertyName {
                set: EntityId(10),
                kept: EntityId(12),
                rejected: EntityId(13),
            },
        ]
    );
    // The constant is still reported as written.
    assert_eq!(
        template.properties[0].template_type.as_deref(),
        Some("Q_NUMBER")
    );
}

/// Only template entities read as templates, and IFC2X3 has none.
#[test]
fn non_templates_and_ifc2x3_are_refused() {
    let records = vec!["#1=IFCPROPERTYSINGLEVALUE('Width',$,$,$);".to_owned()];
    let model = parse("IFC4", &records);
    assert_eq!(property_template(&model, EntityId(1)), None);
    assert_eq!(property_set_template(&model, EntityId(1)), None);
    assert_eq!(
        property_template_checked(&model, EntityId(1)),
        Err(TemplateError::NotATemplate {
            id: EntityId(1),
            type_name: "IFCPROPERTYSINGLEVALUE".into(),
        })
    );
    assert_eq!(
        property_template_checked(&model, EntityId(2)),
        Err(TemplateError::MissingEntity { id: EntityId(2) })
    );

    let ifc2x3 = parse(
        "IFC2X3",
        &["#1=IFCPROPERTYSINGLEVALUE('Width',$,$,$);".to_owned()],
    );
    assert_eq!(
        property_template_checked(&ifc2x3, EntityId(1)),
        Err(TemplateError::NoTemplates {
            schema: SchemaVersion::Ifc2x3,
        })
    );
    assert_eq!(
        property_set_template_checked(&ifc2x3, EntityId(1)),
        Err(TemplateError::NoTemplates {
            schema: SchemaVersion::Ifc2x3,
        })
    );
}

/// The authoring slot constants are positional; they must name the
/// attributes the reader looks up by name, in both releases with templates.
#[test]
fn template_slot_constants_match_the_schema() {
    let expected = [
        (
            "IfcPropertySetTemplate",
            vec![
                (pset_template_slot::GLOBAL_ID, "GlobalId"),
                (pset_template_slot::NAME, "Name"),
                (pset_template_slot::DESCRIPTION, "Description"),
                (pset_template_slot::TEMPLATE_TYPE, "TemplateType"),
                (pset_template_slot::APPLICABLE_ENTITY, "ApplicableEntity"),
                (
                    pset_template_slot::HAS_PROPERTY_TEMPLATES,
                    "HasPropertyTemplates",
                ),
            ],
        ),
        (
            "IfcRelDefinesByTemplate",
            vec![
                (defines_by_template_slot::GLOBAL_ID, "GlobalId"),
                (
                    defines_by_template_slot::RELATED_PROPERTY_SETS,
                    "RelatedPropertySets",
                ),
                (
                    defines_by_template_slot::RELATING_TEMPLATE,
                    "RelatingTemplate",
                ),
            ],
        ),
        // `add_complex_property_template` writes these positions.
        (
            "IfcComplexPropertyTemplate",
            vec![
                (4, "UsageName"),
                (5, "TemplateType"),
                (6, "HasPropertyTemplates"),
            ],
        ),
    ];
    for schema in [ifc4(), ifc4x3()] {
        for (entity, slots) in &expected {
            let names = schema.attribute_names(entity);
            for (slot, name) in slots {
                assert_eq!(
                    names.get(*slot),
                    Some(name),
                    "{entity} in {}",
                    schema.name()
                );
            }
        }
    }
}
