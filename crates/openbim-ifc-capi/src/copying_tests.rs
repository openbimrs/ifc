//! Boundary tests for `copy_product` and `append` (#479), called exactly
//! as C would call them, over the cascade fixtures. The answers are pinned
//! in `openbim-ifc-binding-core`; these check the calling convention, the
//! flags, and that a size query applies nothing.

use std::ptr::{null, null_mut};

use openbim_ifc_binding_core::value::Tagged;

use crate::capability_tests::{last_code, parse, tape};
use crate::*;

const RELEASES: [&str; 3] = [
    "cascade_ifc2x3.ifc",
    "cascade_ifc4.ifc",
    "cascade_ifc4x3.ifc",
];

fn fixture(name: &str) -> OpenbimIfcModel {
    let path = format!(
        "{}/../../test/fixtures/synthetic-relations/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    parse(&std::fs::read(path).expect("fixture"))
}

fn count(model: OpenbimIfcModel) -> usize {
    let mut count = 0;
    // SAFETY: valid out-pointer.
    unsafe { openbim_ifc_v0_1_model_len(model, &mut count) };
    count
}

fn copy_product(
    model: OpenbimIfcModel,
    id: u64,
    placement: u64,
    offset: Option<[f64; 3]>,
    flags: u32,
) -> Result<Tagged, OpenbimIfcStatus> {
    let offset_ptr = offset.as_ref().map_or(null(), |o| o.as_ptr());
    // SAFETY (closure): `offset` outlives the call; forwards the buffers.
    tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_model_copy_product(
            model, id, placement, offset_ptr, flags, n, nc, nr, s, sc, sr,
        )
    })
}

fn append(
    target: OpenbimIfcModel,
    source: OpenbimIfcModel,
    ids: &[u64],
    mode: u32,
    flags: u32,
) -> Result<Tagged, OpenbimIfcStatus> {
    // SAFETY (closure): `ids` outlives the call; forwards the buffers.
    tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_model_append(
            target,
            source,
            ids.as_ptr(),
            ids.len(),
            mode,
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

fn fields(record: &Tagged) -> &[Tagged] {
    let Tagged::List(fields) = record else {
        panic!("a record: {record:?}");
    };
    fields
}

fn list_len(value: &Tagged) -> usize {
    let Tagged::List(items) = value else {
        panic!("a list: {value:?}");
    };
    items.len()
}

#[test]
fn a_copy_crosses_as_a_record_and_a_size_query_applies_nothing() {
    for name in RELEASES {
        let model = fixture(name);
        let before = count(model);
        let (mut nodes, mut strings) = (0, 0);
        // SAFETY: null buffers with capacity 0 are the size query.
        let status = unsafe {
            openbim_ifc_v0_1_model_copy_product(
                model,
                20,
                0,
                null(),
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
        assert_eq!(count(model), before, "{name}: a size query applies nothing");

        let dry = copy_product(model, 20, 0, None, OPENBIM_IFC_COPY_DRY_RUN).unwrap();
        assert_eq!(count(model), before, "{name}");
        assert_eq!(fields(&dry)[6], Tagged::Bool(false));
        let applied = copy_product(model, 20, 0, Some([0.0, 2.0, 0.0]), 0).unwrap();
        let copied = list_len(&fields(&applied)[1]);
        assert_eq!(count(model), before + copied, "{name}");
        assert!(matches!(fields(&applied)[0], Tagged::Ref(_)), "{name}: the copy");
        assert_eq!(fields(&applied)[6], Tagged::Bool(true));

        let bare = copy_product(
            model,
            20,
            0,
            None,
            OPENBIM_IFC_COPY_NO_OPENINGS | OPENBIM_IFC_COPY_NO_CONTAINER,
        )
        .unwrap();
        assert!(list_len(&fields(&bare)[1]) < copied, "{name}");
        assert_eq!(
            copy_product(model, 20, 18, Some([1.0, 0.0, 0.0]), 0),
            Err(OpenbimIfcStatus::InvalidValue)
        );
        assert_eq!(
            copy_product(model, 61, 0, None, 0),
            Err(OpenbimIfcStatus::WrongEntityType)
        );
        assert_eq!(last_code(model), "wrong-entity-type");
        assert_eq!(
            copy_product(model, 20, 0, None, 1 << 5),
            Err(OpenbimIfcStatus::InvalidArgument)
        );
        assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
    }
}

#[test]
fn an_append_crosses_between_two_models() {
    let source = fixture("cascade_ifc4.ifc");
    let target = fixture("cascade_ifc4.ifc");
    let before = count(target);
    assert_eq!(
        append(target, source, &[66], OPENBIM_IFC_GLOBAL_IDS_KEEP, 0),
        Err(OpenbimIfcStatus::InvalidValue),
        "a kept GlobalId the target holds"
    );
    let merged = append(target, source, &[66], OPENBIM_IFC_GLOBAL_IDS_MERGE, 0).unwrap();
    assert_eq!(list_len(&fields(&merged)[1]), 0);
    assert_eq!(fields(&merged)[0], Tagged::Null);
    let dry = append(
        target,
        source,
        &[20],
        OPENBIM_IFC_GLOBAL_IDS_REGENERATE,
        OPENBIM_IFC_APPEND_DRY_RUN,
    )
    .unwrap();
    assert_eq!(count(target), before);
    let applied = append(target, source, &[20], OPENBIM_IFC_GLOBAL_IDS_REGENERATE, 0).unwrap();
    assert_eq!(
        count(target),
        before + list_len(&fields(&applied)[1]),
        "every copy is new"
    );
    assert_eq!(list_len(&fields(&dry)[1]), list_len(&fields(&applied)[1]));
    // Reuse off: the owner history comes along as a copy.
    let copied = append(
        target,
        source,
        &[111],
        OPENBIM_IFC_GLOBAL_IDS_REGENERATE,
        OPENBIM_IFC_APPEND_NO_REUSE_OWNER_HISTORY,
    )
    .unwrap();
    assert!(list_len(&fields(&copied)[1]) > 6);
    assert_eq!(
        append(target, target, &[20], OPENBIM_IFC_GLOBAL_IDS_REGENERATE, 0),
        Err(OpenbimIfcStatus::InvalidArgument),
        "a model into itself is copy_product's"
    );
    assert_eq!(
        append(target, source, &[20], 7, 0),
        Err(OpenbimIfcStatus::InvalidArgument)
    );
    let old = fixture("cascade_ifc2x3.ifc");
    assert_eq!(
        append(old, source, &[111], OPENBIM_IFC_GLOBAL_IDS_REGENERATE, 0),
        Err(OpenbimIfcStatus::UnsupportedSchema)
    );
    assert_eq!(last_code(old), "unsupported-schema");
    for model in [source, target, old] {
        assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
    }
}
