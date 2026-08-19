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

use crate::error::{GeometryError, GeometryResult};
use crate::resource::placement::axis_placement_transform;
use crate::slots::Slots;
use crate::transform::Transform;
use ifc_model::{EntityId, Model};
use std::collections::HashMap;

/// `IfcLocalPlacement` attribute slots.
///
/// From IFC4 ADD2 TC1: `IfcLocalPlacement` has no inherited explicit
/// attributes (its supertype `IfcObjectPlacement` declares only the inverse
/// `PlacesObject`), so these indices are its own.
mod slot {
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
/// corrupt file quickly.
const MAX_CHAIN_DEPTH: usize = 64;

/// Resolves placement chains to world transforms, with memoization.
///
/// # Why a resolver rather than a free function
///
/// Placement chains share their upper links: every element in a storey walks
/// the same storey-building-site tail. Caching per placement turns repeated
/// work into a lookup, which matters because this runs once per element in the
/// file.
#[derive(Debug, Default)]
pub struct PlacementResolver {
    cache: HashMap<EntityId, Transform>,
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

    /// Resolve a placement to its world transform.
    ///
    /// Walks `PlacementRelTo` to the root, then composes downward. Detects
    /// cycles rather than overflowing the stack, and reports the entity where
    /// the cycle closes so the file can be repaired.
    pub fn world_transform(
        &mut self,
        model: &Model,
        placement: EntityId,
    ) -> GeometryResult<Transform> {
        if let Some(cached) = self.cache.get(&placement) {
            return Ok(*cached);
        }

        // Walk up to the root, remembering the path. Iterative rather than
        // recursive so a deep chain cannot blow the stack.
        let mut chain = Vec::new();
        let mut visited = Vec::new();
        let mut current = Some(placement);

        while let Some(id) = current {
            if visited.contains(&id) {
                return Err(GeometryError::CyclicChain {
                    entity: id,
                    kind: "placement",
                });
            }
            if chain.len() >= MAX_CHAIN_DEPTH {
                return Err(GeometryError::ChainTooDeep {
                    entity: id,
                    kind: "placement",
                    limit: MAX_CHAIN_DEPTH,
                });
            }
            visited.push(id);

            // A cached ancestor ends the walk: everything above it is known.
            if self.cache.contains_key(&id) {
                break;
            }

            let entity = model.get(id).ok_or(GeometryError::MissingEntity {
                referrer: placement,
                missing: id,
            })?;

            match entity.type_name.as_ref() {
                "IFCLOCALPLACEMENT" => {
                    let view = LocalPlacement::new(id, entity);
                    chain.push(id);
                    current = view.parent();
                }
                // IfcGridPlacement resolves through grid axes rather than a
                // parent chain; treated as a root here and handled by the
                // grid module. Returning Unsupported keeps the failure
                // honest rather than silently placing the element at origin.
                "IFCGRIDPLACEMENT" => {
                    return Err(GeometryError::Unsupported {
                        entity: id,
                        type_name: entity.type_name.to_string(),
                        detail: "grid placement resolution",
                    });
                }
                other => {
                    return Err(GeometryError::WrongEntityType {
                        entity: id,
                        actual: other.to_string(),
                        expected: "IfcLocalPlacement",
                    });
                }
            }
        }

        // Seed with the cached ancestor's transform if the walk stopped there.
        let mut world = current
            .and_then(|id| self.cache.get(&id).copied())
            .unwrap_or_else(Transform::identity);

        // Compose from the root downward.
        for id in chain.iter().rev() {
            let entity = model.get(*id).ok_or(GeometryError::MissingEntity {
                referrer: placement,
                missing: *id,
            })?;
            let local = LocalPlacement::new(*id, entity).local_transform(model)?;
            world = world.compose(&local);
            self.cache.insert(*id, world);
        }

        Ok(world)
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

    /// Grid placement is valid IFC we do not resolve yet; it must say so
    /// rather than silently placing the element at the origin.
    #[test]
    fn grid_placement_reports_unsupported_rather_than_defaulting_to_origin() {
        let mut model = Model::new();
        model.insert(
            EntityId(1),
            Entity::new("IFCGRIDPLACEMENT", vec![Value::Null, Value::Null]),
        );
        let mut resolver = PlacementResolver::new();
        let err = resolver.world_transform(&model, EntityId(1)).unwrap_err();
        assert!(err.is_unsupported(), "got {err}");
    }
}
