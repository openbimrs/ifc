//! Face surfaces as standalone items, and connection-surface lowering (#155).

#![cfg(feature = "lowering")]
//! Requires the `lowering` feature: this suite exercises the neutral DAG.
//!
//! `IfcFaceSurface` is a member of `IfcSurfaceOrFaceSurface`, the type of
//! `IfcConnectionSurfaceGeometry.SurfaceOnRelatingElement`, so a space
//! boundary may state its surface as a bounded face rather than a bare
//! surface. These tests pin that such a face lowers as ONE open face -- its
//! carrier surface, `SameSense` and bound orientation intact -- and that the
//! connection helper routes every select member and refuses every
//! non-surface connection kind with a typed error.

use axiolid_model::{GeometryNode, NodeId};
use axiolid_surface::Surface;
use axiolid_topology::{BRep, Orientation};
use ifc_geometry::lower::{
    lower_connection_surface, lower_face_surface_node, lower_related_connection_surface,
    lower_representation_item, DegenerateFacePolicy, LoweredGeometry, LoweringSession,
};
use ifc_geometry::transform::Transform;
use ifc_geometry::{units, GeometryError, GeometryResult};
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;
use std::path::PathBuf;

/// A 2 m x 1 m rectangle on the XY plane, as face surfaces and connections.
///
/// `#30` and `#31` differ ONLY in `SameSense`: same loop, same bound
/// orientation, same plane. `#32` reverses the bound instead.
const FILE: &str = "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#2=IFCUNITASSIGNMENT((#1));
#10=IFCCARTESIANPOINT((0.,0.,0.));
#11=IFCCARTESIANPOINT((2.,0.,0.));
#12=IFCCARTESIANPOINT((2.,1.,0.));
#13=IFCCARTESIANPOINT((0.,1.,0.));
#14=IFCDIRECTION((0.,0.,1.));
#15=IFCDIRECTION((1.,0.,0.));
#16=IFCAXIS2PLACEMENT3D(#10,#14,#15);
#17=IFCPLANE(#16);
#20=IFCPOLYLOOP((#10,#11,#12,#13));
#21=IFCFACEOUTERBOUND(#20,.T.);
#22=IFCFACEOUTERBOUND(#20,.F.);
#30=IFCFACESURFACE((#21),#17,.T.);
#31=IFCFACESURFACE((#21),#17,.F.);
#32=IFCFACESURFACE((#22),#17,.T.);
#33=IFCFACE((#21));
#34=IFCFACESURFACE((#21),#99,.T.);
#35=IFCFACESURFACE((#21),#10,.T.);
#40=IFCOPENSHELL((#33));
#41=IFCFACEBASEDSURFACEMODEL((#40));
#50=IFCCONNECTIONSURFACEGEOMETRY(#17,$);
#51=IFCCONNECTIONSURFACEGEOMETRY(#30,#31);
#52=IFCCONNECTIONSURFACEGEOMETRY(#41,$);
#53=IFCCONNECTIONPOINTGEOMETRY(#10,$);
#54=IFCCONNECTIONPOINTECCENTRICITY(#10,$,1.,$,$);
#55=IFCCONNECTIONCURVEGEOMETRY(#60,$);
#56=IFCCONNECTIONVOLUMEGEOMETRY(#61,$);
#57=IFCCONNECTIONPORTGEOMETRY(#16,$,$);
#58=IFCCONNECTIONSURFACEGEOMETRY(#999,$);
#59=IFCCONNECTIONSURFACEGEOMETRY(#10,#998);
#62=IFCCONNECTIONSURFACEGEOMETRY($,$);
#63=IFCCONNECTIONSURFACEGEOMETRY(#17,'not a reference');
#64=IFCCONNECTIONSURFACEGEOMETRY(#34,$);
#60=IFCPOLYLINE((#10,#11));
#61=IFCBLOCK(#16,1.,1.,1.);
#70=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,$,$,$,$,$,$,$);
#80=IFCPOLYLOOP((#10,#11,#11,#10));
#81=IFCFACEOUTERBOUND(#80,.T.);
#82=IFCFACESURFACE((#81),#17,.T.);
ENDSEC;
END-ISO-10303-21;
";

fn model() -> Model {
    StepCodec
        .read_bytes(FILE.as_bytes())
        .expect("fixture parses")
}

/// Run one lowering in a fresh session and finish it.
fn lowered(
    model: &Model,
    lower: impl FnOnce(&mut LoweringSession<'_>) -> GeometryResult<NodeId>,
) -> GeometryResult<LoweredGeometry> {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let root = lower(&mut session)?;
    session.finish(root)
}

fn brep_of(lowered: &LoweredGeometry) -> BRep<NodeId> {
    match lowered.graph.get(lowered.root).expect("root exists") {
        GeometryNode::BRep(brep) => brep.clone(),
        other => panic!("expected a single-face BRep, got {other:?}"),
    }
}

fn face_surface(model: &Model, id: u64) -> LoweredGeometry {
    lowered(model, |s| {
        lower_face_surface_node(s, EntityId(id), Transform::identity())
    })
    .unwrap_or_else(|e| panic!("#{id} lowers: {e}"))
}

/// One face, one OPEN shell, no solid, and the plane kept as its carrier.
#[test]
fn a_planar_face_surface_lowers_as_one_open_face_on_its_plane() {
    let model = model();
    let lowered = face_surface(&model, 30);
    let brep = brep_of(&lowered);
    assert_eq!(brep.faces().len(), 1, "exactly the one authored face");
    assert_eq!(brep.shells().len(), 1, "one shell holds it");
    assert!(
        !brep.shells()[0].closed,
        "a single face never closes a shell"
    );
    assert!(brep.solids().is_empty(), "a face bounds no volume");
    assert_eq!(brep.vertices().len(), 4, "the loop's four points");
    assert_eq!(brep.edges().len(), 4, "the implied closing edge is present");
    let face = &brep.faces()[0];
    assert_eq!(face.orientation, Orientation::Forward);
    assert_eq!(face.bounds.len(), 1);
    assert_eq!(face.bounds[0].orientation, Orientation::Forward);
    let surface = face.surface.expect("a face surface keeps its carrier");
    assert!(
        matches!(
            lowered.graph.get(surface),
            Some(GeometryNode::Surface(Surface::Plane(_)))
        ),
        "the carrier is the authored IfcPlane"
    );
}

/// `SameSense=.F.` reverses the face against its CARRIER and nothing else.
///
/// IFC defines the face normal by its bounds alone (IFC4 ADD2 TC1, `IfcFace`
/// note: `SameSense` "ha[s] no effect on the orientation of the face");
/// `SameSense` relates the carrier's normal to that face normal. The neutral
/// `Face::orientation` is documented as orientation relative to the support
/// surface normal, so the flag lands there, and the loop stays as authored.
#[test]
fn a_false_same_sense_reverses_the_face() {
    let model = model();
    let brep = brep_of(&face_surface(&model, 31));
    assert_eq!(brep.faces()[0].orientation, Orientation::Reversed);
    assert_eq!(
        brep.faces()[0].bounds[0].orientation,
        Orientation::Forward,
        "SameSense must not leak into the bound's own flag"
    );
}

/// A bound's `Orientation=.F.` reverses the loop use and not the face.
#[test]
fn a_false_bound_orientation_reverses_the_bound_only() {
    let model = model();
    let brep = brep_of(&face_surface(&model, 32));
    assert_eq!(brep.faces()[0].orientation, Orientation::Forward);
    assert_eq!(brep.faces()[0].bounds[0].orientation, Orientation::Reversed);
}

/// The total dispatcher reaches the same path: a face surface is an item.
#[test]
fn the_dispatcher_lowers_a_face_surface_item() {
    let model = model();
    let lowered = lowered(&model, |s| {
        lower_representation_item(s, EntityId(31), Transform::identity())
    })
    .expect("a face surface is an implemented representation item");
    assert_eq!(
        brep_of(&lowered).faces()[0].orientation,
        Orientation::Reversed
    );
}

/// `lower_face_surface_node` takes face surfaces only.
///
/// A plain `IfcFace` has no carrier and is not in `IfcSurfaceOrFaceSurface`;
/// accepting it here would make the helper's name a lie.
#[test]
fn a_plain_face_is_the_wrong_type() {
    let model = model();
    let error = lowered(&model, |s| {
        lower_face_surface_node(s, EntityId(33), Transform::identity())
    })
    .expect_err("an IfcFace is not an IfcFaceSurface");
    assert!(
        matches!(
            &error,
            GeometryError::WrongEntityType { entity, expected, .. }
                if *entity == EntityId(33) && *expected == "IfcFaceSurface"
        ),
        "{error}"
    );
}

/// A dangling or wrong-type carrier is a structural error, never a flat face.
#[test]
fn a_dangling_or_wrong_type_carrier_is_refused() {
    let model = model();
    let dangling = lowered(&model, |s| {
        lower_face_surface_node(s, EntityId(34), Transform::identity())
    })
    .expect_err("#99 does not exist");
    assert!(
        matches!(
            dangling,
            GeometryError::MissingEntity { referrer, missing }
                if referrer == EntityId(34) && missing == EntityId(99)
        ),
        "{dangling}"
    );
    let wrong = lowered(&model, |s| {
        lower_face_surface_node(s, EntityId(35), Transform::identity())
    })
    .expect_err("a point is not a surface");
    assert!(
        !wrong.is_unsupported(),
        "a broken file is not unsupported IFC"
    );
    assert_eq!(wrong.entity(), Some(EntityId(10)), "{wrong}");
}

/// A curved `IfcAdvancedFace` lowers exactly: its cylinder, not a polygon.
#[test]
fn an_advanced_face_on_a_cylinder_lowers_with_its_curved_carrier() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-surfaces/synthetic_advanced_brep.ifc");
    let model = StepCodec.read_path(&path).expect("fixture parses");
    // `#48=IFCADVANCEDFACE((#47),#15,.T.)` with `#15=IFCCYLINDRICALSURFACE(#14,120.)`
    // in millimetres.
    assert_eq!(
        model
            .get(EntityId(48))
            .map(|e| e.type_name.to_ascii_uppercase()),
        Some("IFCADVANCEDFACE".to_string())
    );
    let lowered = lowered(&model, |s| {
        lower_representation_item(s, EntityId(48), Transform::identity())
    })
    .expect("the lateral advanced face lowers on its own");
    let brep = brep_of(&lowered);
    assert_eq!(brep.faces().len(), 1);
    assert!(!brep.shells()[0].closed);
    assert!(brep.solids().is_empty());
    let face = &brep.faces()[0];
    assert_eq!(face.orientation, Orientation::Forward);
    let Some(GeometryNode::Surface(Surface::Cylinder(cylinder))) =
        lowered.graph.get(face.surface.expect("carrier kept"))
    else {
        panic!("the carrier must stay an exact cylinder");
    };
    assert!(
        (cylinder.radius - 0.12).abs() < 1e-12,
        "120 mm radius in metres"
    );
    assert!(
        brep.edges().iter().any(|edge| edge.curve.is_some()),
        "the edge curves are kept exact"
    );

    // The planar cap authored `SameSense=.F.` reverses on its own too.
    let cap = lowered_cap(&model);
    assert_eq!(brep_of(&cap).faces()[0].orientation, Orientation::Reversed);
}

fn lowered_cap(model: &Model) -> LoweredGeometry {
    lowered(model, |s| {
        lower_face_surface_node(s, EntityId(59), Transform::identity())
    })
    .expect("the reversed advanced cap lowers")
}

/// Each `IfcSurfaceOrFaceSurface` member routes to its own lowerer.
#[test]
fn the_connection_helper_routes_every_select_member() {
    let model = model();
    let connection = |id: u64| {
        lowered(&model, |s| {
            lower_connection_surface(s, EntityId(id), Transform::identity())
        })
    };

    let surface = connection(50).expect("a bare IfcSurface");
    assert!(matches!(
        surface.graph.get(surface.root),
        Some(GeometryNode::Surface(Surface::Plane(_)))
    ));

    let face = connection(51).expect("an IfcFaceSurface");
    assert_eq!(brep_of(&face).faces()[0].orientation, Orientation::Forward);

    let model_of_faces = connection(52).expect("an IfcFaceBasedSurfaceModel");
    let Some(GeometryNode::Collection(members)) = model_of_faces.graph.get(model_of_faces.root)
    else {
        panic!("a face-based surface model stays a collection of shells");
    };
    assert_eq!(members.len(), 1);
}

/// The related end is optional and lowers in the caller's second frame.
#[test]
fn the_related_end_is_optional_and_independent() {
    let model = model();
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale);
    let related =
        lower_related_connection_surface(&mut session, EntityId(51), Transform::identity())
            .expect("lowers")
            .expect("#51 states a related surface");
    let lowered = session.finish(related).expect("finishes");
    assert_eq!(
        brep_of(&lowered).faces()[0].orientation,
        Orientation::Reversed,
        "the related end is #31, not a copy of the relating #30"
    );

    let mut session = LoweringSession::new(&model, &scale);
    assert_eq!(
        lower_related_connection_surface(&mut session, EntityId(50), Transform::identity())
            .expect("an omitted end is not an error"),
        None
    );
    let error = lower_related_connection_surface(&mut session, EntityId(63), Transform::identity())
        .expect_err("a string is not a surface reference");
    assert!(
        matches!(error, GeometryError::WrongValueKind { .. }),
        "{error}"
    );
}

/// Point, curve, volume and port connections are valid IFC with no surface.
#[test]
fn every_non_surface_connection_kind_is_a_typed_refusal() {
    let model = model();
    for (id, type_name) in [
        (53, "IFCCONNECTIONPOINTGEOMETRY"),
        (54, "IFCCONNECTIONPOINTECCENTRICITY"),
        (55, "IFCCONNECTIONCURVEGEOMETRY"),
        (56, "IFCCONNECTIONVOLUMEGEOMETRY"),
        (57, "IFCCONNECTIONPORTGEOMETRY"),
    ] {
        for related in [false, true] {
            let scale = units::resolve(&model);
            let mut session = LoweringSession::new(&model, &scale);
            let error = if related {
                lower_related_connection_surface(&mut session, EntityId(id), Transform::identity())
                    .map(|_| ())
            } else {
                lower_connection_surface(&mut session, EntityId(id), Transform::identity())
                    .map(|_| ())
            }
            .expect_err("no surface to lower");
            assert!(
                matches!(
                    &error,
                    GeometryError::Unsupported { entity, type_name: t, .. }
                        if *entity == EntityId(id) && t == type_name
                ),
                "#{id} (related={related}): {error}"
            );
        }
    }
}

/// Structural faults stay structural: missing, dangling, and wrong-type.
#[test]
fn broken_connection_references_are_structural_errors() {
    let model = model();
    let connection = |id: u64| {
        lowered(&model, |s| {
            lower_connection_surface(s, EntityId(id), Transform::identity())
        })
    };

    let dangling = connection(58).expect_err("#999 does not exist");
    assert!(
        matches!(
            dangling,
            GeometryError::MissingEntity { referrer, missing }
                if referrer == EntityId(58) && missing == EntityId(999)
        ),
        "{dangling}"
    );

    let wrong = connection(59).expect_err("a point is no IfcSurfaceOrFaceSurface");
    assert!(
        matches!(
            &wrong,
            GeometryError::WrongEntityType { entity, expected, .. }
                if *entity == EntityId(10) && *expected == "IfcSurfaceOrFaceSurface"
        ),
        "{wrong}"
    );

    let absent = connection(62).expect_err("the relating surface is required");
    assert!(
        matches!(
            absent,
            GeometryError::MissingAttribute { entity, attribute, .. }
                if entity == EntityId(62) && attribute == "SurfaceOnRelatingElement"
        ),
        "{absent}"
    );

    let not_a_connection = connection(70).expect_err("a wall is not a connection");
    assert!(
        matches!(
            &not_a_connection,
            GeometryError::WrongEntityType { entity, expected, .. }
                if *entity == EntityId(70) && *expected == "IfcConnectionSurfaceGeometry"
        ),
        "{not_a_connection}"
    );

    let missing = connection(12345).expect_err("no such connection");
    assert!(
        matches!(missing, GeometryError::MissingEntity { missing, .. } if missing == EntityId(12345)),
        "{missing}"
    );

    // A face surface's own dangling carrier surfaces through the helper.
    let nested = connection(64).expect_err("#34 names a missing #99");
    assert!(
        matches!(
            nested,
            GeometryError::MissingEntity { referrer, missing }
                if referrer == EntityId(34) && missing == EntityId(99)
        ),
        "{nested}"
    );
}

/// A face whose only loop collapses is refused under both face policies.
///
/// The default names the collapsed loop. `DropAndReport` drops the face, and
/// a single-face result with its face dropped is nothing, so it is refused
/// naming the face rather than returned as an empty shell.
#[test]
fn a_collapsed_face_surface_is_degenerate_under_both_policies() {
    let model = model();
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale);
    let error = lower_face_surface_node(&mut session, EntityId(82), Transform::identity())
        .expect_err("two distinct edges bound no area");
    assert!(
        matches!(error, GeometryError::Degenerate { entity, .. } if entity == EntityId(80)),
        "{error}"
    );

    let mut session =
        LoweringSession::new(&model, &scale).with_face_policy(DegenerateFacePolicy::DropAndReport);
    let error = lower_face_surface_node(&mut session, EntityId(82), Transform::identity())
        .expect_err("dropping the only face leaves nothing");
    assert!(
        matches!(
            &error,
            GeometryError::Degenerate { entity, type_name, .. }
                if *entity == EntityId(82) && type_name == "IFCFACESURFACE"
        ),
        "{error}"
    );
}

/// The committed space-boundary fixture: two boundaries of one space.
const SPACE_FIXTURE: &str =
    "../../test/fixtures/synthetic-surfaces/synthetic_space_boundary_face_surface.ifc";

/// `#13` is the space; `#37` (west wall, `SameSense=.T.`) and `#46` (floor,
/// `SameSense=.F.`) are the connection geometries of its two boundaries.
fn space_fixture() -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SPACE_FIXTURE);
    StepCodec.read_path(&path).expect("fixture parses")
}

/// `IfcRelSpaceBoundary.ConnectionGeometry`: after the four `IfcRoot`
/// attributes, `RelatingSpace` and `RelatedBuildingElement` (IFC4 ADD2 TC1).
const CONNECTION_GEOMETRY: usize = 6;

/// Every committed space boundary's face surface lowers through the helper.
#[test]
fn committed_space_boundary_face_surfaces_lower() {
    let model = space_fixture();
    let mut senses = Vec::new();
    for &boundary in model.ids_of_type("IFCRELSPACEBOUNDARY") {
        let connection = model
            .get(boundary)
            .and_then(|b| b.attribute(CONNECTION_GEOMETRY))
            .and_then(|v| v.as_ref_id())
            .expect("each boundary states its connection geometry");
        let lowered = lowered(&model, |s| {
            lower_connection_surface(s, connection, Transform::identity())
        })
        .unwrap_or_else(|e| panic!("boundary {boundary}: {e}"));
        let brep = brep_of(&lowered);
        assert_eq!(brep.faces().len(), 1);
        assert!(!brep.shells()[0].closed);
        senses.push((connection, brep.faces()[0].orientation));
    }
    senses.sort_by_key(|(id, _)| *id);
    assert_eq!(
        senses,
        [
            (EntityId(37), Orientation::Forward),
            (EntityId(46), Orientation::Reversed)
        ]
    );
}

#[cfg(feature = "compile-reference-backend")]
mod meshing {
    //! The acceptance criterion: a planar polyloop face surface MESHES, and
    //! the backend reports it as a surface rather than a solid.

    use super::*;
    use axiolid_contracts::ExecutionOptions;
    use axiolid_core::{Tolerance, Vec3};
    use axiolid_mesh::TriMesh;
    use axiolid_mesh_compile_contract::{MeshClosure, MeshCompiler};
    use ifc_geometry::compile::default_backend;

    fn compile(lowered: &LoweredGeometry) -> (TriMesh, MeshClosure) {
        let outcome = default_backend()
            .compile_mesh_reported(
                &lowered.graph,
                lowered.root,
                &ExecutionOptions::new(Tolerance::MILLIMETRE),
            )
            .unwrap_or_else(|e| panic!("the face compiles: {e:?}"));
        (outcome.mesh, outcome.closure)
    }

    /// Sum of triangle area vectors: its length is the area, its sign the side.
    fn area_vector(mesh: &TriMesh) -> Vec3 {
        mesh.indices
            .chunks_exact(3)
            .map(|t| {
                let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize]);
                (b - a).cross(c - a) * 0.5
            })
            .fold(Vec3::ZERO, |sum, v| sum + v)
    }

    #[test]
    fn a_planar_polyloop_face_surface_meshes_as_an_open_surface() {
        let model = model();
        let (mesh, closure) = compile(&face_surface(&model, 30));
        assert_eq!(closure, MeshClosure::Surface, "one face is not a solid");
        let area = area_vector(&mesh);
        assert!((area.length() - 2.0).abs() < 1e-12, "2 m x 1 m: {area:?}");
        assert!(area.z > 0.0, "SameSense=.T. faces +Z: {area:?}");
    }

    /// A `SameSense=.F.` face meshes with its full area, on its plane.
    ///
    /// Its meshed SIDE is deliberately not pinned. IFC takes the face normal
    /// from the bounds alone, while the reference compiler's planar path
    /// starts from the loop winding and then applies `Face::orientation` again,
    /// so for `SameSense=.F.` today's side is the kernel's reading, not the
    /// file's. The lowering half is pinned above, where it is ours.
    #[test]
    fn a_false_same_sense_face_meshes_with_its_full_area() {
        let model = model();
        let (mesh, closure) = compile(&face_surface(&model, 31));
        assert_eq!(closure, MeshClosure::Surface);
        let area = area_vector(&mesh);
        assert!((area.length() - 2.0).abs() < 1e-12, "{area:?}");
        assert!(
            area.x.abs() + area.y.abs() < 1e-12,
            "stays on z=0: {area:?}"
        );
    }

    /// The committed boundaries mesh in the SPACE's frame, in metres.
    ///
    /// The space sits at (1000, 2000, 0) mm, so a helper that ignored the
    /// caller's frame or the millimetre unit would put the faces elsewhere.
    /// The wall (`SameSense=.T.`) pins its side; the floor pins its plane
    /// only, for the reason given on the test above.
    #[test]
    fn committed_space_boundaries_mesh_in_the_space_frame() {
        let model = space_fixture();
        let scale = units::resolve(&model);
        let frame = ifc_geometry::lower::product_world_transform(&model, &scale, EntityId(13))
            .expect("the space is placed");
        for (connection, m2, normal) in [
            (37, 7.5, Vec3::new(1.0, 0.0, 0.0)),
            (46, 12.0, Vec3::new(0.0, 0.0, 1.0)),
        ] {
            let lowered = lowered(&model, |s| {
                lower_connection_surface(s, EntityId(connection), frame)
            })
            .expect("lowers");
            let (mesh, closure) = compile(&lowered);
            assert_eq!(closure, MeshClosure::Surface, "#{connection}");
            let area = area_vector(&mesh);
            assert!((area.length() - m2).abs() < 1e-9, "#{connection}: {area:?}");
            let unit = area / area.length();
            let side = if connection == 37 {
                (unit - normal).length()
            } else {
                1.0 - unit.dot(normal).abs()
            };
            assert!(side < 1e-9, "#{connection} faces {normal:?}: {area:?}");
            let min_x = mesh.positions.iter().map(|p| p.x).fold(f64::MAX, f64::min);
            let min_y = mesh.positions.iter().map(|p| p.y).fold(f64::MAX, f64::min);
            assert!((min_x - 1.0).abs() < 1e-9 && (min_y - 2.0).abs() < 1e-9);
        }
    }

    #[test]
    fn a_connection_surface_given_as_a_face_meshes() {
        let model = model();
        let lowered = lowered(&model, |s| {
            lower_connection_surface(s, EntityId(51), Transform::identity())
        })
        .expect("lowers");
        let (mesh, closure) = compile(&lowered);
        assert_eq!(closure, MeshClosure::Surface);
        assert!((area_vector(&mesh).length() - 2.0).abs() < 1e-12);
    }
}
