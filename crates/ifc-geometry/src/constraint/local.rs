//! `IfcLocalPlacement`: the nesting placement chain.
//!
//! # Semantics
//!
//! An `IfcLocalPlacement` has two attributes: `PlacementRelTo` (the parent
//! placement, optional) and `RelativePlacement` (an `IfcAxis2Placement3D` or
//! 2D giving the offset from that parent).
//!
//! **If `PlacementRelTo` is absent, the placement is absolute** in the project
//! coordinate system. That is the recursion's base case.
//!
//! # Two traps, both from the spec
//!
//! 1. **Cycles happen.** The IFC specification says outright that "rules to
//!    prevent cyclic relative placements have to be introduced on the
//!    application level" -- meaning the schema does not forbid them and real
//!    exporters have produced them. Naive recursion overflows the stack on a
//!    file that merely looks valid.
//!
//! 2. **Chains are walked per element.** A model with 100k elements walks
//!    100k chains that share their upper links. Resolving without a cache is
//!    quadratic in the depth; hence [`PlacementResolver`].

use crate::constraint::grid::resolve::grid_frame;
use crate::constraint::grid::GridPlacement;
use crate::constraint::placement::LinearResolution;
use crate::constraint::tolerance::frames_agree;
use crate::error::{GeometryError, GeometryResult};
use crate::resource::placement::axis_placement_transform;
use crate::slots::Slots;
use crate::transform::Transform;
use crate::units::UnitScale;
use ifc_model::{EntityId, Model};
use std::collections::HashMap;

/// `IfcLocalPlacement` attribute slots.
///
/// From IFC4 ADD2 TC1: `IfcLocalPlacement` has no inherited explicit
/// attributes (its supertype `IfcObjectPlacement` declares only the inverse
/// `PlacesObject`), so these indices are its own.
pub(crate) mod slot {
    /// `PlacementRelTo`: the parent placement, optional.
    pub const PLACEMENT_REL_TO: usize = 0;
    /// `RelativePlacement`: offset from the parent.
    pub const RELATIVE_PLACEMENT: usize = 1;
}

/// A borrowed view of an `IfcLocalPlacement`.
#[derive(Debug, Clone, Copy)]
pub struct LocalPlacement<'m> {
    slots: Slots<'m>,
}

impl<'m> LocalPlacement<'m> {
    /// Wrap an entity assumed to be an `IfcLocalPlacement`.
    pub fn new(id: EntityId, entity: &'m ifc_model::Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The parent placement, if any.
    ///
    /// `None` means this placement is absolute in project coordinates.
    pub fn parent(&self) -> Option<EntityId> {
        self.slots.opt_ref(slot::PLACEMENT_REL_TO)
    }

    /// The `IfcAxis2Placement` giving the offset from the parent.
    pub fn relative_placement(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(slot::RELATIVE_PLACEMENT, "RelativePlacement")
    }

    /// This placement's own offset, not including its parents.
    pub fn local_transform(&self, model: &'m Model) -> GeometryResult<Transform> {
        let placement_id = self.relative_placement()?;
        let entity = self.slots.resolve(model, placement_id)?;
        axis_placement_transform(model, placement_id, entity)
    }
}

/// How deep a placement chain may go before we call it malformed.
///
/// Real hierarchies are site > building > storey > element > opening, so
/// single digits. 64 leaves enormous headroom while still terminating on a
/// corrupt file quickly. The limit counts every placement the walk is
/// inside of, whatever its kind: a local placement relative to a grid
/// placement whose grid is placed relative to a storey is four deep.
const MAX_CHAIN_DEPTH: usize = 64;

/// Resolves placement chains to world transforms, with memoization.
///
/// # Why a resolver rather than a free function
///
/// Placement chains share their upper links: every element in a storey walks
/// the same storey-building-site tail. Caching per placement turns repeated
/// work into a lookup, which matters because this runs once per element in the
/// file.
///
/// # Every `IfcObjectPlacement` kind (#363)
///
/// IFC4.3 ADD2 declares `PlacementRelTo : OPTIONAL IfcObjectPlacement` on
/// `IfcObjectPlacement` itself, so any placement may be relative to any
/// other: a bracket on a linearly placed signal mast, a fixing on a column
/// placed on a grid. The walk resolves each link by its kind --
/// `IfcLocalPlacement` by its `RelativePlacement`, `IfcGridPlacement` by
/// its grid intersection in the grid's frame (#362),
/// `IfcLinearPlacement` along its basis curve in the alignment's frame
/// (#357) -- and caches each. Grid and linear placements reach their frames
/// through further placements (the grid's or the alignment's
/// `ObjectPlacement`), so the walk follows those too; the depth limit and
/// the cycle check cover the whole mixed chain.
///
/// A resolver's cache holds transforms resolved one way: the crate's entry
/// points use a fresh resolver per resolution mode, so a cached linear
/// placement never mixes a cache-only answer with a derived one.
#[derive(Debug, Default)]
pub struct PlacementResolver {
    cache: HashMap<EntityId, Transform>,
}

/// One resolution's inputs and the placements it is currently inside of.
struct Walk<'a, 'e> {
    model: &'a Model,
    units: &'a UnitScale,
    linear: LinearResolution<'e>,
    /// The placements being resolved, outermost first: a cycle revisits one.
    active: Vec<EntityId>,
}

impl PlacementResolver {
    /// A resolver with an empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// How many placements are memoized.
    pub fn cached(&self) -> usize {
        self.cache.len()
    }

    /// Resolve a placement to its world transform, in FILE units.
    ///
    /// Walks `PlacementRelTo` to the root, then composes downward. Detects
    /// cycles rather than overflowing the stack, and reports the entity where
    /// the cycle closes so the file can be repaired.
    ///
    /// Every placement kind resolves (#363): an `IfcGridPlacement` with
    /// straight axes, and an `IfcLinearPlacement` from its cached
    /// `CartesianPosition`. Deriving an uncached linear placement or
    /// intersecting curved grid axes needs a curve evaluator, which this
    /// entry point does not take: those are refused by name, and
    /// `product_world_transform_with_evaluator` (feature `compile`) resolves
    /// them.
    pub fn world_transform(
        &mut self,
        model: &Model,
        placement: EntityId,
    ) -> GeometryResult<Transform> {
        // Unit factors of one: a linear placement's metric frame comes back
        // in file units unchanged.
        self.resolve(
            model,
            &UnitScale::default(),
            placement,
            LinearResolution::cache_only(),
        )
    }

    /// [`Self::world_transform`] with the model's units and a linear
    /// resolution mode, in FILE units.
    pub(crate) fn resolve(
        &mut self,
        model: &Model,
        units: &UnitScale,
        placement: EntityId,
        linear: LinearResolution<'_>,
    ) -> GeometryResult<Transform> {
        let mut walk = Walk {
            model,
            units,
            linear,
            active: Vec::new(),
        };
        self.resolve_in(&mut walk, placement, placement)
    }

    /// Resolve `id`, reached from `referrer`, through the cache.
    fn resolve_in(
        &mut self,
        walk: &mut Walk<'_, '_>,
        referrer: EntityId,
        id: EntityId,
    ) -> GeometryResult<Transform> {
        if let Some(cached) = self.cache.get(&id) {
            return Ok(*cached);
        }
        if walk.active.contains(&id) {
            return Err(GeometryError::CyclicChain {
                entity: id,
                kind: "placement",
            });
        }
        if walk.active.len() >= MAX_CHAIN_DEPTH {
            return Err(GeometryError::ChainTooDeep {
                entity: id,
                kind: "placement",
                limit: MAX_CHAIN_DEPTH,
            });
        }
        let entity = walk.model.get(id).ok_or(GeometryError::MissingEntity {
            referrer,
            missing: id,
        })?;
        walk.active.push(id);
        let resolved = self.compose(walk, id, entity);
        walk.active.pop();
        let world = resolved?;
        self.cache.insert(id, world);
        Ok(world)
    }

    /// Resolve one placement by its kind, its parents through the cache.
    fn compose(
        &mut self,
        walk: &mut Walk<'_, '_>,
        id: EntityId,
        entity: &ifc_model::Entity,
    ) -> GeometryResult<Transform> {
        match entity.type_name.as_ref() {
            "IFCLOCALPLACEMENT" => {
                let view = LocalPlacement::new(id, entity);
                let parent = match view.parent() {
                    Some(parent) => self.resolve_in(walk, id, parent)?,
                    None => Transform::identity(),
                };
                Ok(parent.compose(&view.local_transform(walk.model)?))
            }
            "IFCGRIDPLACEMENT" => self.grid(walk, id, entity),
            "IFCLINEARPLACEMENT" => self.linear(walk, id),
            other => Err(GeometryError::WrongEntityType {
                entity: id,
                actual: other.to_string(),
                expected: "IfcObjectPlacement",
            }),
        }
    }

    /// An `IfcGridPlacement`: its grid's `ObjectPlacement`, then the
    /// intersection frame in the grid's coordinates (#362).
    fn grid(
        &mut self,
        walk: &mut Walk<'_, '_>,
        id: EntityId,
        entity: &ifc_model::Entity,
    ) -> GeometryResult<Transform> {
        let frame = grid_frame(walk.model, walk.units, id, walk.linear)?;
        let grid_world = self.resolve_in(walk, id, frame.grid_placement)?;
        // IFC4X3 states the grid's ObjectPlacement again as PlacementRelTo;
        // it must agree with the grid the axes belong to.
        if let Some(stated) = GridPlacement::new(id, entity).placement_rel_to() {
            if stated != frame.grid_placement {
                let stated_world = self.resolve_in(walk, id, stated)?;
                if !frames_agree(walk.model, &stated_world, &grid_world)? {
                    return Err(GeometryError::PlacementRelToConflict {
                        placement: id,
                        stated,
                        implied: frame.grid_placement,
                    });
                }
            }
        }
        Ok(grid_world.compose(&frame.local))
    }

    /// An `IfcLinearPlacement`: the frame its basis curve is stated in, then
    /// the station frame on the curve (#357).
    #[cfg(feature = "lowering")]
    fn linear(&mut self, walk: &mut Walk<'_, '_>, id: EntityId) -> GeometryResult<Transform> {
        use crate::constraint::placement::linear::{self, CurveFrame};
        let (model, units) = (walk.model, walk.units);
        let basis = linear::basis_curve(model, units, id)?;
        let stated = linear::placement_rel_to(model, id);
        let implied = match linear::curve_frame(model, basis, stated.is_none())? {
            CurveFrame::Placed { placement, .. } => {
                Some((placement, self.resolve_in(walk, id, placement)?))
            }
            CurveFrame::ModelSpace { product } => Some((product, Transform::identity())),
            CurveFrame::Unknown => None,
        };
        let parent = match (stated, implied) {
            (Some(stated), Some((implied, implied_world))) => {
                let stated_world = self.resolve_in(walk, id, stated)?;
                if stated != implied && !frames_agree(model, &stated_world, &implied_world)? {
                    return Err(GeometryError::PlacementRelToConflict {
                        placement: id,
                        stated,
                        implied,
                    });
                }
                stated_world
            }
            (Some(stated), None) => self.resolve_in(walk, id, stated)?,
            (None, Some((_, implied_world))) => implied_world,
            (None, None) => Transform::identity(),
        };
        let scale = units.length_to_metres;
        if !(scale.is_finite() && scale > 0.0) {
            return Err(GeometryError::Units(format!(
                "length factor {scale} cannot place a linear placement"
            )));
        }
        let relative =
            linear::relative_transform(model, units, id, walk.linear, &parent.to_metres(units))?;
        // The station frame is metric; the chain composes in file units.
        let relative = Transform {
            basis: relative.basis,
            origin: relative.origin.map(|coordinate| coordinate / scale),
        };
        Ok(parent.compose(&relative))
    }

    /// Without `lowering` there is no `ifc-alignment` to read the linear
    /// expression with.
    #[cfg(not(feature = "lowering"))]
    fn linear(&mut self, walk: &mut Walk<'_, '_>, id: EntityId) -> GeometryResult<Transform> {
        let _ = walk;
        Err(GeometryError::Unsupported {
            entity: id,
            type_name: "IFCLINEARPLACEMENT".into(),
            detail: "resolving a linear placement needs the `lowering` feature (ifc-alignment)",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::{Entity, Value};

    /// Build `#id = IFCAXIS2PLACEMENT3D(#point, $, $)` at the given offset.
    fn placement_at(model: &mut Model, id: u64, point_id: u64, xyz: [f64; 3]) {
        model.insert(
            EntityId(point_id),
            Entity::new(
                "IFCCARTESIANPOINT",
                vec![Value::List(
                    xyz.iter().map(|v| Value::Real(*v)).collect::<Vec<_>>(),
                )],
            ),
        );
        model.insert(
            EntityId(id),
            Entity::new(
                "IFCAXIS2PLACEMENT3D",
                vec![Value::Ref(EntityId(point_id)), Value::Null, Value::Null],
            ),
        );
    }

    /// `#id = IFCLOCALPLACEMENT(parent, axis_placement)`
    fn local(model: &mut Model, id: u64, parent: Option<u64>, axis: u64) {
        model.insert(
            EntityId(id),
            Entity::new(
                "IFCLOCALPLACEMENT",
                vec![
                    parent.map_or(Value::Null, |p| Value::Ref(EntityId(p))),
                    Value::Ref(EntityId(axis)),
                ],
            ),
        );
    }

    /// site(0,0,0) > storey(0,0,3) > wall(1,0,0) puts the wall at (1,0,3).
    fn three_level_model() -> Model {
        let mut model = Model::new();
        placement_at(&mut model, 10, 11, [0.0, 0.0, 0.0]);
        placement_at(&mut model, 20, 21, [0.0, 0.0, 3.0]);
        placement_at(&mut model, 30, 31, [1.0, 0.0, 0.0]);
        local(&mut model, 1, None, 10);
        local(&mut model, 2, Some(1), 20);
        local(&mut model, 3, Some(2), 30);
        model
    }

    #[test]
    fn absent_parent_means_world_coordinates() {
        let model = three_level_model();
        let mut resolver = PlacementResolver::new();
        let t = resolver.world_transform(&model, EntityId(1)).unwrap();
        assert!(t.is_identity(1e-12));
    }

    #[test]
    fn chain_composes_from_root_downward() {
        let model = three_level_model();
        let mut resolver = PlacementResolver::new();
        let t = resolver.world_transform(&model, EntityId(3)).unwrap();
        assert_eq!(t.origin, [1.0, 0.0, 3.0], "storey height must accumulate");
    }

    /// The spec pushes cycle prevention to the application, so files contain
    /// them. Detect, do not overflow.
    #[test]
    fn cyclic_chains_are_detected_not_stack_overflowed() {
        let mut model = Model::new();
        placement_at(&mut model, 10, 11, [0.0, 0.0, 0.0]);
        local(&mut model, 1, Some(2), 10);
        local(&mut model, 2, Some(1), 10);

        let mut resolver = PlacementResolver::new();
        let err = resolver.world_transform(&model, EntityId(1)).unwrap_err();
        assert!(
            matches!(err, GeometryError::CyclicChain { .. }),
            "expected a cycle error, got {err}"
        );
    }

    /// A self-referencing placement is the degenerate cycle.
    #[test]
    fn self_reference_is_a_cycle() {
        let mut model = Model::new();
        placement_at(&mut model, 10, 11, [0.0, 0.0, 0.0]);
        local(&mut model, 1, Some(1), 10);

        let mut resolver = PlacementResolver::new();
        assert!(matches!(
            resolver.world_transform(&model, EntityId(1)).unwrap_err(),
            GeometryError::CyclicChain { .. }
        ));
    }

    #[test]
    fn shared_ancestors_are_resolved_once() {
        let model = three_level_model();
        let mut resolver = PlacementResolver::new();
        resolver.world_transform(&model, EntityId(3)).unwrap();
        let after_first = resolver.cached();

        // A sibling under the same storey must reuse the cached tail.
        resolver.world_transform(&model, EntityId(2)).unwrap();
        assert_eq!(
            resolver.cached(),
            after_first,
            "resolving an already-cached ancestor must not recompute"
        );
    }

    #[test]
    fn dangling_parent_reference_is_reported() {
        let mut model = Model::new();
        placement_at(&mut model, 10, 11, [0.0, 0.0, 0.0]);
        local(&mut model, 1, Some(999), 10);

        let mut resolver = PlacementResolver::new();
        assert!(matches!(
            resolver.world_transform(&model, EntityId(1)).unwrap_err(),
            GeometryError::MissingEntity { .. }
        ));
    }

    /// A grid placement is resolved now (#362); one without its
    /// `PlacementLocation` is refused by name rather than placed at the
    /// origin.
    #[test]
    fn grid_placement_without_a_location_is_refused_rather_than_defaulting_to_origin() {
        let mut model = Model::new();
        model.insert(
            EntityId(1),
            Entity::new("IFCGRIDPLACEMENT", vec![Value::Null, Value::Null]),
        );
        let mut resolver = PlacementResolver::new();
        let err = resolver.world_transform(&model, EntityId(1)).unwrap_err();
        assert!(
            matches!(
                err,
                GeometryError::MissingAttribute {
                    attribute: "PlacementLocation",
                    ..
                }
            ),
            "got {err}"
        );
    }
}
