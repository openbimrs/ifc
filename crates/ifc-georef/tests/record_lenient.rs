//! The CRS, operation and property-set records in a lenient model (#690),
//! as #686 did for units: a record the reader reported refuses by name, and
//! damage that cannot be attributed refuses as incomplete. Unrelated damage
//! still resolves.

use ifc_georef::{
    resolve_geographic_offset_in, resolve_project_to_map, resolve_property_set_georeferencing,
    GeorefError, GeorefView,
};
use ifc_model::{Codec, EntityId, Model};
use ifc_step::{ParseOptions, StepCodec};

fn text_in(schema: &str, data: &str) -> String {
    format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\n\
         ENDSEC;\nDATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
    )
}

fn checked_in(schema: &str, data: &str) -> Model {
    StepCodec::with_options(ParseOptions::lenient().check_references(true))
        .read_bytes(text_in(schema, data).as_bytes())
        .expect("lenient read")
}

fn lenient_in(schema: &str, data: &str) -> Model {
    StepCodec::lenient()
        .read_bytes(text_in(schema, data).as_bytes())
        .expect("lenient read")
}

fn id(n: u64) -> EntityId {
    EntityId(n)
}

/// A map conversion (#4) from a context (#1) to a CRS (#2) in metres (#3).
fn ifc4(extra: &str) -> String {
    format!(
        "#1=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-5,$,$);\n\
         #2=IFCPROJECTEDCRS('EPSG:25832',$,$,$,$,$,#3);\n\
         #3=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n\
         #4=IFCMAPCONVERSION(#1,#2,0.,0.,0.,1.,0.,1.);\n{extra}"
    )
}

fn map(model: &Model) -> Result<(), GeorefError> {
    resolve_project_to_map(model, id(4), 1.0).map(|_| ())
}

fn damaged(result: Result<(), GeorefError>, record: u64) {
    assert!(
        matches!(
            result,
            Err(GeorefError::RecordDamaged { entity, diagnostic: 0 }) if entity == id(record)
        ),
        "#{record}: {result:?}"
    );
}

#[test]
fn an_undamaged_model_resolves() {
    assert_eq!(map(&checked_in("IFC4", &ifc4(""))), Ok(()));
}

#[test]
fn a_duplicated_projected_crs_is_refused_by_name() {
    let model = checked_in(
        "IFC4",
        &ifc4("#2=IFCPROJECTEDCRS('EPSG:4326',$,$,$,$,$,#3);\n"),
    );
    damaged(map(&model), 2);
}

#[test]
fn a_duplicated_map_conversion_is_refused_by_name() {
    let model = checked_in(
        "IFC4",
        &ifc4("#4=IFCMAPCONVERSION(#1,#2,9.,9.,9.,1.,0.,1.);\n"),
    );
    damaged(map(&model), 4);
}

#[test]
fn a_duplicated_source_context_is_refused_by_name() {
    let model = checked_in(
        "IFC4",
        &ifc4("#1=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Plan',2,1.E-5,$,$);\n"),
    );
    damaged(map(&model), 1);
}

#[test]
fn unrelated_damage_resolves() {
    let model = checked_in(
        "IFC4",
        &ifc4("#20=IFCSITE($);\n#20=IFCSITE($);\n#21=IFCSITE($;\n"),
    );
    assert_eq!(map(&model), Ok(()));
}

#[test]
fn a_held_id_skip_is_incomplete() {
    let model = lenient_in(
        "IFC4",
        &ifc4("#20=IFCSITE($);\n#20=IFCSITE($)\n#30=IFCSIUNIT(*,.MASSUNIT.,$,.GRAM.);\n"),
    );
    assert_eq!(model.diagnostics().len(), 1, "{:?}", model.diagnostics());
    assert!(matches!(
        map(&model),
        Err(GeorefError::RecordsIncomplete { diagnostics: 1 })
    ));
}

/// IFC4X3: a rigid operation (#80) from a context (#7) to a geographic CRS
/// (#70) in degrees (#3) and metres (#1).
const GEOGRAPHIC: &str = "\
#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#2=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#3=IFCCONVERSIONBASEDUNIT(#9,.PLANEANGLEUNIT.,'DEGREE',#10);
#7=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-6,$,$);
#9=IFCDIMENSIONALEXPONENTS(0,0,0,0,0,0,0);
#10=IFCMEASUREWITHUNIT(IFCPLANEANGLEMEASURE(0.0174532925199433),#2);
#70=IFCGEOGRAPHICCRS('EPSG:4979','WGS 84','WGS84','Greenwich',#3,#1);
#80=IFCRIGIDOPERATION(#7,#70,IFCPLANEANGLEMEASURE(50.98),IFCPOSITIVEPLANEANGLEMEASURE(11.32),210.);
";

fn offset(model: &Model) -> Result<(), GeorefError> {
    let view = GeorefView::for_model(model).expect("IFC4X3");
    resolve_geographic_offset_in(&view, id(80)).map(|_| ())
}

#[test]
fn a_duplicated_geographic_crs_or_operation_is_refused_by_name() {
    assert_eq!(offset(&checked_in("IFC4X3_ADD2", GEOGRAPHIC)), Ok(()));
    for (record, duplicate) in [
        (70, "#70=IFCGEOGRAPHICCRS('EPSG:4326',$,$,$,$,$);\n"),
        (
            80,
            "#80=IFCRIGIDOPERATION(#7,#70,IFCPLANEANGLEMEASURE(1.),IFCPOSITIVEPLANEANGLEMEASURE(1.),1.);\n",
        ),
    ] {
        let model = checked_in("IFC4X3_ADD2", &format!("{GEOGRAPHIC}{duplicate}"));
        damaged(offset(&model), record);
    }
}

#[test]
fn a_held_id_skip_makes_a_geographic_offset_incomplete() {
    let data = format!("{GEOGRAPHIC}#20=IFCSITE($);\n#20=IFCSITE($)\n#30=IFCSITE($);\n");
    let model = lenient_in("IFC4X3_ADD2", &data);
    assert!(matches!(
        offset(&model),
        Err(GeorefError::RecordsIncomplete { diagnostics: 1 })
    ));
}

/// The IFC2X3 project pair: `ePSet_MapConversion` (#20) with `Eastings`
/// (#14), `ePSet_ProjectedCRS` (#29) with `Name` (#22).
const PSETS: &str = "\
#6=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#9=IFCUNITASSIGNMENT((#6));
#12=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-5,$,$);
#13=IFCPROJECT('31_msP1GjKXf_TE5tV0OZb',$,'Project',$,$,$,$,(#12),#9);
#14=IFCPROPERTYSINGLEVALUE('Eastings',$,IFCLENGTHMEASURE(1.),$);
#15=IFCPROPERTYSINGLEVALUE('Northings',$,IFCLENGTHMEASURE(2.),$);
#16=IFCPROPERTYSINGLEVALUE('OrthogonalHeight',$,IFCLENGTHMEASURE(3.),$);
#20=IFCPROPERTYSET('24qXbnes5SaQhTiTcqb5LC',$,'ePSet_MapConversion',$,(#14,#15,#16));
#21=IFCRELDEFINESBYPROPERTIES('0i0fBNaZHTMeo$bya9pG$I',$,$,$,(#13),#20);
#22=IFCPROPERTYSINGLEVALUE('Name',$,IFCLABEL('EPSG:7856'),$);
#29=IFCPROPERTYSET('3faM_bSTTRoOz5Hin7_aAf',$,'ePSet_ProjectedCRS',$,(#22));
#30=IFCRELDEFINESBYPROPERTIES('0Z1ZUHtZ5L2f870rSyL3lQ',$,$,$,(#13),#29);
";

fn psets(model: &Model) -> Result<(), GeorefError> {
    resolve_property_set_georeferencing(model, 0.001).map(|_| ())
}

#[test]
fn a_duplicated_pset_record_is_refused_by_name() {
    assert_eq!(psets(&checked_in("IFC2X3", PSETS)), Ok(()));
    for (record, duplicate) in [
        (
            14,
            "#14=IFCPROPERTYSINGLEVALUE('Eastings',$,IFCLENGTHMEASURE(9.),$);\n",
        ),
        (
            22,
            "#22=IFCPROPERTYSINGLEVALUE('Name',$,IFCLABEL('EPSG:1'),$);\n",
        ),
        (
            20,
            "#20=IFCPROPERTYSET('24qXbnes5SaQhTiTcqb5LC',$,'ePSet_MapConversion',$,(#15));\n",
        ),
        (
            29,
            "#29=IFCPROPERTYSET('3faM_bSTTRoOz5Hin7_aAf',$,'ePSet_ProjectedCRS',$,(#15));\n",
        ),
    ] {
        let model = checked_in("IFC2X3", &format!("{PSETS}{duplicate}"));
        damaged(psets(&model), record);
    }
}

#[test]
fn unrelated_pset_damage_resolves() {
    let data = format!("{PSETS}#40=IFCPROPERTYSINGLEVALUE('Other',$,IFCLABEL('a'),$);\n#40=IFCPROPERTYSINGLEVALUE('Other',$,IFCLABEL('b'),$);\n");
    assert_eq!(psets(&checked_in("IFC2X3", &data)), Ok(()));
}

#[test]
fn a_held_id_skip_makes_the_psets_incomplete() {
    let data = format!("{PSETS}#40=IFCSITE($);\n#40=IFCSITE($)\n#41=IFCSITE($);\n");
    let model = lenient_in("IFC2X3", &data);
    assert!(matches!(
        psets(&model),
        Err(GeorefError::RecordsIncomplete { diagnostics: 1 })
    ));
}

/// A site (#50) with its own sets, beside the project's.
const SITE_SETS: &str = "\
#50=IFCSITE('2_msP1GjKXf_TE5tV0OZb',$,'Site',$,$,$,$,$,$,$,$,$,$,$);
#54=IFCPROPERTYSINGLEVALUE('Eastings',$,IFCLENGTHMEASURE(500.),$);
#55=IFCPROPERTYSINGLEVALUE('Northings',$,IFCLENGTHMEASURE(600.),$);
#56=IFCPROPERTYSINGLEVALUE('OrthogonalHeight',$,IFCLENGTHMEASURE(7.),$);
#57=IFCPROPERTYSET('34qXbnes5SaQhTiTcqb5LC',$,'ePSet_MapConversion',$,(#54,#55,#56));
#58=IFCRELDEFINESBYPROPERTIES('1i0fBNaZHTMeo$bya9pG$I',$,$,$,(#50),#57);
#59=IFCPROPERTYSET('4faM_bSTTRoOz5Hin7_aAf',$,'ePSet_ProjectedCRS',$,(#22));
#60=IFCRELDEFINESBYPROPERTIES('1Z1ZUHtZ5L2f870rSyL3lQ',$,$,$,(#50),#59);
";

const OTHER_SET: &str = "\
#40=IFCPROPERTYSINGLEVALUE('Other',$,IFCLABEL('a'),$);
#41=IFCPROPERTYSET('5aaXbnes5SaQhTiTcqb5LC',$,'Pset_Other',$,(#40));
";

#[test]
fn lost_project_sets_do_not_fall_back_to_the_site() {
    // The project's sets are duplicated by unrelated sets, so the kept
    // copies are not georeferencing; the site's would resolve instead.
    let data = format!(
        "{PSETS}{SITE_SETS}{OTHER_SET}\
         #20=IFCPROPERTYSET('5bbXbnes5SaQhTiTcqb5LC',$,'Pset_Other',$,(#40));\n\
         #29=IFCPROPERTYSET('6bbXbnes5SaQhTiTcqb5LC',$,'Pset_Other2',$,(#40));\n"
    );
    damaged(psets(&checked_in("IFC2X3", &data)), 20);
}

#[test]
fn a_duplicated_rel_is_refused_before_a_site_is_chosen() {
    let data = format!(
        "{PSETS}{SITE_SETS}{OTHER_SET}\
         #21=IFCRELDEFINESBYPROPERTIES('7i0fBNaZHTMeo$bya9pG$I',$,$,$,(#13),#41);\n"
    );
    damaged(psets(&checked_in("IFC2X3", &data)), 21);
}

#[test]
fn a_duplicated_host_is_refused() {
    let data =
        format!("{PSETS}#13=IFCPROJECT('41_msP1GjKXf_TE5tV0OZb',$,'Other',$,$,$,$,(#12),#9);\n");
    damaged(psets(&checked_in("IFC2X3", &data)), 13);
}

#[test]
fn a_held_id_skip_with_no_hosts_is_incomplete_when_sets_exist() {
    let data = "#20=IFCPROPERTYSET('24qXbnes5SaQhTiTcqb5LC',$,'ePSet_MapConversion',$,());\n\
                #40=IFCSITE($);\n#40=IFCSITE($)\n#41=IFCSITE($);\n";
    let model = lenient_in("IFC2X3", data);
    assert!(matches!(
        psets(&model),
        Err(GeorefError::RecordsIncomplete { diagnostics: 1 })
    ));
}

/// IFC4X3: a CRS (#2) defined by a well-known text (#5), and another (#6).
const WKT: &str = "\
#1=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-5,$,$);
#2=IFCPROJECTEDCRS('EPSG:25832',$,$,$,$,$,#3);
#3=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#4=IFCMAPCONVERSION(#1,#2,0.,0.,0.,1.,0.,1.);
#5=IFCWELLKNOWNTEXT('PROJCS[\"A\"]',#2);
#6=IFCPROJECTEDCRS('EPSG:1',$,$,$,$,$,#3);
";

#[test]
fn a_duplicated_well_known_text_is_refused_by_name() {
    assert_eq!(map(&checked_in("IFC4X3_ADD2", WKT)), Ok(()));
    let data = format!("{WKT}#5=IFCWELLKNOWNTEXT('PROJCS[\"B\"]',#6);\n");
    damaged(map(&checked_in("IFC4X3_ADD2", &data)), 5);
}

#[test]
fn a_duplicated_operation_is_refused_even_when_the_kept_copy_differs() {
    // The kept copy is a length offset, not the angle one the caller meant.
    let data = format!(
        "{GEOGRAPHIC}#80=IFCRIGIDOPERATION(#7,#70,IFCLENGTHMEASURE(1.),IFCLENGTHMEASURE(1.),1.);\n"
    );
    damaged(offset(&checked_in("IFC4X3_ADD2", &data)), 80);
}

#[test]
fn a_reported_operation_on_the_source_is_refused_before_it_is_counted() {
    // #8 shares the source (#1) with #4 and is duplicated: refused by name
    // rather than as a `HasCoordinateOperation` violation.
    let data = ifc4(
        "#8=IFCMAPCONVERSION(#1,#2,1.,1.,1.,1.,0.,1.);\n\
         #8=IFCMAPCONVERSION(#1,#2,2.,2.,2.,1.,0.,1.);\n",
    );
    damaged(map(&checked_in("IFC4", &data)), 8);
}
