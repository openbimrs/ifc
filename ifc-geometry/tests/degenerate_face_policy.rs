//! Opt-in policy for collapsed `IfcPolyLoop` faces (#46).
//!
//! A real exporter (OfficeBuilding.ifc, 16 `IfcWindow`s) writes a closed
//! faceted brep plus one extra face whose only loop is `(A, A, B, B)`: two
//! distinct points, so it encloses no area. By default the whole brep is
//! refused over that face. `DegenerateFacePolicy::DropAndReport` leaves the
//! sliver out, keeps the solid, and names the face in provenance.
//!
//! The box is a unit cube scaled to 2 x 3 x 4, so every assertion is against
//! a hand-computed volume, not "it compiled".

#![cfg(feature = "compile-reference-backend")]

use axiolid_contracts::ExecutionOptions;
use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;
use axiolid_mesh_compile_contract::MeshCompiler;
use ifc_geometry::compile::default_backend;
use ifc_geometry::lower::{
    lower_faceted_brep_node, lower_product_representation, DegenerateFacePolicy, LoweredGeometry,
    LoweringSession,
};
use ifc_geometry::transform::Transform;
use ifc_geometry::RepresentationPurpose;
use ifc_geometry::{units, GeometryError};
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

const BOX: [f64; 3] = [2.0, 3.0, 4.0];
const BOX_VOLUME: f64 = 24.0;

/// The sliver face is `#90`, its loop `#91`, both on the box's top edge.
const SLIVER_FACE: EntityId = EntityId(90);
const SLIVER_LOOP: EntityId = EntityId(91);

/// A product `#1` whose Body is one `IfcFacetedBrep` `#60`: a closed 2x3x4
/// box, plus `extra` face records listed in the shell after the six real ones.
fn file(extra_faces: &[&str], extra_records: &str) -> String {
    let [x, y, z] = BOX;
    let mut faces = vec!["#30", "#32", "#34", "#36", "#38", "#40"];
    faces.extend_from_slice(extra_faces);
    format!(
        "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCBUILDINGELEMENTPROXY('0YvctVUKr0kugbFTf53O9L',$,'Window',$,$,$,#2,$,$);
#2=IFCPRODUCTDEFINITIONSHAPE($,$,(#3));
#3=IFCSHAPEREPRESENTATION(#4,'Body','Brep',(#60));
#4=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#5,$);
#5=IFCAXIS2PLACEMENT3D(#6,$,$);
#6=IFCCARTESIANPOINT((0.,0.,0.));
#10=IFCCARTESIANPOINT((0.,0.,0.));
#11=IFCCARTESIANPOINT(({x},0.,0.));
#12=IFCCARTESIANPOINT(({x},{y},0.));
#13=IFCCARTESIANPOINT((0.,{y},0.));
#14=IFCCARTESIANPOINT((0.,0.,{z}));
#15=IFCCARTESIANPOINT(({x},0.,{z}));
#16=IFCCARTESIANPOINT(({x},{y},{z}));
#17=IFCCARTESIANPOINT((0.,{y},{z}));
#29=IFCPOLYLOOP((#10,#13,#12,#11));
#30=IFCFACE((#31));
#31=IFCFACEOUTERBOUND(#29,.T.);
#41=IFCPOLYLOOP((#14,#15,#16,#17));
#32=IFCFACE((#33));
#33=IFCFACEOUTERBOUND(#41,.T.);
#42=IFCPOLYLOOP((#10,#11,#15,#14));
#34=IFCFACE((#35));
#35=IFCFACEOUTERBOUND(#42,.T.);
#43=IFCPOLYLOOP((#11,#12,#16,#15));
#36=IFCFACE((#37));
#37=IFCFACEOUTERBOUND(#43,.T.);
#44=IFCPOLYLOOP((#12,#13,#17,#16));
#38=IFCFACE((#39));
#39=IFCFACEOUTERBOUND(#44,.T.);
#45=IFCPOLYLOOP((#13,#10,#14,#17));
#40=IFCFACE((#46));
#46=IFCFACEOUTERBOUND(#45,.T.);
#50=IFCCLOSEDSHELL(({faces}));
#60=IFCFACETEDBREP(#50);
{extra_records}ENDSEC;
END-ISO-10303-21;
",
        faces = faces.join(",")
    )
}

/// The OfficeBuilding shape: one face whose loop is `(A, A, B, B)`.
fn box_with_sliver() -> String {
    file(
        &["#90"],
        "#90=IFCFACE((#92));\n#92=IFCFACEOUTERBOUND(#91,.T.);\n\
         #91=IFCPOLYLOOP((#14,#14,#15,#15));\n",
    )
}

fn parse(text: &str) -> Model {
    StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses")
}

fn lower(model: &Model, policy: DegenerateFacePolicy) -> Result<LoweredGeometry, GeometryError> {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale).with_face_policy(policy);
    let root =
        lower_product_representation(&mut session, EntityId(1), RepresentationPurpose::Body)?
            .expect("the product has a Body");
    session.finish(root)
}

fn volume(lowered: &LoweredGeometry) -> f64 {
    let mesh = default_backend()
        .compile_mesh(
            &lowered.graph,
            lowered.root,
            &ExecutionOptions::new(Tolerance::MILLIMETRE),
        )
        .expect("the lowered body compiles");
    signed_volume(&mesh)
}

/// Signed volume by the divergence theorem, about the first vertex so the
/// sum does not cancel catastrophically far from the origin.
fn signed_volume(mesh: &TriMesh) -> f64 {
    assert!(!mesh.indices.is_empty(), "compiled mesh is empty");
    let base = mesh.positions[0];
    mesh.indices
        .chunks_exact(3)
        .map(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize] - base);
            a.dot(b.cross(c)) / 6.0
        })
        .sum()
}

fn assert_volume(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "{what}: volume {actual}, expected {expected}"
    );
}

/// Default behaviour is unchanged: the brep is refused, naming the loop.
#[test]
fn the_default_still_refuses_a_collapsed_face() {
    let model = parse(&box_with_sliver());
    assert_eq!(
        LoweringSession::new(&model, &units::resolve(&model)).face_policy(),
        DegenerateFacePolicy::Refuse
    );
    let error = lower(&model, DegenerateFacePolicy::default()).expect_err("must refuse");
    assert_eq!(error.entity(), Some(SLIVER_LOOP), "names the loop: {error}");
    assert!(
        error.to_string().contains("collapses to 2 distinct edges"),
        "{error}"
    );
}

/// The issue's done-when: the box with a sliver lowers to the box, and the
/// sliver's id is reported.
#[test]
fn drop_and_report_lowers_the_box_and_names_the_sliver() {
    let model = parse(&box_with_sliver());
    let lowered = lower(&model, DegenerateFacePolicy::DropAndReport).expect("lowers");
    assert_eq!(
        lowered.provenance.dropped_faces().collect::<Vec<_>>(),
        vec![SLIVER_FACE]
    );
    assert_volume(volume(&lowered), BOX_VOLUME, "box without its sliver");
}

/// Dropping a sliver yields exactly the box a file without it yields.
#[test]
fn the_dropped_result_equals_the_box_authored_without_the_sliver() {
    let clean = lower(&parse(&file(&[], "")), DegenerateFacePolicy::Refuse).expect("clean box");
    let dropped = lower(
        &parse(&box_with_sliver()),
        DegenerateFacePolicy::DropAndReport,
    )
    .expect("sliver dropped");
    assert_eq!(clean.provenance.dropped_faces().count(), 0);
    assert_volume(volume(&dropped), volume(&clean), "dropped vs clean");
}

/// A clean file reports nothing dropped under either policy.
#[test]
fn nothing_is_reported_when_nothing_collapses() {
    let model = parse(&file(&[], ""));
    for policy in [
        DegenerateFacePolicy::Refuse,
        DegenerateFacePolicy::DropAndReport,
    ] {
        let lowered = lower(&model, policy).expect("lowers");
        assert_eq!(lowered.provenance.dropped_faces().count(), 0, "{policy:?}");
        assert_volume(volume(&lowered), BOX_VOLUME, "clean box");
    }
}

/// The drop test is exactly the refusal test, so the policy drops precisely
/// what the default refuses. One repeated point and `(A, A, B, B)` collapse.
/// A real triangle does not. `(A, B, A, B)` is NOT a collapse by the
/// existing rule (its four implied edges each join two different points),
/// so it is not dropped either; widening that is a separate decision.
#[test]
fn only_loops_with_fewer_than_three_distinct_edges_are_dropped() {
    for (points, dropped) in [
        ("#14,#14,#14", true),
        ("#14,#14,#15,#15", true),
        ("#14,#15,#14,#15", false),
        ("#14,#15,#16", false),
    ] {
        let text = file(
            &["#90"],
            &format!(
                "#90=IFCFACE((#92));\n#92=IFCFACEOUTERBOUND(#91,.T.);\n\
                 #91=IFCPOLYLOOP(({points}));\n"
            ),
        );
        let lowered = lower(&parse(&text), DegenerateFacePolicy::DropAndReport)
            .unwrap_or_else(|e| panic!("{points}: must lower under DropAndReport, got {e}"));
        let reported: Vec<_> = lowered.provenance.dropped_faces().collect();
        assert_eq!(
            reported,
            if dropped { vec![SLIVER_FACE] } else { vec![] },
            "{points}"
        );
        // The invariant: dropped under DropAndReport iff refused by default.
        let refused = lower(&parse(&text), DegenerateFacePolicy::Refuse).is_err();
        assert_eq!(refused, dropped, "{points}: policies must agree");
    }
}

/// A collapsed HOLE in a face with a real outer bound is still refused:
/// dropping that face would remove the whole bottom of the box.
#[test]
fn a_collapsed_inner_bound_is_still_refused() {
    let text = file(
        &[],
        "#93=IFCFACEBOUND(#91,.T.);\n#91=IFCPOLYLOOP((#10,#10,#11,#11));\n",
    )
    .replace("#30=IFCFACE((#31));", "#30=IFCFACE((#31,#93));");
    assert!(
        text.contains("#30=IFCFACE((#31,#93));"),
        "substitution applied"
    );
    let error = lower(&parse(&text), DegenerateFacePolicy::DropAndReport)
        .expect_err("a face with real area is never dropped");
    assert_eq!(error.entity(), Some(SLIVER_LOOP), "{error}");
}

/// A face shared by several items is reported once.
#[test]
fn a_sliver_reached_twice_is_reported_once() {
    let text = box_with_sliver().replace("(#60));", "(#60,#61));").replace(
        "#60=IFCFACETEDBREP(#50);",
        "#60=IFCFACETEDBREP(#50);\n#61=IFCFACETEDBREP(#50);",
    );
    assert!(text.contains("#61=IFCFACETEDBREP"), "substitution applied");
    let lowered = lower(&parse(&text), DegenerateFacePolicy::DropAndReport).expect("lowers");
    assert_eq!(
        lowered.provenance.dropped_faces().collect::<Vec<_>>(),
        vec![SLIVER_FACE]
    );
}

/// A shell whose every face collapses is refused, not emitted empty.
#[test]
fn a_shell_of_only_slivers_is_refused() {
    let text = "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#10=IFCCARTESIANPOINT((0.,0.,0.));
#11=IFCCARTESIANPOINT((1.,0.,0.));
#91=IFCPOLYLOOP((#10,#10,#11,#11));
#92=IFCFACEOUTERBOUND(#91,.T.);
#90=IFCFACE((#92));
#50=IFCCLOSEDSHELL((#90));
#60=IFCFACETEDBREP(#50);
ENDSEC;
END-ISO-10303-21;
";
    let model = parse(text);
    let scale = units::resolve(&model);
    let mut session =
        LoweringSession::new(&model, &scale).with_face_policy(DegenerateFacePolicy::DropAndReport);
    let error = lower_faceted_brep_node(&mut session, EntityId(60), Transform::identity())
        .expect_err("an empty shell is not a solid");
    assert_eq!(
        error.entity(),
        Some(EntityId(50)),
        "names the shell: {error}"
    );
}
