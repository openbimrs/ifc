//! Composing a project frame with a resolved project-to-map operation.
//!
//! [`crate::resolve_project_to_map`] produces the operation IFC declares:
//! project engineering coordinates to map coordinates. It does not know, and
//! must not know, where in the project a given product actually sits --
//! that is `IfcLocalPlacement` chain resolution, which is `ifc-geometry`'s
//! job (see this crate's `AGENTS.md`: "never place individual products").
//!
//! A caller that *has* resolved a product's placement down to a single
//! neutral transform (its "project frame": the product's placement relative
//! to the project's `IfcGeometricRepresentationContext.WorldCoordinateSystem`)
//! can compose it with the project-to-map operation here to get that
//! product's map-frame transform. This module owns exactly that
//! composition -- it takes the project frame as an opaque, already-resolved
//! `Transform3` rather than reaching into `ifc-geometry` for it, which is
//! what keeps the crate boundary intact.

use axiolid_core::Transform3;

use crate::conversion::ProjectToMap;
use crate::error::{GeorefError, GeorefResult};

/// Compose an externally supplied project frame with a resolved
/// project-to-map operation, producing the project frame's map-frame
/// transform.
///
/// `project_frame` maps a point from some product-local space into the
/// project's own engineering coordinate system (metres, matching
/// `operation.project_unit`). The result maps that same point straight into
/// map coordinates: `map_frame = operation.transform * project_frame`, i.e.
/// `project_frame` is applied first.
///
/// Refuses (rather than silently propagating `NaN`/`Inf`) when
/// `project_frame`'s linear part is not invertible: a singular frame means
/// the supplied placement collapsed a dimension, and composing it here would
/// produce map coordinates that look plausible but encode no real position.
pub fn compose_project_frame(
    operation: &ProjectToMap,
    project_frame: Transform3,
) -> GeorefResult<Transform3> {
    let determinant = project_frame.matrix3.determinant();
    if !determinant.is_finite() || determinant.abs() <= f64::EPSILON {
        return Err(GeorefError::DegenerateProjectFrame);
    }
    Ok(operation.transform * project_frame)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axiolid_core::{Mat3, Point3, Vec3};
    use ifc_model::value::Value;
    use ifc_model::{Entity, EntityId, Model};

    use super::*;
    use crate::conversion::resolve_project_to_map;

    fn id(value: u64) -> EntityId {
        EntityId(value)
    }

    fn r(value: u64) -> Value {
        Value::Ref(id(value))
    }

    fn real(value: f64) -> Value {
        Value::Real(value)
    }

    fn model_with_map_conversion() -> Model {
        let mut model = Model::new();
        model.insert(
            id(1),
            Entity::new("IFCGEOMETRICREPRESENTATIONCONTEXT", vec![]),
        );
        model.insert(
            id(2),
            Entity::new(
                "IFCPROJECTEDCRS",
                vec![Value::Text(Arc::from("EPSG:25832"))],
            ),
        );
        model.insert(
            id(4),
            Entity::new(
                "IFCMAPCONVERSION",
                vec![
                    r(1),
                    r(2),
                    real(1000.0),
                    real(2000.0),
                    real(50.0),
                    Value::Null,
                    Value::Null,
                    real(1.0),
                ],
            ),
        );
        model
    }

    #[test]
    fn composes_a_translated_project_frame_into_map_coordinates() {
        let model = model_with_map_conversion();
        let operation = resolve_project_to_map(&model, id(4), 1.0).expect("resolves");

        // The project frame places a product's local origin at (10, 20, 0)
        // in project engineering coordinates.
        let project_frame =
            Transform3::from_mat3_translation(Mat3::IDENTITY, Vec3::new(10.0, 20.0, 0.0));
        let map_frame = compose_project_frame(&operation, project_frame).expect("composes");

        let mapped = map_frame.transform_point3(Point3::ZERO);
        assert_eq!(mapped, Vec3::new(1010.0, 2020.0, 50.0));
    }

    #[test]
    fn refuses_a_singular_project_frame() {
        let model = model_with_map_conversion();
        let operation = resolve_project_to_map(&model, id(4), 1.0).expect("resolves");

        // A degenerate frame that collapses the Y axis to zero.
        let singular = Transform3::from_mat3_translation(
            Mat3::from_cols(
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, 1.0),
            ),
            Vec3::ZERO,
        );

        assert!(matches!(
            compose_project_frame(&operation, singular),
            Err(GeorefError::DegenerateProjectFrame)
        ));
    }

    #[test]
    fn composition_order_applies_the_project_frame_before_the_map_operation() {
        let model = model_with_map_conversion();
        let operation = resolve_project_to_map(&model, id(4), 1.0).expect("resolves");

        // A project frame that rotates 90 degrees about Z before translating.
        let rotated = Transform3::from_mat3_translation(
            Mat3::from_cols(Vec3::new(0.0, 1.0, 0.0), Vec3::new(-1.0, 0.0, 0.0), Vec3::Z),
            Vec3::new(5.0, 0.0, 0.0),
        );
        let map_frame = compose_project_frame(&operation, rotated).expect("composes");

        // A point one unit along local X ends up rotated, then translated by
        // the frame, then carried through the (unrotated) map conversion.
        let mapped = map_frame.transform_point3(Point3::new(1.0, 0.0, 0.0));
        assert_eq!(mapped, Vec3::new(1005.0, 2001.0, 50.0));
    }
}
