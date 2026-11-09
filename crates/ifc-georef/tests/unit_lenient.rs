//! The CRS map unit in a lenient model (#686): a unit record the reader
//! reported refuses by name, and damage that cannot be attributed refuses as
//! incomplete. Georef refuses the records it reads: the dimensions are not
//! read, so a damaged one resolves, and a reported record is checked before
//! it is fetched, so a record skipped with no kept copy gives
//! `UnitRecordDamaged` where `ifc-geometry` may give `MissingEntity`.

use ifc_georef::{resolve_geographic_offset_in, resolve_project_to_map, GeorefError, GeorefView};
use ifc_model::{Codec, EntityId, Model};
use ifc_step::{ParseOptions, StepCodec};

fn text(data: &str) -> String {
    text_in("IFC4", data)
}

fn text_in(schema: &str, data: &str) -> String {
    format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\n\
         ENDSEC;\nDATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
    )
}

fn lenient(data: &str) -> Model {
    StepCodec::lenient()
        .read_bytes(text(data).as_bytes())
        .expect("lenient read")
}

fn checked(data: &str) -> Model {
    StepCodec::with_options(ParseOptions::lenient().check_references(true))
        .read_bytes(text(data).as_bytes())
        .expect("lenient read")
}

fn checked_in(schema: &str, data: &str) -> Model {
    StepCodec::with_options(ParseOptions::lenient().check_references(true))
        .read_bytes(text_in(schema, data).as_bytes())
        .expect("lenient read")
}

fn id(n: u64) -> EntityId {
    EntityId(n)
}

/// A map conversion (#4) whose CRS (#2) names a foot unit (#3) that reads
/// a measure (#5), a metre (#6) and, unread, its dimensions (#7).
fn crs(extra: &str) -> String {
    format!(
        "#1=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-5,$,$);\n\
         #2=IFCPROJECTEDCRS('EPSG:25832',$,$,$,$,$,#3);\n\
         #3=IFCCONVERSIONBASEDUNIT(#7,.LENGTHUNIT.,'FOOT',#5);\n\
         #4=IFCMAPCONVERSION(#1,#2,0.,0.,0.,1.,0.,1.);\n\
         #5=IFCMEASUREWITHUNIT(IFCLENGTHMEASURE(0.3048),#6);\n\
         #6=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n\
         #7=IFCDIMENSIONALEXPONENTS(1,0,0,0,0,0,0);\n{extra}"
    )
}

fn resolves(model: &Model) {
    let result = resolve_project_to_map(model, id(4), 1.0);
    assert!(result.is_ok(), "{result:?}");
}

fn refused(model: &Model) -> GeorefError {
    resolve_project_to_map(model, id(4), 1.0).expect_err("refused")
}

#[test]
fn an_undamaged_model_resolves() {
    resolves(&lenient(&crs("")));
}

#[test]
fn a_duplicate_of_each_unit_record_read_is_refused_by_name() {
    for (record, duplicate) in [
        (3, "#3=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n"),
        (5, "#5=IFCMEASUREWITHUNIT(IFCLENGTHMEASURE(1.0),#6);\n"),
        (6, "#6=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n"),
    ] {
        let error = refused(&checked(&crs(duplicate)));
        assert!(
            matches!(
                error,
                GeorefError::UnitRecordDamaged { entity, diagnostic: 0 } if entity == id(record)
            ),
            "#{record}: {error:?}"
        );
    }
}

#[test]
fn a_skipped_duplicate_of_the_map_unit_is_refused_by_name() {
    let model = lenient(&crs("#3=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.;\n"));
    assert!(matches!(
        refused(&model),
        GeorefError::UnitRecordDamaged { entity, diagnostic: 0 } if entity == id(3)
    ));
}

#[test]
fn a_skip_without_an_id_is_incomplete() {
    let model = lenient(&crs("@;\n"));
    assert!(matches!(
        refused(&model),
        GeorefError::RecordsIncomplete { diagnostics: 1 }
    ));
}

#[test]
fn a_skip_naming_a_held_id_is_incomplete() {
    // The skip names #20, which a kept record holds; the conflicting #3
    // after it is kept (openbim-step 0.11.4) and reported as a duplicate.
    let model = lenient(&crs(
        "#20=IFCSITE($);\n#20=IFCSITE($)\n#3=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n",
    ));
    assert_eq!(model.diagnostics().len(), 2, "{:?}", model.diagnostics());
    assert!(matches!(
        refused(&model),
        GeorefError::UnitRecordDamaged { entity, diagnostic: 1 } if entity == id(3)
    ));
}

#[test]
fn unrelated_damage_resolves() {
    resolves(&lenient(&crs("#18=IFCSITE($;\n")));
    resolves(&checked(&crs(
        "#20=IFCSIUNIT(*,.MASSUNIT.,$,.GRAM.);\n#20=IFCSIUNIT(*,.MASSUNIT.,$,.GRAM.);\n",
    )));
    // The dimensions are not read by the resolver.
    resolves(&checked(&crs(
        "#7=IFCDIMENSIONALEXPONENTS(0,0,0,0,0,0,0);\n",
    )));
}

#[test]
fn a_skipped_unit_with_no_kept_copy_is_refused_by_name() {
    let data = crs("").replace(
        "#6=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);",
        "#6=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.;",
    );
    let model = lenient(&data);
    assert_eq!(model.diagnostics().len(), 1, "{:?}", model.diagnostics());
    assert!(matches!(
        refused(&model),
        GeorefError::UnitRecordDamaged { entity, diagnostic: 0 } if entity == id(6)
    ));
}

/// IFC4X3: `#1` metre, `#3` degree, `#70` a geographic CRS reading both.
const GEOGRAPHIC: &str = "\
#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#2=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#3=IFCCONVERSIONBASEDUNIT(#9,.PLANEANGLEUNIT.,'DEGREE',#10);
#4=IFCCARTESIANPOINT((0.,0.,0.));
#5=IFCAXIS2PLACEMENT3D(#4,$,$);
#7=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-6,#5,$);
#9=IFCDIMENSIONALEXPONENTS(0,0,0,0,0,0,0);
#10=IFCMEASUREWITHUNIT(IFCPLANEANGLEMEASURE(0.0174532925199433),#2);
#70=IFCGEOGRAPHICCRS('EPSG:4979','WGS 84','WGS84','Greenwich',#3,#1);
#80=IFCRIGIDOPERATION(#7,#70,IFCPLANEANGLEMEASURE(50.98),IFCPOSITIVEPLANEANGLEMEASURE(11.32),210.);
";

fn geographic_offset(model: &Model) -> Result<(), GeorefError> {
    let view = GeorefView::for_model(model).expect("IFC4X3");
    resolve_geographic_offset_in(&view, id(80)).map(|_| ())
}

#[test]
fn a_geographic_crs_resolves_without_damage() {
    let model = checked_in("IFC4X3_ADD2", GEOGRAPHIC);
    assert_eq!(geographic_offset(&model), Ok(()));
}

#[test]
fn a_duplicate_angle_unit_of_a_geographic_crs_is_refused_by_name() {
    let data = format!("{GEOGRAPHIC}#10=IFCMEASUREWITHUNIT(IFCPLANEANGLEMEASURE(1.0),#2);\n");
    let model = checked_in("IFC4X3_ADD2", &data);
    assert!(matches!(
        geographic_offset(&model),
        Err(GeorefError::UnitRecordDamaged { entity, diagnostic: 0 }) if entity == id(10)
    ));
}

#[test]
fn a_held_id_skip_before_a_unit_duplicate_damages_a_geographic_crs() {
    let data = format!(
        "{GEOGRAPHIC}#20=IFCSITE($);\n#20=IFCSITE($)\n#3=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n"
    );
    let model = StepCodec::lenient()
        .read_bytes(text_in("IFC4X3_ADD2", &data).as_bytes())
        .expect("lenient read");
    assert!(matches!(
        geographic_offset(&model),
        Err(GeorefError::UnitRecordDamaged { entity, diagnostic: 1 }) if entity == id(3)
    ));
}
