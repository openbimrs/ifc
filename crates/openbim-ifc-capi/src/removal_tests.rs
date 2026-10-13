//! Boundary tests for `remove_product` and `purge` (#478), called exactly
//! as C would call them, over the IFC2X3, IFC4 and IFC4X3 cascade
//! fixtures. The answers are pinned in `openbim-ifc-binding-core`; these
//! check the calling convention, and that a size query applies nothing.

use std::ptr::{null, null_mut};

use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{IfcModel, ToRecord};

use crate::capability_tests::{last_code, parse, tape};
use crate::*;

const RELEASES: [&str; 3] = [
    "cascade_ifc2x3.ifc",
    "cascade_ifc4.ifc",
    "cascade_ifc4x3.ifc",
];

fn path(name: &str) -> String {
    format!(
        "{}/../../test/fixtures/synthetic-relations/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn fixture(name: &str) -> OpenbimIfcModel {
    parse(&std::fs::read(path(name)).expect("fixture"))
}

fn count(model: OpenbimIfcModel) -> usize {
    let mut count = 0;
    // SAFETY: valid out-pointer.
    unsafe { openbim_ifc_v0_1_model_len(model, &mut count) };
    count
}

fn remove_product(model: OpenbimIfcModel, id: u64, flags: u32) -> Result<Tagged, OpenbimIfcStatus> {
    // SAFETY (closure): forwards the buffers the helper sized.
    tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_model_remove_product(model, id, flags, n, nc, nr, s, sc, sr)
    })
}

fn purge(model: OpenbimIfcModel, classes: &[&str], flags: u32) -> Result<Tagged, OpenbimIfcStatus> {
    let pointers: Vec<*const u8> = classes.iter().map(|class| class.as_ptr()).collect();
    let lengths: Vec<usize> = classes.iter().map(|class| class.len()).collect();
    let (table, sizes) = if classes.is_empty() {
        (null(), null())
    } else {
        (pointers.as_ptr(), lengths.as_ptr())
    };
    // SAFETY (closure): `classes` outlives the call; forwards the buffers.
    tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_model_purge(
            model,
            table,
            sizes,
            classes.len(),
            flags,
            n,
            nc,
            nr,
            s,
            sc,
            sr,
        )
    })
}

/// What the core answers on its own model of the same file.
fn core(name: &str, run: impl FnOnce(&mut IfcModel) -> Tagged) -> Tagged {
    let mut model = IfcModel::open(std::path::Path::new(&path(name))).expect("fixture");
    run(&mut model)
}

#[test]
fn a_removal_crosses_as_the_core_record_and_a_size_query_applies_nothing() {
    for name in RELEASES {
        let model = fixture(name);
        let before = count(model);
        // The size query alone: nothing changes.
        let (mut nodes, mut strings) = (0, 0);
        // SAFETY: null buffers with capacity 0 are the size query.
        let status = unsafe {
            openbim_ifc_v0_1_model_remove_product(
                model,
                100,
                0,
                null_mut(),
                0,
                &mut nodes,
                null_mut(),
                0,
                &mut strings,
            )
        };
        assert_eq!(status, OpenbimIfcStatus::BufferTooSmall, "{name}");
        assert!(nodes > 0 && strings > 0);
        assert_eq!(count(model), before, "{name}: a size query applies nothing");

        let dry = remove_product(model, 100, OPENBIM_IFC_REMOVE_DRY_RUN).unwrap();
        assert_eq!(count(model), before, "{name}");
        let applied = remove_product(model, 100, 0).unwrap();
        assert_eq!(count(model), before - 11, "{name}");
        let expected = core(name, |m| {
            let mut options = openbim_ifc_binding_core::RemoveProductOptions::new();
            options.dry_run = true;
            m.remove_product(100, options)
                .unwrap()
                .to_record()
                .to_tagged()
        });
        assert_eq!(dry, expected, "{name}");
        let Tagged::List(fields) = &applied else {
            panic!("a record");
        };
        assert_eq!(fields[4], Tagged::Bool(true), "{name}: applied");
        assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
    }
}

#[test]
fn refusals_and_flags() {
    for name in RELEASES {
        let model = fixture(name);
        assert_eq!(
            remove_product(model, 20, 0),
            Err(OpenbimIfcStatus::StillReferenced),
            "{name}"
        );
        assert_eq!(last_code(model), "still-referenced");
        assert_eq!(
            remove_product(model, 20, 1 << 5),
            Err(OpenbimIfcStatus::InvalidArgument)
        );
        let rebased = remove_product(model, 20, OPENBIM_IFC_REMOVE_REBASE_PLACEMENTS).unwrap();
        let Tagged::List(fields) = &rebased else {
            panic!("a record");
        };
        assert_eq!(
            fields[3],
            Tagged::List(vec![Tagged::Ref(101), Tagged::Ref(131)]),
            "{name}"
        );
        assert!(remove_product(model, 66, OPENBIM_IFC_REMOVE_UNTYPE_OCCURRENCES).is_ok());
        assert_eq!(
            purge(model, &["IfcNoSuchThing"], 0),
            Err(OpenbimIfcStatus::UnsupportedSchema)
        );
        assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
    }
}

#[test]
fn a_purge_by_class_then_by_default() {
    for name in RELEASES {
        let model = fixture(name);
        let before = count(model);
        let dry = purge(model, &[], OPENBIM_IFC_PURGE_DRY_RUN).unwrap();
        assert_eq!(count(model), before);
        let expected = core(name, |m| {
            let mut options = openbim_ifc_binding_core::PurgeOptions::new();
            options.dry_run = true;
            m.purge(options).unwrap().to_record().to_tagged()
        });
        assert_eq!(dry, expected, "{name}");
        purge(model, &["IfcPropertySet"], OPENBIM_IFC_PURGE_INCLUDE_ROOTS).unwrap();
        purge(model, &[], 0).unwrap();
        assert_eq!(count(model), before - 5, "{name}");
        assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
    }
}
