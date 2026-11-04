//! Authoring batches that read their numbers in SI (#638): the guard that
//! keeps the units a batch converts into, the batch-level refusals, and the
//! placement and copy offsets the batch states. Attribute values are in
//! `authoring_si_values.rs`.
//!
//! One IFC4 wall (#22) stands at (1000, 0, 0) of the project's length unit
//! (#32); the project units are millimetres or feet, by fixture.

#![cfg(all(
    feature = "step",
    feature = "authoring",
    feature = "properties",
    feature = "ifc4"
))]

use ifc::properties::{ExactUnitError, UnitConversionError};
use ifc::{
    apply_authoring, apply_authoring_with, authoring_handle, stage_authoring_with, AuthorOp,
    AuthoringError, AuthoringFailure, AuthoringOptions, Codec, CopyPlacement, CopyProductOptions,
    EntityId, InputUnits, Model, SiInputRefusal, StepCodec, Value,
};

const COMMON: &str = "#1=IFCPERSON($,'T',$,$,$,$,$,$);
#2=IFCORGANIZATION($,'T',$,$,$);
#3=IFCPERSONANDORGANIZATION(#1,#2,$);
#4=IFCAPPLICATION(#2,'1','t','t');
#5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,0);
#6=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#7=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
{UNITS}
#21=IFCPROJECT('0WtIqeFgfBFQdcY1sx7FNt',#5,'P',$,$,$,$,$,#20);
#22=IFCWALL('31t64Uzj98DhdKplMxBpF5',#5,'Wall',$,$,#32,$,$,$);
#23=IFCQUANTITYLENGTH('Width',$,#6,200.,$);
#30=IFCCARTESIANPOINT((1000.,0.,0.));
#31=IFCAXIS2PLACEMENT3D(#30,$,$);
#32=IFCLOCALPLACEMENT($,#31);{EXTRA}";

/// Millimetre length, square metre, radian.
const MM: &str = "#8=IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.);
#10=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#20=IFCUNITASSIGNMENT((#6,#8,#10));";

/// Foot, square foot and degree, over metre, square metre and radian.
const FOOT: &str = "#8=IFCDIMENSIONALEXPONENTS(1,0,0,0,0,0,0);
#9=IFCMEASUREWITHUNIT(IFCLENGTHMEASURE(0.3048),#7);
#10=IFCCONVERSIONBASEDUNIT(#8,.LENGTHUNIT.,'foot',#9);
#11=IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.);
#12=IFCDIMENSIONALEXPONENTS(2,0,0,0,0,0,0);
#13=IFCMEASUREWITHUNIT(IFCAREAMEASURE(0.09290304),#11);
#14=IFCCONVERSIONBASEDUNIT(#12,.AREAUNIT.,'square foot',#13);
#15=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#16=IFCDIMENSIONALEXPONENTS(0,0,0,0,0,0,0);
#17=IFCMEASUREWITHUNIT(IFCPLANEANGLEMEASURE(0.017453292519943295),#15);
#18=IFCCONVERSIONBASEDUNIT(#16,.PLANEANGLEUNIT.,'degree',#17);
#20=IFCUNITASSIGNMENT((#10,#14,#18));";

/// A project that states no length unit.
const NO_LENGTH: &str = "#10=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#20=IFCUNITASSIGNMENT((#10));";

fn parse(schema: &str, units: &str, extra: &str) -> Model {
    let data = COMMON.replace("{UNITS}", units).replace("{EXTRA}", extra);
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\n\
         ENDSEC;\nDATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
    );
    let model = StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

fn model(units: &str) -> Model {
    parse("IFC4", units, "")
}

fn si() -> AuthoringOptions {
    AuthoringOptions::default().with_input_units(InputUnits::Si)
}

fn bytes(model: &Model) -> Vec<u8> {
    StepCodec.write_bytes(model).expect("writes")
}

fn failure(error: &AuthoringError) -> &AuthoringFailure {
    &error.failure
}

fn placement(location: [f64; 3]) -> AuthorOp {
    AuthorOp::Placement {
        relative_to: None,
        location,
        axes: None,
    }
}

/// The coordinates of the point an `IfcLocalPlacement` stands at.
fn location_of(model: &Model, placement: EntityId) -> Vec<f64> {
    let follow = |id: EntityId, slot: usize| match &model.get(id).expect("exists").attributes[slot]
    {
        Value::Ref(next) => *next,
        other => panic!("#{id:?} slot {slot}: {other:?}"),
    };
    let axis = follow(placement, 1);
    let point = follow(axis, 0);
    match &model.get(point).expect("point").attributes[0] {
        Value::List(items) => items
            .iter()
            .map(|item| item.unwrap_typed().as_f64().expect("number"))
            .collect(),
        other => panic!("{other:?}"),
    }
}

/// `ops` refused under SI, leaving the model exactly as it was.
fn refused(model: &Model, ops: &[AuthorOp]) -> AuthoringError {
    let mut copy = model.clone();
    let (revision, len, written) = (copy.revision(), copy.len(), bytes(&copy));
    let error = apply_authoring_with(&mut copy, ifc::schema::ifc4(), ops, 1, &si())
        .expect_err("refused in SI");
    assert_eq!(
        (copy.revision(), copy.len(), bytes(&copy)),
        (revision, len, written)
    );
    error
}

/// `ops` accepted when numbers are read raw.
fn raw_passes(model: &Model, ops: &[AuthorOp]) {
    let mut copy = model.clone();
    apply_authoring(&mut copy, ifc::schema::ifc4(), ops, 1).expect("passes raw");
}

fn si_input(error: &AuthoringError) -> &SiInputRefusal {
    match failure(error) {
        AuthoringFailure::SiInput { reason, .. } => reason,
        other => panic!("{other:?}"),
    }
}

#[test]
fn raw_is_the_default_and_equals_apply_authoring() {
    assert_eq!(AuthoringOptions::default().input_units, InputUnits::Raw);
    let ops = [placement([1.0, 2.0, 0.5])];
    let mut plain = model(MM);
    let mut raw = model(MM);
    apply_authoring(&mut plain, ifc::schema::ifc4(), &ops, 1).unwrap();
    apply_authoring_with(
        &mut raw,
        ifc::schema::ifc4(),
        &ops,
        1,
        &AuthoringOptions::default(),
    )
    .unwrap();
    assert_eq!(bytes(&plain), bytes(&raw));
}

#[test]
fn a_placement_location_is_converted_and_axes_are_not() {
    let mut mm = model(MM);
    let ops = [AuthorOp::Placement {
        relative_to: None,
        location: [1.0, 2.0, 0.5],
        axes: Some(([0.0, 0.0, 1.0], [1.0, 0.0, 0.0])),
    }];
    let outcome = apply_authoring_with(&mut mm, ifc::schema::ifc4(), &ops, 1, &si()).unwrap();
    let placed = outcome.ids[0].unwrap();
    assert_eq!(location_of(&mm, placed), [1000.0, 2000.0, 500.0]);
    let axis = match &mm.get(placed).unwrap().attributes[1] {
        Value::Ref(axis) => *axis,
        other => panic!("{other:?}"),
    };
    let direction = |slot: usize| match &mm.get(axis).unwrap().attributes[slot] {
        Value::Ref(id) => mm.get(*id).unwrap().attributes[0].clone(),
        other => panic!("{other:?}"),
    };
    let ratios = |value: Value| match value {
        Value::List(items) => items
            .iter()
            .map(|item| item.as_f64().unwrap())
            .collect::<Vec<_>>(),
        other => panic!("{other:?}"),
    };
    assert_eq!(ratios(direction(1)), [0.0, 0.0, 1.0]);
    assert_eq!(ratios(direction(2)), [1.0, 0.0, 0.0]);

    let mut foot = model(FOOT);
    let outcome =
        apply_authoring_with(&mut foot, ifc::schema::ifc4(), &placement_ops(), 1, &si()).unwrap();
    let located = location_of(&foot, outcome.ids[0].unwrap());
    assert!(
        (located[0] - 3.280839895013123).abs() < 1e-12,
        "{located:?}"
    );
}

fn placement_ops() -> [AuthorOp; 1] {
    [placement([1.0, 0.0, 0.0])]
}

#[test]
fn a_copy_offset_is_converted_into_the_project_unit() {
    let ops = [AuthorOp::CopyProduct {
        product: EntityId(22),
        options: CopyProductOptions::new().placement(CopyPlacement::Offset([1.0, 0.0, 0.0])),
    }];
    let mut mm = model(MM);
    let outcome = apply_authoring_with(&mut mm, ifc::schema::ifc4(), &ops, 1, &si()).unwrap();
    let copy = outcome.ids[0].unwrap();
    let object_placement = match &mm.get(copy).unwrap().attributes[5] {
        Value::Ref(id) => *id,
        other => panic!("{other:?}"),
    };
    // 1000 mm from the original plus 1 m.
    assert_eq!(location_of(&mm, object_placement), [2000.0, 0.0, 0.0]);

    let mut raw = model(MM);
    let outcome = apply_authoring(&mut raw, ifc::schema::ifc4(), &ops, 1).unwrap();
    let object_placement = match &raw.get(outcome.ids[0].unwrap()).unwrap().attributes[5] {
        Value::Ref(id) => *id,
        other => panic!("{other:?}"),
    };
    assert_eq!(location_of(&raw, object_placement), [1001.0, 0.0, 0.0]);
}

#[test]
fn a_project_without_a_length_unit_refuses_the_conversion() {
    let error = refused(&model(NO_LENGTH), &[placement([1.0, 0.0, 0.0])]);
    assert_eq!(error.op, Some(0));
    match failure(&error) {
        AuthoringFailure::UnitConversion {
            entity_type,
            attribute,
            error,
            ..
        } => {
            assert_eq!(
                (entity_type.as_str(), attribute.as_str()),
                ("IFCCARTESIANPOINT", "Coordinates")
            );
            assert!(matches!(
                **error,
                UnitConversionError::Resolve(ExactUnitError::NoProjectUnit { .. })
            ));
        }
        other => panic!("{other:?}"),
    }
    raw_passes(&model(NO_LENGTH), &[placement([1.0, 0.0, 0.0])]);
}

#[test]
fn a_non_finite_number_is_refused() {
    let error = refused(&model(MM), &[placement([f64::NAN, 0.0, 0.0])]);
    let AuthoringFailure::UnitConversion { error, .. } = failure(&error) else {
        panic!("{error:?}");
    };
    assert!(matches!(**error, UnitConversionError::NonFinite { .. }));
}

#[test]
fn a_batch_that_changes_the_units_is_refused_where_raw_runs_it() {
    let foot = model(FOOT);
    let edit = |entity: u64, name: &str, value: Value| AuthorOp::Edit {
        entity: EntityId(entity),
        attributes: vec![(name.to_owned(), value)],
    };
    let cases = [
        // The project's unit assignment.
        edit(21, "UnitsInContext", Value::Ref(EntityId(20))),
        // A unit.
        edit(7, "Name", Value::Enum("KELVIN".into())),
        // The measure a conversion-based unit is defined by.
        edit(
            9,
            "ValueComponent",
            Value::Typed {
                type_name: "IFCLENGTHMEASURE".into(),
                value: Box::new(Value::Real(0.3)),
            },
        ),
        // A value holder's own unit.
        edit(23, "Unit", Value::Ref(EntityId(7))),
    ];
    for case in cases {
        raw_passes(&foot, std::slice::from_ref(&case));
        let error = refused(&foot, std::slice::from_ref(&case));
        assert!(
            matches!(si_input(&error), SiInputRefusal::UnitsChanged { .. }),
            "{case:?}: {error}"
        );
    }
}

#[test]
fn removing_a_unit_assignment_is_refused() {
    let foot = model(FOOT);
    let remove = AuthorOp::Remove {
        entity: EntityId(20),
    };
    let error = refused(&foot, &[remove]);
    assert!(matches!(
        si_input(&error),
        SiInputRefusal::UnitsChanged {
            entity: Some(EntityId(20))
        }
    ));
}

#[test]
fn a_new_project_with_units_is_refused_but_an_unrelated_edit_is_not() {
    let mm = model(MM);
    let project = AuthorOp::Project {
        attributes: vec![("UnitsInContext".into(), Value::Ref(EntityId(20)))],
        owner_history: None,
    };
    let error = refused(&mm, &[project]);
    assert!(matches!(
        si_input(&error),
        SiInputRefusal::UnitsChanged { entity: None }
    ));
    let rename = AuthorOp::Edit {
        entity: EntityId(22),
        attributes: vec![("Name".into(), Value::Text("W2".into()))],
    };
    let mut copy = mm.clone();
    apply_authoring_with(&mut copy, ifc::schema::ifc4(), &[rename], 1, &si()).unwrap();
}

#[test]
fn a_release_the_header_does_not_declare_is_refused() {
    let wrong = parse("IFC2X3", MM, "");
    let error = refused(&wrong, &[placement([1.0, 0.0, 0.0])]);
    assert_eq!(error.op, None);
    assert!(matches!(
        si_input(&error),
        SiInputRefusal::ReleaseMismatch { .. }
    ));
}

#[test]
fn a_project_library_with_units_of_its_own_is_refused() {
    let library = "\n#40=IFCPROJECTLIBRARY('1Wd8Cm0sH3D8DXsvbHLGDh',#5,'L',$,$,$,$,$,#20);";
    let model = parse("IFC4", MM, library);
    let error = refused(&model, &[placement([1.0, 0.0, 0.0])]);
    assert_eq!(error.op, None);
    assert!(matches!(
        si_input(&error),
        SiInputRefusal::ContextUnits {
            context: EntityId(40)
        }
    ));
    raw_passes(&model, &[placement([1.0, 0.0, 0.0])]);
}

#[test]
fn handles_resolve_before_the_conversion() {
    let mut mm = model(MM);
    let ops = [
        placement([1.0, 0.0, 0.0]),
        AuthorOp::Placement {
            relative_to: Some(authoring_handle(0)),
            location: [0.0, 1.0, 0.0],
            axes: None,
        },
    ];
    let staged = stage_authoring_with(&mm, ifc::schema::ifc4(), &ops, 1, &si()).unwrap();
    assert_eq!(staged.ids.len(), 2);
    apply_authoring_with(&mut mm, ifc::schema::ifc4(), &ops, 1, &si()).unwrap();
}

#[test]
fn removing_the_project_or_purging_is_refused_but_renaming_it_is_not() {
    let mm = model(MM);
    let remove = AuthorOp::Remove {
        entity: EntityId(21),
    };
    let error = refused(&mm, &[remove, placement([1.0, 0.0, 0.0])]);
    assert_eq!(error.op, Some(0));
    assert!(matches!(
        si_input(&error),
        SiInputRefusal::UnitsChanged {
            entity: Some(EntityId(21))
        }
    ));
    let purge = AuthorOp::Purge(ifc::PurgeOptions::new());
    raw_passes(&mm, std::slice::from_ref(&purge));
    let error = refused(&mm, &[purge]);
    assert!(matches!(
        si_input(&error),
        SiInputRefusal::UnitsChanged { entity: None }
    ));
    let rename = AuthorOp::Edit {
        entity: EntityId(21),
        attributes: vec![("Name".into(), Value::Text("Q".into()))],
    };
    let mut copy = mm.clone();
    apply_authoring_with(&mut copy, ifc::schema::ifc4(), &[rename], 1, &si()).unwrap();
}

#[test]
fn re_pointing_an_enumeration_is_refused() {
    let extra = "\n#26=IFCPROPERTYENUMERATION('Sizes',(IFCLENGTHMEASURE(100.)),#6);
#27=IFCPROPERTYENUMERATEDVALUE('Size',$,(IFCLENGTHMEASURE(100.)),#26);
#28=IFCPROPERTYENUMERATION('Other',(IFCLENGTHMEASURE(100.)),#7);";
    let model = parse("IFC4", MM, extra);
    let edit = AuthorOp::Edit {
        entity: EntityId(27),
        attributes: vec![("EnumerationReference".into(), Value::Ref(EntityId(28)))],
    };
    raw_passes(&model, std::slice::from_ref(&edit));
    let error = refused(&model, &[edit]);
    assert!(matches!(
        si_input(&error),
        SiInputRefusal::UnitsChanged { .. }
    ));
}

#[test]
fn a_model_with_a_pcurve_off_a_plane_is_refused() {
    let points = "\n#41=IFCCARTESIANPOINT((0.,0.));
#42=IFCCARTESIANPOINT((1.,1.));
#43=IFCPOLYLINE((#41,#42));";
    let cylinder = format!("{points}\n#40=IFCCYLINDRICALSURFACE(#31,1.);\n#44=IFCPCURVE(#40,#43);");
    let model = parse("IFC4", MM, &cylinder);
    let error = refused(&model, &[placement([1.0, 0.0, 0.0])]);
    assert_eq!(error.op, None);
    assert!(matches!(
        si_input(&error),
        SiInputRefusal::ParameterSpaceCurve { .. }
    ));
    raw_passes(&model, &[placement([1.0, 0.0, 0.0])]);

    let plane = format!("{points}\n#40=IFCPLANE(#31);\n#44=IFCPCURVE(#40,#43);");
    let model = parse("IFC4", MM, &plane);
    let mut copy = model.clone();
    apply_authoring_with(
        &mut copy,
        ifc::schema::ifc4(),
        &[placement([1.0, 0.0, 0.0])],
        1,
        &si(),
    )
    .unwrap();
}

#[cfg(feature = "ifc4x1")]
#[test]
fn a_release_without_reviewed_tables_is_refused() {
    let mut model = parse("IFC4X1", MM, "");
    let error = apply_authoring_with(
        &mut model,
        ifc::schema::ifc4x1(),
        &[placement([1.0, 0.0, 0.0])],
        1,
        &si(),
    )
    .expect_err("refused");
    assert_eq!(error.op, None);
    assert!(matches!(
        si_input(&error),
        SiInputRefusal::UnsupportedRelease { .. }
    ));
}
