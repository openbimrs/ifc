//! Placement, connection geometry, grids and geometric sets.
//!
//! # A clipping result is always a difference
//!
//! `IfcBooleanClippingResult` fixes `Operator = DIFFERENCE` and
//! requires the second operand be a half space. There is no operator
//! to choose, so the writer does not offer one -- a union of a solid
//! and a half space is not a clip, it is the universe.
//!
//! # A spine pairs sections with positions
//!
//! `IfcSectionedSpine` carries `CrossSections` and
//! `CrossSectionPositions` as two independent `LIST [2:?]`s. Nothing
//! in the schema ties their lengths together, but a section without a
//! position (or the reverse) describes no solid, so this writer
//! requires they match.

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::error::GeometryError;
use crate::solid::swept::spine_slot;

use super::{invalid, refs, require_finite};

/// Stage an `IfcLocalPlacement`.
///
/// `placement_rel_to` absent means the placement is absolute, in the
/// project's own coordinate system. A chain of these is how IFC nests
/// a component inside an assembly inside a storey.
pub fn local_placement(
    tx: &mut Transaction,
    placement_rel_to: Option<EntityId>,
    relative_placement: EntityId,
) -> EntityId {
    let attrs = vec![
        placement_rel_to.map_or(Value::Null, Value::Ref),
        Value::Ref(relative_placement),
    ];
    tx.create(Entity::new("IFCLOCALPLACEMENT", attrs))
}

/// Stage an `IfcGridAxis`.
///
/// `same_sense` says whether the axis runs along its curve's own
/// direction; it decides which way offsets are measured at an
/// intersection.
pub fn grid_axis(
    tx: &mut Transaction,
    axis_tag: Option<&str>,
    axis_curve: EntityId,
    same_sense: bool,
) -> EntityId {
    let attrs = vec![
        axis_tag.map_or(Value::Null, |t| Value::Text(t.into())),
        Value::Ref(axis_curve),
        Value::Bool(same_sense),
    ];
    tx.create(Entity::new("IFCGRIDAXIS", attrs))
}

/// Stage an `IfcVirtualGridIntersection`.
///
/// # Errors
///
/// Refuses anything but exactly two axes -- `LIST [2:2]` -- the same
/// axis twice, or an offset list outside `LIST [2:3]`. Two identical
/// axes do not intersect anywhere in particular.
pub fn virtual_grid_intersection(
    tx: &mut Transaction,
    intersecting_axes: &[EntityId],
    offset_distances: &[f64],
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCVIRTUALGRIDINTERSECTION";
    if intersecting_axes.len() != 2 {
        return Err(invalid(
            T,
            "IntersectingAxes",
            format!("expected exactly 2 axes, got {}", intersecting_axes.len()),
        ));
    }
    if intersecting_axes[0] == intersecting_axes[1] {
        return Err(invalid(
            T,
            "IntersectingAxes",
            "the list is UNIQUE; an axis does not intersect itself",
        ));
    }
    if offset_distances.len() < 2 || offset_distances.len() > 3 {
        return Err(invalid(
            T,
            "OffsetDistances",
            format!("expected 2 or 3 offsets, got {}", offset_distances.len()),
        ));
    }
    require_finite(T, "OffsetDistances", offset_distances)?;
    let attrs = vec![
        refs(intersecting_axes),
        Value::List(offset_distances.iter().copied().map(Value::Real).collect()),
    ];
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcGridPlacement`: a placement located on a grid.
pub fn grid_placement(
    tx: &mut Transaction,
    placement_rel_to: Option<EntityId>,
    placement_location: EntityId,
    placement_ref_direction: Option<EntityId>,
) -> EntityId {
    let attrs = vec![
        placement_rel_to.map_or(Value::Null, Value::Ref),
        Value::Ref(placement_location),
        placement_ref_direction.map_or(Value::Null, Value::Ref),
    ];
    tx.create(Entity::new("IFCGRIDPLACEMENT", attrs))
}

/// Which connection geometry form to author.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionKind {
    /// `IfcConnectionPointGeometry`: a point or vertex point.
    Point,
    /// `IfcConnectionCurveGeometry`: a curve or edge curve.
    Curve,
    /// `IfcConnectionSurfaceGeometry`: a surface or face surface.
    Surface,
    /// `IfcConnectionVolumeGeometry`: a solid or shell.
    Volume,
}

impl ConnectionKind {
    /// The entity type name.
    fn type_name(self) -> &'static str {
        match self {
            Self::Point => "IFCCONNECTIONPOINTGEOMETRY",
            Self::Curve => "IFCCONNECTIONCURVEGEOMETRY",
            Self::Surface => "IFCCONNECTIONSURFACEGEOMETRY",
            Self::Volume => "IFCCONNECTIONVOLUMEGEOMETRY",
        }
    }
}

/// Stage a connection geometry.
///
/// The two slots are the geometry as seen from each side of the
/// connection. `on_related` absent means both elements agree on the
/// same geometry -- which is the common case, and is why omitting it
/// is not the same as repeating the first reference.
pub fn connection_geometry(
    tx: &mut Transaction,
    kind: ConnectionKind,
    on_relating: EntityId,
    on_related: Option<EntityId>,
) -> EntityId {
    let attrs = vec![
        Value::Ref(on_relating),
        on_related.map_or(Value::Null, Value::Ref),
    ];
    tx.create(Entity::new(kind.type_name(), attrs))
}

/// Stage an `IfcConnectionPointEccentricity`.
///
/// The eccentricities offset the connection from the stated point --
/// how a beam meets a column off its centreline. All three are
/// optional and signed: `IfcLengthMeasure`, not the positive form.
///
/// # Errors
///
/// Refuses a non-finite eccentricity.
pub fn connection_point_eccentricity(
    tx: &mut Transaction,
    on_relating: EntityId,
    on_related: Option<EntityId>,
    eccentricity: [Option<f64>; 3],
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCCONNECTIONPOINTECCENTRICITY";
    const NAMES: [&str; 3] = ["EccentricityInX", "EccentricityInY", "EccentricityInZ"];
    let mut attrs = vec![Value::Null; 5];
    attrs[0] = Value::Ref(on_relating);
    attrs[1] = on_related.map_or(Value::Null, Value::Ref);
    for (offset, value) in eccentricity.iter().enumerate() {
        if let Some(value) = value {
            require_finite(T, NAMES[offset], &[*value])?;
            attrs[2 + offset] = Value::Real(*value);
        }
    }
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcPointOnCurve`.
///
/// # Errors
///
/// Refuses a non-finite parameter.
pub fn point_on_curve(
    tx: &mut Transaction,
    basis_curve: EntityId,
    point_parameter: f64,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCPOINTONCURVE";
    require_finite(T, "PointParameter", &[point_parameter])?;
    let attrs = vec![Value::Ref(basis_curve), parameter(point_parameter)];
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcPointOnSurface`.
///
/// # Errors
///
/// Refuses a non-finite parameter.
pub fn point_on_surface(
    tx: &mut Transaction,
    basis_surface: EntityId,
    u: f64,
    v: f64,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCPOINTONSURFACE";
    require_finite(T, "PointParameterU", &[u, v])?;
    let attrs = vec![Value::Ref(basis_surface), parameter(u), parameter(v)];
    Ok(tx.create(Entity::new(T, attrs)))
}

/// An `IfcParameterValue`, carrying its measure type.
fn parameter(value: f64) -> Value {
    Value::Typed {
        type_name: "IFCPARAMETERVALUE".into(),
        value: Box::new(Value::Real(value)),
    }
}

/// Stage an `IfcGeometricSet` or `IfcGeometricCurveSet`.
///
/// The curve-set form restricts its elements to curves; that is a
/// claim about the referenced entities, which this writer does not
/// resolve, so the caller chooses the type and the validator checks it.
///
/// # Errors
///
/// Refuses an empty element set: `SET [1:?]`.
pub fn geometric_set(
    tx: &mut Transaction,
    curves_only: bool,
    elements: &[EntityId],
) -> Result<EntityId, GeometryError> {
    let type_name = if curves_only {
        "IFCGEOMETRICCURVESET"
    } else {
        "IFCGEOMETRICSET"
    };
    if elements.is_empty() {
        return Err(invalid(
            type_name,
            "Elements",
            "expected at least one element",
        ));
    }
    Ok(tx.create(Entity::new(type_name, vec![refs(elements)])))
}

/// Stage an `IfcPath`: an ordered run of oriented edges.
///
/// # Errors
///
/// Refuses an empty edge list, or a repeated edge: the list is
/// `LIST [1:?] OF UNIQUE`.
pub fn path(tx: &mut Transaction, edge_list: &[EntityId]) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCPATH";
    if edge_list.is_empty() {
        return Err(invalid(T, "EdgeList", "expected at least one edge"));
    }
    let mut seen = edge_list.to_vec();
    seen.sort_unstable();
    seen.dedup();
    if seen.len() != edge_list.len() {
        return Err(invalid(
            T,
            "EdgeList",
            "the edge list is UNIQUE; a path does not repeat an edge",
        ));
    }
    Ok(tx.create(Entity::new(T, vec![refs(edge_list)])))
}

/// Stage an `IfcBooleanClippingResult`.
///
/// `Operator` is fixed to `DIFFERENCE` by `OperatorType`, so it is not
/// an argument: a clip that unions is not a clip. The second operand
/// must be a half space, which the validator checks by type.
pub fn boolean_clipping_result(
    tx: &mut Transaction,
    first_operand: EntityId,
    second_operand: EntityId,
) -> EntityId {
    let attrs = vec![
        Value::Enum("DIFFERENCE".into()),
        Value::Ref(first_operand),
        Value::Ref(second_operand),
    ];
    tx.create(Entity::new("IFCBOOLEANCLIPPINGRESULT", attrs))
}

/// Stage an `IfcSectionedSpine`.
///
/// # Errors
///
/// Refuses fewer than two cross sections or positions -- both are
/// `LIST [2:?]` -- or lists of differing length. The schema does not
/// relate the two lengths, but a section with no position places
/// nothing.
pub fn sectioned_spine(
    tx: &mut Transaction,
    spine_curve: EntityId,
    cross_sections: &[EntityId],
    cross_section_positions: &[EntityId],
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCSECTIONEDSPINE";
    if cross_sections.len() < 2 {
        return Err(invalid(
            T,
            "CrossSections",
            format!(
                "expected at least 2 cross sections, got {}",
                cross_sections.len()
            ),
        ));
    }
    if cross_sections.len() != cross_section_positions.len() {
        return Err(invalid(
            T,
            "CrossSectionPositions",
            format!(
                "{} positions for {} cross sections",
                cross_section_positions.len(),
                cross_sections.len()
            ),
        ));
    }
    let mut attrs = vec![Value::Null; 3];
    attrs[spine_slot::SPINE_CURVE] = Value::Ref(spine_curve);
    attrs[spine_slot::CROSS_SECTIONS] = refs(cross_sections);
    attrs[spine_slot::CROSS_SECTION_POSITIONS] = refs(cross_section_positions);
    Ok(tx.create(Entity::new(T, attrs)))
}
