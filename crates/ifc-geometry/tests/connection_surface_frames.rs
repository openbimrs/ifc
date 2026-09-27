//! Frames for geometry placed outside a product's own body (#163, #164).
//!
//! A space boundary's connection surface is authored in the relating space's
//! coordinate system, so a consumer lowers it under the frame the space's body
//! is placed in (`product_representation_frame`) and expects it to land where
//! the same surface would after lowering at the identity and moving the mesh.
//! For an `IfcCurveBoundedPlane` that holds only if the boundaries stay in the
//! plane's parameter space and the frame reaches the basis plane alone.

#[cfg(feature = "lowering")]
use ifc_geometry::transform::Transform;
use ifc_geometry::{product_representation_frame, units, RepresentationPurpose};
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

/// `#40`: a 4 m x 3 m curve-bounded plane with a 1 m x 1 m hole, its plane
/// at the origin. `#41`: the same boundaries on a plane whose own `Position`
/// sits at x = 10 with the parameter plane spanning y (u) and z (v).
///
/// `#124` is a wall in a body sub-context whose parent's
/// `WorldCoordinateSystem` is moved to (100, 0, 0) and turned 90 degrees about
/// z; the wall's own placement is (1, 2, 0). `#125` has no representation.
const FILE: &str = "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#2=IFCUNITASSIGNMENT((#1));
#10=IFCCARTESIANPOINT((0.,0.));
#11=IFCCARTESIANPOINT((4.,0.));
#12=IFCCARTESIANPOINT((4.,3.));
#13=IFCCARTESIANPOINT((0.,3.));
#14=IFCCARTESIANPOINT((1.,1.));
#15=IFCCARTESIANPOINT((2.,1.));
#16=IFCCARTESIANPOINT((2.,2.));
#17=IFCCARTESIANPOINT((1.,2.));
#20=IFCPOLYLINE((#10,#11,#12,#13,#10));
#21=IFCPOLYLINE((#14,#15,#16,#17,#14));
#30=IFCCARTESIANPOINT((0.,0.,0.));
#31=IFCDIRECTION((0.,0.,1.));
#32=IFCDIRECTION((1.,0.,0.));
#33=IFCAXIS2PLACEMENT3D(#30,#31,#32);
#34=IFCPLANE(#33);
#35=IFCCARTESIANPOINT((10.,0.,0.));
#36=IFCDIRECTION((1.,0.,0.));
#37=IFCDIRECTION((0.,1.,0.));
#38=IFCAXIS2PLACEMENT3D(#35,#36,#37);
#39=IFCPLANE(#38);
#40=IFCCURVEBOUNDEDPLANE(#34,#20,(#21));
#41=IFCCURVEBOUNDEDPLANE(#39,#20,(#21));
#100=IFCPROJECT('0YvctVUKr0kugbFTf53O9A',$,'P',$,$,$,$,(#101),#2);
#101=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-5,#104,$);
#102=IFCCARTESIANPOINT((100.,0.,0.));
#103=IFCDIRECTION((0.,1.,0.));
#104=IFCAXIS2PLACEMENT3D(#102,#31,#103);
#105=IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#101,$,.MODEL_VIEW.,$);
#110=IFCCARTESIANPOINT((1.,2.,0.));
#111=IFCAXIS2PLACEMENT3D(#110,$,$);
#112=IFCLOCALPLACEMENT($,#111);
#120=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,2.,0.5);
#121=IFCEXTRUDEDAREASOLID(#120,$,#31,3.);
#122=IFCSHAPEREPRESENTATION(#105,'Body','SweptSolid',(#121));
#123=IFCPRODUCTDEFINITIONSHAPE($,$,(#122));
#124=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,$,$,$,#112,#123,$,$);
#125=IFCWALL('2YvctVUKr0kugbFTf53O9L',$,$,$,$,#112,$,$,$);
ENDSEC;
END-ISO-10303-21;
";

fn model() -> Model {
    StepCodec
        .read_bytes(FILE.as_bytes())
        .expect("fixture parses")
}

fn close(a: [f64; 3], b: [f64; 3]) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 1e-9)
}

/// A frame that rotates about a tilted axis and translates, so no boundary
/// point is left where it was.
#[cfg(feature = "lowering")]
fn tilted_frame() -> Transform {
    Transform::from_axes(
        [5.0, 7.0, 2.0],
        Some([0.0, 1.0, 1.0]),
        Some([1.0, 0.0, 0.0]),
    )
    .expect("valid axes")
}

/// The context frame composes ABOVE the product placement: the wall's
/// (1, 2, 0) is turned 90 degrees about z and moved to (100, 0, 0).
#[test]
fn the_body_frame_composes_the_context_above_the_placement() {
    let model = model();
    let scale = units::resolve(&model);
    let frame =
        product_representation_frame(&model, &scale, EntityId(124), RepresentationPurpose::Body)
            .expect("resolves")
            .expect("the wall has a body");
    assert!(
        close(frame.apply([0.0, 0.0, 0.0]), [98.0, 1.0, 0.0]),
        "{frame:?}"
    );
    assert!(
        close(frame.apply_direction([1.0, 0.0, 0.0]), [0.0, 1.0, 0.0]),
        "{frame:?}"
    );
}

#[test]
fn a_product_without_that_representation_has_no_frame() {
    let model = model();
    let scale = units::resolve(&model);
    let frame =
        product_representation_frame(&model, &scale, EntityId(125), RepresentationPurpose::Body)
            .expect("resolves");
    assert_eq!(frame, None);
}

#[cfg(feature = "lowering")]
mod lowering {
    use super::*;
    use axiolid_model::{GeometryNode, SurfaceRelation};
    use ifc_geometry::lower::{
        lower_product_items, lower_representation_item, lower_surface_node, LoweredGeometry,
        LoweringSession,
    };
    use ifc_geometry::GeometryResult;

    pub(super) fn lowered(
        model: &Model,
        lower: impl FnOnce(&mut LoweringSession<'_>) -> GeometryResult<axiolid_model::NodeId>,
    ) -> LoweredGeometry {
        let scale = units::resolve(model);
        let mut session = LoweringSession::new(model, &scale);
        let root = lower(&mut session).expect("lowers");
        session.finish(root).expect("finishes")
    }

    fn curve_bounded(
        lowered: &LoweredGeometry,
    ) -> (axiolid_model::NodeId, Vec<axiolid_model::NodeId>) {
        match lowered.graph.get(lowered.root) {
            Some(GeometryNode::SurfaceRelation(SurfaceRelation::CurveBounded {
                basis,
                boundaries,
                ..
            })) => (*basis, boundaries.clone()),
            other => panic!("expected a curve-bounded surface, got {other:?}"),
        }
    }

    /// The frame moves the basis plane and leaves the boundaries in its
    /// parameter space (#163).
    #[test]
    fn a_frame_moves_the_plane_and_not_its_boundaries() {
        let model = model();
        let at_identity = lowered(&model, |s| {
            lower_surface_node(s, EntityId(40), Transform::identity())
        });
        let framed = lowered(&model, |s| {
            lower_surface_node(s, EntityId(40), tilted_frame())
        });
        let (basis_identity, boundaries_identity) = curve_bounded(&at_identity);
        let (basis_framed, boundaries_framed) = curve_bounded(&framed);
        assert_ne!(
            at_identity.graph.get(basis_identity),
            framed.graph.get(basis_framed),
            "the frame reaches the basis plane"
        );
        assert_eq!(boundaries_identity.len(), 2, "outer and one hole");
        for (a, b) in boundaries_identity.iter().zip(&boundaries_framed) {
            assert_eq!(
                at_identity.graph.get(*a),
                framed.graph.get(*b),
                "a boundary is in the plane's parameters, whatever the frame"
            );
        }
    }

    /// Lowering a body's item under the public frame reproduces the body
    /// path exactly (#164). Both run in one session, whose memo is keyed by
    /// the frame's exact bits: the item is the body's own node, and lowering
    /// it again appends nothing.
    #[test]
    fn the_public_frame_reproduces_body_lowering() {
        let model = model();
        let scale = units::resolve(&model);
        let frame = product_representation_frame(
            &model,
            &scale,
            EntityId(124),
            RepresentationPurpose::Body,
        )
        .expect("resolves")
        .expect("has a body");
        let mut session = LoweringSession::new(&model, &scale);
        let body = lower_product_items(&mut session, EntityId(124))
            .expect("lowers")
            .expect("has a body");
        let nodes = session.node_count();
        let item = lower_representation_item(&mut session, EntityId(121), frame).expect("lowers");
        assert_eq!(item, body, "the same node, so the same frame");
        assert_eq!(session.node_count(), nodes, "a memo hit appends nothing");
    }
}

#[cfg(feature = "compile-reference-backend")]
mod meshing {
    //! The acceptance criterion of #163: under any frame the plane meshes to
    //! the identity result moved by that frame.

    use super::lowering::lowered;
    use super::*;
    use axiolid_contracts::ExecutionOptions;
    use axiolid_core::{Tolerance, Vec3};
    use axiolid_mesh::TriMesh;
    use axiolid_mesh_compile_contract::{MeshClosure, MeshCompiler};
    use ifc_geometry::compile::default_backend;
    use ifc_geometry::lower::lower_surface_node;

    fn mesh(id: u64, frame: Transform) -> TriMesh {
        let model = model();
        let lowered = lowered(&model, |s| lower_surface_node(s, EntityId(id), frame));
        let outcome = default_backend()
            .compile_mesh_reported(
                &lowered.graph,
                lowered.root,
                &ExecutionOptions::new(Tolerance::MILLIMETRE),
            )
            .unwrap_or_else(|e| panic!("#{id} compiles: {e:?}"));
        assert_eq!(outcome.closure, MeshClosure::Surface);
        outcome.mesh
    }

    fn area(mesh: &TriMesh) -> f64 {
        mesh.indices
            .chunks_exact(3)
            .map(|t| {
                let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize]);
                (b - a).cross(c - a) * 0.5
            })
            .fold(Vec3::ZERO, |sum, v| sum + v)
            .length()
    }

    #[test]
    fn a_framed_plane_meshes_to_the_moved_identity_mesh() {
        let frame = tilted_frame();
        let at_identity = mesh(40, Transform::identity());
        let framed = mesh(40, frame);
        assert!((area(&at_identity) - 11.0).abs() < 1e-9);
        assert!((area(&framed) - 11.0).abs() < 1e-9, "12 m2 less the hole");
        assert_eq!(at_identity.positions.len(), framed.positions.len());
        assert_eq!(at_identity.indices, framed.indices);
        for (p, q) in at_identity.positions.iter().zip(&framed.positions) {
            let moved = frame.apply([p.x, p.y, p.z]);
            assert!(close(moved, [q.x, q.y, q.z]), "{moved:?} vs {q:?}");
        }
    }

    /// The plane's own `Position` carries the boundaries: `(u, v)` lands at
    /// `(10, u, v)`.
    #[test]
    fn the_plane_position_places_its_parameter_space() {
        let placed = mesh(41, Transform::identity());
        assert!((area(&placed) - 11.0).abs() < 1e-9);
        for p in &placed.positions {
            assert!((p.x - 10.0).abs() < 1e-9, "{p:?}");
            assert!((-1e-9..=4.0 + 1e-9).contains(&p.y), "{p:?}");
            assert!((-1e-9..=3.0 + 1e-9).contains(&p.z), "{p:?}");
        }
    }
}
