//! Semantic validation for graph references.

use crate::{CurveRelation, GeometryNode, GraphError, NodeId, SolidOperation, SurfaceRelation};

#[derive(Debug, Clone, Copy)]
enum ExpectedReference {
    Curve,
    Surface,
    CurveOrSurface,
    Profile,
    Solid,
    HalfSpace,
}

impl ExpectedReference {
    const fn description(self) -> &'static str {
        match self {
            Self::Curve => "curve",
            Self::Surface => "surface",
            Self::CurveOrSurface => "curve or surface",
            Self::Profile => "profile",
            Self::Solid => "solid",
            Self::HalfSpace => "half-space",
        }
    }

    fn accepts(self, node: &GeometryNode) -> bool {
        let curve = matches!(
            node,
            GeometryNode::Curve2(_) | GeometryNode::Curve3(_) | GeometryNode::CurveRelation(_)
        );
        let surface = matches!(
            node,
            GeometryNode::Surface(_) | GeometryNode::SurfaceRelation(_)
        );
        match self {
            Self::Curve => curve,
            Self::Surface => surface,
            Self::CurveOrSurface => curve || surface,
            Self::Profile => matches!(node, GeometryNode::Profile(_)),
            Self::Solid => matches!(
                node,
                GeometryNode::Primitive(_)
                    | GeometryNode::HalfSpace(_)
                    | GeometryNode::SolidOperation(_)
                    | GeometryNode::BRep(_)
                    | GeometryNode::PolygonMesh(_)
                    | GeometryNode::TriMesh(_)
                    | GeometryNode::Instance(_)
            ),
            Self::HalfSpace => matches!(node, GeometryNode::HalfSpace(_)),
        }
    }
}

pub(crate) fn validate_reference_types(
    node: &GeometryNode,
    nodes: &[GeometryNode],
) -> Result<(), GraphError> {
    match node {
        GeometryNode::CurveRelation(value) => validate_curve_relation(value, nodes),
        GeometryNode::PointOnCurve(value) => {
            expect_reference(nodes, value.curve, ExpectedReference::Curve)
        }
        GeometryNode::SurfaceRelation(value) => validate_surface_relation(value, nodes),
        GeometryNode::PointOnSurface(value) => {
            expect_reference(nodes, value.surface, ExpectedReference::Surface)
        }
        GeometryNode::SolidOperation(value) => validate_solid_operation(value, nodes),
        GeometryNode::BRep(value) => {
            for edge in value.edges() {
                if let Some(curve) = edge.curve {
                    expect_reference(nodes, curve, ExpectedReference::Curve)?;
                }
            }
            for face in value.faces() {
                if let Some(surface) = face.surface {
                    expect_reference(nodes, surface, ExpectedReference::Surface)?;
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn expect_reference(
    nodes: &[GeometryNode],
    reference: NodeId,
    expected: ExpectedReference,
) -> Result<(), GraphError> {
    let actual = &nodes[reference.index()];
    if expected.accepts(actual) {
        return Ok(());
    }
    Err(GraphError::InvalidReferenceType {
        reference,
        expected: expected.description(),
        actual: node_kind(actual),
    })
}

fn node_kind(node: &GeometryNode) -> &'static str {
    match node {
        GeometryNode::Point2(_) => "point2",
        GeometryNode::Point3(_) => "point3",
        GeometryNode::Vector2(_) => "vector2",
        GeometryNode::Vector3(_) => "vector3",
        GeometryNode::Frame2(_) => "frame2",
        GeometryNode::Frame3(_) => "frame3",
        GeometryNode::Transform(_) => "transform",
        GeometryNode::PointList2(_) => "point-list2",
        GeometryNode::PointList3(_) => "point-list3",
        GeometryNode::Curve2(_) => "curve2",
        GeometryNode::Curve3(_) => "curve3",
        GeometryNode::CurveRelation(_) => "curve-relation",
        GeometryNode::PointOnCurve(_) => "point-on-curve",
        GeometryNode::Surface(_) => "surface",
        GeometryNode::SurfaceRelation(_) => "surface-relation",
        GeometryNode::PointOnSurface(_) => "point-on-surface",
        GeometryNode::Profile(_) => "profile",
        GeometryNode::Primitive(_) => "primitive",
        GeometryNode::HalfSpace(_) => "half-space",
        GeometryNode::SolidOperation(_) => "solid-operation",
        GeometryNode::BRep(_) => "brep",
        GeometryNode::PolygonMesh(_) => "polygon-mesh",
        GeometryNode::TriMesh(_) => "triangle-mesh",
        GeometryNode::BoundingBox(_) => "bounding-box",
        GeometryNode::Instance(_) => "instance",
        GeometryNode::Collection(_) => "collection",
    }
}

fn validate_curve_relation(
    relation: &CurveRelation,
    nodes: &[GeometryNode],
) -> Result<(), GraphError> {
    match relation {
        CurveRelation::Composite { segments } => {
            for segment in segments {
                expect_reference(nodes, segment.curve, ExpectedReference::Curve)?;
            }
            Ok(())
        }
        CurveRelation::Trimmed { basis, .. } | CurveRelation::Offset { basis, .. } => {
            expect_reference(nodes, *basis, ExpectedReference::Curve)
        }
        CurveRelation::SurfaceCurve {
            curve_3d,
            associated_geometry,
            ..
        } => {
            expect_reference(nodes, *curve_3d, ExpectedReference::Curve)?;
            for reference in associated_geometry {
                expect_reference(nodes, *reference, ExpectedReference::CurveOrSurface)?;
            }
            Ok(())
        }
        CurveRelation::ParameterCurve {
            basis_surface,
            reference_curve,
        } => {
            expect_reference(nodes, *basis_surface, ExpectedReference::Surface)?;
            expect_reference(nodes, *reference_curve, ExpectedReference::Curve)
        }
    }
}

fn validate_surface_relation(
    relation: &SurfaceRelation,
    nodes: &[GeometryNode],
) -> Result<(), GraphError> {
    match relation {
        SurfaceRelation::CurveBounded {
            basis, boundaries, ..
        } => {
            expect_reference(nodes, *basis, ExpectedReference::Surface)?;
            for boundary in boundaries {
                expect_reference(nodes, *boundary, ExpectedReference::Curve)?;
            }
            Ok(())
        }
        SurfaceRelation::RectangularTrimmed { basis, .. }
        | SurfaceRelation::Offset { basis, .. } => {
            expect_reference(nodes, *basis, ExpectedReference::Surface)
        }
        SurfaceRelation::LinearExtrusion { swept_curve, .. }
        | SurfaceRelation::Revolution { swept_curve, .. } => {
            expect_reference(nodes, *swept_curve, ExpectedReference::Curve)
        }
    }
}

fn validate_solid_operation(
    operation: &SolidOperation,
    nodes: &[GeometryNode],
) -> Result<(), GraphError> {
    match operation {
        SolidOperation::Extrusion { profile, .. } | SolidOperation::Revolution { profile, .. } => {
            expect_reference(nodes, *profile, ExpectedReference::Profile)
        }
        SolidOperation::TaperedExtrusion {
            start_profile,
            end_profile,
            ..
        }
        | SolidOperation::TaperedRevolution {
            start_profile,
            end_profile,
            ..
        } => {
            expect_reference(nodes, *start_profile, ExpectedReference::Profile)?;
            expect_reference(nodes, *end_profile, ExpectedReference::Profile)
        }
        SolidOperation::SweptDisk { directrix, .. } => {
            expect_reference(nodes, *directrix, ExpectedReference::Curve)
        }
        SolidOperation::FixedReferenceSweep {
            profile, directrix, ..
        } => {
            expect_reference(nodes, *profile, ExpectedReference::Profile)?;
            expect_reference(nodes, *directrix, ExpectedReference::Curve)
        }
        SolidOperation::SurfaceCurveSweep {
            profile,
            directrix,
            reference_surface,
            ..
        } => {
            expect_reference(nodes, *profile, ExpectedReference::Profile)?;
            expect_reference(nodes, *directrix, ExpectedReference::Curve)?;
            expect_reference(nodes, *reference_surface, ExpectedReference::Surface)
        }
        SolidOperation::SectionedSpine { spine, sections } => {
            expect_reference(nodes, *spine, ExpectedReference::Curve)?;
            for section in sections {
                expect_reference(nodes, section.profile, ExpectedReference::Profile)?;
            }
            Ok(())
        }
        SolidOperation::Boolean { left, right, .. } => {
            expect_reference(nodes, *left, ExpectedReference::Solid)?;
            expect_reference(nodes, *right, ExpectedReference::Solid)
        }
        SolidOperation::BoundedHalfSpace {
            half_space,
            boundary,
            ..
        } => {
            expect_reference(nodes, *half_space, ExpectedReference::HalfSpace)?;
            expect_reference(nodes, *boundary, ExpectedReference::Curve)
        }
    }
}
