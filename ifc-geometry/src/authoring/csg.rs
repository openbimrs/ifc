//! CSG primitives and half spaces: solids given by parameters, not faces.
//!
//! A primitive is a placement plus a handful of lengths. There is no
//! mesh to build and nothing to evaluate, so the writer is a bounds
//! check and a slot fill. The interesting part is what a primitive
//! *denotes*: an `IfcBlock` is a solid, an `IfcHalfSpaceSolid` is an
//! infinite region, and a caller that confuses them gets a boolean
//! result the size of the universe.
//!
//! # Half spaces are infinite on purpose
//!
//! `AgreementFlag` picks which side of `BaseSurface` is solid. Neither
//! side is bounded. `IfcBoxedHalfSpace` adds an `Enclosure` box, but
//! that is a *hint for display*, not a trim: the half space stays
//! infinite and the box only tells a viewer where to stop drawing.
//! `IfcPolygonalBoundedHalfSpace` is the one that genuinely bounds,
//! via a closed polygonal boundary prism.

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::error::GeometryError;
use crate::solid::csg::{csg_solid_slot, primitive_slot};
use crate::solid::halfspace::slot as half_slot;

use super::std_profile::positive;

/// Stage one of the five `IfcCsgPrimitive3D` subtypes.
///
/// `dims` are written from absolute slot 1 upward, in schema order.
fn primitive(
    tx: &mut Transaction,
    type_name: &'static str,
    names: &[&'static str],
    position: EntityId,
    dims: &[f64],
) -> Result<EntityId, GeometryError> {
    debug_assert_eq!(names.len(), dims.len());
    for (name, value) in names.iter().zip(dims) {
        positive(type_name, name, *value)?;
    }
    let mut attrs = vec![Value::Null; 1 + dims.len()];
    attrs[primitive_slot::POSITION] = Value::Ref(position);
    for (offset, value) in dims.iter().enumerate() {
        attrs[primitive_slot::DIM_0 + offset] = Value::Real(*value);
    }
    Ok(tx.create(Entity::new(type_name, attrs)))
}

/// Stage an `IfcBlock`: a box with a corner at `position`.
///
/// # Errors
///
/// Refuses a non-positive or non-finite edge length.
pub fn block(
    tx: &mut Transaction,
    position: EntityId,
    x_length: f64,
    y_length: f64,
    z_length: f64,
) -> Result<EntityId, GeometryError> {
    primitive(
        tx,
        "IFCBLOCK",
        &["XLength", "YLength", "ZLength"],
        position,
        &[x_length, y_length, z_length],
    )
}

/// Stage an `IfcSphere`.
///
/// # Errors
///
/// Refuses a non-positive or non-finite radius.
pub fn sphere(
    tx: &mut Transaction,
    position: EntityId,
    radius: f64,
) -> Result<EntityId, GeometryError> {
    primitive(tx, "IFCSPHERE", &["Radius"], position, &[radius])
}

/// Stage an `IfcRightCircularCylinder`.
///
/// # Errors
///
/// Refuses a non-positive or non-finite height or radius.
pub fn cylinder(
    tx: &mut Transaction,
    position: EntityId,
    height: f64,
    radius: f64,
) -> Result<EntityId, GeometryError> {
    primitive(
        tx,
        "IFCRIGHTCIRCULARCYLINDER",
        &["Height", "Radius"],
        position,
        &[height, radius],
    )
}

/// Stage an `IfcRightCircularCone`.
///
/// # Errors
///
/// Refuses a non-positive or non-finite height or bottom radius.
pub fn cone(
    tx: &mut Transaction,
    position: EntityId,
    height: f64,
    bottom_radius: f64,
) -> Result<EntityId, GeometryError> {
    primitive(
        tx,
        "IFCRIGHTCIRCULARCONE",
        &["Height", "BottomRadius"],
        position,
        &[height, bottom_radius],
    )
}

/// Stage an `IfcRectangularPyramid`.
///
/// # Errors
///
/// Refuses a non-positive or non-finite dimension.
pub fn rectangular_pyramid(
    tx: &mut Transaction,
    position: EntityId,
    x_length: f64,
    y_length: f64,
    height: f64,
) -> Result<EntityId, GeometryError> {
    primitive(
        tx,
        "IFCRECTANGULARPYRAMID",
        &["XLength", "YLength", "Height"],
        position,
        &[x_length, y_length, height],
    )
}

/// Stage an `IfcCsgSolid` over a tree root.
///
/// The root is an `IfcCsgSelect`: either an `IfcBooleanResult` or an
/// `IfcCsgPrimitive3D`. Both are entity references, so the writer cannot
/// tell them apart and does not try -- a wrong root is a schema error the
/// validator reports, not something to guess at here.
pub fn csg_solid(tx: &mut Transaction, tree_root: EntityId) -> EntityId {
    let mut attrs = vec![Value::Null; 1];
    attrs[csg_solid_slot::TREE_ROOT_EXPRESSION] = Value::Ref(tree_root);
    tx.create(Entity::new("IFCCSGSOLID", attrs))
}

/// Stage an `IfcHalfSpaceSolid`: everything on one side of a surface.
///
/// `agreement` true means the solid is the side the surface normal points
/// *away* from, per the schema's definition. The result is unbounded.
pub fn half_space(tx: &mut Transaction, base_surface: EntityId, agreement: bool) -> EntityId {
    let mut attrs = vec![Value::Null; 2];
    attrs[half_slot::BASE_SURFACE] = Value::Ref(base_surface);
    attrs[half_slot::AGREEMENT_FLAG] = Value::Bool(agreement);
    tx.create(Entity::new("IFCHALFSPACESOLID", attrs))
}

/// Stage an `IfcBoxedHalfSpace`.
///
/// `enclosure` is a *display hint*, not a trim: the half space remains
/// infinite and a consumer that treats the box as the solid's extent is
/// reading the schema wrong. Authoring it anyway, because files in the
/// wild carry it and round-tripping must not drop it.
pub fn boxed_half_space(
    tx: &mut Transaction,
    base_surface: EntityId,
    agreement: bool,
    enclosure: EntityId,
) -> EntityId {
    let mut attrs = vec![Value::Null; 3];
    attrs[half_slot::BASE_SURFACE] = Value::Ref(base_surface);
    attrs[half_slot::AGREEMENT_FLAG] = Value::Bool(agreement);
    attrs[half_slot::ENCLOSURE] = Value::Ref(enclosure);
    tx.create(Entity::new("IFCBOXEDHALFSPACE", attrs))
}

/// Stage an `IfcPolygonalBoundedHalfSpace`.
///
/// Unlike the other two this one is genuinely bounded: the polygonal
/// boundary sweeps a prism, and the solid is the intersection of that
/// prism with the half space.
///
/// # Errors
///
/// The boundary must be a closed curve. This writer cannot verify
/// closure without evaluating the curve, so it takes the reference as
/// given -- the same position the profile writers take for
/// `IfcArbitraryClosedProfileDef`.
pub fn polygonal_bounded_half_space(
    tx: &mut Transaction,
    base_surface: EntityId,
    agreement: bool,
    position: EntityId,
    polygonal_boundary: EntityId,
) -> EntityId {
    let mut attrs = vec![Value::Null; 4];
    attrs[half_slot::BASE_SURFACE] = Value::Ref(base_surface);
    attrs[half_slot::AGREEMENT_FLAG] = Value::Bool(agreement);
    attrs[half_slot::POSITION] = Value::Ref(position);
    attrs[half_slot::POLYGONAL_BOUNDARY] = Value::Ref(polygonal_boundary);
    tx.create(Entity::new("IFCPOLYGONALBOUNDEDHALFSPACE", attrs))
}

/// Stage an `IfcBoundingBox`.
///
/// # Errors
///
/// Refuses a non-positive or non-finite extent: the schema declares all
/// three as `IfcPositiveLengthMeasure`, so a flat box is not expressible.
pub fn bounding_box(
    tx: &mut Transaction,
    corner: EntityId,
    x_dim: f64,
    y_dim: f64,
    z_dim: f64,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCBOUNDINGBOX";
    positive(T, "XDim", x_dim)?;
    positive(T, "YDim", y_dim)?;
    positive(T, "ZDim", z_dim)?;
    let attrs = vec![
        Value::Ref(corner),
        Value::Real(x_dim),
        Value::Real(y_dim),
        Value::Real(z_dim),
    ];
    Ok(tx.create(Entity::new(T, attrs)))
}
