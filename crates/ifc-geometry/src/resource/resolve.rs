//! Resolve a raw reference into a type-checked resource view.
//!
//! Curve, surface and solid views keep their raw `*_ref` getters, because a
//! caller walking the graph often wants the id alone. The resolving accessors
//! beside them all need the same two checks -- the reference is not dangling,
//! and it names an entity of the declared type -- and doing those checks in one
//! place keeps the errors identical everywhere:
//!
//! - a dangling reference is [`GeometryError::MissingEntity`] naming the
//!   referrer, so the error points at the record that is wrong;
//! - a reference to the wrong type is [`GeometryError::WrongEntityType`]
//!   naming the target, never a view that silently misreads its slots.
//!
//! Only entity types are checked here. A view's own slot errors (a 1-element
//! point, a zero direction) still surface when the caller reads it.
//!
//! An attribute typed by an abstract supertype (`IfcSurface`,
//! `IfcBoundedCurve`) has no resolving accessor: it stays a reference,
//! because choosing among the concrete subtypes is dispatch, and dispatch
//! belongs to `lower`. `IfcProfileDef` has no view here either;
//! `input::profile` owns profile reading.

use crate::error::{GeometryError, GeometryResult};
use crate::resource::direction::Direction;
use crate::resource::placement::{
    Axis1Placement, Axis2Placement, Axis2Placement2D, Axis2Placement3D,
};
use crate::resource::point::{
    CartesianPoint, CartesianPointList, CartesianPointList2D, CartesianPointList3D,
};
use ifc_model::{Entity, EntityId, Model};

/// Resolve a reference that must be an `IfcCartesianPoint`.
pub fn cartesian_point<'m>(
    model: &'m Model,
    referrer: EntityId,
    id: EntityId,
) -> GeometryResult<CartesianPoint<'m>> {
    let entity = typed(
        model,
        referrer,
        id,
        &["IFCCARTESIANPOINT"],
        "IfcCartesianPoint",
    )?;
    Ok(CartesianPoint::new(id, entity))
}

/// Resolve every reference of a `LIST OF IfcCartesianPoint`, in order.
///
/// Stops at the first bad reference: a polyline with one dangling vertex has
/// no defined shape, so a partial list would be a silent substitute.
pub fn cartesian_points<'m>(
    model: &'m Model,
    referrer: EntityId,
    ids: &[EntityId],
) -> GeometryResult<Vec<CartesianPoint<'m>>> {
    ids.iter()
        .map(|&id| cartesian_point(model, referrer, id))
        .collect()
}

/// Resolve a reference that must be an `IfcCartesianPointList3D`.
pub fn cartesian_point_list_3d<'m>(
    model: &'m Model,
    referrer: EntityId,
    id: EntityId,
) -> GeometryResult<CartesianPointList3D<'m>> {
    let entity = typed(
        model,
        referrer,
        id,
        &["IFCCARTESIANPOINTLIST3D"],
        "IfcCartesianPointList3D",
    )?;
    Ok(CartesianPointList3D::new(id, entity))
}

/// Resolve a reference typed as the abstract `IfcCartesianPointList`.
///
/// The concrete subtype decides the row width, so it is carried in the
/// returned enum instead of being guessed from the first row.
pub fn cartesian_point_list<'m>(
    model: &'m Model,
    referrer: EntityId,
    id: EntityId,
) -> GeometryResult<CartesianPointList<'m>> {
    let entity = typed(
        model,
        referrer,
        id,
        &["IFCCARTESIANPOINTLIST2D", "IFCCARTESIANPOINTLIST3D"],
        "IfcCartesianPointList2D or IfcCartesianPointList3D",
    )?;
    Ok(if entity.is_type("IFCCARTESIANPOINTLIST2D") {
        CartesianPointList::TwoD(CartesianPointList2D::new(id, entity))
    } else {
        CartesianPointList::ThreeD(CartesianPointList3D::new(id, entity))
    })
}

/// Resolve a reference that must be an `IfcDirection`.
///
/// The view keeps the raw ratios; call [`Direction::unit`] to normalize.
pub fn direction<'m>(
    model: &'m Model,
    referrer: EntityId,
    id: EntityId,
) -> GeometryResult<Direction<'m>> {
    let entity = typed(model, referrer, id, &["IFCDIRECTION"], "IfcDirection")?;
    Ok(Direction::new(id, entity))
}

/// Resolve a reference that must be an `IfcAxis1Placement`.
pub fn axis1_placement<'m>(
    model: &'m Model,
    referrer: EntityId,
    id: EntityId,
) -> GeometryResult<Axis1Placement<'m>> {
    let entity = typed(
        model,
        referrer,
        id,
        &["IFCAXIS1PLACEMENT"],
        "IfcAxis1Placement",
    )?;
    Ok(Axis1Placement::new(id, entity))
}

/// Resolve a reference that must be an `IfcAxis2Placement3D`.
pub fn axis2_placement_3d<'m>(
    model: &'m Model,
    referrer: EntityId,
    id: EntityId,
) -> GeometryResult<Axis2Placement3D<'m>> {
    let entity = typed(
        model,
        referrer,
        id,
        &["IFCAXIS2PLACEMENT3D"],
        "IfcAxis2Placement3D",
    )?;
    Ok(Axis2Placement3D::new(id, entity))
}

/// Resolve a reference typed as the `IfcAxis2Placement` SELECT.
///
/// `IfcAxis1Placement` is rejected, as in
/// [`crate::resource::placement::axis_placement_transform`]: it has no local
/// X, so it is not a member of the select.
pub fn axis2_placement<'m>(
    model: &'m Model,
    referrer: EntityId,
    id: EntityId,
) -> GeometryResult<Axis2Placement<'m>> {
    let entity = typed(
        model,
        referrer,
        id,
        &["IFCAXIS2PLACEMENT2D", "IFCAXIS2PLACEMENT3D"],
        "IfcAxis2Placement2D or IfcAxis2Placement3D",
    )?;
    Ok(if entity.is_type("IFCAXIS2PLACEMENT2D") {
        Axis2Placement::TwoD(Axis2Placement2D::new(id, entity))
    } else {
        Axis2Placement::ThreeD(Axis2Placement3D::new(id, entity))
    })
}

/// Look `id` up and require one of `accepted` (upper-case STEP names).
fn typed<'m>(
    model: &'m Model,
    referrer: EntityId,
    id: EntityId,
    accepted: &[&str],
    expected: &'static str,
) -> GeometryResult<&'m Entity> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer,
        missing: id,
    })?;
    if accepted.iter().any(|name| entity.is_type(name)) {
        Ok(entity)
    } else {
        Err(GeometryError::WrongEntityType {
            entity: id,
            actual: entity.type_name.to_string(),
            expected,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::Value;

    fn reals(values: &[f64]) -> Value {
        Value::List(values.iter().copied().map(Value::Real).collect())
    }

    fn model() -> Model {
        let mut m = Model::new();
        m.insert(
            EntityId(1),
            Entity::new("IFCCARTESIANPOINT", vec![reals(&[1.0, 2.0])]),
        );
        m.insert(
            EntityId(2),
            Entity::new("IFCDIRECTION", vec![reals(&[1.0, 0.0])]),
        );
        m.insert(
            EntityId(3),
            Entity::new("IFCAXIS2PLACEMENT2D", vec![Value::Ref(EntityId(1))]),
        );
        m.insert(
            EntityId(4),
            Entity::new(
                "IFCCARTESIANPOINTLIST2D",
                vec![Value::List(vec![reals(&[0.0, 0.0])])],
            ),
        );
        m
    }

    #[test]
    fn select_resolution_keeps_the_concrete_subtype() {
        let m = model();
        let placement = axis2_placement(&m, EntityId(9), EntityId(3)).unwrap();
        assert!(matches!(placement, Axis2Placement::TwoD(_)));
        let list = cartesian_point_list(&m, EntityId(9), EntityId(4)).unwrap();
        assert_eq!(list.dimension(), 2);
    }

    #[test]
    fn a_dangling_reference_names_the_referrer_not_the_target() {
        let m = model();
        let err = direction(&m, EntityId(9), EntityId(99)).unwrap_err();
        assert!(matches!(
            err,
            GeometryError::MissingEntity {
                referrer: EntityId(9),
                missing: EntityId(99)
            }
        ));
    }

    #[test]
    fn a_wrong_type_names_the_target_and_the_expected_family() {
        let m = model();
        let err = axis2_placement(&m, EntityId(9), EntityId(2)).unwrap_err();
        assert!(matches!(
            err,
            GeometryError::WrongEntityType {
                entity: EntityId(2),
                expected: "IfcAxis2Placement2D or IfcAxis2Placement3D",
                ..
            }
        ));
    }

    /// One bad vertex fails the whole list rather than dropping the vertex.
    #[test]
    fn a_point_list_with_one_bad_reference_fails_as_a_whole() {
        let m = model();
        let err = cartesian_points(&m, EntityId(9), &[EntityId(1), EntityId(2)]).unwrap_err();
        assert!(matches!(err, GeometryError::WrongEntityType { .. }));
        assert_eq!(
            cartesian_points(&m, EntityId(9), &[EntityId(1)])
                .unwrap()
                .len(),
            1
        );
    }
}
