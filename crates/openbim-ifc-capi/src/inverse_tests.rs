//! Boundary tests for the `INVERSE` exports (#462), called exactly as C
//! would call them, over the IFC2X3, IFC4 and IFC4X3 relation fixtures.
//! The answers are pinned in `openbim-ifc-binding-core`; these check the
//! calling convention.

use std::cell::Cell;
use std::ptr::null_mut;

use openbim_ifc_binding_core::value::Tagged;

use crate::capability_tests::{last_code, parse, tape};
use crate::*;

fn fixture(name: &str) -> OpenbimIfcModel {
    parse(
        &std::fs::read(format!(
            "{}/../../test/fixtures/synthetic-relations/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("fixture"),
    )
}

fn by_name(model: OpenbimIfcModel, id: u64, name: &str) -> Result<Tagged, OpenbimIfcStatus> {
    // SAFETY (closure): `name` outlives the call; forwards the sized buffers.
    tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_entity_attribute_by_name(
            model,
            id,
            name.as_ptr(),
            name.len(),
            n,
            nc,
            nr,
            s,
            sc,
            sr,
        )
    })
}

fn refs(ids: &[u64]) -> Tagged {
    Tagged::List(ids.iter().copied().map(Tagged::Ref).collect())
}

#[test]
fn inverse_names_references_and_totals_cross_the_boundary() {
    for (name, ifc2x3) in [
        ("relations_ifc2x3.ifc", true),
        ("relations_ifc4.ifc", false),
        ("relations_ifc4x3.ifc", false),
    ] {
        let model = fixture(name);
        let defined = if ifc2x3 { refs(&[20, 36]) } else { refs(&[36]) };
        assert_eq!(by_name(model, 17, "IsDefinedBy"), Ok(defined), "{name}");
        assert_eq!(
            by_name(model, 21, "VoidsElements"),
            Ok(Tagged::Ref(24)),
            "{name}"
        );
        assert_eq!(by_name(model, 11, "Decomposes"), Ok(refs(&[])), "{name}");

        let count = Cell::new(0);
        // SAFETY (closure): `count` outlives the call; forwards the buffers.
        let references = tape(|n, nc, nr, s, sc, sr| unsafe {
            openbim_ifc_v0_1_entity_inverse_references(
                model,
                21,
                count.as_ptr(),
                n,
                nc,
                nr,
                s,
                sc,
                sr,
            )
        });
        assert_eq!(count.get(), 2, "{name}");
        assert_eq!(
            references,
            Ok(Tagged::List(vec![
                Tagged::List(vec![Tagged::Ref(24), Tagged::Integer(5)]),
                Tagged::List(vec![Tagged::Ref(30), Tagged::Integer(4)]),
            ])),
            "{name}"
        );

        let names = Cell::new(0);
        // SAFETY (closure): `names` outlives the call; forwards the buffers.
        let declared = tape(|n, nc, nr, s, sc, sr| unsafe {
            openbim_ifc_v0_1_entity_inverse_attribute_names(
                model,
                21,
                names.as_ptr(),
                n,
                nc,
                nr,
                s,
                sc,
                sr,
            )
        })
        .unwrap();
        let Tagged::List(records) = declared else {
            panic!("{declared:?}")
        };
        assert_eq!(records.len(), names.get());
        let voids = records
            .iter()
            .find(|record| {
                matches!(record, Tagged::List(fields) if fields[0] == Tagged::Text("VoidsElements".into()))
            })
            .expect("VoidsElements is declared");
        assert_eq!(
            voids,
            &Tagged::List(vec![
                Tagged::Text("VoidsElements".into()),
                Tagged::Text("IfcRelVoidsElement".into()),
                Tagged::Text("RelatedOpeningElement".into()),
                Tagged::Bool(false),
                Tagged::Integer(1),
                Tagged::Integer(1),
                Tagged::Text("IfcFeatureElementSubtraction".into()),
            ])
        );

        let mut total = 0;
        // SAFETY: valid out-pointer.
        let status = unsafe { openbim_ifc_v0_1_entity_total_inverses(model, 17, &mut total) };
        assert_eq!((status, total), (OpenbimIfcStatus::Ok, 5), "{name}");
        // SAFETY: a null out-pointer is refused before it is used.
        let status = unsafe { openbim_ifc_v0_1_entity_total_inverses(model, 17, null_mut()) };
        assert_eq!(status, OpenbimIfcStatus::NullPointer);
        // SAFETY: valid out-pointer.
        let status = unsafe { openbim_ifc_v0_1_entity_total_inverses(model, 999, &mut total) };
        assert_eq!(status, OpenbimIfcStatus::MissingEntity);
        assert_eq!(last_code(model), "missing-entity");

        let value = Tagged::List(vec![]);
        let tape = crate::tape::Tape::encode(&value);
        let inverse = "HasOpenings";
        // SAFETY: the tape just built; `inverse` outlives the call.
        let status = unsafe {
            openbim_ifc_v0_1_entity_set_attribute_by_name(
                model,
                17,
                inverse.as_ptr(),
                inverse.len(),
                tape.nodes.as_ptr(),
                tape.nodes.len(),
                tape.strings.as_ptr(),
                tape.strings.len(),
            )
        };
        assert_eq!(status, OpenbimIfcStatus::InverseAttribute, "{name}");
        assert_eq!(last_code(model), "inverse-attribute");
        assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
    }
}
