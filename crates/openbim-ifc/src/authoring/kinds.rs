//! How the cascading removal and the purge treat each entity type.
//!
//! The answer depends only on the type and the declared release, and a
//! purge asks it for every entity of the model, so it is cached per type
//! name.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use ifc_model::{Entity, EntityId, Value};
use ifc_schema::Schema;

/// The role an entity type plays when something it references, or that
/// references it, is removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    /// An objectified relationship (`IfcRelationship`), or a resource-level
    /// one (`IfcMaterialRelationship`, `IfcExternalReferenceRelationship`,
    /// IFC2X3's `IfcPropertyConstraintRelationship`, ...): it loses a
    /// removed member from a list, and goes when left without an end.
    Relationship,
    /// `IfcPresentationLayerAssignment`: loses a removed item, and goes
    /// when left with none.
    LayerAssignment,
    /// `IfcStyledItem`: goes with the item it styles.
    StyledItem,
    /// `IfcShapeAspect`: goes with the product definition shape it is
    /// part of.
    ShapeAspect,
    /// A property or quantity set (`IfcPropertySetDefinition`): an
    /// `IfcRoot` the cascade removes when only removed objects use it.
    PropertySet,
    /// An opening or other subtraction feature: removed with the element
    /// it voids.
    Opening,
    /// A port: removed with the element it is nested in or connected to.
    Port,
    /// Any other `IfcRoot`: an object with an identity of its own, never
    /// removed by the cascade.
    Root,
    /// A definition the cascade never deletes (see
    /// [`KeepReason::Definition`](super::KeepReason::Definition)).
    Definition,
    /// The owner history and its actors: a definition, and protected
    /// from a purge unless asked.
    OwnerChain,
    /// A representation context: a definition, protected like the owner
    /// history.
    Context,
    /// Any other resource entity: removed once nothing that stays uses it.
    Resource,
}

/// Types the purge keeps while what they annotate is kept: they reference
/// it and nothing references them.
const ANNOTATORS: &[&str] = &[
    "IfcStyledItem",
    "IfcPresentationLayerAssignment",
    "IfcShapeAspect",
    "IfcMaterialDefinitionRepresentation",
    "IfcMaterialProperties",
    "IfcCoordinateOperation",
    "IfcIndexedColourMap",
    "IfcTextureCoordinate",
];

const OWNER_CHAIN: &[&str] = &[
    "IfcOwnerHistory",
    "IfcPerson",
    "IfcOrganization",
    "IfcPersonAndOrganization",
    "IfcApplication",
    "IfcActorRole",
    "IfcAddress",
];

const DEFINITIONS: &[&str] = &[
    "IfcMaterialDefinition",
    "IfcMaterial",
    "IfcMaterialLayer",
    "IfcMaterialLayerSet",
    "IfcMaterialList",
    "IfcExternalReference",
    "IfcExternalInformation",
    "IfcClassification",
    "IfcClassificationNotation",
    "IfcClassificationItem",
    "IfcDocumentInformation",
    "IfcLibraryInformation",
    "IfcApproval",
    "IfcConstraint",
    "IfcTimeSeries",
    "IfcNamedUnit",
    "IfcDerivedUnit",
    "IfcMonetaryUnit",
    "IfcUnitAssignment",
    "IfcCoordinateReferenceSystem",
    "IfcCoordinateOperation",
];

/// What [`Kinds`] knows of one type.
#[derive(Debug, Clone, Copy)]
struct Traits {
    kind: Kind,
    annotates: bool,
    root: bool,
}

/// Entity kinds by type name, against one release.
pub(super) struct Kinds<'s> {
    schema: &'s Schema,
    cache: RefCell<HashMap<Arc<str>, Traits>>,
}

impl<'s> Kinds<'s> {
    pub(super) fn new(schema: &'s Schema) -> Self {
        Self {
            schema,
            cache: RefCell::new(HashMap::new()),
        }
    }

    /// The kind of an entity of `type_name`.
    pub(super) fn of(&self, type_name: &Arc<str>) -> Kind {
        self.lookup(type_name).kind
    }

    /// Whether `type_name` is an `IfcRoot`.
    pub(super) fn is_root(&self, type_name: &Arc<str>) -> bool {
        self.lookup(type_name).root
    }

    /// Whether the purge keeps an entity of `type_name` while something it
    /// references is kept.
    pub(super) fn annotates(&self, type_name: &Arc<str>) -> bool {
        self.lookup(type_name).annotates
    }

    fn lookup(&self, type_name: &Arc<str>) -> Traits {
        if let Some(found) = self.cache.borrow().get(type_name) {
            return *found;
        }
        let found = Traits {
            kind: self.classify(type_name),
            annotates: self.classify_annotator(type_name),
            root: self.is_a(type_name, "IfcRoot"),
        };
        self.cache.borrow_mut().insert(type_name.clone(), found);
        found
    }

    pub(super) fn is_a(&self, type_name: &str, ancestor: &str) -> bool {
        self.schema.is_a(type_name, ancestor)
    }

    fn any(&self, type_name: &str, ancestors: &[&str]) -> bool {
        ancestors
            .iter()
            .any(|ancestor| self.is_a(type_name, ancestor))
    }

    fn classify(&self, t: &str) -> Kind {
        if self.is_a(t, "IfcRelationship") {
            return Kind::Relationship;
        }
        if self.is_a(t, "IfcRoot") {
            return if self.is_a(t, "IfcPropertySetDefinition") {
                Kind::PropertySet
            } else if self.is_a(t, "IfcFeatureElementSubtraction") {
                Kind::Opening
            } else if self.is_a(t, "IfcPort") {
                Kind::Port
            } else {
                Kind::Root
            };
        }
        if is_resource_relationship(t) {
            Kind::Relationship
        } else if self.is_a(t, "IfcPresentationLayerAssignment") {
            Kind::LayerAssignment
        } else if self.is_a(t, "IfcStyledItem") {
            Kind::StyledItem
        } else if self.is_a(t, "IfcShapeAspect") {
            Kind::ShapeAspect
        } else if self.any(t, OWNER_CHAIN) {
            Kind::OwnerChain
        } else if self.is_a(t, "IfcRepresentationContext") {
            Kind::Context
        } else if self.any(t, DEFINITIONS) {
            Kind::Definition
        } else {
            Kind::Resource
        }
    }

    fn classify_annotator(&self, t: &str) -> bool {
        !self.is_a(t, "IfcRoot") && (is_resource_relationship(t) || self.any(t, ANNOTATORS))
    }

    /// The position of attribute `name` of `type_name` in the release.
    pub(super) fn slot(&self, type_name: &str, name: &str) -> Option<usize> {
        self.schema
            .attributes(type_name)
            .iter()
            .position(|attribute| attribute.name.eq_ignore_ascii_case(name))
    }

    /// The entity `entity` names in attribute `name`, if it is a reference.
    pub(super) fn reference(&self, entity: &Entity, name: &str) -> Option<EntityId> {
        let slot = self.slot(&entity.type_name, name)?;
        entity.attributes.get(slot)?.as_ref_id()
    }

    /// The entities `entity` names in attribute `name`, a list or one.
    pub(super) fn references(&self, entity: &Entity, name: &str) -> Vec<EntityId> {
        let mut found = Vec::new();
        if let Some(value) = self
            .slot(&entity.type_name, name)
            .and_then(|slot| entity.attributes.get(slot))
        {
            value.for_each_ref(&mut |id| found.push(id));
        }
        found
    }
}

/// A resource-level relationship: not an `IfcRoot`, named
/// `...Relationship` (`IfcMaterialLayerSetUsage` is not one).
fn is_resource_relationship(type_name: &str) -> bool {
    let upper = type_name.to_ascii_uppercase();
    upper.ends_with("RELATIONSHIP") && upper != "IFCRELATIONSHIP"
}

/// Whether `value` references an entity `gone` holds, at any depth.
pub(super) fn references_any(value: &Value, gone: &impl Fn(EntityId) -> bool) -> bool {
    let mut found = false;
    value.for_each_ref(&mut |id| found |= gone(id));
    found
}

/// Whether a relationship is left without an end once every entity `gone`
/// holds is removed: a single reference to one, or a list all of whose
/// members are.
pub(super) fn loses_an_end(entity: &Entity, gone: &impl Fn(EntityId) -> bool) -> bool {
    entity.attributes.iter().any(|value| match value {
        Value::List(items) => {
            let removed = items
                .iter()
                .filter(|item| references_any(item, gone))
                .count();
            removed > 0 && removed == items.len()
        }
        other => references_any(other, gone),
    })
}

/// `entity` with every list member that references an entity `gone`
/// holds taken out; whether any list was left empty.
pub(super) fn detach(entity: &mut Entity, gone: &impl Fn(EntityId) -> bool) -> bool {
    let mut emptied = false;
    for value in &mut entity.attributes {
        if let Value::List(items) = value {
            let before = items.len();
            items.retain(|item| !references_any(item, gone));
            emptied |= before > 0 && items.is_empty();
        }
    }
    emptied
}
