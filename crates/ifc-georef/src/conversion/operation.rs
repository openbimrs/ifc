//! The resolved project-to-map value and the entry points that dispatch
//! every `IfcCoordinateOperation` subtype.
//!
//! | Subtype | Release | Lowered by |
//! | --- | --- | --- |
//! | `IfcMapConversion` | IFC4, IFC4X3 | `map.rs` |
//! | `IfcMapConversionScaled` | IFC4X3 | `map.rs`, per-axis factors |
//! | `IfcRigidOperation`, length coordinates | IFC4X3 | `rigid.rs`, a translation |
//! | `IfcRigidOperation`, plane-angle coordinates | IFC4X3 | refused here; [`crate::resolve_geographic_offset_in`] |
//!
//! A plane-angle rigid operation offsets latitude and longitude on an
//! `IfcGeographicCRS`. No metre-to-metre affine transform expresses that,
//! so it is refused with [`GeorefError::CoordinateMeasureMismatch`] rather
//! than approximated.
//!
//! # Kernel-free parameters
//!
//! [`ProjectToMap`] carries the resolved operation as plain `f64` (its
//! authored eastings, northings and height, and the metre-to-metre linear
//! part and translation behind [`ProjectToMap::map_point`]). The
//! `axiolid_core::Transform3` view of the same operation exists only with
//! the default `transform` feature, so a semantic consumer can read every
//! parameter without linking a geometry kernel (#268).

#[cfg(feature = "transform")]
use axiolid_core::Transform3;
use ifc_model::value::Value;
use ifc_model::{Entity, EntityId, Model};

use crate::context::OperationSource;
use crate::crs::{LengthUnit, ProjectedCrs};
use crate::error::{GeorefError, GeorefResult};
use crate::view::GeorefView;

use super::{map, rigid};

/// Which coordinate-operation subtype a [`ProjectToMap`] was lowered from.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum OperationKind {
    /// `IfcMapConversion`.
    MapConversion,
    /// IFC4X3 `IfcMapConversionScaled`, with its `(FactorX, FactorY,
    /// FactorZ)` as authored. They are folded into `transform`.
    MapConversionScaled {
        /// `(FactorX, FactorY, FactorZ)`.
        factors: (f64, f64, f64),
    },
    /// IFC4X3 `IfcRigidOperation` with `IfcLengthMeasure` coordinates.
    RigidOperation {
        /// `Height` in map units as authored. `None` when the file states
        /// no height: the transform then shifts heights by nothing, and
        /// this records that no vertical offset was declared.
        height: Option<f64>,
    },
}

/// A resolved project-to-map coordinate operation, normalised to metres.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ProjectToMap {
    /// The operation's `SourceCRS`, an
    /// `IfcCoordinateReferenceSystemSelect`: an
    /// `IfcGeometricRepresentationContext` or an
    /// `IfcCoordinateReferenceSystem`, in IFC4 and IFC4X3 alike. `source`
    /// says which.
    pub source_crs: EntityId,
    /// The validated source, typed.
    pub source: OperationSource,
    /// The coordinate-operation entity this was resolved from.
    pub operation: EntityId,
    /// Which subtype `operation` is, with its subtype-only attributes.
    pub kind: OperationKind,
    /// The target projected CRS.
    pub target_crs: ProjectedCrs,
    /// Affine operation from neutral project metres to neutral map metres.
    ///
    /// Only with the default `transform` feature. The same operation is
    /// always available as plain numbers through [`Self::map_point`],
    /// [`Self::linear_part`] and [`Self::translation`].
    #[cfg(feature = "transform")]
    pub transform: Transform3,
    /// `Eastings` as authored, in `map_unit`. For a rigid operation,
    /// `FirstCoordinate`, the offset along the target CRS's first axis.
    pub eastings: f64,
    /// `Northings` as authored, in `map_unit`. For a rigid operation,
    /// `SecondCoordinate`, the offset along the target CRS's second axis.
    pub northings: f64,
    /// `OrthogonalHeight` as authored, in `map_unit`. For a rigid
    /// operation, `Height`, or `0.0` when the file states none (see
    /// [`OperationKind::RigidOperation`]).
    pub orthogonal_height: f64,
    /// Length unit the project authored its coordinates in.
    pub project_unit: LengthUnit,
    /// Length unit the map coordinates are expressed in.
    pub map_unit: LengthUnit,
    /// IFC's declared scale before source/target unit normalization. `1.0`
    /// for a rigid operation, which has none.
    pub declared_scale: f64,
    /// Normalized `(XAxisAbscissa, XAxisOrdinate)`: the project's local X
    /// axis, as a unit vector, expressed in the map's XY plane. Exposed
    /// (rather than only folded into `transform`) because grid-north
    /// resolution needs the rotation alone, without `transform`'s scale
    /// and translation. `(1, 0)` for a rigid operation.
    pub x_axis_direction: (f64, f64),
    /// Linear part, metres to metres, as three columns (the images of the
    /// project's X, Y and Z axes).
    linear: [[f64; 3]; 3],
    /// Translation, map metres: the map position of the project origin.
    translation: [f64; 3],
}

impl ProjectToMap {
    /// Assemble the resolved value from its metre-to-metre affine parts,
    /// deriving the `Transform3` view when the feature is on.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        source: OperationSource,
        operation: EntityId,
        kind: OperationKind,
        target_crs: ProjectedCrs,
        authored: [f64; 3],
        units: (LengthUnit, LengthUnit),
        declared_scale: f64,
        x_axis_direction: (f64, f64),
        linear: [[f64; 3]; 3],
        translation: [f64; 3],
    ) -> Self {
        let (project_unit, map_unit) = units;
        Self {
            source_crs: source.entity(),
            source,
            operation,
            kind,
            target_crs,
            #[cfg(feature = "transform")]
            transform: transform3(&linear, &translation),
            eastings: authored[0],
            northings: authored[1],
            orthogonal_height: authored[2],
            project_unit,
            map_unit,
            declared_scale,
            x_axis_direction,
            linear,
            translation,
        }
    }

    /// Carry a point from project metres to map metres.
    ///
    /// Plain arithmetic on the resolved parameters, identical to
    /// `transform.transform_point3` (same operation order, so the same
    /// rounding) and available without the `transform` feature.
    #[must_use]
    pub fn map_point(&self, point: [f64; 3]) -> [f64; 3] {
        let [c0, c1, c2] = self.linear;
        let t = self.translation;
        std::array::from_fn(|i| c0[i] * point[0] + c1[i] * point[1] + c2[i] * point[2] + t[i])
    }

    /// The linear part, metres to metres, as three columns: the images of
    /// the project's unit X, Y and Z axes. Scale, unit conversion, per-axis
    /// factors and rotation are folded in.
    #[must_use]
    pub fn linear_part(&self) -> [[f64; 3]; 3] {
        self.linear
    }

    /// The translation in map metres: where the project origin lands.
    /// `[eastings, northings, orthogonal_height]` converted from
    /// `map_unit` to metres.
    #[must_use]
    pub fn translation(&self) -> [f64; 3] {
        self.translation
    }
}

/// The `Transform3` view of an affine operation held as plain numbers.
#[cfg(feature = "transform")]
fn transform3(linear: &[[f64; 3]; 3], translation: &[f64; 3]) -> Transform3 {
    use axiolid_core::{Mat3, Vec3};
    let column = |c: &[f64; 3]| Vec3::new(c[0], c[1], c[2]);
    Transform3::from_mat3_translation(
        Mat3::from_cols(column(&linear[0]), column(&linear[1]), column(&linear[2])),
        column(translation),
    )
}

/// Resolve a project-to-map coordinate operation and normalize both frames
/// to metres.
///
/// `project_metres_per_unit` is the project's `IfcUnitAssignment` length scale.
/// It is explicit here because project units are owned by the caller's model
/// loading boundary, while `MapUnit` is owned by the target CRS.
///
/// When the header pins IFC4 or IFC4X3 this is
/// [`resolve_project_to_map_in`] on that view, so an IFC4X3-only entity in
/// an IFC4 file is refused as undeclared. Without a usable header the
/// IFC4X3 set is accepted unpinned: the attribute layouts coincide (see
/// `view.rs`), but "not declared in this schema" cannot be told apart.
pub fn resolve_project_to_map(
    model: &Model,
    id: EntityId,
    project_metres_per_unit: f64,
) -> GeorefResult<ProjectToMap> {
    match GeorefView::for_model(model) {
        Ok(view) => resolve_project_to_map_in(&view, id, project_metres_per_unit),
        Err(_) => dispatch(model, None, id, project_metres_per_unit),
    }
}

/// Resolve a project-to-map coordinate operation within a schema-pinned
/// [`GeorefView`].
///
/// Refuses an entity the pinned schema does not declare
/// (`IfcMapConversionScaled`, `IfcRigidOperation` or `IfcGeographicCRS`
/// read from an IFC4 file) with [`GeorefError::UnsupportedOperation`]
/// naming the release, and checks the operation's source against the
/// pinned release's `IfcCoordinateReferenceSystemSelect`.
pub fn resolve_project_to_map_in(
    view: &GeorefView,
    id: EntityId,
    project_metres_per_unit: f64,
) -> GeorefResult<ProjectToMap> {
    view.require_known_type(id)?;
    dispatch(view.model, Some(view), id, project_metres_per_unit)
}

fn dispatch(
    model: &Model,
    view: Option<&GeorefView>,
    id: EntityId,
    project_metres_per_unit: f64,
) -> GeorefResult<ProjectToMap> {
    let entity = model.get(id).ok_or(GeorefError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    if !project_metres_per_unit.is_finite() || project_metres_per_unit <= 0.0 {
        return Err(GeorefError::InvalidUnit {
            entity: id,
            detail: "project length scale must be finite and positive",
        });
    }
    let project_unit = LengthUnit {
        name: "PROJECT_LENGTH_UNIT".into(),
        metres_per_unit: project_metres_per_unit,
    };
    let actual_type = entity.type_name.to_ascii_uppercase();
    let operation = Operation {
        model,
        view,
        id,
        entity,
    };
    match actual_type.as_str() {
        "IFCMAPCONVERSION" => map::lower(&operation, project_unit, false),
        "IFCMAPCONVERSIONSCALED" => map::lower(&operation, project_unit, true),
        "IFCRIGIDOPERATION" => rigid::lower(&operation, project_unit),
        _ => Err(GeorefError::WrongType {
            entity: id,
            expected: "IFCCOORDINATEOPERATION",
            actual: entity.type_name.to_string(),
        }),
    }
}

/// The operation being lowered, with the view that pinned it (if any).
pub(super) struct Operation<'m, 'v> {
    pub model: &'m Model,
    pub view: Option<&'v GeorefView<'m>>,
    pub id: EntityId,
    pub entity: &'m Entity,
}

impl Operation<'_, '_> {
    pub fn required_ref(&self, index: usize, name: &'static str) -> GeorefResult<EntityId> {
        self.entity
            .reference(index)
            .ok_or(GeorefError::MissingAttribute {
                entity: self.id,
                index,
                name,
            })
    }

    pub fn required_number(&self, index: usize, name: &'static str) -> GeorefResult<f64> {
        self.entity
            .attribute(index)
            .and_then(|v| v.unwrap_typed().as_f64())
            .ok_or(GeorefError::MissingAttribute {
                entity: self.id,
                index,
                name,
            })
    }

    pub fn optional_number(&self, index: usize, name: &'static str) -> GeorefResult<Option<f64>> {
        match self.entity.attribute(index).map(Value::unwrap_typed) {
            None | Some(Value::Null) => Ok(None),
            Some(value) => value
                .as_f64()
                .map(Some)
                .ok_or(GeorefError::InvalidAttribute {
                    entity: self.id,
                    index,
                    name,
                }),
        }
    }

    /// Refuse a non-finite value, naming the slot it came from.
    pub fn finite(&self, index: usize, name: &'static str, value: f64) -> GeorefResult<f64> {
        if value.is_finite() {
            Ok(value)
        } else {
            Err(GeorefError::InvalidAttribute {
                entity: self.id,
                index,
                name,
            })
        }
    }
}
