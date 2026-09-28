//! A directrix sweep's trim is a parameter or a length, never both (#210).

#![cfg(feature = "lowering")]
//! Requires the `lowering` feature: the refusal is a lowering outcome.
//!
//! IFC4X3 declares `StartParam`/`EndParam` of the directrix sweeps as
//! `IfcCurveMeasureSelect = SELECT (IfcLengthMeasure, IfcParameterValue)`.
//! The readers report which member the file wrote, and lowering, whose range
//! is a curve-parameter range, refuses a length instead of reading it as a
//! parameter. The directrix is a circle, where the difference is an angle
//! against a distance: a 1.5 m trim read as 1.5 rad is a different solid.

use axiolid_model::{GeometryNode, SolidOperation};
use ifc_geometry::lower::{lower_representation_item, LoweringSession};
use ifc_geometry::solid::swept::{
    FixedReferenceSweptAreaSolid, SurfaceCurveSweptAreaSolid, TrimMeasure,
};
use ifc_geometry::transform::Transform;
use ifc_geometry::{units, GeometryError};
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

const SWEEPS: [&str; 2] = [
    "IFCSURFACECURVESWEPTAREASOLID",
    "IFCFIXEDREFERENCESWEPTAREASOLID",
];

/// An IFC4X3 file holding one sweep of `kind` with trims `start`, `end`.
fn ifc4x3(kind: &str, start: &str, end: &str) -> Model {
    let last = if kind == SWEEPS[0] { "#5" } else { "#6" };
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4X3_ADD2'));\nENDSEC;\n\
         DATA;\n\
         #1=IFCCARTESIANPOINT((0.,0.,0.));\n\
         #2=IFCAXIS2PLACEMENT3D(#1,$,$);\n\
         #3=IFCCIRCLE(#2,2.);\n\
         #4=IFCCIRCLEPROFILEDEF(.AREA.,$,$,0.1);\n\
         #5=IFCPLANE(#2);\n\
         #6=IFCDIRECTION((0.,0.,1.));\n\
         #7={kind}(#4,$,#3,{start},{end},{last});\n\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

const SWEEP: EntityId = EntityId(7);

fn trims(model: &Model, kind: &str) -> (Option<TrimMeasure>, Option<TrimMeasure>) {
    let entity = model.get(SWEEP).expect("sweep");
    if kind == SWEEPS[0] {
        let view = SurfaceCurveSweptAreaSolid::new(SWEEP, entity);
        (
            view.start_param().expect("start"),
            view.end_param().expect("end"),
        )
    } else {
        let view = FixedReferenceSweptAreaSolid::new(SWEEP, entity);
        (
            view.start_param().expect("start"),
            view.end_param().expect("end"),
        )
    }
}

fn lower(model: &Model) -> Result<Option<(f64, f64)>, GeometryError> {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let node = lower_representation_item(&mut session, SWEEP, Transform::identity())?;
    let lowered = session.finish(node)?;
    let range = lowered
        .graph
        .iter()
        .find_map(|(_, node)| match node {
            GeometryNode::SolidOperation(SolidOperation::SurfaceCurveSweep {
                parameter_range,
                ..
            })
            | GeometryNode::SolidOperation(SolidOperation::FixedReferenceSweep {
                parameter_range,
                ..
            }) => Some(*parameter_range),
            _ => None,
        })
        .expect("a directrix sweep reaches the graph");
    Ok(range)
}

#[test]
fn a_parameter_trim_is_read_and_lowered_as_a_parameter() {
    for kind in SWEEPS {
        let model = ifc4x3(kind, "IFCPARAMETERVALUE(0.5)", "IFCPARAMETERVALUE(1.5)");
        assert_eq!(
            trims(&model, kind),
            (
                Some(TrimMeasure::Parameter(0.5)),
                Some(TrimMeasure::Parameter(1.5))
            ),
            "{kind}"
        );
        let range = lower(&model).unwrap_or_else(|e| panic!("{kind} lowers: {e}"));
        let (start, end) = range.expect("trimmed");
        assert!(
            (start - 0.5).abs() < 1e-12 && (end - 1.5).abs() < 1e-12,
            "{kind}"
        );
    }
}

#[test]
fn a_length_trim_is_reported_as_a_length_and_not_lowered_as_a_parameter() {
    for kind in SWEEPS {
        for (start, end) in [
            ("IFCLENGTHMEASURE(0.5)", "IFCLENGTHMEASURE(1.5)"),
            ("IFCPARAMETERVALUE(0.5)", "IFCLENGTHMEASURE(1.5)"),
        ] {
            let model = ifc4x3(kind, start, end);
            let (read_start, read_end) = trims(&model, kind);
            assert_eq!(read_end, Some(TrimMeasure::Length(1.5)), "{kind}");
            assert_eq!(read_end.and_then(TrimMeasure::parameter), None);
            assert!(read_start.is_some());
            let refused = lower(&model);
            assert!(
                matches!(
                    &refused,
                    Err(GeometryError::Unsupported { entity, .. }) if *entity == SWEEP
                ),
                "{kind} {start},{end}: {refused:?}"
            );
        }
    }
}

#[test]
fn a_measure_outside_the_select_is_refused_by_the_reader() {
    for kind in SWEEPS {
        let model = ifc4x3(kind, "IFCNONNEGATIVELENGTHMEASURE(0.5)", "$");
        let entity = model.get(SWEEP).expect("sweep");
        let start = if kind == SWEEPS[0] {
            SurfaceCurveSweptAreaSolid::new(SWEEP, entity).start_param()
        } else {
            FixedReferenceSweptAreaSolid::new(SWEEP, entity).start_param()
        };
        assert!(
            matches!(
                start,
                Err(GeometryError::WrongValueKind {
                    attribute: "StartParam",
                    ..
                })
            ),
            "{kind}: {start:?}"
        );
    }
}
