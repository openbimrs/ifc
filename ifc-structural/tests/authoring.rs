mod support;

use ifc_model::{Model, Transaction};
use ifc_schema::{ifc2x3, ifc4};
use ifc_structural::{
    stage_analysis_model, stage_load, AnalysisModelDraft, AnalysisModelType, LoadDraft,
    StructuralError, StructuralView,
};

use support::{model, GUID};

#[test]
fn stages_and_commits_an_ifc4_analysis_model_atomically() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let mut tx = Transaction::new(&model);
    let id = stage_analysis_model(
        &mut tx,
        &model,
        schema,
        AnalysisModelDraft {
            global_id: GUID.into(),
            name: Some("Frame".into()),
            predefined_type: AnalysisModelType::Loading3d,
            ..AnalysisModelDraft::default()
        },
    )
    .unwrap();
    assert_eq!(tx.len(), 1);
    tx.commit(&mut model).unwrap();
    assert_eq!(
        StructuralView::new(&model, schema)
            .analysis_model(id)
            .unwrap()
            .name()
            .unwrap(),
        Some("Frame")
    );
}

#[test]
fn ifc2x3_requires_owner_history_before_staging() {
    let schema = ifc2x3();
    let model = model("IFC2X3");
    let mut tx = Transaction::new(&model);
    let err = stage_analysis_model(
        &mut tx,
        &model,
        schema,
        AnalysisModelDraft {
            global_id: GUID.into(),
            predefined_type: AnalysisModelType::Loading3d,
            ..AnalysisModelDraft::default()
        },
    )
    .unwrap_err();
    assert!(matches!(err, StructuralError::MissingRequired { .. }));
    assert!(tx.is_empty(), "rejected draft must not stage partial edits");
}

#[test]
fn user_defined_authoring_requires_object_type_before_staging() {
    let schema = ifc4();
    let model = model("IFC4");
    let mut tx = Transaction::new(&model);
    let err = stage_analysis_model(
        &mut tx,
        &model,
        schema,
        AnalysisModelDraft {
            global_id: GUID.into(),
            predefined_type: AnalysisModelType::UserDefined,
            ..AnalysisModelDraft::default()
        },
    )
    .unwrap_err();
    assert!(matches!(err, StructuralError::SemanticViolation { .. }));
    assert!(tx.is_empty());
}

#[test]
fn stages_core_static_loads_without_a_codec_dependency() {
    let schema = ifc4();
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);
    let id = stage_load(
        &mut tx,
        schema,
        LoadDraft::SingleForce {
            name: Some("Wind node".into()),
            force: [Some(1.0), Some(2.0), Some(3.0)],
            moment: [None, None, Some(4.0)],
        },
    )
    .unwrap();
    tx.commit(&mut model).unwrap();
    assert_eq!(
        StructuralView::new(&model, schema)
            .load(id)
            .unwrap()
            .components()
            .unwrap(),
        vec![Some(1.0), Some(2.0), Some(3.0), None, None, Some(4.0)]
    );
}

#[test]
fn non_finite_load_drafts_are_rejected_before_staging() {
    let schema = ifc4();
    let model = Model::new();
    let drafts = [
        LoadDraft::SingleForce {
            name: None,
            force: [Some(f64::NAN), None, None],
            moment: [None, None, None],
        },
        LoadDraft::LinearForce {
            name: None,
            force: [Some(f64::INFINITY), None, None],
            moment: [None, None, None],
        },
        LoadDraft::PlanarForce {
            name: None,
            force: [Some(f64::NEG_INFINITY), None, None],
        },
        LoadDraft::Temperature {
            name: None,
            delta: [None, Some(f64::NAN), None],
        },
    ];

    for draft in drafts {
        let mut tx = Transaction::new(&model);
        assert!(matches!(
            stage_load(&mut tx, schema, draft),
            Err(StructuralError::InvalidDraftValue {
                expected: "finite load value or null",
                ..
            })
        ));
        assert!(tx.is_empty(), "rejected load must not stage partial edits");
    }
}

#[test]
fn stages_ifc2x3_temperature_using_legacy_attribute_names() {
    let mut model = model("IFC2X3");
    let schema = ifc2x3();
    let mut tx = Transaction::new(&model);
    let id = stage_load(
        &mut tx,
        schema,
        LoadDraft::Temperature {
            name: Some("Gradient".into()),
            delta: [Some(18.0), Some(2.0), Some(-1.0)],
        },
    )
    .unwrap();
    tx.commit(&mut model).unwrap();

    assert_eq!(
        StructuralView::new(&model, schema)
            .static_load(id)
            .unwrap()
            .components()
            .unwrap(),
        vec![Some(18.0), Some(2.0), Some(-1.0)]
    );
}
