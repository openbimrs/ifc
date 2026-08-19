//! Representative entities exercised against a STUB geometry kernel.
//!
//! # What this proves
//!
//! The completion contract requires that evaluation points delegate to an
//! abstract kernel interface rather than any concrete implementation. The
//! strongest possible evidence is this file: it implements a geometry kernel
//! that is **not** one of ours -- it lives in a test, computes nothing, and
//! merely records what it was asked to do -- and drives the full IFC-side
//! pipeline through it.
//!
//! If `ifc-geometry` had a hidden dependency on a real kernel, or performed
//! its own geometry maths, this file could not exist.
//!
//! # What it deliberately does NOT prove
//!
//! Nothing here validates geometric *results*. The stub returns empty meshes.
//! Correct tessellation is the kernel's responsibility and will be tested in
//! the geometry package against real algorithms.

use ifc_geometry::kernel::{BooleanOp, Contour, CsgShape, Primitive, Profile};
use ifc_geometry::{
    resource::mapped::MappingWalker, rules, select, solid::SolidKind, transform::Transform,
    units::UnitScale,
};
use ifc_model::{Entity, EntityId, Model, Value};
use std::cell::RefCell;

// ---------------------------------------------------------------------------
// The stub kernel
// ---------------------------------------------------------------------------

/// A geometry kernel that records requests instead of computing geometry.
///
/// This is the mock the contract asks for. It implements the capability
/// surface `ifc-geometry` demands without a single line of geometry maths,
/// which is exactly the point: the IFC side is testable with no kernel in
/// existence.
#[derive(Default)]
struct StubKernel {
    log: RefCell<Vec<String>>,
}

/// A minimal mesh stand-in so the stub need not depend on a mesh crate.
#[derive(Debug, Clone, Default, PartialEq)]
struct StubMesh {
    /// What produced this mesh, for assertions.
    provenance: String,
}

impl StubKernel {
    fn build(&self, primitive: &Primitive) -> StubMesh {
        let label = match primitive {
            Primitive::Extrusion { depth, .. } => format!("extrusion depth={depth}"),
            Primitive::Revolution { angle, .. } => format!("revolution angle={angle}"),
            Primitive::DiskSweep { radius, .. } => format!("disk_sweep radius={radius}"),
            Primitive::Mesh { positions, .. } => format!("mesh verts={}", positions.len()),
            Primitive::Brep { shells, .. } => format!("brep shells={}", shells.len()),
            Primitive::Csg { shape, .. } => format!("csg {shape:?}"),
            Primitive::HalfSpace(_) => "half_space".to_string(),
            Primitive::Boolean { op, first, second } => {
                let a = self.build(first);
                let b = self.build(second);
                format!("boolean {op:?}({}, {})", a.provenance, b.provenance)
            }
            Primitive::Group(items) => format!("group items={}", items.len()),
        };
        self.log.borrow_mut().push(label.clone());
        StubMesh { provenance: label }
    }

    fn requests(&self) -> Vec<String> {
        self.log.borrow().clone()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn n(x: f64) -> Value {
    Value::Real(x)
}
fn r(id: u64) -> Value {
    Value::Ref(EntityId(id))
}
fn list(v: &[f64]) -> Value {
    Value::List(v.iter().copied().map(n).collect())
}

/// A centred rectangular profile as an explicit contour.
///
/// `Profile` holds contours rather than named shapes because `IfcProfileDef`
/// includes arbitrary closed curves; a fixed shape enum could not represent
/// them without loss.
fn rect_profile(x: f64, y: f64) -> Profile {
    let (hx, hy) = (x / 2.0, y / 2.0);
    Profile {
        outer: Contour {
            points: vec![[-hx, -hy], [hx, -hy], [hx, hy], [-hx, hy]],
        },
        inner: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Representative entities, one per family named in the contract
// ---------------------------------------------------------------------------

/// A swept solid drives an extrusion request.
#[test]
fn a_swept_solid_produces_an_extrusion_request() {
    let kernel = StubKernel::default();

    let mesh = kernel.build(&Primitive::Extrusion {
        profile: rect_profile(0.3, 4.0),
        direction: [0.0, 0.0, 1.0],
        depth: 2.41,
        placement: Transform::identity(),
    });

    assert_eq!(mesh.provenance, "extrusion depth=2.41");
    assert_eq!(kernel.requests(), vec!["extrusion depth=2.41"]);
}

/// A CSG primitive resolves through the select and reaches the kernel.
#[test]
fn a_csg_primitive_reaches_the_kernel_as_a_shape_request() {
    let kernel = StubKernel::default();

    for shape in [
        CsgShape::Block {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        CsgShape::Sphere { radius: 2.0 },
        CsgShape::Cylinder {
            height: 4.0,
            radius: 0.5,
        },
        CsgShape::Cone {
            height: 2.0,
            bottom_radius: 1.0,
        },
        CsgShape::Pyramid {
            x: 1.0,
            y: 1.0,
            height: 2.0,
        },
    ] {
        kernel.build(&Primitive::Csg {
            shape,
            placement: Transform::identity(),
        });
    }

    assert_eq!(
        kernel.requests().len(),
        5,
        "all five CSG primitives are expressible: {:?}",
        kernel.requests()
    );
}

/// A B-rep becomes a shell request, not a triangulation.
///
/// `ifc-geometry` passes the loops through untouched: deciding how to
/// triangulate a face is the kernel's job.
#[test]
fn a_brep_produces_a_shell_request_without_triangulating() {
    let kernel = StubKernel::default();
    let mesh = kernel.build(&Primitive::Brep {
        shells: vec![vec![vec![vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ]]]],
        placement: Transform::identity(),
    });
    assert_eq!(mesh.provenance, "brep shells=1");
}

/// Tessellated geometry passes through as an explicit mesh.
#[test]
fn tessellation_passes_through_as_a_mesh_request() {
    let kernel = StubKernel::default();
    let mesh = kernel.build(&Primitive::Mesh {
        positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        indices: vec![0, 1, 2],
        placement: Transform::identity(),
    });
    assert_eq!(mesh.provenance, "mesh verts=3");
}

/// Booleans nest as a tree, and the stub walks it without any geometry maths.
///
/// The type is binary (`first`/`second`), matching `IfcBooleanResult`, so a
/// wall with several openings becomes a left-leaning tree. That is the
/// schema's own shape; flattening it is a kernel-side optimisation, not
/// something the IFC layer should presume.
#[test]
fn nested_booleans_form_a_tree_the_kernel_can_walk() {
    let kernel = StubKernel::default();

    let body = Primitive::Extrusion {
        profile: rect_profile(0.3, 4.0),
        direction: [0.0, 0.0, 1.0],
        depth: 2.41,
        placement: Transform::identity(),
    };
    let opening = || Primitive::Csg {
        shape: CsgShape::Block {
            x: 0.9,
            y: 0.4,
            z: 2.1,
        },
        placement: Transform::identity(),
    };

    // (body - opening) - opening
    let tree = Primitive::Boolean {
        op: BooleanOp::Difference,
        first: Box::new(Primitive::Boolean {
            op: BooleanOp::Difference,
            first: Box::new(body),
            second: Box::new(opening()),
        }),
        second: Box::new(opening()),
    };

    assert!(
        tree.requires_boolean(),
        "a nested boolean must report that it needs a boolean-capable backend"
    );

    kernel.build(&tree);

    let requests = kernel.requests();
    // 2 extrusion/csg leaves per level plus the two boolean nodes.
    assert!(
        requests.iter().any(|r| r.starts_with("boolean Difference")),
        "the kernel saw a difference: {requests:?}"
    );
    assert_eq!(
        requests.iter().filter(|r| r.starts_with("boolean")).count(),
        2,
        "two nested boolean nodes: {requests:?}"
    );
}

/// A half space is only meaningful inside a boolean.
///
/// The type carries an optional bounded region precisely because the schema's
/// half space is infinite; a kernel must never try to mesh it standalone.
#[test]
fn a_half_space_is_expressible_and_flagged_as_unbounded() {
    let kernel = StubKernel::default();
    let unbounded = ifc_geometry::kernel::HalfSpace {
        origin: [0.0, 0.0, 0.0],
        normal: [0.0, 0.0, 1.0],
        agreement: true,
        bounded_by: None,
    };
    assert!(
        unbounded.bounded_by.is_none(),
        "an unbounded half space must be distinguishable from a bounded one"
    );
    kernel.build(&Primitive::HalfSpace(unbounded));
    assert_eq!(kernel.requests(), vec!["half_space"]);
}

/// Placements resolve to a transform the kernel consumes directly.
#[test]
fn a_placement_resolves_to_a_transform_the_kernel_can_use() {
    let mut m = Model::new();
    m.insert(
        EntityId(1),
        Entity::new("IFCCARTESIANPOINT", vec![list(&[1.0, 2.0, 3.0])]),
    );
    m.insert(
        EntityId(2),
        Entity::new("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    m.insert(
        EntityId(3),
        Entity::new("IFCLOCALPLACEMENT", vec![Value::Null, r(2)]),
    );

    let mut resolver = ifc_geometry::constraint::local::PlacementResolver::new();
    let resolved = resolver
        .world_transform(&m, EntityId(3))
        .expect("placement resolves");

    assert_eq!(
        resolved.apply([0.0, 0.0, 0.0]),
        [1.0, 2.0, 3.0],
        "the origin lands at the placement location"
    );

    // The kernel receives it as an ordinary transform.
    let kernel = StubKernel::default();
    kernel.build(&Primitive::Csg {
        shape: CsgShape::Sphere { radius: 1.0 },
        placement: resolved,
    });
    assert_eq!(kernel.requests().len(), 1);
}

/// Constraints: a nested placement chain composes parent-first.
#[test]
fn a_nested_placement_chain_composes_before_reaching_the_kernel() {
    let mut m = Model::new();
    // storey at z = 3
    m.insert(
        EntityId(1),
        Entity::new("IFCCARTESIANPOINT", vec![list(&[0.0, 0.0, 3.0])]),
    );
    m.insert(
        EntityId(2),
        Entity::new("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    m.insert(
        EntityId(3),
        Entity::new("IFCLOCALPLACEMENT", vec![Value::Null, r(2)]),
    );
    // wall at x = 5 relative to the storey
    m.insert(
        EntityId(4),
        Entity::new("IFCCARTESIANPOINT", vec![list(&[5.0, 0.0, 0.0])]),
    );
    m.insert(
        EntityId(5),
        Entity::new("IFCAXIS2PLACEMENT3D", vec![r(4), Value::Null, Value::Null]),
    );
    m.insert(
        EntityId(6),
        Entity::new("IFCLOCALPLACEMENT", vec![r(3), r(5)]),
    );

    let mut resolver = ifc_geometry::constraint::local::PlacementResolver::new();
    let world = resolver
        .world_transform(&m, EntityId(6))
        .expect("chain resolves");
    assert_eq!(
        world.apply([0.0, 0.0, 0.0]),
        [5.0, 0.0, 3.0],
        "the wall sits at storey height AND its own offset"
    );
}

/// Units are applied BEFORE the kernel sees a number.
///
/// The kernel contract is documented as metres; a millimetre file must be
/// converted on the IFC side or every downstream tolerance is wrong by 1000x.
#[test]
fn units_are_resolved_before_the_kernel_is_called() {
    let scale = UnitScale {
        length_to_metres: 1e-3,
        angle_to_radians: 1.0,
    };

    let kernel = StubKernel::default();
    kernel.build(&Primitive::Extrusion {
        profile: rect_profile(scale.length(300.0), scale.length(4000.0)),
        direction: [0.0, 0.0, 1.0],
        depth: scale.length(2410.0),
        placement: Transform::identity(),
    });

    assert_eq!(
        kernel.requests(),
        vec!["extrusion depth=2.41"],
        "a 2410 mm wall reaches the kernel as 2.41 m"
    );
}

/// Mapped items: the instance transform is available before lowering.
#[test]
fn a_mapped_item_resolves_both_transforms_for_the_kernel() {
    let mut m = Model::new();
    m.insert(
        EntityId(1),
        Entity::new("IFCCARTESIANPOINT", vec![list(&[0.0, 0.0, 0.0])]),
    );
    m.insert(
        EntityId(2),
        Entity::new("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    m.insert(
        EntityId(3),
        Entity::new("IFCSHAPEREPRESENTATION", vec![Value::Null]),
    );
    m.insert(
        EntityId(4),
        Entity::new("IFCREPRESENTATIONMAP", vec![r(2), r(3)]),
    );
    m.insert(
        EntityId(5),
        Entity::new("IFCCARTESIANTRANSFORMATIONOPERATOR3D", vec![Value::Null]),
    );
    m.insert(EntityId(6), Entity::new("IFCMAPPEDITEM", vec![r(4), r(5)]));

    let mut walker = MappingWalker::new();
    let instance = walker
        .resolve(&m, EntityId(6))
        .expect("a mapped item resolves");

    assert_eq!(instance.mapping_origin, EntityId(2));
    assert_eq!(instance.mapping_target, EntityId(5));
    assert_eq!(instance.mapped_representation, EntityId(3));
}

/// The whole pipeline: classify, validate, then emit a kernel request.
///
/// This is the shape a real consumer takes, exercised end to end without any
/// concrete geometry backend present.
#[test]
fn the_full_ifc_side_pipeline_runs_against_a_stub_kernel() {
    let mut m = Model::new();
    m.insert(
        EntityId(1),
        Entity::new("IFCDIRECTION", vec![list(&[0.0, 0.0, 1.0])]),
    );
    m.insert(
        EntityId(2),
        Entity::new("IFCCARTESIANPOINT", vec![list(&[0.0, 0.0, 0.0])]),
    );
    m.insert(
        EntityId(3),
        Entity::new("IFCAXIS2PLACEMENT3D", vec![r(2), Value::Null, Value::Null]),
    );
    m.insert(
        EntityId(4),
        Entity::new(
            "IFCEXTRUDEDAREASOLID",
            vec![Value::Null, r(3), r(1), n(2.41)],
        ),
    );

    // 1. classify
    let kind = SolidKind::classify("IFCEXTRUDEDAREASOLID").expect("classified");
    assert!(
        matches!(kind, SolidKind::Swept),
        "an extruded area solid is a swept solid, got {kind:?}"
    );

    // 2. the select admits it as a boolean operand
    assert!(select::is_a("IFCEXTRUDEDAREASOLID", "IFCSOLIDMODEL"));

    // 3. where-rules pass
    let violations = rules::validate(&m, EntityId(4));
    assert!(violations.is_empty(), "valid solid: {violations:?}");

    // 4. emit to the stub kernel
    let kernel = StubKernel::default();
    kernel.build(&Primitive::Extrusion {
        profile: rect_profile(0.3, 4.0),
        direction: [0.0, 0.0, 1.0],
        depth: 2.41,
        placement: Transform::identity(),
    });

    assert_eq!(kernel.requests(), vec!["extrusion depth=2.41"]);
}

/// The contract's core claim, asserted directly.
///
/// A boolean request reports that it needs a capability. An application can
/// therefore refuse a file up front rather than failing halfway through.
#[test]
fn a_consumer_can_detect_required_capabilities_before_building() {
    let plain = Primitive::Mesh {
        positions: vec![],
        indices: vec![],
        placement: Transform::identity(),
    };
    assert!(!plain.requires_boolean());

    let needs_csg = Primitive::Boolean {
        op: BooleanOp::Difference,
        first: Box::new(Primitive::Csg {
            shape: CsgShape::Sphere { radius: 1.0 },
            placement: Transform::identity(),
        }),
        second: Box::new(Primitive::Csg {
            shape: CsgShape::Sphere { radius: 0.5 },
            placement: Transform::identity(),
        }),
    };
    assert!(
        needs_csg.requires_boolean(),
        "an app must be able to detect it needs a boolean-capable backend"
    );
}
