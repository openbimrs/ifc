//! Parameter values in the form each release declares (#200).
//!
//! `IfcParameterValue` is a defined type (`= REAL`), so a slot declared with
//! it takes the bare value; only a SELECT slot takes the typed parameter
//! `IFCPARAMETERVALUE(..)`. The directrix sweeps' trims are the one case that
//! changes between releases: `IfcParameterValue` in IFC2X3 and IFC4,
//! `IfcCurveMeasureSelect` in IFC4X3. Each release is authored, written to
//! STEP, read back with `ifc-step` and read through the crate's views; the
//! STEP text is asserted directly, because either wrong form survives a
//! round trip unnoticed.

use ifc_geometry::authoring::{
    axis2_placement_3d, cartesian_point, circle, direction, fixed_reference_swept_area_solid_in,
    plane, point_on_curve, point_on_surface, polyline, rectangle_profile,
    rectangular_trimmed_surface, reparametrised_composite_curve_segment,
    surface_curve_swept_area_solid_in, swept_disk_solid_in, swept_disk_solid_polygonal, SweepTrim,
};
use ifc_geometry::curve::{CompositeCurveSegment, TransitionCode};
use ifc_geometry::resource::point::{PointOnCurve, PointOnSurface};
use ifc_geometry::solid::swept::{
    FixedReferenceSweptAreaSolid, SurfaceCurveSweptAreaSolid, SweptDiskSolid, TrimMeasure,
};
use ifc_geometry::surface::bounded::RectangularTrimmedSurface;
use ifc_geometry::GeometryError;
use ifc_model::codec::Codec;
use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_schema::{for_version, SchemaVersion, TypeKind};
use ifc_step::StepCodec;

const RELEASES: [(&str, SchemaVersion); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3),
    ("IFC4", SchemaVersion::Ifc4),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
];

const TRIM: SweepTrim = SweepTrim {
    start: Some(0.25),
    end: Some(0.75),
};

/// Attributes declared `IfcParameterValue` in every release declaring them.
const BARE: [(&str, &[&str]); 6] = [
    ("IfcRectangularTrimmedSurface", &["U1", "V1", "U2", "V2"]),
    ("IfcPointOnCurve", &["PointParameter"]),
    ("IfcPointOnSurface", &["PointParameterU", "PointParameterV"]),
    ("IfcReparametrisedCompositeCurveSegment", &["ParamLength"]),
    ("IfcSweptDiskSolid", &["StartParam", "EndParam"]),
    ("IfcSweptDiskSolidPolygonal", &["StartParam", "EndParam"]),
];

/// Attributes whose declaration changes between releases.
const SWEEPS: [&str; 2] = [
    "IfcSurfaceCurveSweptAreaSolid",
    "IfcFixedReferenceSweptAreaSolid",
];

fn declared_select(version: SchemaVersion, entity: &str, attribute: &str) -> Option<bool> {
    let schema = for_version(version).expect("bundled");
    schema.entity(entity)?;
    let declared = schema
        .attributes(entity)
        .into_iter()
        .find(|a| a.name == attribute)
        .expect("declared attribute")
        .type_name
        .clone();
    assert!(
        schema.accepts_type(&declared, "IfcParameterValue"),
        "{version:?} {entity}.{attribute} admits IfcParameterValue"
    );
    Some(matches!(
        schema.type_def(&declared).map(|d| &d.kind),
        Some(TypeKind::Select(_))
    ))
}

/// The forms this file expects are the declared types of the bundled
/// tables, so the expectations below are the schema's.
#[test]
fn the_expected_forms_are_the_declared_types() {
    for (_, version) in RELEASES {
        for (entity, attributes) in BARE {
            for attribute in attributes {
                if let Some(select) = declared_select(version, entity, attribute) {
                    assert!(
                        !select,
                        "{version:?} {entity}.{attribute} is a defined type"
                    );
                }
            }
        }
        for entity in SWEEPS {
            for attribute in ["StartParam", "EndParam"] {
                if let Some(select) = declared_select(version, entity, attribute) {
                    assert_eq!(
                        select,
                        version == SchemaVersion::Ifc4x3,
                        "{version:?} {entity}.{attribute}"
                    );
                }
            }
        }
    }
    assert_eq!(
        declared_select(SchemaVersion::Ifc2x3, SWEEPS[1], "StartParam"),
        None,
        "IFC2X3 declares no fixed-reference sweep"
    );
}

fn model(tokens: &[&str]) -> Model {
    let mut model = Model::default();
    model.header_mut().schema = tokens.iter().map(|t| (*t).to_owned()).collect();
    model
}

/// Every writer #200 names, authored in `version`'s release.
struct Authored {
    trimmed: EntityId,
    on_curve: EntityId,
    on_surface: EntityId,
    disk: EntityId,
    surface_curve: EntityId,
    /// Entities IFC2X3 does not declare.
    later: Option<(EntityId, EntityId, EntityId)>,
}

fn author(model: &mut Model, version: SchemaVersion) -> Authored {
    let mut tx = Transaction::new(model);
    let point = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("point");
    let at = axis2_placement_3d(&mut tx, point, None, None);
    let flat = plane(&mut tx, at);
    let arc = circle(&mut tx, at, 1.0).expect("arc");
    let far = cartesian_point(&mut tx, &[5.0, 0.0, 0.0]).expect("far");
    let line = polyline(&mut tx, &[point, far]).expect("line");
    let profile = rectangle_profile(&mut tx, None, None, 0.3, 0.2).expect("profile");
    let up = direction(&mut tx, &[0.0, 0.0, 1.0]).expect("up");

    let trimmed = rectangular_trimmed_surface(&mut tx, flat, (0.25, 2.5), (0.5, 3.5)).expect("t");
    let on_curve = point_on_curve(&mut tx, arc, 0.25).expect("on curve");
    let on_surface = point_on_surface(&mut tx, flat, 0.25, 0.75).expect("on surface");
    let disk = swept_disk_solid_in(&mut tx, model, line, 0.2, None, TRIM).expect("disk");
    let surface_curve =
        surface_curve_swept_area_solid_in(&mut tx, model, profile, Some(at), line, TRIM, flat)
            .expect("surface-curve sweep");
    let later = (version != SchemaVersion::Ifc2x3).then(|| {
        let segment = reparametrised_composite_curve_segment(
            &mut tx,
            TransitionCode::Continuous,
            true,
            arc,
            2.5,
        )
        .expect("segment");
        let polygonal =
            swept_disk_solid_polygonal(&mut tx, line, 0.2, None, TRIM, None).expect("polygonal");
        let fixed =
            fixed_reference_swept_area_solid_in(&mut tx, model, profile, Some(at), line, TRIM, up)
                .expect("fixed-reference sweep");
        (segment, polygonal, fixed)
    });
    tx.commit(model).expect("commit");
    Authored {
        trimmed,
        on_curve,
        on_surface,
        disk,
        surface_curve,
        later,
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

fn bare(text: &str, id: EntityId, values: &str) {
    let line = record(text, id);
    assert!(!line.contains("IFCPARAMETERVALUE("), "bare: {line}");
    assert!(line.contains(values), "{values} in {line}");
}

#[test]
fn every_release_writes_its_declared_form_and_reads_back() {
    for (token, version) in RELEASES {
        let mut model = model(&[token]);
        let ids = author(&mut model, version);
        let bytes = StepCodec.write_bytes(&model).expect("written");
        let text = String::from_utf8(bytes.clone()).expect("utf8");

        bare(&text, ids.trimmed, ",0.25,0.5,2.5,3.5,");
        bare(&text, ids.on_curve, ",0.25)");
        bare(&text, ids.on_surface, ",0.25,0.75)");
        bare(&text, ids.disk, ",0.25,0.75)");
        let sweep = record(&text, ids.surface_curve);
        if version == SchemaVersion::Ifc4x3 {
            assert!(
                sweep.contains(",IFCPARAMETERVALUE(0.25),IFCPARAMETERVALUE(0.75),#"),
                "{token}: {sweep}"
            );
        } else {
            bare(&text, ids.surface_curve, ",0.25,0.75,#");
        }
        if let Some((segment, polygonal, fixed)) = ids.later {
            bare(&text, segment, ",2.5)");
            bare(&text, polygonal, ",0.25,0.75,$)");
            let line = record(&text, fixed);
            if version == SchemaVersion::Ifc4x3 {
                assert!(
                    line.contains(",IFCPARAMETERVALUE(0.25),IFCPARAMETERVALUE(0.75),#"),
                    "{token}: {line}"
                );
            } else {
                bare(&text, fixed, ",0.25,0.75,#");
            }
        }

        let back = StepCodec.read_bytes(&bytes).expect("read back");
        let get = |id| back.get(id).expect("read back entity");
        let rect = RectangularTrimmedSurface::new(ids.trimmed, get(ids.trimmed))
            .rectangle()
            .expect("rectangle");
        assert_eq!((rect.u1, rect.v1, rect.u2, rect.v2), (0.25, 0.5, 2.5, 3.5));
        let on_curve = PointOnCurve::new(ids.on_curve, get(ids.on_curve));
        assert_eq!(on_curve.point_parameter().expect("parameter"), 0.25);
        let on_surface = PointOnSurface::new(ids.on_surface, get(ids.on_surface));
        assert_eq!(on_surface.parameters().expect("parameters"), (0.25, 0.75));
        let disk = SweptDiskSolid::new(ids.disk, get(ids.disk));
        assert_eq!(
            (disk.start_param(), disk.end_param()),
            (Some(0.25), Some(0.75))
        );
        // Every release's trim reads back as a parameter, never a length.
        let parameters = (
            Some(TrimMeasure::Parameter(0.25)),
            Some(TrimMeasure::Parameter(0.75)),
        );
        let sweep = SurfaceCurveSweptAreaSolid::new(ids.surface_curve, get(ids.surface_curve));
        assert_eq!(
            (
                sweep.start_param().expect("s"),
                sweep.end_param().expect("e")
            ),
            parameters,
            "{token}"
        );
        if let Some((segment, polygonal, fixed)) = ids.later {
            let segment = CompositeCurveSegment::new(segment, get(segment));
            assert_eq!(segment.param_length().expect("length"), Some(2.5));
            let polygonal = SweptDiskSolid::new(polygonal, get(polygonal));
            assert_eq!(polygonal.start_param(), Some(0.25));
            let fixed = FixedReferenceSweptAreaSolid::new(fixed, get(fixed));
            assert_eq!(
                (
                    fixed.start_param().expect("s"),
                    fixed.end_param().expect("e")
                ),
                parameters,
                "{token}"
            );
        }
    }
}

/// The release-bound writers follow `FILE_SCHEMA`, and a model without one
/// binds IFC4: typed trims only in IFC4X3, and the swept disk bare in all.
#[test]
fn the_bound_sweeps_follow_the_declared_release() {
    for (token, typed) in [("IFC4X3_ADD2", true), ("IFC4", false), ("", false)] {
        let tokens: &[&str] = if token.is_empty() { &[] } else { &[token] };
        let model = model(tokens);
        let mut tx = Transaction::new(&model);
        let p = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("p");
        let at = axis2_placement_3d(&mut tx, p, None, None);
        let flat = plane(&mut tx, at);
        let up = direction(&mut tx, &[0.0, 0.0, 1.0]).expect("up");
        // Any staged entity will do: nothing here resolves the references.
        let (area, line) = (p, flat);
        let sweeps = [
            surface_curve_swept_area_solid_in(&mut tx, &model, area, Some(at), line, TRIM, flat),
            fixed_reference_swept_area_solid_in(&mut tx, &model, area, Some(at), line, TRIM, up),
        ];
        let disk = swept_disk_solid_in(&mut tx, &model, line, 0.2, None, TRIM).expect("disk");
        let mut model = model;
        tx.commit(&mut model).expect("commit");
        for sweep in sweeps {
            let sweep = model.get(sweep.expect("sweep")).expect("sweep");
            let expected = if typed {
                Value::Typed {
                    type_name: "IFCPARAMETERVALUE".into(),
                    value: Box::new(Value::Real(0.25)),
                }
            } else {
                Value::Real(0.25)
            };
            assert_eq!(sweep.attributes[3], expected, "{token:?} StartParam");
        }
        let disk = model.get(disk).expect("disk");
        assert_eq!(disk.attributes[3], Value::Real(0.25), "{token:?}");
    }
}

#[test]
fn what_the_release_cannot_hold_is_refused() {
    let area = EntityId(900);
    let line = EntityId(901);
    let reference = EntityId(902);
    let at = EntityId(903);

    let ifc2x3 = model(&["IFC2X3"]);
    let mut tx = Transaction::new(&ifc2x3);
    let fixed = fixed_reference_swept_area_solid_in(
        &mut tx,
        &ifc2x3,
        area,
        Some(at),
        line,
        TRIM,
        reference,
    );
    assert!(
        matches!(
            fixed,
            Err(GeometryError::AuthoringEntityNotInSchema {
                schema: SchemaVersion::Ifc2x3,
                ..
            })
        ),
        "{fixed:?}"
    );
    let untrimmed = SweepTrim::default();
    for (position, trim, attribute) in [
        (Some(at), untrimmed, "StartParam"),
        (None, TRIM, "Position"),
    ] {
        let refused = surface_curve_swept_area_solid_in(
            &mut tx, &ifc2x3, area, position, line, trim, reference,
        );
        assert!(
            matches!(
                &refused,
                Err(GeometryError::InvalidAuthoredValue { attribute: a, .. }) if *a == attribute
            ),
            "{refused:?}"
        );
    }
    assert!(tx.is_empty(), "a refusal stages nothing");

    for tokens in [&["IFC5"][..], &["IFC4", "IFC4X3_ADD2"][..]] {
        let unbound = model(tokens);
        let mut tx = Transaction::new(&unbound);
        let refused = surface_curve_swept_area_solid_in(
            &mut tx,
            &unbound,
            area,
            Some(at),
            line,
            TRIM,
            reference,
        );
        assert!(
            matches!(refused, Err(GeometryError::AuthoringSchemaUnbound { .. })),
            "{tokens:?}: {refused:?}"
        );
        assert!(tx.is_empty());
    }
}

/// The readers accept the bare form written now and the typed form written
/// before #200 (and by other tools), in every slot this change touches.
#[test]
fn the_readers_accept_both_forms() {
    let typed = |v: f64| Value::Typed {
        type_name: "IFCPARAMETERVALUE".into(),
        value: Box::new(Value::Real(v)),
    };
    let r = EntityId(1);
    for form in [Value::Real as fn(f64) -> Value, typed] {
        let trimmed = Entity::new(
            "IFCRECTANGULARTRIMMEDSURFACE",
            vec![
                Value::Ref(r),
                form(0.25),
                form(0.5),
                form(2.5),
                form(3.5),
                Value::Bool(true),
                Value::Bool(true),
            ],
        );
        let rect = RectangularTrimmedSurface::new(r, &trimmed)
            .rectangle()
            .expect("rectangle");
        assert_eq!((rect.u1, rect.v2), (0.25, 3.5));

        let on_curve = Entity::new("IFCPOINTONCURVE", vec![Value::Ref(r), form(0.25)]);
        let view = PointOnCurve::new(r, &on_curve);
        assert_eq!(view.point_parameter().expect("parameter"), 0.25);

        let on_surface = Entity::new(
            "IFCPOINTONSURFACE",
            vec![Value::Ref(r), form(0.25), form(0.75)],
        );
        let view = PointOnSurface::new(r, &on_surface);
        assert_eq!(view.parameters().expect("parameters"), (0.25, 0.75));

        let segment = Entity::new(
            "IFCREPARAMETRISEDCOMPOSITECURVESEGMENT",
            vec![
                Value::Enum("CONTINUOUS".into()),
                Value::Bool(true),
                Value::Ref(r),
                form(2.5),
            ],
        );
        let view = CompositeCurveSegment::new(r, &segment);
        assert_eq!(view.param_length().expect("length"), Some(2.5));

        let disk = Entity::new(
            "IFCSWEPTDISKSOLID",
            vec![
                Value::Ref(r),
                Value::Real(0.2),
                Value::Null,
                form(0.25),
                form(0.75),
            ],
        );
        let view = SweptDiskSolid::new(r, &disk);
        assert_eq!(
            (view.start_param(), view.end_param()),
            (Some(0.25), Some(0.75))
        );

        let sweep_attrs = vec![
            Value::Ref(r),
            Value::Null,
            Value::Ref(r),
            form(0.25),
            form(0.75),
            Value::Ref(r),
        ];
        let parameters = (
            Some(TrimMeasure::Parameter(0.25)),
            Some(TrimMeasure::Parameter(0.75)),
        );
        let sweep = Entity::new("IFCSURFACECURVESWEPTAREASOLID", sweep_attrs.clone());
        let view = SurfaceCurveSweptAreaSolid::new(r, &sweep);
        assert_eq!(
            (view.start_param().expect("s"), view.end_param().expect("e")),
            parameters
        );
        let fixed = Entity::new("IFCFIXEDREFERENCESWEPTAREASOLID", sweep_attrs);
        let view = FixedReferenceSweptAreaSolid::new(r, &fixed);
        assert_eq!(
            (view.start_param().expect("s"), view.end_param().expect("e")),
            parameters
        );
    }
}

/// An IFC4X3 trim may be a length along the directrix (#210). The readers
/// say so rather than handing the length back as a parameter.
#[test]
fn a_length_trim_reads_as_a_length() {
    let r = EntityId(1);
    let length = |v: f64| Value::Typed {
        type_name: "IFCLENGTHMEASURE".into(),
        value: Box::new(Value::Real(v)),
    };
    let attrs = vec![
        Value::Ref(r),
        Value::Null,
        Value::Ref(r),
        length(0.5),
        length(4.0),
        Value::Ref(r),
    ];
    let lengths = (
        Some(TrimMeasure::Length(0.5)),
        Some(TrimMeasure::Length(4.0)),
    );
    let sweep = Entity::new("IFCSURFACECURVESWEPTAREASOLID", attrs.clone());
    let view = SurfaceCurveSweptAreaSolid::new(r, &sweep);
    assert_eq!(
        (view.start_param().expect("s"), view.end_param().expect("e")),
        lengths
    );
    let fixed = Entity::new("IFCFIXEDREFERENCESWEPTAREASOLID", attrs);
    let view = FixedReferenceSweptAreaSolid::new(r, &fixed);
    assert_eq!(
        (view.start_param().expect("s"), view.end_param().expect("e")),
        lengths
    );
}

/// IFC2X3 requires the swept disk's trim; later releases do not (#210).
#[test]
fn an_ifc2x3_swept_disk_needs_both_trim_values() {
    let line = EntityId(901);
    let ifc2x3 = model(&["IFC2X3"]);
    let mut tx = Transaction::new(&ifc2x3);
    for (trim, attribute) in [
        (SweepTrim::default(), "StartParam"),
        (
            SweepTrim {
                start: Some(0.0),
                end: None,
            },
            "EndParam",
        ),
    ] {
        let refused = swept_disk_solid_in(&mut tx, &ifc2x3, line, 0.2, None, trim);
        assert!(
            matches!(
                &refused,
                Err(GeometryError::InvalidAuthoredValue { attribute: a, .. }) if *a == attribute
            ),
            "{refused:?}"
        );
    }
    assert!(tx.is_empty(), "a refusal stages nothing");
    swept_disk_solid_in(&mut tx, &ifc2x3, line, 0.2, None, TRIM).expect("trimmed IFC2X3 disk");

    for token in ["IFC4", "IFC4X3_ADD2"] {
        let later = model(&[token]);
        let mut tx = Transaction::new(&later);
        swept_disk_solid_in(&mut tx, &later, line, 0.2, None, SweepTrim::default())
            .unwrap_or_else(|e| panic!("{token} leaves the trim optional: {e}"));
    }
}
