//! The remaining geometry `SELECT` types plus the two indexed defined types.
//!
//! Split from `entity_selects` to keep both files well under the 800-line cap
//! and because these share a theme: they are the selects used by curves on
//! surfaces, geometric sets, grids and indexed polycurves.

use crate::error::{GeometryError, GeometryResult};
use crate::select::subtype::is_a;
use ifc_model::{Entity, EntityId, Model, Value};

/// Resolve a reference then classify it, reporting dangling refs uniformly.
fn classify<T>(
    model: &Model,
    referrer: EntityId,
    target: EntityId,
    expected: &'static str,
    f: impl Fn(&str) -> Option<T>,
) -> GeometryResult<T> {
    let entity: &Entity = model.get(target).ok_or(GeometryError::MissingEntity {
        referrer,
        missing: target,
    })?;
    f(&entity.type_name).ok_or_else(|| GeometryError::WrongEntityType {
        entity: target,
        actual: entity.type_name.to_string(),
        expected,
    })
}

/// `IfcCurveOnSurface` = `IfcCompositeCurveOnSurface` | `IfcPcurve` |
/// `IfcSurfaceCurve`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveOnSurface {
    /// A composite curve whose segments all lie on the surface.
    Composite(EntityId),
    /// A curve defined in the surface's parameter space.
    PCurve(EntityId),
    /// A curve defined in 3D that lies on the surface.
    SurfaceCurve(EntityId),
}

impl CurveOnSurface {
    /// Classify the reference.
    pub fn resolve(model: &Model, referrer: EntityId, target: EntityId) -> GeometryResult<Self> {
        classify(model, referrer, target, "IfcCurveOnSurface", |t| {
            // Checked most-derived first: IfcCompositeCurveOnSurface is a
            // subtype of IfcCompositeCurve, not of the other two.
            if is_a(t, "IFCCOMPOSITECURVEONSURFACE") {
                Some(Self::Composite(target))
            } else if is_a(t, "IFCPCURVE") {
                Some(Self::PCurve(target))
            } else if is_a(t, "IFCSURFACECURVE") {
                Some(Self::SurfaceCurve(target))
            } else {
                None
            }
        })
    }

    /// The referenced entity.
    pub fn id(&self) -> EntityId {
        match self {
            Self::Composite(id) | Self::PCurve(id) | Self::SurfaceCurve(id) => *id,
        }
    }
}

/// `IfcCurveOrEdgeCurve` = `IfcBoundedCurve` | `IfcEdgeCurve`.
///
/// The topological branch matters: an `IfcEdgeCurve` carries orientation from
/// its topology, so a sweep along one may run opposite to the underlying
/// geometric curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveOrEdgeCurve {
    /// Pure geometry.
    Bounded(EntityId),
    /// Topological edge with an underlying curve.
    EdgeCurve(EntityId),
}

impl CurveOrEdgeCurve {
    /// Classify the reference.
    pub fn resolve(model: &Model, referrer: EntityId, target: EntityId) -> GeometryResult<Self> {
        classify(model, referrer, target, "IfcCurveOrEdgeCurve", |t| {
            if is_a(t, "IFCEDGECURVE") {
                Some(Self::EdgeCurve(target))
            } else if is_a(t, "IFCBOUNDEDCURVE") {
                Some(Self::Bounded(target))
            } else {
                None
            }
        })
    }

    /// The referenced entity.
    pub fn id(&self) -> EntityId {
        match self {
            Self::Bounded(id) | Self::EdgeCurve(id) => *id,
        }
    }
}

/// `IfcGeometricSetSelect` = `IfcCurve` | `IfcPoint` | `IfcSurface`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometricSetSelect {
    /// A curve element.
    Curve(EntityId),
    /// A point element.
    Point(EntityId),
    /// A surface element.
    Surface(EntityId),
}

impl GeometricSetSelect {
    /// Classify the reference.
    pub fn resolve(model: &Model, referrer: EntityId, target: EntityId) -> GeometryResult<Self> {
        classify(model, referrer, target, "IfcGeometricSetSelect", |t| {
            if is_a(t, "IFCCURVE") {
                Some(Self::Curve(target))
            } else if is_a(t, "IFCPOINT") {
                Some(Self::Point(target))
            } else if is_a(t, "IFCSURFACE") {
                Some(Self::Surface(target))
            } else {
                None
            }
        })
    }

    /// The referenced entity.
    pub fn id(&self) -> EntityId {
        match self {
            Self::Curve(id) | Self::Point(id) | Self::Surface(id) => *id,
        }
    }
}

/// `IfcSurfaceOrFaceSurface` = `IfcFaceBasedSurfaceModel` | `IfcFaceSurface` |
/// `IfcSurface`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceOrFaceSurface {
    /// A shell of faces.
    FaceBasedSurfaceModel(EntityId),
    /// A topological face carrying a surface.
    FaceSurface(EntityId),
    /// A pure geometric surface.
    Surface(EntityId),
}

impl SurfaceOrFaceSurface {
    /// Classify the reference.
    pub fn resolve(model: &Model, referrer: EntityId, target: EntityId) -> GeometryResult<Self> {
        classify(model, referrer, target, "IfcSurfaceOrFaceSurface", |t| {
            if is_a(t, "IFCFACEBASEDSURFACEMODEL") {
                Some(Self::FaceBasedSurfaceModel(target))
            } else if is_a(t, "IFCFACESURFACE") {
                Some(Self::FaceSurface(target))
            } else if is_a(t, "IFCSURFACE") {
                Some(Self::Surface(target))
            } else {
                None
            }
        })
    }

    /// The referenced entity.
    pub fn id(&self) -> EntityId {
        match self {
            Self::FaceBasedSurfaceModel(id) | Self::FaceSurface(id) | Self::Surface(id) => *id,
        }
    }
}

/// `IfcPointOrVertexPoint` = `IfcPoint` | `IfcVertexPoint`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointOrVertexPoint {
    /// A geometric point.
    Point(EntityId),
    /// A topological vertex carrying a point.
    VertexPoint(EntityId),
}

impl PointOrVertexPoint {
    /// Classify the reference.
    pub fn resolve(model: &Model, referrer: EntityId, target: EntityId) -> GeometryResult<Self> {
        classify(model, referrer, target, "IfcPointOrVertexPoint", |t| {
            if is_a(t, "IFCVERTEXPOINT") {
                Some(Self::VertexPoint(target))
            } else if is_a(t, "IFCPOINT") {
                Some(Self::Point(target))
            } else {
                None
            }
        })
    }

    /// The referenced entity.
    pub fn id(&self) -> EntityId {
        match self {
            Self::Point(id) | Self::VertexPoint(id) => *id,
        }
    }
}

/// `IfcGridPlacementDirectionSelect` = `IfcDirection` |
/// `IfcVirtualGridIntersection`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridPlacementDirectionSelect {
    /// An explicit direction.
    Direction(EntityId),
    /// A second grid intersection to point at.
    GridIntersection(EntityId),
}

impl GridPlacementDirectionSelect {
    /// Classify the reference.
    pub fn resolve(model: &Model, referrer: EntityId, target: EntityId) -> GeometryResult<Self> {
        classify(
            model,
            referrer,
            target,
            "IfcGridPlacementDirectionSelect",
            |t| {
                if is_a(t, "IFCVIRTUALGRIDINTERSECTION") {
                    Some(Self::GridIntersection(target))
                } else if is_a(t, "IFCDIRECTION") {
                    Some(Self::Direction(target))
                } else {
                    None
                }
            },
        )
    }

    /// The referenced entity.
    pub fn id(&self) -> EntityId {
        match self {
            Self::Direction(id) | Self::GridIntersection(id) => *id,
        }
    }
}

/// `IfcLineIndex` = `LIST [2:?] OF IfcPositiveInteger`.
///
/// # 1-based, and that is not a detail
///
/// Indices point into an `IfcCartesianPointList`, and EXPRESS aggregates are
/// 1-based. Rust slices are 0-based. Every index must be decremented exactly
/// once, and doing it twice or not at all shifts the entire polyline by one
/// vertex without any error being raised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineIndex(Vec<usize>);

/// `IfcArcIndex` = `LIST [3:3] OF IfcPositiveInteger`.
///
/// Exactly three: start, a point on the arc, and end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArcIndex([usize; 3]);

/// One entry of `IfcIndexedPolyCurve.Segments`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegmentIndexSelect {
    /// A straight run through two or more points.
    Line(LineIndex),
    /// A circular arc through exactly three points.
    Arc(ArcIndex),
}

impl LineIndex {
    /// Zero-based indices into the point list.
    pub fn as_zero_based(&self) -> &[usize] {
        &self.0
    }

    /// How many points this run covers.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Is the run empty? Never true for a schema-valid index.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl ArcIndex {
    /// Zero-based `[start, mid, end]`.
    pub fn as_zero_based(&self) -> [usize; 3] {
        self.0
    }
}

impl SegmentIndexSelect {
    /// Parse one `IfcSegmentIndexSelect` value.
    ///
    /// The discriminator is the typed wrapper written by the codec:
    /// `IFCLINEINDEX((1,2,3))` versus `IFCARCINDEX((3,4,5))`. Without the
    /// wrapper the two are indistinguishable lists of integers, so an
    /// untagged value is rejected rather than guessed.
    pub fn from_value(entity: EntityId, value: &Value) -> GeometryResult<Self> {
        let (tag, inner) = match value {
            Value::Typed { type_name, value } => (type_name.to_ascii_uppercase(), value.as_ref()),
            _ => {
                return Err(GeometryError::Degenerate {
                    entity,
                    type_name: "IFCSEGMENTINDEXSELECT".into(),
                    detail: "segment is not tagged IFCLINEINDEX or IFCARCINDEX, so its \
                             kind cannot be determined"
                        .into(),
                })
            }
        };

        let raw = inner.as_list().ok_or_else(|| GeometryError::Degenerate {
            entity,
            type_name: tag.clone(),
            detail: "segment index is not a list".into(),
        })?;

        // EXPRESS aggregates are 1-based; convert exactly once, here.
        let mut indices = Vec::with_capacity(raw.len());
        for value in raw {
            let n = match value.unwrap_typed() {
                Value::Integer(i) => *i,
                other => {
                    return Err(GeometryError::Degenerate {
                        entity,
                        type_name: tag.clone(),
                        detail: format!("segment index is not an integer: {other:?}"),
                    })
                }
            };
            if n < 1 {
                return Err(GeometryError::Degenerate {
                    entity,
                    type_name: tag.clone(),
                    detail: format!("index {n} is not a positive integer (EXPRESS is 1-based)"),
                });
            }
            indices.push((n - 1) as usize);
        }

        match tag.as_str() {
            "IFCARCINDEX" => {
                let three: [usize; 3] =
                    indices
                        .as_slice()
                        .try_into()
                        .map_err(|_| GeometryError::Degenerate {
                            entity,
                            type_name: tag.clone(),
                            detail: format!(
                                "IfcArcIndex requires exactly 3 indices, found {}",
                                indices.len()
                            ),
                        })?;
                Ok(Self::Arc(ArcIndex(three)))
            }
            "IFCLINEINDEX" => {
                if indices.len() < 2 {
                    return Err(GeometryError::Degenerate {
                        entity,
                        type_name: tag,
                        detail: format!(
                            "IfcLineIndex requires at least 2 indices, found {}",
                            indices.len()
                        ),
                    });
                }
                Ok(Self::Line(LineIndex(indices)))
            }
            other => Err(GeometryError::Degenerate {
                entity,
                type_name: other.to_string(),
                detail: "not a member of IfcSegmentIndexSelect".into(),
            }),
        }
    }

    /// Zero-based indices, whichever branch this is.
    pub fn indices(&self) -> Vec<usize> {
        match self {
            Self::Line(l) => l.0.clone(),
            Self::Arc(a) => a.0.to_vec(),
        }
    }
}

/// `IfcDimensionCount` = `INTEGER` with `WHERE WR1: { 0 < SELF <= 3 }`.
///
/// The where-rule is the whole point of the type, so it is enforced at
/// construction and the invalid states are unrepresentable thereafter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DimensionCount(u8);

impl DimensionCount {
    /// Construct, enforcing `0 < count <= 3`.
    pub fn new(count: i64) -> Option<Self> {
        (1..=3).contains(&count).then_some(Self(count as u8))
    }

    /// The dimension as a number.
    pub fn get(&self) -> usize {
        self.0 as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_with(id: u64, type_name: &str) -> Model {
        let mut model = Model::new();
        model.insert(EntityId(id), Entity::new(type_name, vec![Value::Null; 4]));
        model
    }

    fn tagged(tag: &str, values: &[i64]) -> Value {
        Value::Typed {
            type_name: tag.into(),
            value: Box::new(Value::List(
                values.iter().map(|v| Value::Integer(*v)).collect(),
            )),
        }
    }

    /// The off-by-one that silently shifts every polyline.
    #[test]
    fn one_based_express_indices_become_zero_based_exactly_once() {
        let seg = SegmentIndexSelect::from_value(EntityId(1), &tagged("IFCLINEINDEX", &[1, 2, 3]))
            .unwrap();
        assert_eq!(
            seg.indices(),
            vec![0, 1, 2],
            "IfcLineIndex((1,2,3)) addresses points 0,1,2"
        );
    }

    #[test]
    fn arc_indices_require_exactly_three_points() {
        let ok = SegmentIndexSelect::from_value(EntityId(1), &tagged("IFCARCINDEX", &[3, 4, 5]));
        assert_eq!(ok.unwrap().indices(), vec![2, 3, 4]);

        let short = SegmentIndexSelect::from_value(EntityId(1), &tagged("IFCARCINDEX", &[3, 4]));
        assert!(short.is_err(), "two indices cannot define an arc");
    }

    #[test]
    fn line_indices_require_at_least_two_points() {
        assert!(
            SegmentIndexSelect::from_value(EntityId(1), &tagged("IFCLINEINDEX", &[7])).is_err()
        );
    }

    /// Index 0 is invalid in a 1-based schema and would underflow.
    #[test]
    fn a_zero_index_is_rejected_rather_than_underflowing() {
        let err = SegmentIndexSelect::from_value(EntityId(1), &tagged("IFCLINEINDEX", &[0, 1]))
            .unwrap_err();
        assert!(err.to_string().contains("1-based"), "got {err}");
    }

    /// Untagged lists are ambiguous between line and arc.
    #[test]
    fn an_untagged_segment_is_rejected_rather_than_guessed() {
        let bare = Value::List(vec![Value::Integer(1), Value::Integer(2)]);
        assert!(SegmentIndexSelect::from_value(EntityId(1), &bare).is_err());
    }

    #[test]
    fn dimension_count_enforces_its_where_rule() {
        assert_eq!(DimensionCount::new(3).map(|d| d.get()), Some(3));
        assert_eq!(DimensionCount::new(1).map(|d| d.get()), Some(1));
        assert_eq!(DimensionCount::new(0), None, "0 < SELF is required");
        assert_eq!(DimensionCount::new(4), None, "SELF <= 3 is required");
        assert_eq!(DimensionCount::new(-1), None);
    }

    #[test]
    fn geometric_set_members_classify_by_family() {
        let curve = model_with(5, "IFCPOLYLINE");
        assert_eq!(
            GeometricSetSelect::resolve(&curve, EntityId(1), EntityId(5)).unwrap(),
            GeometricSetSelect::Curve(EntityId(5))
        );
        let surface = model_with(5, "IFCPLANE");
        assert_eq!(
            GeometricSetSelect::resolve(&surface, EntityId(1), EntityId(5)).unwrap(),
            GeometricSetSelect::Surface(EntityId(5))
        );
        let point = model_with(5, "IFCCARTESIANPOINT");
        assert_eq!(
            GeometricSetSelect::resolve(&point, EntityId(1), EntityId(5)).unwrap(),
            GeometricSetSelect::Point(EntityId(5))
        );
    }

    /// A composite curve on a surface must not classify as a plain surface
    /// curve; the most derived branch wins.
    #[test]
    fn curve_on_surface_picks_the_most_derived_branch() {
        let model = model_with(5, "IFCCOMPOSITECURVEONSURFACE");
        assert_eq!(
            CurveOnSurface::resolve(&model, EntityId(1), EntityId(5)).unwrap(),
            CurveOnSurface::Composite(EntityId(5))
        );
    }
}
