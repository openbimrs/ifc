//! `IfcMappedItem` and `IfcRepresentationMap`: geometry reuse by reference.
//!
//! # What it means
//!
//! A mapped item is IFC's block/instance mechanism. One geometry definition
//! (the [`RepresentationMap`]) is authored once, and every place it appears is
//! an [`MappedItem`] that references it plus a transform. A file with 400
//! identical windows stores one window and 400 small records.
//!
//! ```text
//!   IfcMappedItem ---- MappingSource ---> IfcRepresentationMap
//!         |                                      |
//!         |                                MappingOrigin (IfcAxis2Placement)
//!         +---- MappingTarget --->               |
//!               IfcCartesianTransformationOperator
//!                                          MappedRepresentation
//!                                                |
//!                                                v
//!                                      the actual geometry
//! ```
//!
//! # The trap: mapped items nest
//!
//! The spec is explicit that "an IfcMappedItem can reuse other mapped items
//! (ako nested blocks)": the `MappedRepresentation` may itself contain mapped
//! items. A naive resolver that assumes one level silently drops geometry, and
//! one that recurses without a guard hangs on the cyclic files that real
//! exporters have produced. The fixture corpus contains
//! `nested_mapped_item_cycle.ifc` for exactly this reason.
//!
//! # The full transform
//!
//! Placing a mapped item is **two** transforms composed, not one:
//!
//! 1. `MappingOrigin` on the representation map -- where the source geometry
//!    sits in its own definition space;
//! 2. `MappingTarget` on the mapped item -- where that space lands in the
//!    consuming representation.
//!
//! Applying only the target is a common bug, and it is invisible whenever
//! `MappingOrigin` happens to be the identity, which it usually is.

use crate::error::{GeometryError, GeometryResult};
use crate::slots::Slots;
use ifc_model::{Entity, EntityId, Model};

/// `IfcMappedItem` attribute slots.
///
/// `IfcMappedItem` inherits from `IfcRepresentationItem`, which declares no
/// explicit attributes, so these indices are its own.
mod item_slot {
    /// `MappingSource`: the `IfcRepresentationMap` being instanced.
    pub const MAPPING_SOURCE: usize = 0;
    /// `MappingTarget`: an `IfcCartesianTransformationOperator`.
    pub const MAPPING_TARGET: usize = 1;
}

/// `IfcRepresentationMap` attribute slots.
mod map_slot {
    /// `MappingOrigin`: an `IfcAxis2Placement` for the source geometry.
    pub const MAPPING_ORIGIN: usize = 0;
    /// `MappedRepresentation`: the `IfcRepresentation` being reused.
    pub const MAPPED_REPRESENTATION: usize = 1;
}

/// How deep mapped items may nest before the file is called malformed.
///
/// Legitimate nesting is shallow (a window in a wall assembly in a facade
/// module). 32 leaves ample room while terminating quickly on a cycle.
const MAX_NESTING_DEPTH: usize = 32;

/// A borrowed view of an `IfcMappedItem`.
#[derive(Debug, Clone, Copy)]
pub struct MappedItem<'m> {
    slots: Slots<'m>,
}

impl<'m> MappedItem<'m> {
    /// Wrap an entity assumed to be an `IfcMappedItem`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcRepresentationMap` this item instances.
    pub fn mapping_source(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(item_slot::MAPPING_SOURCE, "MappingSource")
    }

    /// The `IfcCartesianTransformationOperator` placing the instance.
    pub fn mapping_target(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(item_slot::MAPPING_TARGET, "MappingTarget")
    }
}

/// A borrowed view of an `IfcRepresentationMap`.
#[derive(Debug, Clone, Copy)]
pub struct RepresentationMap<'m> {
    slots: Slots<'m>,
}

impl<'m> RepresentationMap<'m> {
    /// Wrap an entity assumed to be an `IfcRepresentationMap`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcAxis2Placement` locating the source geometry.
    pub fn mapping_origin(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(map_slot::MAPPING_ORIGIN, "MappingOrigin")
    }

    /// The `IfcRepresentation` being reused.
    pub fn mapped_representation(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(map_slot::MAPPED_REPRESENTATION, "MappedRepresentation")
    }
}

/// Walks mapped-item nesting, detecting cycles and excessive depth.
///
/// Separate from the views because the views are stateless borrows while a
/// safe walk needs to remember where it has been.
#[derive(Debug, Default)]
pub struct MappingWalker {
    visited: Vec<EntityId>,
}

impl MappingWalker {
    /// A walker with no history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Current nesting depth.
    pub fn depth(&self) -> usize {
        self.visited.len()
    }

    /// Enter a mapped item, or fail if that would revisit or go too deep.
    ///
    /// Call [`Self::exit`] when done with the item, so sibling instances of
    /// the same map do not look like a cycle. A map legitimately appears many
    /// times; what is illegal is a map containing *itself*.
    pub fn enter(&mut self, item: EntityId) -> GeometryResult<()> {
        if self.visited.contains(&item) {
            return Err(GeometryError::CyclicChain {
                entity: item,
                kind: "mapped item",
            });
        }
        if self.visited.len() >= MAX_NESTING_DEPTH {
            return Err(GeometryError::ChainTooDeep {
                entity: item,
                kind: "mapped item",
                limit: MAX_NESTING_DEPTH,
            });
        }
        self.visited.push(item);
        Ok(())
    }

    /// Leave the most recently entered item.
    pub fn exit(&mut self) {
        self.visited.pop();
    }

    /// Resolve the source map and target operator of a mapped item.
    ///
    /// Returns both ids so a caller can compose `MappingOrigin` with
    /// `MappingTarget`. Composing only the target is the bug this signature
    /// is shaped to prevent.
    pub fn resolve(&mut self, model: &Model, item_id: EntityId) -> GeometryResult<MappedInstance> {
        let entity = model.get(item_id).ok_or(GeometryError::MissingEntity {
            referrer: item_id,
            missing: item_id,
        })?;
        let item = MappedItem::new(item_id, entity);

        let source_id = item.mapping_source()?;
        let target_id = item.mapping_target()?;

        let source = model.get(source_id).ok_or(GeometryError::MissingEntity {
            referrer: item_id,
            missing: source_id,
        })?;
        let map = RepresentationMap::new(source_id, source);

        Ok(MappedInstance {
            item: item_id,
            mapping_origin: map.mapping_origin()?,
            mapped_representation: map.mapped_representation()?,
            mapping_target: target_id,
        })
    }
}

/// One resolved mapped item: everything needed to place the reused geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MappedInstance {
    /// The `IfcMappedItem` this came from.
    pub item: EntityId,
    /// `IfcAxis2Placement` locating the source geometry in its own space.
    pub mapping_origin: EntityId,
    /// The `IfcRepresentation` holding the reused geometry.
    pub mapped_representation: EntityId,
    /// `IfcCartesianTransformationOperator` placing the instance.
    pub mapping_target: EntityId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::Value;

    fn model_with_mapping() -> Model {
        let mut model = Model::new();
        // #14 = IFCREPRESENTATIONMAP(#2, #13)
        model.insert(
            EntityId(14),
            Entity::new(
                "IFCREPRESENTATIONMAP",
                vec![Value::Ref(EntityId(2)), Value::Ref(EntityId(13))],
            ),
        );
        // #19 = IFCMAPPEDITEM(#14, #18)
        model.insert(
            EntityId(19),
            Entity::new(
                "IFCMAPPEDITEM",
                vec![Value::Ref(EntityId(14)), Value::Ref(EntityId(18))],
            ),
        );
        model
    }

    #[test]
    fn reads_both_halves_of_the_mapping() {
        let model = model_with_mapping();
        let mut walker = MappingWalker::new();
        let resolved = walker.resolve(&model, EntityId(19)).unwrap();

        assert_eq!(resolved.mapping_origin, EntityId(2));
        assert_eq!(resolved.mapped_representation, EntityId(13));
        assert_eq!(resolved.mapping_target, EntityId(18));
    }

    /// A map used many times is normal; that must not look like a cycle.
    #[test]
    fn the_same_map_may_be_instanced_repeatedly() {
        let mut walker = MappingWalker::new();
        for _ in 0..100 {
            walker
                .enter(EntityId(19))
                .expect("sibling instances are legal");
            walker.exit();
        }
        assert_eq!(walker.depth(), 0);
    }

    /// A map containing itself is not legal, and must not hang.
    #[test]
    fn revisiting_an_item_while_inside_it_is_a_cycle() {
        let mut walker = MappingWalker::new();
        walker.enter(EntityId(19)).unwrap();
        let err = walker.enter(EntityId(19)).unwrap_err();
        assert!(
            matches!(err, GeometryError::CyclicChain { .. }),
            "got {err}"
        );
    }

    #[test]
    fn nesting_is_bounded() {
        let mut walker = MappingWalker::new();
        for i in 0..MAX_NESTING_DEPTH {
            walker.enter(EntityId(i as u64)).unwrap();
        }
        let err = walker.enter(EntityId(9999)).unwrap_err();
        assert!(
            matches!(err, GeometryError::ChainTooDeep { .. }),
            "got {err}"
        );
    }

    #[test]
    fn dangling_mapping_source_is_reported() {
        let mut model = Model::new();
        model.insert(
            EntityId(19),
            Entity::new(
                "IFCMAPPEDITEM",
                vec![Value::Ref(EntityId(999)), Value::Ref(EntityId(18))],
            ),
        );
        let mut walker = MappingWalker::new();
        assert!(matches!(
            walker.resolve(&model, EntityId(19)).unwrap_err(),
            GeometryError::MissingEntity { .. }
        ));
    }
}
