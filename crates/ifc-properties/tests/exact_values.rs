//! Exact enumerated, list, bounded, table and reference values (#150).
//!
//! Each kind resolves on an occurrence and on a type, bound to the release
//! `FILE_SCHEMA` declares. The attribute layouts differ between releases
//! (`references/ifc-spec/*/*.exp`):
//!
//! - IFC2X3 TC1 requires `EnumerationValues`, `ListValues`, both table
//!   columns and `PropertyReference`; IFC4 ADD2 TC1 and IFC4X3 ADD2 make
//!   them optional.
//! - `IfcPropertyBoundedValue.SetPointValue` and
//!   `IfcPropertyTableValue.CurveInterpolation` are IFC4 additions.
//! - `IfcObjectReferenceSelect` admits `IfcTable` from IFC4 on.
//!
//! Models are parsed from STEP text through the strict codec.

use ifc_model::{Codec, EntityId, Model};
use ifc_properties::{
    exact_properties, exact_property, exact_unit, ExactEntityRef, ExactProperty,
    ExactPropertyError, ExactResolution, ExactSource, ExactTypedValue, ExactValue,
};
use ifc_step::StepCodec;

const WALL: EntityId = EntityId(1);
const WALL_TYPE: EntityId = EntityId(2);

/// A wall typed by #2, with occurrence set `Pset_Occurrence` (#20) and type
/// set `Pset_Type` (#40) holding the given `(id, record)` members.
fn model(schema: &str, occurrence: &[(u64, &str)], typed: &[(u64, &str)], extra: &[&str]) -> Model {
    let wall_slots = if schema == "IFC2X3" {
        "$,$,$,$,$"
    } else {
        "$,$,$,$,$,$"
    };
    let refs = |members: &[(u64, &str)]| {
        members
            .iter()
            .map(|(id, _)| format!("#{id}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    let mut records = vec![
        format!("#1=IFCWALL('0000000000000000000001',$,'Wall',{wall_slots});"),
        "#2=IFCWALLTYPE('0000000000000000000002',$,'WT',$,$,(#40),$,$,$,.STANDARD.);".into(),
        "#3=IFCRELDEFINESBYTYPE('0000000000000000000003',$,$,$,(#1),#2);".into(),
        format!(
            "#20=IFCPROPERTYSET('0000000000000000000020',$,'Pset_Occurrence',$,({}));",
            refs(occurrence)
        ),
        "#21=IFCRELDEFINESBYPROPERTIES('0000000000000000000021',$,$,$,(#1),#20);".into(),
        format!(
            "#40=IFCPROPERTYSET('0000000000000000000040',$,'Pset_Type',$,({}));",
            refs(typed)
        ),
        "#90=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);".into(),
    ];
    for (id, record) in occurrence.iter().chain(typed) {
        records.push(format!("#{id}={record};"));
    }
    records.extend(extra.iter().map(|record| (*record).to_owned()));
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

/// A model with one property on the occurrence and a filler on the type.
fn one(schema: &str, record: &str, extra: &[&str]) -> Model {
    let filler = "IFCPROPERTYSINGLEVALUE('Filler',$,IFCLABEL('x'),$)";
    model(schema, &[(10, record)], &[(41, filler)], extra)
}

fn get(model: &Model, name: &str) -> Result<ExactProperty, ExactPropertyError> {
    match exact_property(model, WALL, None, name)? {
        ExactResolution::Present(property) => Ok(property),
        other => panic!("{name}: expected a present property, got {other:?}"),
    }
}

fn typed(value_type: &str, value: ExactValue) -> ExactTypedValue {
    ExactTypedValue {
        value_type: value_type.into(),
        value,
    }
}

fn label(text: &str) -> ExactTypedValue {
    typed("IFCLABEL", ExactValue::Text(text.into()))
}

fn length(value: f64) -> ExactTypedValue {
    typed("IFCLENGTHMEASURE", ExactValue::Real(value))
}

fn inconsistent(entity: u64, rule: &'static str) -> Result<ExactProperty, ExactPropertyError> {
    Err(ExactPropertyError::InconsistentValues {
        entity: EntityId(entity),
        rule,
    })
}

const ENUMERATION: &str =
    "#80=IFCPROPERTYENUMERATION('PEnum_Status',(IFCLABEL('NEW'),IFCLABEL('OLD')),$);";

#[test]
fn an_enumerated_value_resolves_on_occurrence_and_type() {
    for schema in ["IFC2X3", "IFC4", "IFC4X3_ADD2"] {
        let m = model(
            schema,
            &[(
                10,
                "IFCPROPERTYENUMERATEDVALUE('Status',$,(IFCLABEL('NEW')),#80)",
            )],
            &[(
                41,
                "IFCPROPERTYENUMERATEDVALUE('Grade',$,(IFCLABEL('A'),IFCINTEGER(2)),$)",
            )],
            &[ENUMERATION],
        );
        let status = get(&m, "Status").expect(schema);
        assert_eq!(status.source, ExactSource::Occurrence);
        assert_eq!(
            (status.property_id, status.value_type),
            (EntityId(10), None)
        );
        assert_eq!(status.unit_id, None);
        let ExactValue::Enumerated(status) = status.value else {
            panic!("{schema}: {:?}", status.value)
        };
        assert_eq!(status.values, [label("NEW")]);
        let enumeration = status.enumeration.expect("referenced");
        assert_eq!(enumeration.id, EntityId(80));
        assert_eq!(enumeration.name.as_ref(), "PEnum_Status");
        assert_eq!(enumeration.values, [label("NEW"), label("OLD")]);

        let grade = get(&m, "Grade").expect(schema);
        assert_eq!(grade.source, ExactSource::Type(WALL_TYPE));
        let ExactValue::Enumerated(grade) = grade.value else {
            panic!("{schema}: {:?}", grade.value)
        };
        // Without a reference, selected values of several types are legal.
        assert_eq!(
            grade.values,
            [label("A"), typed("IFCINTEGER", ExactValue::Integer(2))]
        );
        assert_eq!(grade.enumeration, None);
    }
}

#[test]
fn an_empty_enumeration_is_a_value_in_ifc4_and_malformed_in_ifc2x3() {
    let record = "IFCPROPERTYENUMERATEDVALUE('Status',$,$,#80)";
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let m = one(schema, record, &[ENUMERATION]);
        let ExactValue::Enumerated(status) = get(&m, "Status").expect(schema).value else {
            panic!("{schema}: not enumerated")
        };
        assert!(status.values.is_empty(), "{schema}: nothing is selected");
        assert_eq!(status.enumeration.map(|e| e.values.len()), Some(2));
    }
    // IFC2X3 declares `EnumerationValues : LIST [1:?]` without OPTIONAL.
    let m = one("IFC2X3", record, &[ENUMERATION]);
    let malformed = Err(ExactPropertyError::MalformedAggregate {
        entity: EntityId(10),
        attribute: "EnumerationValues",
    });
    assert_eq!(get(&m, "Status"), malformed);
    // A present list is never empty in any release.
    for schema in ["IFC2X3", "IFC4", "IFC4X3_ADD2"] {
        let m = one(schema, "IFCPROPERTYENUMERATEDVALUE('Status',$,(),$)", &[]);
        assert_eq!(get(&m, "Status"), malformed, "{schema}");
    }
}

#[test]
fn a_malformed_enumeration_is_refused() {
    // A selected value the reference does not permit: IFC2X3 WR1, IFC4 WR21.
    let record = "IFCPROPERTYENUMERATEDVALUE('Status',$,(IFCLABEL('GONE')),#80)";
    assert_eq!(
        get(&one("IFC2X3", record, &[ENUMERATION]), "Status"),
        inconsistent(10, "WR1")
    );
    assert_eq!(
        get(&one("IFC4", record, &[ENUMERATION]), "Status"),
        inconsistent(10, "WR21")
    );
    // Same text, other type: not the permitted value.
    let record = "IFCPROPERTYENUMERATEDVALUE('Status',$,(IFCTEXT('NEW')),#80)";
    assert_eq!(
        get(&one("IFC4", record, &[ENUMERATION]), "Status"),
        inconsistent(10, "WR21")
    );

    let record = "IFCPROPERTYENUMERATEDVALUE('Status',$,(IFCLABEL('NEW')),#80)";
    let repeated = "#80=IFCPROPERTYENUMERATION('E',(IFCLABEL('NEW'),IFCLABEL('NEW')),$);";
    assert_eq!(
        get(&one("IFC4", record, &[repeated]), "Status"),
        inconsistent(80, "EnumerationValues UNIQUE")
    );
    let mixed = "#80=IFCPROPERTYENUMERATION('E',(IFCLABEL('NEW'),IFCINTEGER(1)),$);";
    assert_eq!(
        get(&one("IFC4", record, &[mixed]), "Status"),
        inconsistent(80, "WR01")
    );
    let empty = "#80=IFCPROPERTYENUMERATION('E',$,$);";
    assert_eq!(
        get(&one("IFC4", record, &[empty]), "Status"),
        Err(ExactPropertyError::MalformedAggregate {
            entity: EntityId(80),
            attribute: "EnumerationValues",
        })
    );
    // The reference must be an `IfcPropertyEnumeration`.
    let wrong = "IFCPROPERTYENUMERATEDVALUE('Status',$,(IFCLABEL('NEW')),#90)";
    assert_eq!(
        get(&one("IFC4", wrong, &[]), "Status"),
        Err(ExactPropertyError::UnsupportedValue {
            property: EntityId(10)
        })
    );
    let dangling = "IFCPROPERTYENUMERATEDVALUE('Status',$,(IFCLABEL('NEW')),#99)";
    assert_eq!(
        get(&one("IFC4", dangling, &[]), "Status"),
        Err(ExactPropertyError::MissingReference {
            from: EntityId(10),
            to: EntityId(99),
        })
    );
    // An untyped member is no `IfcValue`.
    let untyped = "IFCPROPERTYENUMERATEDVALUE('Status',$,('NEW'),$)";
    assert_eq!(
        get(&one("IFC4", untyped, &[]), "Status"),
        Err(ExactPropertyError::UnsupportedValue {
            property: EntityId(10)
        })
    );
    // The enumeration's unit resolves like a single value's.
    let with_unit =
        "#80=IFCPROPERTYENUMERATION('E',(IFCLENGTHMEASURE(1.),IFCLENGTHMEASURE(2.)),#90);";
    let record = "IFCPROPERTYENUMERATEDVALUE('Width',$,(IFCLENGTHMEASURE(2.)),#80)";
    let width = get(&one("IFC4", record, &[with_unit]), "Width").expect("resolves");
    assert_eq!(width.unit_id, Some(EntityId(90)));
}

#[test]
fn a_list_value_resolves_with_its_unit_on_occurrence_and_type() {
    for schema in ["IFC2X3", "IFC4", "IFC4X3_ADD2"] {
        let m = model(
            schema,
            &[(10, "IFCPROPERTYLISTVALUE('Widths',$,(IFCLENGTHMEASURE(900.),IFCLENGTHMEASURE(1200.)),#90)")],
            &[(41, "IFCPROPERTYLISTVALUE('Tags',$,(IFCLABEL('a')),$)")],
            &[],
        );
        let widths = get(&m, "Widths").expect(schema);
        assert_eq!(widths.source, ExactSource::Occurrence);
        assert_eq!(widths.unit_id, Some(EntityId(90)));
        assert_eq!(
            widths.value,
            ExactValue::List(vec![length(900.0), length(1200.0)])
        );
        let tags = get(&m, "Tags").expect(schema);
        assert_eq!(tags.source, ExactSource::Type(WALL_TYPE));
        assert_eq!(tags.value, ExactValue::List(vec![label("a")]));
    }
    // `ListValues` is optional from IFC4 on, required in IFC2X3.
    let record = "IFCPROPERTYLISTVALUE('Widths',$,$,$)";
    assert_eq!(
        get(&one("IFC4", record, &[]), "Widths").unwrap().value,
        ExactValue::List(vec![])
    );
    assert_eq!(
        get(&one("IFC2X3", record, &[]), "Widths"),
        Err(ExactPropertyError::MalformedAggregate {
            entity: EntityId(10),
            attribute: "ListValues",
        })
    );
    // WR31: one type per list, in every release.
    let mixed = "IFCPROPERTYLISTVALUE('Widths',$,(IFCLENGTHMEASURE(1.),IFCREAL(2.)),$)";
    for schema in ["IFC2X3", "IFC4", "IFC4X3_ADD2"] {
        assert_eq!(
            get(&one(schema, mixed, &[]), "Widths"),
            inconsistent(10, "WR31")
        );
    }
    // A unit that is no `IfcUnit`.
    let bad_unit = "IFCPROPERTYLISTVALUE('Widths',$,(IFCLENGTHMEASURE(1.)),#80)";
    assert_eq!(
        get(&one("IFC4", bad_unit, &[ENUMERATION]), "Widths"),
        Err(ExactPropertyError::UnsupportedUnit {
            property: EntityId(10)
        })
    );
}

#[test]
fn a_bounded_value_resolves_with_its_unit_and_ifc4_set_point() {
    let ifc4 = "IFCPROPERTYBOUNDEDVALUE('Width',$,IFCLENGTHMEASURE(1200.),IFCLENGTHMEASURE(800.),#90,IFCLENGTHMEASURE(900.))";
    let ifc2x3 =
        "IFCPROPERTYBOUNDEDVALUE('Width',$,IFCLENGTHMEASURE(1200.),IFCLENGTHMEASURE(800.),#90)";
    for (schema, record, set_point) in [
        ("IFC2X3", ifc2x3, None),
        ("IFC4", ifc4, Some(length(900.0))),
        ("IFC4X3_ADD2", ifc4, Some(length(900.0))),
    ] {
        let type_record = record.replace("'Width'", "'TypeWidth'");
        let m = model(schema, &[(10, record)], &[(41, &type_record)], &[]);
        let width = get(&m, "Width").expect(schema);
        assert_eq!(width.unit_id, Some(EntityId(90)));
        let ExactValue::Bounded(bounds) = &width.value else {
            panic!("{schema}: {:?}", width.value)
        };
        assert_eq!(bounds.lower, Some(length(800.0)));
        assert_eq!(bounds.upper, Some(length(1200.0)));
        assert_eq!(bounds.set_point, set_point);
        // The stated unit scales the bounds exactly.
        let unit = exact_unit(&m, "IFCLENGTHMEASURE", width.unit_id).expect(schema);
        assert_eq!(unit.scale, 0.001);
        assert!(!unit.from_project);
        assert_eq!(
            get(&m, "TypeWidth").expect(schema).source,
            ExactSource::Type(WALL_TYPE)
        );
    }
    // IFC2X3 declares no SetPointValue: a sixth slot is a malformed record.
    assert!(matches!(
        get(&one("IFC2X3", ifc4, &[]), "Width"),
        Err(ExactPropertyError::MalformedEntitySlots {
            entity: EntityId(10),
            expected: 5,
            actual: 6,
            ..
        })
    ));
}

#[test]
fn a_malformed_bounded_value_is_refused() {
    let bounded = |upper: &str, lower: &str, set_point: &str| {
        format!("IFCPROPERTYBOUNDEDVALUE('Width',$,{upper},{lower},$,{set_point})")
    };
    let (metre, real) = ("IFCLENGTHMEASURE(2.)", "IFCREAL(1.)");
    for (record, rule) in [
        (bounded(metre, real, "$"), "SameUnitUpperLower"),
        (bounded(metre, "$", real), "SameUnitUpperSet"),
        (bounded("$", metre, real), "SameUnitLowerSet"),
    ] {
        assert_eq!(
            get(&one("IFC4", &record, &[]), "Width"),
            inconsistent(10, rule)
        );
        assert_eq!(
            get(&one("IFC4X3_ADD2", &record, &[]), "Width"),
            inconsistent(10, rule)
        );
    }
    let ifc2x3 =
        |upper: &str, lower: &str| format!("IFCPROPERTYBOUNDEDVALUE('Width',$,{upper},{lower},$)");
    assert_eq!(
        get(&one("IFC2X3", &ifc2x3(metre, real), &[]), "Width"),
        inconsistent(10, "WR21")
    );
    // IFC2X3 WR22 requires a bound; IFC4 dropped the rule.
    assert_eq!(
        get(&one("IFC2X3", &ifc2x3("$", "$"), &[]), "Width"),
        inconsistent(10, "WR22")
    );
    let none = get(&one("IFC4", &bounded("$", "$", "$"), &[]), "Width").expect("legal in IFC4");
    let ExactValue::Bounded(bounds) = none.value else {
        panic!("{:?}", none.value)
    };
    assert_eq!(
        (bounds.lower, bounds.upper, bounds.set_point),
        (None, None, None)
    );
    // A bare number is no `IfcValue`.
    assert_eq!(
        get(&one("IFC4", &bounded("2.", "$", "$"), &[]), "Width"),
        Err(ExactPropertyError::UnsupportedValue {
            property: EntityId(10)
        })
    );
}

#[test]
fn a_table_value_resolves_rows_units_expression_and_interpolation() {
    let ifc4 = "IFCPROPERTYTABLEVALUE('Curve',$,(IFCREAL(0.),IFCREAL(1.)),(IFCLENGTHMEASURE(10.),IFCLENGTHMEASURE(20.)),'y = 10x + 10',$,#90,.LINEAR.)";
    let ifc2x3 = "IFCPROPERTYTABLEVALUE('Curve',$,(IFCREAL(0.),IFCREAL(1.)),(IFCLENGTHMEASURE(10.),IFCLENGTHMEASURE(20.)),'y = 10x + 10',$,#90)";
    for (schema, record, interpolation) in [
        ("IFC2X3", ifc2x3, None),
        ("IFC4", ifc4, Some("LINEAR")),
        ("IFC4X3_ADD2", ifc4, Some("LINEAR")),
    ] {
        let type_record = record.replace("'Curve'", "'TypeCurve'");
        let m = model(schema, &[(10, record)], &[(41, &type_record)], &[]);
        let curve = get(&m, "Curve").expect(schema);
        assert_eq!((curve.value_type.clone(), curve.unit_id), (None, None));
        let ExactValue::Table(table) = &curve.value else {
            panic!("{schema}: {:?}", curve.value)
        };
        let rows: Vec<_> = table
            .rows
            .iter()
            .map(|r| (r.defining.clone(), r.defined.clone()))
            .collect();
        let real = |v| typed("IFCREAL", ExactValue::Real(v));
        assert_eq!(rows, [(real(0.0), length(10.0)), (real(1.0), length(20.0))]);
        assert_eq!(table.expression.as_deref(), Some("y = 10x + 10"));
        assert_eq!(
            (table.defining_unit, table.defined_unit),
            (None, Some(EntityId(90)))
        );
        assert_eq!(table.interpolation.as_deref(), interpolation);
        assert_eq!(
            get(&m, "TypeCurve").expect(schema).source,
            ExactSource::Type(WALL_TYPE)
        );
    }
    // IFC2X3 declares no CurveInterpolation.
    assert!(matches!(
        get(&one("IFC2X3", ifc4, &[]), "Curve"),
        Err(ExactPropertyError::MalformedEntitySlots {
            expected: 7,
            actual: 8,
            ..
        })
    ));
}

#[test]
fn a_malformed_table_value_is_refused() {
    let table = |defining: &str, defined: &str, interpolation: &str| {
        format!("IFCPROPERTYTABLEVALUE('Curve',$,{defining},{defined},$,$,$,{interpolation})")
    };
    let ifc2x3 = |defining: &str, defined: &str| {
        format!("IFCPROPERTYTABLEVALUE('Curve',$,{defining},{defined},$,$,$)")
    };
    let (one_row, two_rows) = ("(IFCREAL(0.))", "(IFCREAL(0.),IFCREAL(1.))");
    assert_eq!(
        get(&one("IFC4", &table(one_row, two_rows, "$"), &[]), "Curve"),
        inconsistent(10, "WR21")
    );
    assert_eq!(
        get(&one("IFC2X3", &ifc2x3(one_row, two_rows), &[]), "Curve"),
        inconsistent(10, "WR1")
    );
    // Only one column stated breaks IFC4 WR21 too.
    assert_eq!(
        get(&one("IFC4", &table(one_row, "$", "$"), &[]), "Curve"),
        inconsistent(10, "WR21")
    );
    let both_absent =
        get(&one("IFC4", &table("$", "$", "$"), &[]), "Curve").expect("legal in IFC4");
    assert!(matches!(both_absent.value, ExactValue::Table(t) if t.rows.is_empty()));
    assert_eq!(
        get(&one("IFC2X3", &ifc2x3("$", "$"), &[]), "Curve"),
        Err(ExactPropertyError::MalformedAggregate {
            entity: EntityId(10),
            attribute: "DefiningValues",
        })
    );
    let mixed = "(IFCREAL(0.),IFCINTEGER(1))";
    assert_eq!(
        get(&one("IFC4", &table(mixed, two_rows, "$"), &[]), "Curve"),
        inconsistent(10, "WR22")
    );
    assert_eq!(
        get(&one("IFC4", &table(two_rows, mixed, "$"), &[]), "Curve"),
        inconsistent(10, "WR23")
    );
    assert_eq!(
        get(&one("IFC2X3", &ifc2x3(mixed, two_rows), &[]), "Curve"),
        inconsistent(10, "WR2")
    );
    assert_eq!(
        get(&one("IFC2X3", &ifc2x3(two_rows, mixed), &[]), "Curve"),
        inconsistent(10, "WR3")
    );
    let repeated = "(IFCREAL(0.),IFCREAL(0.))";
    assert_eq!(
        get(&one("IFC4", &table(repeated, two_rows, "$"), &[]), "Curve"),
        inconsistent(10, "DefiningValues UNIQUE")
    );
    // Only `IfcCurveInterpolationEnum` constants of the release.
    assert_eq!(
        get(
            &one("IFC4", &table(one_row, one_row, ".CUBIC."), &[]),
            "Curve"
        ),
        Err(ExactPropertyError::UnsupportedValue {
            property: EntityId(10)
        })
    );
}

#[test]
fn a_reference_value_resolves_its_target_on_occurrence_and_type() {
    let person = "#81=IFCPERSON($,'Doe','Jane',$,$,$,$,$);";
    for schema in ["IFC2X3", "IFC4", "IFC4X3_ADD2"] {
        let m = model(
            schema,
            &[(10, "IFCPROPERTYREFERENCEVALUE('Owner',$,'Responsible',#81)")],
            &[(41, "IFCPROPERTYREFERENCEVALUE('TypeOwner',$,$,#81)")],
            &[person],
        );
        let owner = get(&m, "Owner").expect(schema);
        let ExactValue::Reference(reference) = &owner.value else {
            panic!("{schema}: {:?}", owner.value)
        };
        assert_eq!(reference.usage_name.as_deref(), Some("Responsible"));
        let target = ExactEntityRef {
            id: EntityId(81),
            type_name: "IFCPERSON".into(),
        };
        assert_eq!(reference.target, Some(target));
        let type_owner = get(&m, "TypeOwner").expect(schema);
        assert_eq!(type_owner.source, ExactSource::Type(WALL_TYPE));
    }
    // `PropertyReference` is optional from IFC4 on, required in IFC2X3.
    let record = "IFCPROPERTYREFERENCEVALUE('Owner',$,$,$)";
    let absent = get(&one("IFC4", record, &[]), "Owner").expect("legal in IFC4");
    assert!(matches!(absent.value, ExactValue::Reference(r) if r.target.is_none()));
    assert_eq!(
        get(&one("IFC2X3", record, &[]), "Owner"),
        Err(ExactPropertyError::MissingValueSlot {
            property: EntityId(10)
        })
    );
}

#[test]
fn a_reference_target_is_checked_against_the_release_select() {
    // `IfcTable` joined `IfcObjectReferenceSelect` in IFC4.
    let record = "IFCPROPERTYREFERENCEVALUE('Doc',$,$,#82)";
    assert!(get(&one("IFC4", record, &["#82=IFCTABLE('T',$,$);"]), "Doc").is_ok());
    assert_eq!(
        get(&one("IFC2X3", record, &["#82=IFCTABLE('T',$);"]), "Doc"),
        Err(ExactPropertyError::UnsupportedValue {
            property: EntityId(10)
        })
    );
    // A wall is no reference target anywhere; a dangling or mis-sized one
    // is refused as such.
    let wall = "IFCPROPERTYREFERENCEVALUE('Doc',$,$,#1)";
    assert_eq!(
        get(&one("IFC4", wall, &[]), "Doc"),
        Err(ExactPropertyError::UnsupportedValue {
            property: EntityId(10)
        })
    );
    let dangling = "IFCPROPERTYREFERENCEVALUE('Doc',$,$,#99)";
    assert_eq!(
        get(&one("IFC4", dangling, &[]), "Doc"),
        Err(ExactPropertyError::MissingReference {
            from: EntityId(10),
            to: EntityId(99),
        })
    );
    assert!(matches!(
        get(
            &one(
                "IFC4",
                "IFCPROPERTYREFERENCEVALUE('Doc',$,$,#82)",
                &["#82=IFCTABLE('T',$);"]
            ),
            "Doc"
        ),
        Err(ExactPropertyError::MalformedEntitySlots {
            entity: EntityId(82),
            ..
        })
    ));
}

#[test]
fn a_value_type_foreign_to_the_release_is_refused() {
    // `IfcPositiveInteger` is an IFC4 type; IFC2X3 does not declare it.
    assert!(ifc_schema::ifc2x3()
        .type_def("IFCPOSITIVEINTEGER")
        .is_none());
    let record = "IFCPROPERTYLISTVALUE('Counts',$,(IFCPOSITIVEINTEGER(3)),$)";
    assert!(get(&one("IFC4", record, &[]), "Counts").is_ok());
    assert!(matches!(
        get(&one("IFC2X3", record, &[]), "Counts"),
        Err(ExactPropertyError::NotInSchema {
            entity: EntityId(10),
            ..
        })
    ));
}

#[test]
fn enumeration_lists_every_composite_kind() {
    let m = model(
        "IFC4",
        &[
            (
                10,
                "IFCPROPERTYENUMERATEDVALUE('Status',$,(IFCLABEL('NEW')),#80)",
            ),
            (
                11,
                "IFCPROPERTYLISTVALUE('Widths',$,(IFCLENGTHMEASURE(1.)),$)",
            ),
            (
                12,
                "IFCPROPERTYBOUNDEDVALUE('Range',$,IFCREAL(2.),IFCREAL(1.),$,$)",
            ),
            (
                13,
                "IFCPROPERTYTABLEVALUE('Curve',$,(IFCREAL(0.)),(IFCREAL(1.)),$,$,$,$)",
            ),
            (14, "IFCPROPERTYREFERENCEVALUE('Link',$,$,$)"),
        ],
        &[(41, "IFCPROPERTYLISTVALUE('Tags',$,(IFCLABEL('a')),$)")],
        &[ENUMERATION],
    );
    let entries = exact_properties(&m, WALL).expect("every kind resolves");
    let names: Vec<_> = entries.iter().map(|entry| entry.name.as_ref()).collect();
    assert_eq!(
        names,
        ["Status", "Widths", "Range", "Curve", "Link", "Tags"]
    );
    for entry in &entries {
        assert_eq!(
            Ok(ExactResolution::Present(entry.property.clone())),
            exact_property(&m, WALL, Some(&entry.property.property_set), &entry.name)
        );
    }
}
