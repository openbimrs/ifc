//! Boundary-condition stiffness values in the form each release declares
//! (#200, #201).
//!
//! IFC2X3 declares every stiffness a plain measure, so the value is bare.
//! IFC4 and IFC4X3 declare a `SELECT (IfcBoolean, <measure>)`, so the value
//! carries the wrapper of the member it is. Each release is authored, written
//! to STEP, read back with `ifc-step`, and read through `StructuralView`; the
//! STEP text is asserted directly, because a typed wrapper in a non-SELECT
//! slot and a bare value in a SELECT both survive a round trip unnoticed.

use ifc_model::{Codec, EntityId, Model, Transaction};
use ifc_schema::{ifc2x3, ifc4, ifc4x3, Schema, SchemaVersion, TypeKind};
use ifc_step::StepCodec;
use ifc_structural::{
    stage_boundary_condition, stage_boundary_condition_in, AxisValues, BoundaryConditionDraft,
    BoundaryConditionKind, StiffnessValue, StructuralError, StructuralView,
};

/// A bundled table, by its accessor.
type Table = fn() -> &'static Schema;

const RELEASES: [(&str, Table); 3] = [("IFC2X3", ifc2x3), ("IFC4", ifc4), ("IFC4X3_ADD2", ifc4x3)];

fn model(token: &str) -> Model {
    let mut model = Model::default();
    model.header_mut().schema = vec![token.to_owned()];
    model
}

fn measures(x: f64, y: f64, z: f64) -> AxisValues<Option<StiffnessValue>> {
    AxisValues {
        x: Some(StiffnessValue::Measure(x)),
        y: Some(StiffnessValue::Measure(y)),
        z: Some(StiffnessValue::Measure(z)),
    }
}

fn draft(kind: BoundaryConditionKind) -> BoundaryConditionDraft<'static> {
    {
        let mut draft = BoundaryConditionDraft::new()
            .name("Support")
            .translational(measures(1.5, 2.5, 3.5))
            .rotational(if kind == BoundaryConditionKind::Face {
                AxisValues::default()
            } else {
                measures(4.5, 5.5, 6.5)
            });
        draft.warping =
            (kind == BoundaryConditionKind::NodeWarping).then_some(StiffnessValue::Measure(7.5));
        draft
    }
}

const KINDS: [BoundaryConditionKind; 4] = [
    BoundaryConditionKind::Node,
    BoundaryConditionKind::NodeWarping,
    BoundaryConditionKind::Edge,
    BoundaryConditionKind::Face,
];

/// The wrapper each family's measures carry in IFC4 and IFC4X3:
/// translational, rotational, warping.
fn wrappers(
    kind: BoundaryConditionKind,
) -> (&'static str, Option<&'static str>, Option<&'static str>) {
    match kind {
        BoundaryConditionKind::Node => (
            "IFCLINEARSTIFFNESSMEASURE",
            Some("IFCROTATIONALSTIFFNESSMEASURE"),
            None,
        ),
        BoundaryConditionKind::NodeWarping => (
            "IFCLINEARSTIFFNESSMEASURE",
            Some("IFCROTATIONALSTIFFNESSMEASURE"),
            Some("IFCWARPINGMOMENTMEASURE"),
        ),
        BoundaryConditionKind::Edge => (
            "IFCMODULUSOFLINEARSUBGRADEREACTIONMEASURE",
            Some("IFCMODULUSOFROTATIONALSUBGRADEREACTIONMEASURE"),
            None,
        ),
        BoundaryConditionKind::Face => ("IFCMODULUSOFSUBGRADEREACTIONMEASURE", None, None),
    }
}

/// The STEP record of `id`, from the written text.
fn record(text: &str, id: EntityId) -> String {
    let prefix = format!("#{}=", id.0);
    text.lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("no record {prefix} in\n{text}"))
        .to_owned()
}

/// Author every family in `token`'s release and round-trip through STEP.
fn round_trip(
    token: &str,
    schema: &Schema,
) -> (Model, String, Vec<(BoundaryConditionKind, EntityId)>) {
    let mut model = model(token);
    let mut tx = Transaction::new(&model);
    let ids: Vec<_> = KINDS
        .iter()
        .map(|&kind| {
            let id = stage_boundary_condition_in(&mut tx, schema, kind, draft(kind))
                .unwrap_or_else(|error| panic!("{token} {kind:?}: {error}"));
            (kind, id)
        })
        .collect();
    tx.commit(&mut model).expect("commit");
    let bytes = StepCodec.write_bytes(&model).expect("written");
    let text = String::from_utf8(bytes.clone()).expect("utf8");
    let back = StepCodec.read_bytes(&bytes).expect("read back");
    (back, text, ids)
}

#[test]
fn every_release_writes_its_declared_form_and_reads_back() {
    for (token, schema) in RELEASES {
        let schema = schema();
        let (back, text, ids) = round_trip(token, schema);
        let view = StructuralView::for_model(&back).expect("view");
        for (kind, id) in ids {
            let line = record(&text, id);
            let (translational, rotational, warping) = wrappers(kind);
            if schema.version() == Some(SchemaVersion::Ifc2x3) {
                assert!(
                    !line.contains("MEASURE("),
                    "{token} {kind:?} is bare: {line}"
                );
                assert!(line.contains(",1.5,2.5,3.5"), "{token} {kind:?}: {line}");
            } else {
                assert!(
                    line.contains(&format!("{translational}(1.5)")),
                    "{token} {kind:?}: {line}"
                );
                if let Some(rotational) = rotational {
                    assert!(
                        line.contains(&format!("{rotational}(6.5)")),
                        "{token} {kind:?}: {line}"
                    );
                }
                if let Some(warping) = warping {
                    assert!(
                        line.contains(&format!("{warping}(7.5)")),
                        "{token} {kind:?}: {line}"
                    );
                }
            }
            let condition = view.boundary_condition(id).expect("readable");
            assert_eq!(condition.kind(), kind);
            let t = condition
                .translational_stiffnesses()
                .expect("translational");
            assert_eq!(t, measures(1.5, 2.5, 3.5), "{token} {kind:?}");
            let r = condition.rotational_stiffnesses().expect("rotational");
            assert_eq!(r, draft(kind).rotational, "{token} {kind:?}");
            assert_eq!(
                condition.warping_stiffness().expect("warping"),
                draft(kind).warping,
                "{token} {kind:?}"
            );
        }
    }
}

/// Every wrapper the IFC4/IFC4X3 test expects is a member of its slot's
/// SELECT in that release's table, and every IFC2X3 slot is a defined
/// type -- so the expectations above are the schema's, not this file's.
#[test]
fn the_expected_forms_are_the_declared_types() {
    for (token, schema) in RELEASES {
        let schema = schema();
        let (_, _, ids) = round_trip(token, schema);
        for (kind, _) in ids {
            let entity = match kind {
                BoundaryConditionKind::Node => "IfcBoundaryNodeCondition",
                BoundaryConditionKind::NodeWarping => "IfcBoundaryNodeConditionWarping",
                BoundaryConditionKind::Edge => "IfcBoundaryEdgeCondition",
                BoundaryConditionKind::Face => "IfcBoundaryFaceCondition",
            };
            let (translational, rotational, warping) = wrappers(kind);
            let attributes = schema.attributes(entity);
            let stiffness = &attributes[1..];
            let members = [Some(translational); 3]
                .into_iter()
                .chain([rotational; 3])
                .chain([warping])
                .zip(stiffness);
            for (member, attribute) in members {
                let member = member.expect("declared slot has a member");
                let declared = attribute.type_name.as_str();
                let select = matches!(
                    schema.type_def(declared).map(|d| &d.kind),
                    Some(TypeKind::Select(_))
                );
                assert_eq!(
                    select,
                    schema.version() != Some(SchemaVersion::Ifc2x3),
                    "{token} {entity}.{}",
                    attribute.name
                );
                assert!(
                    schema.accepts_type(declared, member),
                    "{token} {entity}.{} admits {member}",
                    attribute.name
                );
                assert_eq!(
                    select,
                    schema.accepts_type(declared, "IfcBoolean"),
                    "{token} {entity}.{}: a boolean only where a SELECT admits one",
                    attribute.name
                );
            }
        }
    }
}

#[test]
fn a_boolean_is_typed_where_a_select_admits_it_and_refused_in_ifc2x3() {
    let flags = AxisValues {
        x: Some(StiffnessValue::Boolean(true)),
        y: Some(StiffnessValue::Boolean(false)),
        z: None,
    };
    for (token, schema) in RELEASES {
        let schema = schema();
        for kind in KINDS {
            let mut model = model(token);
            let mut tx = Transaction::new(&model);
            let draft = {
                let mut draft = BoundaryConditionDraft::new()
                    .translational(flags)
                    .rotational(AxisValues::default());
                draft.warping = (kind == BoundaryConditionKind::NodeWarping)
                    .then_some(StiffnessValue::Boolean(true));
                draft
            };
            let staged = stage_boundary_condition_in(&mut tx, schema, kind, draft);
            if schema.version() == Some(SchemaVersion::Ifc2x3) {
                assert!(
                    matches!(staged, Err(StructuralError::InvalidDraftValue { .. })),
                    "{token} {kind:?}: {staged:?}"
                );
                assert!(tx.is_empty(), "{token}: a refusal stages nothing");
                continue;
            }
            let id = staged.expect("boolean stiffness");
            tx.commit(&mut model).expect("commit");
            let bytes = StepCodec.write_bytes(&model).expect("written");
            let text = String::from_utf8(bytes.clone()).expect("utf8");
            let line = record(&text, id);
            assert!(
                line.contains("IFCBOOLEAN(.T.),IFCBOOLEAN(.F.),$"),
                "{token} {kind:?}: {line}"
            );
            let back = StepCodec.read_bytes(&bytes).expect("read back");
            let view = StructuralView::for_model(&back).expect("view");
            let condition = view.boundary_condition(id).expect("readable");
            assert_eq!(condition.translational_stiffnesses().expect("t"), flags);
            if kind == BoundaryConditionKind::NodeWarping {
                assert_eq!(
                    condition.warping_stiffness().expect("warping"),
                    Some(StiffnessValue::Boolean(true))
                );
            }
        }
    }
}

/// The writer without a schema is the IFC4 form, which IFC4X3 shares.
#[test]
fn the_schema_free_writer_is_the_ifc4_and_ifc4x3_form() {
    for kind in KINDS {
        let model = model("IFC4");
        let mut plain = Transaction::new(&model);
        let mut four = Transaction::new(&model);
        let mut four_x3 = Transaction::new(&model);
        let a = stage_boundary_condition(&mut plain, kind, draft(kind)).expect("plain");
        let b = stage_boundary_condition_in(&mut four, ifc4(), kind, draft(kind)).expect("ifc4");
        let c =
            stage_boundary_condition_in(&mut four_x3, ifc4x3(), kind, draft(kind)).expect("ifc4x3");
        let (mut ma, mut mb, mut mc) = (model.clone(), model.clone(), model.clone());
        plain.commit(&mut ma).expect("commit");
        four.commit(&mut mb).expect("commit");
        four_x3.commit(&mut mc).expect("commit");
        assert_eq!(ma.get(a), mb.get(b), "{kind:?}");
        assert_eq!(mb.get(b), mc.get(c), "{kind:?}");
    }
}

/// Readers accept a bare value and a typed one in every release: files
/// written before #200 carry wrappers in IFC2X3, and other tools write bare
/// values in the SELECT slots.
#[test]
fn the_reader_accepts_both_forms_in_every_release() {
    let cases = [
        (
            "IFC2X3",
            "IFCBOUNDARYNODECONDITION('b',1.5,IFCLINEARSTIFFNESSMEASURE(2.5),$,$,$,$)",
        ),
        (
            "IFC4",
            "IFCBOUNDARYNODECONDITION('b',1.5,IFCLINEARSTIFFNESSMEASURE(2.5),$,$,$,$)",
        ),
        (
            "IFC4X3_ADD2",
            "IFCBOUNDARYNODECONDITION('b',1.5,IFCLINEARSTIFFNESSMEASURE(2.5),$,$,$,$)",
        ),
    ];
    for (token, body) in cases {
        let text = format!(
            "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
             FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{token}'));\nENDSEC;\n\
             DATA;\n#1={body};\nENDSEC;\nEND-ISO-10303-21;\n"
        );
        let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
        let view = StructuralView::for_model(&model).expect("view");
        let t = view
            .boundary_condition(EntityId(1))
            .expect("readable")
            .translational_stiffnesses()
            .expect("translational");
        assert_eq!(t.x, Some(StiffnessValue::Measure(1.5)), "{token} bare");
        assert_eq!(t.y, Some(StiffnessValue::Measure(2.5)), "{token} typed");
    }
    for token in ["IFC4", "IFC4X3_ADD2"] {
        let text = format!(
            "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
             FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{token}'));\nENDSEC;\n\
             DATA;\n#1=IFCBOUNDARYNODECONDITION('b',.T.,IFCBOOLEAN(.F.),$,$,$,$);\n\
             ENDSEC;\nEND-ISO-10303-21;\n"
        );
        let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
        let view = StructuralView::for_model(&model).expect("view");
        let t = view
            .boundary_condition(EntityId(1))
            .expect("readable")
            .translational_stiffnesses()
            .expect("translational");
        assert_eq!(t.x, Some(StiffnessValue::Boolean(true)), "{token} bare");
        assert_eq!(t.y, Some(StiffnessValue::Boolean(false)), "{token} typed");
    }
}
