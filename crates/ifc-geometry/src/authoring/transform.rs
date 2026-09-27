//! Transformation operators, representation maps and mapped items.
//!
//! # `Scale2` is not at the same slot in both branches
//!
//! `IfcCartesianTransformationOperator3D` inserts `Axis3` at slot 4,
//! so the 3D non-uniform subtype carries `Scale2` at 5 and `Scale3` at
//! 6, while the 2D non-uniform one has `Scale2` at 4. An operator
//! written with the 2D layout into a 3D entity puts a scale where a
//! direction belongs, which parses as a type error only if a validator
//! looks. The slot constants in [`crate::resource::operator`] record
//! both, and this module uses them rather than counting.
//!
//! # A scale of zero collapses everything
//!
//! The supertype derives `Scl := NVL(Scale, 1.0)` and requires
//! `Scl > 0.0`. Absent means one, not zero, so `None` is safe -- but an
//! explicit zero or negative scale is refused here.

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::error::GeometryError;
use crate::resource::operator::slot;

use super::{invalid, refs, require_finite};

/// The axes and scale shared by every transformation operator.
///
/// Every field is optional except the origin: the schema defaults the
/// axes to the identity frame and the scale to one.
#[derive(Debug, Default, Clone, Copy)]
pub struct Transform {
    /// `Axis1`: the local X direction.
    pub axis1: Option<EntityId>,
    /// `Axis2`: the local Y direction.
    pub axis2: Option<EntityId>,
    /// `Scale`. Absent means one, which is why `None` is not zero.
    pub scale: Option<f64>,
}

/// Check a scale against `ScaleGreaterZero`.
fn check_scale(
    type_name: &'static str,
    attribute: &'static str,
    scale: Option<f64>,
) -> Result<(), GeometryError> {
    let Some(scale) = scale else {
        // Absent derives to 1.0, which satisfies the rule.
        return Ok(());
    };
    require_finite(type_name, attribute, &[scale])?;
    if scale <= 0.0 {
        return Err(invalid(
            type_name,
            attribute,
            format!("expected a scale above zero, got {scale}"),
        ));
    }
    Ok(())
}

/// Fill the four slots every operator shares.
fn base(
    type_name: &'static str,
    width: usize,
    local_origin: EntityId,
    transform: Transform,
) -> Result<Vec<Value>, GeometryError> {
    check_scale(type_name, "Scale", transform.scale)?;
    let mut attrs = vec![Value::Null; width];
    attrs[slot::AXIS1] = transform.axis1.map_or(Value::Null, Value::Ref);
    attrs[slot::AXIS2] = transform.axis2.map_or(Value::Null, Value::Ref);
    attrs[slot::LOCAL_ORIGIN] = Value::Ref(local_origin);
    attrs[slot::SCALE] = transform.scale.map_or(Value::Null, Value::Real);
    Ok(attrs)
}

/// Stage an `IfcCartesianTransformationOperator2D`.
///
/// # Errors
///
/// Refuses a scale that is zero, negative or non-finite.
pub fn transformation_operator_2d(
    tx: &mut Transaction,
    local_origin: EntityId,
    transform: Transform,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCCARTESIANTRANSFORMATIONOPERATOR2D";
    let attrs = base(T, 4, local_origin, transform)?;
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcCartesianTransformationOperator2DnonUniform`.
///
/// `scale2` scales the local Y axis independently. Its slot is 4 here
/// and 5 on the 3D variant; see the module note.
///
/// # Errors
///
/// Refuses either scale being zero, negative or non-finite.
pub fn transformation_operator_2d_non_uniform(
    tx: &mut Transaction,
    local_origin: EntityId,
    transform: Transform,
    scale2: Option<f64>,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCCARTESIANTRANSFORMATIONOPERATOR2DNONUNIFORM";
    check_scale(T, "Scale2", scale2)?;
    let mut attrs = base(T, 5, local_origin, transform)?;
    attrs[slot::SCALE2_2D] = scale2.map_or(Value::Null, Value::Real);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcCartesianTransformationOperator3D`.
///
/// # Errors
///
/// Refuses a scale that is zero, negative or non-finite.
pub fn transformation_operator_3d(
    tx: &mut Transaction,
    local_origin: EntityId,
    transform: Transform,
    axis3: Option<EntityId>,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCCARTESIANTRANSFORMATIONOPERATOR3D";
    let mut attrs = base(T, 5, local_origin, transform)?;
    attrs[slot::AXIS3] = axis3.map_or(Value::Null, Value::Ref);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcCartesianTransformationOperator3DnonUniform`.
///
/// `Scale2` and `Scale3` sit at slots 5 and 6, *after* `Axis3` -- not
/// at 4 as on the 2D non-uniform operator.
///
/// # Errors
///
/// Refuses any of the three scales being zero, negative or non-finite.
pub fn transformation_operator_3d_non_uniform(
    tx: &mut Transaction,
    local_origin: EntityId,
    transform: Transform,
    axis3: Option<EntityId>,
    scale2: Option<f64>,
    scale3: Option<f64>,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCCARTESIANTRANSFORMATIONOPERATOR3DNONUNIFORM";
    check_scale(T, "Scale2", scale2)?;
    check_scale(T, "Scale3", scale3)?;
    let mut attrs = base(T, 7, local_origin, transform)?;
    attrs[slot::AXIS3] = axis3.map_or(Value::Null, Value::Ref);
    attrs[slot::SCALE2_3D] = scale2.map_or(Value::Null, Value::Real);
    attrs[slot::SCALE3_3D] = scale3.map_or(Value::Null, Value::Real);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcRepresentationMap`: a reusable shape and its origin.
///
/// The map is what an `IfcMappedItem` points at, so one map serves
/// many instances -- that is the whole point of mapping rather than
/// repeating the geometry.
pub fn representation_map(
    tx: &mut Transaction,
    mapping_origin: EntityId,
    mapped_representation: EntityId,
) -> EntityId {
    let attrs = vec![
        Value::Ref(mapping_origin),
        Value::Ref(mapped_representation),
    ];
    tx.create(Entity::new("IFCREPRESENTATIONMAP", attrs))
}

/// Stage an `IfcMappedItem`: one placed instance of a mapped shape.
///
/// `mapping_target` is a transformation operator, so the same source
/// map appears at a different place and scale for each item.
pub fn mapped_item(
    tx: &mut Transaction,
    mapping_source: EntityId,
    mapping_target: EntityId,
) -> EntityId {
    let attrs = vec![Value::Ref(mapping_source), Value::Ref(mapping_target)];
    tx.create(Entity::new("IFCMAPPEDITEM", attrs))
}

/// Stage an `IfcTopologyRepresentation`.
///
/// A shape representation whose items are topological rather than
/// geometric -- vertices, edges, faces and shells.
///
/// # Errors
///
/// Refuses an empty item set: `SET [1:?]`.
pub fn topology_representation(
    tx: &mut Transaction,
    context: EntityId,
    identifier: Option<&str>,
    representation_type: Option<&str>,
    items: &[EntityId],
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCTOPOLOGYREPRESENTATION";
    if items.is_empty() {
        return Err(invalid(T, "Items", "expected at least one item"));
    }
    let attrs = vec![
        Value::Ref(context),
        identifier.map_or(Value::Null, |v| Value::Text(v.into())),
        representation_type.map_or(Value::Null, |v| Value::Text(v.into())),
        refs(items),
    ];
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcShapeAspect`: a named part of a product shape.
///
/// This is how a subtype points at one component of a larger
/// representation -- a varying structural member naming the aspect
/// that carries its thickness, for instance.
///
/// `ProductDefinitional` is an `IfcLogical`, not a boolean: it may be
/// UNKNOWN, meaning nobody has stated whether the aspect defines the
/// product shape. `None` is written as that third state rather than
/// being collapsed to false, which would assert something untrue.
///
/// # Errors
///
/// Refuses an empty `shape_representations` list, which is bounded
/// `LIST [1:?]`: an aspect representing nothing names nothing.
pub fn shape_aspect(
    tx: &mut Transaction,
    shape_representations: &[EntityId],
    name: Option<&str>,
    description: Option<&str>,
    product_definitional: Option<bool>,
    part_of_product_definition_shape: Option<EntityId>,
) -> Result<EntityId, GeometryError> {
    const ENTITY: &str = "IFCSHAPEASPECT";
    if shape_representations.is_empty() {
        return Err(invalid(
            ENTITY,
            "ShapeRepresentations",
            "expected at least one representation, per LIST [1:?]",
        ));
    }
    let attrs = vec![
        refs(shape_representations),
        name.map_or(Value::Null, |v| Value::Text(v.into())),
        description.map_or(Value::Null, |v| Value::Text(v.into())),
        product_definitional.map_or(Value::LogicalUnknown, Value::Bool),
        part_of_product_definition_shape.map_or(Value::Null, Value::Ref),
    ];
    Ok(tx.create(Entity::new(ENTITY, attrs)))
}

/// Stage an `IfcGrid`: the U/V/W axis system a plan is dimensioned
/// against.
///
/// `UAxes` and `VAxes` are `LIST [1:?] OF UNIQUE`, so a grid needs at
/// least one axis in each direction and may not list the same axis
/// twice. A repeated axis is not a harmless duplicate: it makes the
/// grid ambiguous about which intersection a gridline names.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty U or V list, and a repeated
/// axis within any one list.
pub fn grid(
    tx: &mut Transaction,
    global_id: &str,
    placement: Option<EntityId>,
    axes: (&[EntityId], &[EntityId], &[EntityId]),
    predefined_type: Option<&str>,
) -> Result<EntityId, GeometryError> {
    const ENTITY: &str = "IFCGRID";
    if Guid::parse(global_id).is_none() {
        return Err(invalid(ENTITY, "GlobalId", global_id));
    }
    let (u_axes, v_axes, w_axes) = axes;
    for (values, attribute) in [(u_axes, "UAxes"), (v_axes, "VAxes")] {
        if values.is_empty() {
            return Err(invalid(
                ENTITY,
                attribute,
                "expected at least one axis, per LIST [1:?]",
            ));
        }
    }
    for (values, attribute) in [(u_axes, "UAxes"), (v_axes, "VAxes"), (w_axes, "WAxes")] {
        let mut seen = std::collections::HashSet::new();
        if let Some(repeat) = values.iter().find(|axis| !seen.insert(**axis)) {
            return Err(invalid(
                ENTITY,
                attribute,
                format!("axis {repeat:?} appears twice in a UNIQUE list"),
            ));
        }
    }
    let mut attrs = vec![Value::Null; 11];
    attrs[0] = Value::Text(global_id.into());
    attrs[5] = placement.map_or(Value::Null, Value::Ref);
    attrs[7] = refs(u_axes);
    attrs[8] = refs(v_axes);
    if !w_axes.is_empty() {
        attrs[9] = refs(w_axes);
    }
    attrs[10] = predefined_type.map_or(Value::Null, |t| Value::Enum(t.into()));
    Ok(tx.create(Entity::new(ENTITY, attrs)))
}
