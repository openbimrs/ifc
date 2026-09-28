//! Values authored in the form each release declares validate against their
//! own release (#200, #201).
//!
//! The writers #200 and #201 name are authored in IFC2X3, IFC4 and IFC4X3
//! (each only where the release declares the entity), written to STEP, read
//! back with `ifc-step` and checked by `ifc-validate` against the declared
//! release's table. No record this test wrote may carry an error finding.
//!
//! `ifc-validate` does not yet flag a typed wrapper in a non-SELECT slot or a
//! bare value in a SELECT slot (#199), so the STEP text form of every
//! written attribute is asserted directly as well: bare or typed, and which
//! wrapper.

#![cfg(all(
    feature = "validate",
    feature = "geometry-select",
    feature = "structural",
    feature = "schedule",
    feature = "alignment",
    feature = "step"
))]

use ifc::alignment::{
    axis2_placement_linear, linear_placement, point_by_distance, referent, stationing,
};
use ifc::geometry::authoring::{
    axis2_placement_3d, cartesian_point, circle, direction, fixed_reference_swept_area_solid_in,
    plane, point_on_curve, point_on_surface, polyline, rectangle_profile,
    rectangular_trimmed_surface, reparametrised_composite_curve_segment,
    surface_curve_swept_area_solid_in, swept_disk_solid, swept_disk_solid_polygonal, SweepTrim,
};
use ifc::geometry::curve::TransitionCode;
use ifc::schedule::create_lag_time;
use ifc::structural::{
    stage_boundary_condition_in, AxisValues, BoundaryConditionDraft, BoundaryConditionKind,
    StiffnessValue,
};
use ifc::{Codec, Model, SchemaVersion, StepCodec, Value};
use ifc_model::{EntityId, Transaction};
use ifc_schema::for_version;

const TRIM: SweepTrim = SweepTrim {
    start: Some(0.25),
    end: Some(0.75),
};

fn model(token: &str) -> Model {
    let mut model = Model::default();
    model.header_mut().schema = vec![token.to_owned()];
    model
}

/// One written record and the STEP text its value slots must contain.
type Expect = (EntityId, &'static str);

fn stiffness(x: f64) -> AxisValues<Option<StiffnessValue>> {
    AxisValues {
        x: Some(StiffnessValue::Measure(x)),
        y: None,
        z: None,
    }
}

/// Author through every writer #200/#201 names that `version` declares.
#[allow(clippy::too_many_lines)]
fn author(model: &mut Model, version: SchemaVersion) -> Vec<Expect> {
    let x3 = version == SchemaVersion::Ifc4x3;
    let x2 = version == SchemaVersion::Ifc2x3;
    let schema = for_version(version).expect("bundled");
    let mut tx = Transaction::new(model);
    let p = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("p");
    let q = cartesian_point(&mut tx, &[5.0, 0.0, 0.0]).expect("q");
    let at = axis2_placement_3d(&mut tx, p, None, None);
    let flat = plane(&mut tx, at);
    let arc = circle(&mut tx, at, 1.0).expect("arc");
    let line = polyline(&mut tx, &[p, q]).expect("line");
    let profile = rectangle_profile(&mut tx, None, None, 0.3, 0.2).expect("profile");
    let up = direction(&mut tx, &[0.0, 0.0, 1.0]).expect("up");

    let sweep_trim = if x3 {
        ",IFCPARAMETERVALUE(0.25),IFCPARAMETERVALUE(0.75),#"
    } else {
        ",0.25,0.75,#"
    };
    let mut out = vec![
        (
            rectangular_trimmed_surface(&mut tx, flat, (0.25, 2.5), (0.5, 3.5)).expect("trim"),
            ",0.25,0.5,2.5,3.5,",
        ),
        (point_on_curve(&mut tx, arc, 0.25).expect("poc"), ",0.25)"),
        (
            point_on_surface(&mut tx, flat, 0.25, 0.75).expect("pos"),
            ",0.25,0.75)",
        ),
        (
            swept_disk_solid(&mut tx, line, 0.2, None, TRIM).expect("disk"),
            ",0.25,0.75)",
        ),
        (
            surface_curve_swept_area_solid_in(&mut tx, model, profile, Some(at), line, TRIM, flat)
                .expect("surface-curve sweep"),
            sweep_trim,
        ),
    ];
    if !x2 {
        out.extend([
            (
                reparametrised_composite_curve_segment(
                    &mut tx,
                    TransitionCode::Continuous,
                    true,
                    arc,
                    2.5,
                )
                .expect("segment"),
                ",2.5)",
            ),
            (
                swept_disk_solid_polygonal(&mut tx, line, 0.2, None, TRIM, None).expect("poly"),
                ",0.25,0.75,$)",
            ),
            (
                fixed_reference_swept_area_solid_in(
                    &mut tx,
                    model,
                    profile,
                    Some(at),
                    line,
                    TRIM,
                    up,
                )
                .expect("fixed-reference sweep"),
                sweep_trim,
            ),
        ]);
    }

    // Boundary conditions: one measure per family, plus warping.
    let families: [(BoundaryConditionKind, &str, &str); 4] = [
        (
            BoundaryConditionKind::Node,
            "($,1.5,$,$,4.5,$,$)",
            "($,IFCLINEARSTIFFNESSMEASURE(1.5),$,$,IFCROTATIONALSTIFFNESSMEASURE(4.5),$,$)",
        ),
        (
            BoundaryConditionKind::NodeWarping,
            "($,1.5,$,$,4.5,$,$,7.5)",
            "($,IFCLINEARSTIFFNESSMEASURE(1.5),$,$,IFCROTATIONALSTIFFNESSMEASURE(4.5),$,$,\
             IFCWARPINGMOMENTMEASURE(7.5))",
        ),
        (
            BoundaryConditionKind::Edge,
            "($,1.5,$,$,4.5,$,$)",
            "($,IFCMODULUSOFLINEARSUBGRADEREACTIONMEASURE(1.5),$,$,\
             IFCMODULUSOFROTATIONALSUBGRADEREACTIONMEASURE(4.5),$,$)",
        ),
        (
            BoundaryConditionKind::Face,
            "($,1.5,$,$)",
            "($,IFCMODULUSOFSUBGRADEREACTIONMEASURE(1.5),$,$)",
        ),
    ];
    for (kind, bare, typed) in families {
        let draft = BoundaryConditionDraft {
            name: None,
            translational: stiffness(1.5),
            rotational: if kind == BoundaryConditionKind::Face {
                AxisValues::default()
            } else {
                stiffness(4.5)
            },
            warping: (kind == BoundaryConditionKind::NodeWarping)
                .then_some(StiffnessValue::Measure(7.5)),
        };
        let id = stage_boundary_condition_in(&mut tx, schema, kind, draft).expect("condition");
        out.push((id, if x2 { bare } else { typed }));
    }
    if !x2 {
        let flags = BoundaryConditionDraft {
            name: None,
            translational: AxisValues {
                x: Some(StiffnessValue::Boolean(true)),
                y: None,
                z: None,
            },
            ..BoundaryConditionDraft::default()
        };
        let id = stage_boundary_condition_in(&mut tx, schema, BoundaryConditionKind::Node, flags)
            .expect("boolean condition");
        out.push((id, "($,IFCBOOLEAN(.T.),$,$,$,$,$)"));

        // IfcLagTime: IFC4 and IFC4X3 only.
        let duration = create_lag_time(&mut tx, None, Value::Text("P5D".into()), "WORKTIME")
            .expect("duration lag");
        let ratio = create_lag_time(&mut tx, None, Value::Real(0.5), "WORKTIME").expect("ratio");
        out.push((duration, ",IFCDURATION('P5D'),.WORKTIME.)"));
        out.push((ratio, ",IFCRATIOMEASURE(0.5),.WORKTIME.)"));
    }
    let mut properties = Vec::new();
    if x3 {
        // Referents: IFC4X3 only.
        let point = point_by_distance(&mut tx, 125.0, (None, None, None), line).expect("point");
        out.push((point, "(IFCLENGTHMEASURE(125.),$,$,$,#"));
        let axis = axis2_placement_linear(&mut tx, point, None, None).expect("axis");
        let placement = linear_placement(&mut tx, axis, None, None).expect("placement");
        let marker = referent(
            &mut tx,
            "0aBcDeFgHiJkLmNoPqRsTu",
            None,
            None,
            Some(placement),
        )
        .expect("referent");
        let before = tx.edits().len();
        stationing(
            &mut tx,
            "1aBcDeFgHiJkLmNoPqRsTu",
            "2aBcDeFgHiJkLmNoPqRsTu",
            marker,
            1125.0,
            Some(1100.0),
            Some(true),
        )
        .expect("stationing");
        properties.extend(tx.edits()[before..].iter().filter_map(|edit| match edit {
            ifc_model::Edit::Create { id, entity } if entity.is_type("IFCPROPERTYSINGLEVALUE") => {
                Some(*id)
            }
            _ => None,
        }));
    }
    let expected = [
        "IFCLENGTHMEASURE(1125.)",
        "IFCLENGTHMEASURE(1100.)",
        "IFCBOOLEAN(.T.)",
    ];
    out.extend(properties.into_iter().zip(expected));
    tx.commit(model).expect("commit");
    out
}

#[test]
fn authored_values_have_their_declared_form_and_validate() {
    for (token, version, count) in [
        ("IFC2X3", SchemaVersion::Ifc2x3, 9),
        ("IFC4", SchemaVersion::Ifc4, 15),
        ("IFC4X3_ADD2", SchemaVersion::Ifc4x3, 19),
    ] {
        let mut model = model(token);
        let written = author(&mut model, version);
        assert_eq!(written.len(), count, "{token}: every writer is covered");

        let bytes = StepCodec.write_bytes(&model).expect("written");
        let text = String::from_utf8(bytes.clone()).expect("utf8");
        for (id, form) in &written {
            let prefix = format!("#{}=", id.0);
            let line = text
                .lines()
                .find(|line| line.starts_with(&prefix))
                .expect("record");
            assert!(line.contains(form), "{token}: {form} in {line}");
            // A record expected bare carries no wrapper anywhere.
            const WRAPPERS: [&str; 4] = [
                "MEASURE(",
                "IFCPARAMETERVALUE(",
                "IFCBOOLEAN(",
                "IFCDURATION(",
            ];
            if !WRAPPERS.iter().any(|wrapper| form.contains(wrapper)) {
                for wrapper in WRAPPERS {
                    assert!(!line.contains(wrapper), "{token}: bare {line}");
                }
            }
        }

        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        let ids: Vec<EntityId> = written.iter().map(|(id, _)| *id).collect();
        let report = ifc_validate::validate(&back, for_version(version).expect("bundled"));
        let errors: Vec<String> = report
            .findings()
            .iter()
            .filter(|finding| finding.severity == ifc_validate::Severity::Error)
            .filter(|finding| match &finding.path {
                ifc_validate::Path::Entity(id)
                | ifc_validate::Path::Attribute { entity: id, .. } => ids.contains(id),
                ifc_validate::Path::File => false,
            })
            .map(|finding| format!("{} at {}: {}", finding.rule, finding.path, finding.message))
            .collect();
        assert!(errors.is_empty(), "{token}:\n  {}", errors.join("\n  "));
    }
}

/// The oracle above is only trusted because it fails when it should: the
/// wrappers `stage_boundary_condition` wrote before #200 for an edge
/// condition (a type no release declares) and for warping (a measure its
/// SELECT does not admit) are error findings in IFC4.
#[test]
fn the_validator_catches_the_old_boundary_condition_wrappers() {
    let typed = |name: &str| Value::Typed {
        type_name: name.into(),
        value: Box::new(Value::Real(1.5)),
    };
    let mut model = model("IFC4");
    let mut tx = Transaction::new(&model);
    let mut edge = vec![Value::Null; 7];
    edge[1] = typed("IFCMODULUSOFTRANSLATIONALSUBGRADEREACTIONMEASURE");
    let edge = tx.create(ifc::Entity::new("IFCBOUNDARYEDGECONDITION", edge));
    let mut warping = vec![Value::Null; 8];
    warping[7] = typed("IFCROTATIONALSTIFFNESSMEASURE");
    let warping = tx.create(ifc::Entity::new("IFCBOUNDARYNODECONDITIONWARPING", warping));
    tx.commit(&mut model).expect("commit");
    let report = ifc_validate::validate(&model, for_version(SchemaVersion::Ifc4).unwrap());
    for (id, index) in [(edge, 1), (warping, 7)] {
        assert!(
            report.findings().iter().any(|finding| {
                finding.severity == ifc_validate::Severity::Error
                    && matches!(
                        &finding.path,
                        ifc_validate::Path::Attribute { entity, index: i, .. }
                            if *entity == id && *i == index
                    )
            }),
            "#{} slot {index}: {:?}",
            id.0,
            report.findings()
        );
    }
}
