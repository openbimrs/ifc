//! Boundary tests for the domain exports (#123), called exactly as C would
//! call them. Each tape must be the core's record, field for field: the C
//! binding adds calling convention, never content.

use openbim_ifc_binding_core::record::to_records;
use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{IfcModel, Record, ToRecord};

use crate::capability_tests::{last_code, parse, tape};
use crate::*;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../../test/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture")
}

/// The core's answer a tape must equal.
type Expected = fn(&IfcModel) -> Tagged;

fn list(records: Vec<Record>) -> Tagged {
    Tagged::List(records.iter().map(Record::to_tagged).collect())
}

/// A wall classified directly, with a layered type object.
const CLASSIFIED: &[u8] = b"ISO-10303-21;
HEADER;FILE_DESCRIPTION((''),'2;1');FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;
DATA;
#2=IFCWALLTYPE('1YvctVUKr0kugbFTf53O9L',$,'WT',$,$,$,$,$,$,.SOLIDWALL.);
#3=IFCWALL('2YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,$,$,$,.STANDARD.);
#4=IFCRELDEFINESBYTYPE('3YvctVUKr0kugbFTf53O9L',$,$,$,(#3),#2);
#10=IFCCLASSIFICATION('CSI',$,$,'Uniclass 2015',$,$,$);
#12=IFCCLASSIFICATIONREFERENCE($,'Ss_25_10','Wall systems',#10,$,$);
#13=IFCRELASSOCIATESCLASSIFICATION('0ZvctVUKr0kugbFTf53O9L',$,$,$,(#3),#12);
#20=IFCMATERIAL('Concrete',$,$);
#22=IFCMATERIALLAYER(#20,0.2,.U.,'Core',$,$,$);
#24=IFCMATERIALLAYERSET((#22),'WT-200',$);
#26=IFCRELASSOCIATESMATERIAL('2ZvctVUKr0kugbFTf53O9L',$,$,$,(#2),#24);
ENDSEC;
END-ISO-10303-21;
";

#[test]
fn property_sets_cross_as_the_core_records() {
    let bytes = fixture("synthetic-properties/synthetic_properties.ifc");
    let core = IfcModel::parse(&bytes).unwrap();
    let model = parse(&bytes);
    let mut count = usize::MAX;
    let count_out: *mut usize = &mut count;
    // SAFETY (closure): forwards the buffers the helper sized; `count`
    // outlives every call.
    let sets = tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_model_property_sets(model, 30, count_out, n, nc, nr, s, sc, sr)
    })
    .unwrap();
    assert_eq!(count, 3);
    assert_eq!(sets, list(to_records(&core.property_sets(30).unwrap())));

    let measure = "IFCLENGTHMEASURE";
    // SAFETY (closure): as above; `measure` outlives every call.
    let unit = tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_model_resolve_unit(
            model,
            measure.as_ptr(),
            measure.len(),
            7,
            n,
            nc,
            nr,
            s,
            sc,
            sr,
        )
    })
    .unwrap();
    let Tagged::List(fields) = unit else {
        panic!("a ResolvedUnit record");
    };
    assert_eq!(
        (&fields[0], &fields[1]),
        (&Tagged::Ref(7), &Tagged::Bool(false))
    );
    assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
}

#[test]
fn every_other_domain_crosses_as_the_core_records() {
    let cases: [(&str, Expected); 4] = [
        ("synthetic-properties/synthetic_properties.ifc", |m| {
            m.spatial_tree().unwrap().to_record().to_tagged()
        }),
        ("synthetic-systems/synthetic_systems.ifc", |m| {
            m.systems().unwrap().to_record().to_tagged()
        }),
        ("synthetic-cost-schedule/synthetic_cost_schedule.ifc", |m| {
            m.cost().unwrap().to_record().to_tagged()
        }),
        (
            "synthetic-surfaces/synthetic_conic_offset_bounded.ifc",
            |m| list(to_records(&m.georeferencing().unwrap())),
        ),
    ];
    for (index, (name, expected)) in cases.into_iter().enumerate() {
        let bytes = fixture(name);
        let model = parse(&bytes);
        let mut count = 0;
        let count_out: *mut usize = &mut count;
        // SAFETY (closure): forwards the buffers the helper sized; `count`
        // outlives every call.
        let got = tape(|n, nc, nr, s, sc, sr| unsafe {
            match index {
                0 => openbim_ifc_v0_1_model_spatial_tree(model, n, nc, nr, s, sc, sr),
                1 => openbim_ifc_v0_1_model_systems(model, n, nc, nr, s, sc, sr),
                2 => openbim_ifc_v0_1_model_cost(model, n, nc, nr, s, sc, sr),
                _ => openbim_ifc_v0_1_model_georeferencing(model, count_out, n, nc, nr, s, sc, sr),
            }
        })
        .unwrap();
        assert_eq!(got, expected(&IfcModel::parse(&bytes).unwrap()), "{name}");
        assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
    }
}

#[test]
fn classification_and_material_cross_and_no_material_is_null() {
    let core = IfcModel::parse(CLASSIFIED).unwrap();
    let model = parse(CLASSIFIED);
    let mut count = 0;
    let count_out: *mut usize = &mut count;
    // SAFETY (closure): forwards the buffers the helper sized.
    let classes = tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_model_classifications(model, 3, count_out, n, nc, nr, s, sc, sr)
    })
    .unwrap();
    assert_eq!(count, 1);
    assert_eq!(classes, list(to_records(&core.classifications(3).unwrap())));
    // SAFETY (closure): as above.
    let material = tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_model_material(model, 3, n, nc, nr, s, sc, sr)
    })
    .unwrap();
    assert_eq!(
        material,
        core.material(3).unwrap().unwrap().to_record().to_tagged()
    );
    // SAFETY (closure): as above; the classification system has no material.
    let none = tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_model_material(model, 10, n, nc, nr, s, sc, sr)
    })
    .unwrap();
    assert_eq!(none, Tagged::Null);
    assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
}

#[test]
fn domain_refusals_keep_their_shared_status_and_code() {
    let model = parse(CLASSIFIED);
    let mut count = 0;
    let count_out: *mut usize = &mut count;
    // SAFETY (closure): forwards the buffers the helper sized.
    let missing = tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_model_property_sets(model, 999, count_out, n, nc, nr, s, sc, sr)
    });
    assert_eq!(missing, Err(OpenbimIfcStatus::MissingEntity));
    assert_eq!(last_code(model), "missing-entity");
    assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);

    let text = String::from_utf8(CLASSIFIED.to_vec()).unwrap();
    let ifc2x3 = parse(text.replace("'IFC4'", "'IFC2X3'").as_bytes());
    // SAFETY (closure): as above.
    let refused = tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_model_georeferencing(ifc2x3, count_out, n, nc, nr, s, sc, sr)
    });
    assert_eq!(refused, Err(OpenbimIfcStatus::UnsupportedSchema));
    assert_eq!(last_code(ifc2x3), "unsupported-schema");
    assert_eq!(openbim_ifc_v0_1_model_destroy(ifc2x3), OpenbimIfcStatus::Ok);

    // A null count pointer is refused before the model is read.
    // SAFETY: deliberately null count; no buffer is touched.
    let status = unsafe {
        openbim_ifc_v0_1_model_property_sets(
            1,
            3,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            &mut count,
            std::ptr::null_mut(),
            0,
            &mut count,
        )
    };
    assert_eq!(status, OpenbimIfcStatus::NullPointer);
}
