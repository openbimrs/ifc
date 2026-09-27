//! Property sets checked against their in-file templates (#109).
//!
//! One clean fixture: a wall carrying `Pset_Custom` (#20), linked by
//! `IfcRelDefinesByTemplate` (#41) to a template (#120) with one property
//! template per form, a complex one included. Each test changes the few
//! records that produce exactly one kind of deviation. Semantics are those
//! of the IFC4 ADD2 TC1 documentation, cited in `src/template/check/`.

use std::collections::BTreeMap;

use ifc_model::{Codec, EntityId, Model};
use ifc_properties::{
    template_deviations, MeasureRole, PropertyAnomaly, SchemaVersion, TemplateError,
    TemplateFinding, TemplateReport, UndecidedReason,
};
use ifc_step::StepCodec;

const SET: EntityId = EntityId(20);
const TEMPLATE: EntityId = EntityId(120);
const WALL: EntityId = EntityId(1);

fn guid(id: u64) -> String {
    format!("'{id:0>22}'")
}

fn simple(
    id: u64,
    name: &str,
    kind: &str,
    primary: &str,
    secondary: &str,
    enumerators: &str,
) -> String {
    format!(
        "IFCSIMPLEPROPERTYTEMPLATE({},$,'{name}',$,{kind},{primary},{secondary},{enumerators},$,$,$,$)",
        guid(id)
    )
}

/// The clean fixture's records, by id.
fn base() -> BTreeMap<u64, String> {
    let mut r = BTreeMap::new();
    let mut put = |id: u64, record: String| {
        r.insert(id, record);
    };
    put(
        1,
        format!("IFCWALL({},$,'Wall',$,$,$,$,$,.SOLIDWALL.)", guid(1)),
    );
    put(
        2,
        format!("IFCSLAB({},$,'Slab',$,$,$,$,$,.FLOOR.)", guid(2)),
    );
    put(
        3,
        format!("IFCWALLTYPE({},$,'WT',$,$,$,$,$,$,.STANDARD.)", guid(3)),
    );
    put(
        20,
        format!(
            "IFCPROPERTYSET({},$,'Pset_Custom',$,(#21,#22,#23,#24,#25,#26,#27))",
            guid(20)
        ),
    );
    put(
        21,
        "IFCPROPERTYSINGLEVALUE('Width',$,IFCLENGTHMEASURE(0.2),$)".into(),
    );
    put(
        22,
        "IFCPROPERTYENUMERATEDVALUE('Status',$,(IFCLABEL('New')),#110)".into(),
    );
    put(
        23,
        "IFCPROPERTYBOUNDEDVALUE('Range',$,IFCTHERMODYNAMICTEMPERATUREMEASURE(300.),\
         IFCTHERMODYNAMICTEMPERATUREMEASURE(250.),$,$)"
            .into(),
    );
    put(
        24,
        "IFCPROPERTYLISTVALUE('Loads',$,(IFCFORCEMEASURE(1.),IFCFORCEMEASURE(2.)),$)".into(),
    );
    put(
        25,
        "IFCPROPERTYTABLEVALUE('Curve',$,(IFCLENGTHMEASURE(0.),IFCLENGTHMEASURE(1.)),\
         (IFCFORCEMEASURE(0.),IFCFORCEMEASURE(5.)),$,$,$,$)"
            .into(),
    );
    put(26, "IFCPROPERTYREFERENCEVALUE('Material',$,$,#30)".into());
    put(27, "IFCCOMPLEXPROPERTY('Leaf',$,'LeafUsage',(#28))".into());
    put(
        28,
        "IFCPROPERTYSINGLEVALUE('Thickness',$,IFCPOSITIVELENGTHMEASURE(0.05),$)".into(),
    );
    put(30, "IFCMATERIAL('Concrete',$,$)".into());
    put(
        40,
        format!("IFCRELDEFINESBYPROPERTIES({},$,$,$,(#1),#20)", guid(40)),
    );
    put(
        41,
        format!("IFCRELDEFINESBYTEMPLATE({},$,$,$,(#20),#120)", guid(41)),
    );
    put(
        100,
        simple(
            100,
            "Width",
            ".P_SINGLEVALUE.",
            "'IfcLengthMeasure'",
            "$",
            "$",
        ),
    );
    put(
        101,
        simple(
            101,
            "Status",
            ".P_ENUMERATEDVALUE.",
            "'IfcLabel'",
            "$",
            "#110",
        ),
    );
    put(
        102,
        simple(
            102,
            "Range",
            ".P_BOUNDEDVALUE.",
            "'IfcThermodynamicTemperatureMeasure'",
            "'IfcThermodynamicTemperatureMeasure'",
            "$",
        ),
    );
    put(
        103,
        simple(103, "Loads", ".P_LISTVALUE.", "'IfcForceMeasure'", "$", "$"),
    );
    put(
        104,
        simple(
            104,
            "Curve",
            ".P_TABLEVALUE.",
            "'IfcLengthMeasure'",
            "'IfcForceMeasure'",
            "$",
        ),
    );
    put(
        105,
        simple(
            105,
            "Material",
            ".P_REFERENCEVALUE.",
            "'IfcMaterial'",
            "$",
            "$",
        ),
    );
    put(
        106,
        format!(
            "IFCCOMPLEXPROPERTYTEMPLATE({},$,'Leaf',$,'LeafUsage',.P_COMPLEX.,(#107))",
            guid(106)
        ),
    );
    put(
        107,
        simple(
            107,
            "Thickness",
            ".P_SINGLEVALUE.",
            "'IfcPositiveLengthMeasure'",
            "$",
            "$",
        ),
    );
    put(
        110,
        "IFCPROPERTYENUMERATION('Statuses',(IFCLABEL('New'),IFCLABEL('Old')),$)".into(),
    );
    put(
        120,
        format!(
            "IFCPROPERTYSETTEMPLATE({},$,'Pset_Custom',$,.PSET_OCCURRENCEDRIVEN.,'IfcWall',\
             (#100,#101,#102,#103,#104,#105,#106))",
            guid(120)
        ),
    );
    r
}

/// Parse `records` under `schema`.
fn parse(schema: &str, records: &BTreeMap<u64, String>) -> Model {
    let data: Vec<String> = records
        .iter()
        .map(|(id, record)| format!("#{id}={record};"))
        .collect();
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\n\
         DATA;\n{}\nENDSEC;\nEND-ISO-10303-21;\n",
        data.join("\n")
    );
    let model = StepCodec
        .read_bytes(text.as_bytes())
        .unwrap_or_else(|e| panic!("fixture must parse: {e:?}"));
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

/// The base fixture with `changes` applied (an empty record removes one).
fn check_with(schema: &str, changes: &[(u64, &str)]) -> TemplateReport {
    let mut records = base();
    for (id, record) in changes {
        if record.is_empty() {
            records.remove(id);
        } else {
            records.insert(*id, (*record).to_owned());
        }
    }
    let report = template_deviations(&parse(schema, &records)).expect("IFC4 has templates");
    assert!(report.anomalies.is_empty(), "{:?}", report.anomalies);
    report
}

fn check(changes: &[(u64, &str)]) -> Vec<TemplateFinding> {
    check_with("IFC4", changes).findings
}

fn set_of(members: &str) -> String {
    format!("IFCPROPERTYSET({},$,'Pset_Custom',$,({members}))", guid(20))
}

fn template_of(kind: &str, applicable: &str, members: &str) -> String {
    format!(
        "IFCPROPERTYSETTEMPLATE({},$,'Pset_Custom',$,{kind},{applicable},({members}))",
        guid(120)
    )
}

const ALL_TEMPLATES: &str = "#100,#101,#102,#103,#104,#105,#106";

#[test]
fn a_conforming_set_has_no_findings() {
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let report = check_with(schema, &[]);
        assert_eq!(report.findings, [], "{schema}");
    }
}

/// A measure type's specialisation conforms: `IfcPositiveLengthMeasure` is
/// defined as an `IfcLengthMeasure`, so the reading that reports nothing is
/// taken.
#[test]
fn a_specialised_measure_type_conforms() {
    let width = "IFCPROPERTYSINGLEVALUE('Width',$,IFCPOSITIVELENGTHMEASURE(0.2),$)";
    assert_eq!(check(&[(21, width)]), []);
}

#[test]
fn a_missing_property_is_reported() {
    let set = set_of("#21,#22,#23,#24,#25,#27");
    assert_eq!(
        check(&[(20, &set), (26, "")]),
        [TemplateFinding::MissingProperty {
            set: SET,
            template: TEMPLATE,
            container: SET,
            property_template: EntityId(105),
            name: "Material".into(),
        }]
    );
}

#[test]
fn an_unexpected_property_is_reported() {
    let set = set_of("#21,#22,#23,#24,#25,#26,#27,#29");
    let colour = "IFCPROPERTYSINGLEVALUE('Colour',$,IFCLABEL('Red'),$)";
    assert_eq!(
        check(&[(20, &set), (29, colour)]),
        [TemplateFinding::UnexpectedProperty {
            set: SET,
            template: TEMPLATE,
            container: SET,
            property: EntityId(29),
            name: "Colour".into(),
        }]
    );
}

#[test]
fn a_single_value_where_an_enumerated_value_is_prescribed_is_the_wrong_form() {
    let status = "IFCPROPERTYSINGLEVALUE('Status',$,IFCLABEL('New'),$)";
    assert_eq!(
        check(&[(22, status)]),
        [TemplateFinding::WrongForm {
            set: SET,
            template: TEMPLATE,
            property: EntityId(22),
            property_template: EntityId(101),
            template_type: Some("P_ENUMERATEDVALUE".into()),
            expected: &["IFCPROPERTYENUMERATEDVALUE"],
            found: "IFCPROPERTYSINGLEVALUE".into(),
        }]
    );
}

#[test]
fn a_single_value_where_a_complex_property_is_prescribed_is_the_wrong_form() {
    let leaf = "IFCPROPERTYSINGLEVALUE('Leaf',$,IFCLABEL('x'),$)";
    assert_eq!(
        check(&[(27, leaf), (28, "")]),
        [TemplateFinding::WrongForm {
            set: SET,
            template: TEMPLATE,
            property: EntityId(27),
            property_template: EntityId(106),
            template_type: Some("P_COMPLEX".into()),
            expected: &["IFCCOMPLEXPROPERTY"],
            found: "IFCPROPERTYSINGLEVALUE".into(),
        }]
    );
}

#[test]
fn a_primary_measure_type_mismatch_is_reported() {
    let width = "IFCPROPERTYSINGLEVALUE('Width',$,IFCLABEL('wide'),$)";
    assert_eq!(
        check(&[(21, width)]),
        [TemplateFinding::WrongMeasureType {
            set: SET,
            template: TEMPLATE,
            property: EntityId(21),
            property_template: EntityId(100),
            measure: MeasureRole::Primary,
            attribute: "NominalValue",
            expected: "IfcLengthMeasure".into(),
            found: "IFCLABEL".into(),
        }]
    );
}

/// `SecondaryMeasureType` governs a table's defined values and a bounded
/// value's upper bound; `PrimaryMeasureType` its lower bound.
#[test]
fn secondary_measure_type_mismatches_are_reported() {
    let curve = "IFCPROPERTYTABLEVALUE('Curve',$,(IFCLENGTHMEASURE(0.)),\
                 (IFCLENGTHMEASURE(0.)),$,$,$,$)";
    let range = "IFCPROPERTYBOUNDEDVALUE('Range',$,IFCREAL(300.),\
                 IFCTHERMODYNAMICTEMPERATUREMEASURE(250.),$,$)";
    let wrong = |property: u64, template: u64, attribute, expected: &str, found: &str| {
        TemplateFinding::WrongMeasureType {
            set: SET,
            template: TEMPLATE,
            property: EntityId(property),
            property_template: EntityId(template),
            measure: MeasureRole::Secondary,
            attribute,
            expected: expected.into(),
            found: found.into(),
        }
    };
    assert_eq!(
        check(&[(25, curve), (23, range)]),
        [
            wrong(
                23,
                102,
                "UpperBoundValue",
                "IfcThermodynamicTemperatureMeasure",
                "IFCREAL"
            ),
            wrong(
                25,
                104,
                "DefinedValues",
                "IfcForceMeasure",
                "IFCLENGTHMEASURE"
            ),
        ]
    );
}

/// A reference value's measure type is the referenced entity's type.
#[test]
fn a_reference_to_the_wrong_entity_is_a_measure_type_mismatch() {
    let layer = "IFCMATERIALLAYER(#30,0.2,$,$,$,$,$)";
    let reference = "IFCPROPERTYREFERENCEVALUE('Material',$,$,#31)";
    assert_eq!(
        check(&[(26, reference), (31, layer)]),
        [TemplateFinding::WrongMeasureType {
            set: SET,
            template: TEMPLATE,
            property: EntityId(26),
            property_template: EntityId(105),
            measure: MeasureRole::Primary,
            attribute: "PropertyReference",
            expected: "IfcMaterial".into(),
            found: "IFCMATERIALLAYER".into(),
        }]
    );
}

/// A complex property is compared with its complex template, member by
/// member, by the same rules.
#[test]
fn nested_members_are_checked_against_nested_templates() {
    let leaf = "IFCCOMPLEXPROPERTY('Leaf',$,'LeafUsage',(#28,#29))";
    let thickness = "IFCPROPERTYSINGLEVALUE('Thickness',$,IFCLABEL('thin'),$)";
    let extra = "IFCPROPERTYSINGLEVALUE('Hinge',$,$,$)";
    assert_eq!(
        check(&[(27, leaf), (28, thickness), (29, extra)]),
        [
            TemplateFinding::WrongMeasureType {
                set: SET,
                template: TEMPLATE,
                property: EntityId(28),
                property_template: EntityId(107),
                measure: MeasureRole::Primary,
                attribute: "NominalValue",
                expected: "IfcPositiveLengthMeasure".into(),
                found: "IFCLABEL".into(),
            },
            TemplateFinding::UnexpectedProperty {
                set: SET,
                template: TEMPLATE,
                container: EntityId(27),
                property: EntityId(29),
                name: "Hinge".into(),
            },
        ]
    );
    let hollow = "IFCCOMPLEXPROPERTY('Leaf',$,'LeafUsage',(#29))";
    assert_eq!(
        check(&[(27, hollow), (28, ""), (29, extra)]),
        [
            TemplateFinding::UnexpectedProperty {
                set: SET,
                template: TEMPLATE,
                container: EntityId(27),
                property: EntityId(29),
                name: "Hinge".into(),
            },
            TemplateFinding::MissingProperty {
                set: SET,
                template: TEMPLATE,
                container: EntityId(27),
                property_template: EntityId(107),
                name: "Thickness".into(),
            },
        ]
    );
}

#[test]
fn an_object_outside_applicable_entity_is_reported() {
    let relationship = format!("IFCRELDEFINESBYPROPERTIES({},$,$,$,(#1,#2),#20)", guid(40));
    assert_eq!(
        check(&[(40, &relationship)]),
        [TemplateFinding::OutsideApplicableEntity {
            set: SET,
            template: TEMPLATE,
            object: EntityId(2),
            found: "IFCSLAB".into(),
            applicable_entity: "IfcWall".into(),
        }]
    );
}

/// `IfcEntity/PREDEFINEDTYPE`, several entries, and a subtype of a listed
/// entity (the conservative reading).
#[test]
fn applicable_entity_entries_and_predefined_types() {
    let with = |applicable: &str| {
        let template = template_of(".PSET_OCCURRENCEDRIVEN.", applicable, ALL_TEMPLATES);
        check(&[(120, template.as_str())])
    };
    assert_eq!(with("'IfcWall/SOLIDWALL'"), []);
    assert_eq!(with("'IfcSlab, IfcWall'"), []);
    assert_eq!(with("'IfcBuildingElement'"), [], "IfcWall is one");
    let outside = TemplateFinding::OutsideApplicableEntity {
        set: SET,
        template: TEMPLATE,
        object: WALL,
        found: "IFCWALL".into(),
        applicable_entity: "IfcWall/PARTITIONING".into(),
    };
    assert_eq!(with("'IfcWall/PARTITIONING'"), [outside]);
    assert_eq!(
        with("'IfcWalll'"),
        [TemplateFinding::Undecided {
            set: SET,
            template: TEMPLATE,
            subject: WALL,
            reason: UndecidedReason::UnknownApplicableEntity {
                entry: "IfcWalll".into(),
            },
        }]
    );
    // The wall states no predefined type of its own.
    let wall = format!("IFCWALL({},$,'Wall',$,$,$,$,$,$)", guid(1));
    let template = template_of(
        ".PSET_OCCURRENCEDRIVEN.",
        "'IfcWall/SOLIDWALL'",
        ALL_TEMPLATES,
    );
    assert_eq!(
        check(&[(1, &wall), (120, &template)]),
        [TemplateFinding::Undecided {
            set: SET,
            template: TEMPLATE,
            subject: WALL,
            reason: UndecidedReason::PredefinedTypeUnstated {
                entry: "IfcWall/SOLIDWALL".into(),
            },
        }]
    );
}

/// `PSET_TYPEDRIVENONLY` sets belong on types, `PSET_OCCURRENCEDRIVEN`
/// sets on occurrences.
#[test]
fn a_set_on_the_wrong_side_is_reported() {
    let type_only = template_of(
        ".PSET_TYPEDRIVENONLY.",
        "'IfcWall,IfcWallType'",
        ALL_TEMPLATES,
    );
    assert_eq!(
        check(&[(120, &type_only)]),
        [TemplateFinding::WrongAttachment {
            set: SET,
            template: TEMPLATE,
            object: WALL,
            template_type: "PSET_TYPEDRIVENONLY".into(),
            found: "IFCWALL".into(),
        }]
    );
    // The same set carried by the wall type through `HasPropertySets`.
    let wall_type = format!("IFCWALLTYPE({},$,'WT',$,$,(#20),$,$,$,.STANDARD.)", guid(3));
    assert_eq!(check(&[(120, &type_only), (3, &wall_type), (40, "")]), []);
    let occurrence = template_of(
        ".PSET_OCCURRENCEDRIVEN.",
        "'IfcWall,IfcWallType'",
        ALL_TEMPLATES,
    );
    assert_eq!(
        check(&[(120, &occurrence), (3, &wall_type), (40, "")]),
        [TemplateFinding::WrongAttachment {
            set: SET,
            template: TEMPLATE,
            object: EntityId(3),
            template_type: "PSET_OCCURRENCEDRIVEN".into(),
            found: "IFCWALLTYPE".into(),
        }]
    );
}

#[test]
fn a_quantity_template_on_a_property_set_is_the_wrong_set_kind() {
    let template = template_of(".QTO_OCCURRENCEDRIVEN.", "'IfcWall'", ALL_TEMPLATES);
    assert_eq!(
        check(&[(120, &template)]),
        [TemplateFinding::WrongSetKind {
            set: SET,
            template: TEMPLATE,
            template_type: "QTO_OCCURRENCEDRIVEN".into(),
            expected: "IFCELEMENTQUANTITY",
            found: "IFCPROPERTYSET".into(),
        }]
    );
}

/// `QTO_*` set templates with `Q_*` property templates govern element
/// quantities by the same rules.
#[test]
fn quantity_sets_are_checked_like_property_sets() {
    let quantities = format!("IFCELEMENTQUANTITY({},$,'Qto_Custom',$,$,(#21))", guid(20));
    let template = template_of(".QTO_OCCURRENCEDRIVEN.", "'IfcWall'", "#100");
    let length = simple(100, "Length", ".Q_LENGTH.", "$", "$", "$");
    let clean = [
        (20, quantities.as_str()),
        (120, template.as_str()),
        (100, length.as_str()),
        (21, "IFCQUANTITYLENGTH('Length',$,$,3.,$)"),
    ];
    let removed: Vec<(u64, &str)> = [
        22, 23, 24, 25, 26, 27, 28, 101, 102, 103, 104, 105, 106, 107,
    ]
    .into_iter()
    .map(|id| (id, ""))
    .collect();
    let mut changes = clean.to_vec();
    changes.extend(&removed);
    assert_eq!(check(&changes), []);

    changes[3] = (21, "IFCQUANTITYAREA('Length',$,$,3.,$)");
    assert_eq!(
        check(&changes),
        [TemplateFinding::WrongForm {
            set: SET,
            template: TEMPLATE,
            property: EntityId(21),
            property_template: EntityId(100),
            template_type: Some("Q_LENGTH".into()),
            expected: &["IFCQUANTITYLENGTH"],
            found: "IFCQUANTITYAREA".into(),
        }]
    );
}

/// A constant the declared release does not define, or one it does not
/// document, is undecided rather than guessed.
#[test]
fn unknown_and_undocumented_template_types_are_undecided() {
    let count = simple(100, "Width", ".Q_NUMBER.", "$", "$", "$");
    let mut records = base();
    records.insert(100, count);
    let report = template_deviations(&parse("IFC4", &records)).expect("IFC4");
    assert_eq!(
        report.findings,
        [TemplateFinding::Undecided {
            set: SET,
            template: TEMPLATE,
            subject: EntityId(100),
            reason: UndecidedReason::UnknownTemplateType {
                value: "Q_NUMBER".into(),
            },
        }]
    );
    assert!(matches!(
        report.anomalies.as_slice(),
        [PropertyAnomaly::MalformedAttribute {
            attribute: "TemplateType",
            ..
        }]
    ));
    let report = template_deviations(&parse("IFC4X3_ADD2", &records)).expect("IFC4X3");
    assert_eq!(report.anomalies, []);
    assert_eq!(
        report.findings,
        [TemplateFinding::Undecided {
            set: SET,
            template: TEMPLATE,
            subject: EntityId(100),
            reason: UndecidedReason::UndocumentedTemplateType {
                value: "Q_NUMBER".into(),
            },
        }]
    );
}

#[test]
fn ifc2x3_is_refused() {
    let mut records = BTreeMap::new();
    records.insert(1, format!("IFCWALL({},$,'Wall',$,$,$,$,$)", guid(1)));
    assert_eq!(
        template_deviations(&parse("IFC2X3", &records)),
        Err(TemplateError::NoTemplates {
            schema: SchemaVersion::Ifc2x3,
        })
    );
}
