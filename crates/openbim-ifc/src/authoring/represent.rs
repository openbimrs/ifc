//! Shape representations (#481): assemble one from items, assign it to a
//! product or a type, take it off again, copy it, and remove it.
//!
//! # Where a representation belongs
//!
//! `IfcShapeModel.WR11` holds a shape representation to exactly one use:
//! the `Representations` of one `IfcProductDefinitionShape`, the
//! `MappedRepresentation` of one `IfcRepresentationMap`, or the
//! `ShapeRepresentations` of one `IfcShapeAspect`. Assignment refuses a
//! representation that already has one; unassignment leaves it with none,
//! for the batch to assign elsewhere or remove.
//!
//! # What a copy shares
//!
//! A copy is independent geometry: every representation item and profile
//! definition the original reaches is copied once (a point two items share
//! stays shared between their copies), as are the styled items, colour
//! maps and texture maps attached to a copied item. Definitions are
//! shared: the context, the representation map a mapped item maps (the
//! type's geometry), presentation styles and every other resource
//! (units, materials, external references). Presentation layers holding
//! the original or an item gain the copy.
//!
//! # Removal
//!
//! Removal takes the representation off its uses first (a product
//! definition shape or shape aspect left empty, and the representation map,
//! go with it), then runs the cascade of `RemoveProduct` from it: items and
//! what they are built from go unless something that stays still uses them.

use std::collections::{BTreeMap, BTreeSet};

use ifc_model::{Entity, EntityId, Value};
use ifc_schema::SchemaVersion;

use super::check::{recheck, relations};
use super::kinds::Kinds;
use super::plan::Planner;
use super::report::RemovalReport;
use super::shape_type::{ShapeTypes, Verdict};
use super::{AuthoringFailure as F, RemoveProductOptions};

const SHAPE: &str = "IFCSHAPEREPRESENTATION";

/// A copied representation: the copy, every original copied with its
/// copy, and the presentation layers that gained a copy.
pub(super) struct CopiedRepresentation {
    pub(super) copy: EntityId,
    pub(super) copies: BTreeMap<EntityId, EntityId>,
    pub(super) layers: Vec<EntityId>,
}

/// The one use `IfcShapeModel.WR11` allows a representation.
#[derive(Debug, Clone, Copy)]
enum Usage {
    Shape(EntityId),
    Map(EntityId),
    Aspect(EntityId),
}

impl Planner<'_, '_> {
    /// `AuthorOp::ShapeRepresentation`.
    pub(super) fn shape_representation(
        &mut self,
        context: EntityId,
        items: &[EntityId],
        identifier: Option<String>,
        representation_type: Option<String>,
    ) -> Result<EntityId, F> {
        let reference = |attribute: &str, target: EntityId| F::MissingReference {
            entity: SHAPE.to_owned(),
            attribute: attribute.to_owned(),
            target,
        };
        let wrong = |attribute: &str, target: EntityId, actual: &str, expected: &str| {
            F::WrongReferenceType {
                entity: SHAPE.to_owned(),
                attribute: attribute.to_owned(),
                target,
                actual: actual.to_owned(),
                expected: expected.to_owned(),
            }
        };
        let found = self
            .view
            .get(context)
            .ok_or_else(|| reference("ContextOfItems", context))?;
        if !self
            .schema
            .is_a(&found.type_name, "IfcGeometricRepresentationContext")
        {
            return Err(wrong(
                "ContextOfItems",
                context,
                &found.type_name,
                "IfcGeometricRepresentationContext (CorrectContext)",
            ));
        }
        let kinds = Kinds::new(self.schema);
        let context_identifier = kinds
            .slot(&found.type_name, "ContextIdentifier")
            .and_then(|slot| found.attributes.get(slot))
            .and_then(|value| value.unwrap_typed().as_text())
            .map(str::to_owned);
        if items.is_empty() {
            return Err(F::Cardinality {
                entity: SHAPE.to_owned(),
                attribute: "Items".to_owned(),
                detail: "0 items; at least 1".to_owned(),
            });
        }
        for item in items {
            let entity = self
                .view
                .get(*item)
                .ok_or_else(|| reference("Items", *item))?;
            let t = &entity.type_name;
            if !self.schema.is_a(t, "IfcRepresentationItem") {
                return Err(wrong("Items", *item, t, "IfcRepresentationItem"));
            }
            let topological = self.schema.is_a(t, "IfcTopologicalRepresentationItem")
                && !["IfcVertexPoint", "IfcEdgeCurve", "IfcFaceSurface"]
                    .iter()
                    .any(|allowed| self.schema.is_a(t, allowed));
            if topological {
                return Err(F::InvalidRepresentationItem {
                    item: *item,
                    detail: format!(
                        "an {t} is topological; a shape representation holds only \
                         IfcVertexPoint, IfcEdgeCurve and IfcFaceSurface of those \
                         (NoTopologicalItem)"
                    ),
                });
            }
        }
        let identifier = identifier.or(context_identifier);
        if identifier.is_none() && self.schema.version() != Some(SchemaVersion::Ifc2x3) {
            return Err(F::MissingRepresentationIdentifier { context });
        }
        let types = ShapeTypes::new(self.schema, &self.view);
        let representation_type = match representation_type {
            Some(given) => match types.check(&given, items) {
                Verdict::Admits | Verdict::Unlisted => given,
                Verdict::Rejects(items, expected) => {
                    return Err(F::RepresentationTypeMismatch {
                        representation_type: given,
                        items,
                        expected,
                    })
                }
                Verdict::Undecided(item) => {
                    return Err(F::UndecidableRepresentationType {
                        representation_type: given,
                        item,
                    })
                }
            },
            None => types
                .infer(items)
                .ok_or_else(|| F::NoRepresentationType {
                    items: items.to_vec(),
                })?
                .to_owned(),
        };
        let mut attributes = vec![
            ("ContextOfItems".to_owned(), Value::Ref(context)),
            (
                "RepresentationType".to_owned(),
                Value::Text(representation_type.into()),
            ),
            (
                "Items".to_owned(),
                Value::List(items.iter().copied().map(Value::Ref).collect()),
            ),
        ];
        if let Some(identifier) = identifier {
            attributes.push((
                "RepresentationIdentifier".to_owned(),
                Value::Text(identifier.into()),
            ));
        }
        self.create("IfcShapeRepresentation", attributes)
    }

    /// `AuthorOp::AssignRepresentation`.
    pub(super) fn assign_representation(
        &mut self,
        representation: EntityId,
        to: EntityId,
    ) -> Result<EntityId, F> {
        let kinds = Kinds::new(self.schema);
        self.shape_model(representation)?;
        if let Some(usage) = self.usages(&kinds, representation).first() {
            let (relationship, existing) = match *usage {
                Usage::Shape(id) => ("IFCPRODUCTDEFINITIONSHAPE", id),
                Usage::Map(id) => ("IFCREPRESENTATIONMAP", id),
                Usage::Aspect(id) => ("IFCSHAPEASPECT", id),
            };
            return Err(F::AlreadyRelated {
                id: representation,
                relationship,
                existing,
            });
        }
        if self.target_is_type(to)? {
            let origin = self.axis_placement([0.0; 3], None)?;
            let map = self.create(
                "IfcRepresentationMap",
                vec![
                    ("MappingOrigin".to_owned(), Value::Ref(origin)),
                    (
                        "MappedRepresentation".to_owned(),
                        Value::Ref(representation),
                    ),
                ],
            )?;
            let mut maps = self.list(&kinds, to, "RepresentationMaps");
            maps.push(map);
            self.edit(to, vec![("RepresentationMaps".to_owned(), refs(&maps))])?;
            return Ok(map);
        }
        match self.product_shape(&kinds, to)? {
            Some(shape) => {
                let mut members = self.list(&kinds, shape, "Representations");
                members.push(representation);
                self.edit(shape, vec![("Representations".to_owned(), refs(&members))])
            }
            None => {
                let shape = self.create(
                    "IfcProductDefinitionShape",
                    vec![("Representations".to_owned(), refs(&[representation]))],
                )?;
                self.edit(to, vec![("Representation".to_owned(), Value::Ref(shape))])?;
                Ok(shape)
            }
        }
    }

    /// `AuthorOp::UnassignRepresentation`.
    pub(super) fn unassign_representation(
        &mut self,
        representation: EntityId,
        from: EntityId,
    ) -> Result<RemovalReport, F> {
        let kinds = Kinds::new(self.schema);
        self.shape_model(representation)?;
        let not_assigned = F::NotAssigned {
            representation,
            to: from,
        };
        let keep = BTreeSet::from([representation]);
        if self.target_is_type(from)? {
            let maps = self.list(&kinds, from, "RepresentationMaps");
            let Some(map) = maps.iter().copied().find(|map| {
                self.view
                    .get(*map)
                    .and_then(|entity| kinds.reference(entity, "MappedRepresentation"))
                    == Some(representation)
            }) else {
                return Err(not_assigned);
            };
            self.drop_member(&kinds, from, "RepresentationMaps", map, true)?;
            return self.cascade(&[map], keep, RemoveProductOptions::new());
        }
        let Some(shape) = self.product_shape(&kinds, from)? else {
            return Err(not_assigned);
        };
        let members = self.list(&kinds, shape, "Representations");
        if !members.contains(&representation) {
            return Err(not_assigned);
        }
        if members.len() > 1 {
            self.drop_member(&kinds, shape, "Representations", representation, false)?;
            return Ok(RemovalReport::default());
        }
        self.edit(from, vec![("Representation".to_owned(), Value::Null)])?;
        self.cascade(&[shape], keep, RemoveProductOptions::new())
    }

    /// `AuthorOp::RemoveRepresentation`.
    pub(super) fn remove_representation(
        &mut self,
        representation: EntityId,
    ) -> Result<RemovalReport, F> {
        let kinds = Kinds::new(self.schema);
        self.shape_model(representation)?;
        let mut roots = vec![representation];
        for usage in self.usages(&kinds, representation) {
            match usage {
                Usage::Shape(shape) => {
                    if self.list(&kinds, shape, "Representations").len() > 1 {
                        self.drop_member(&kinds, shape, "Representations", representation, false)?;
                        continue;
                    }
                    for product in self.shape_products(&kinds, shape) {
                        self.edit(product, vec![("Representation".to_owned(), Value::Null)])?;
                    }
                    roots.push(shape);
                }
                Usage::Map(map) => {
                    for holder in self.view.referrers(map) {
                        let holds = self.view.get(holder).is_some_and(|entity| {
                            self.schema.is_a(&entity.type_name, "IfcTypeProduct")
                                && kinds
                                    .references(entity, "RepresentationMaps")
                                    .contains(&map)
                        });
                        if holds {
                            self.drop_member(&kinds, holder, "RepresentationMaps", map, true)?;
                        }
                    }
                    roots.push(map);
                }
                Usage::Aspect(aspect) => {
                    if self.list(&kinds, aspect, "ShapeRepresentations").len() > 1 {
                        self.drop_member(
                            &kinds,
                            aspect,
                            "ShapeRepresentations",
                            representation,
                            false,
                        )?;
                    } else {
                        roots.push(aspect);
                    }
                }
            }
        }
        self.cascade(&roots, BTreeSet::new(), RemoveProductOptions::new())
    }

    /// `AuthorOp::CopyRepresentation`.
    pub(super) fn copy_representation(&mut self, representation: EntityId) -> Result<EntityId, F> {
        self.copy_representation_mapped(representation)
            .map(|copied| copied.copy)
    }

    /// A copy of `representation` by the rules of `CopyRepresentation`,
    /// with every original it copied and the layers that gained a copy.
    pub(super) fn copy_representation_mapped(
        &mut self,
        representation: EntityId,
    ) -> Result<CopiedRepresentation, F> {
        let kinds = Kinds::new(self.schema);
        self.shape_model(representation)?;
        let order = self.copy_order(representation)?;
        let mut copies: BTreeMap<EntityId, EntityId> = BTreeMap::new();
        for original in &order {
            let copy = self.copy_of(*original, &copies)?;
            copies.insert(*original, copy);
        }
        // What attaches to a copied item, and the layers that gain it.
        let mut layers: BTreeMap<EntityId, Vec<EntityId>> = BTreeMap::new();
        for original in &order {
            for referrer in self.view.referrers(*original) {
                if copies.contains_key(&referrer) {
                    continue;
                }
                let Some(entity) = self.view.get(referrer) else {
                    continue;
                };
                let t = &entity.type_name;
                let attached = (self.schema.is_a(t, "IfcStyledItem")
                    && kinds.reference(entity, "Item") == Some(*original))
                    || ((self.schema.is_a(t, "IfcIndexedColourMap")
                        || self.schema.is_a(t, "IfcTextureCoordinate"))
                        && kinds.reference(entity, "MappedTo") == Some(*original));
                if attached {
                    let copy = self.copy_of(referrer, &copies)?;
                    copies.insert(referrer, copy);
                } else if self.schema.is_a(t, "IfcPresentationLayerAssignment") {
                    layers.entry(referrer).or_default().push(copies[original]);
                }
            }
        }
        let joined: Vec<EntityId> = layers.keys().copied().collect();
        for (layer, added) in layers {
            let mut members = self.list(&kinds, layer, "AssignedItems");
            members.extend(added);
            self.edit(layer, vec![("AssignedItems".to_owned(), refs(&members))])?;
        }
        Ok(CopiedRepresentation {
            copy: copies[&representation],
            copies,
            layers: joined,
        })
    }

    /// What a copy of `representation` copies, each before what uses it.
    pub(super) fn copy_order(&self, representation: EntityId) -> Result<Vec<EntityId>, F> {
        self.subgraph_order(representation, &|id| self.copied(id))
            .map_err(|item| F::InvalidRepresentationItem {
                item,
                detail: "its references form a cycle".to_owned(),
            })
    }

    /// Whether a copy copies `id` when it reaches it: representation items
    /// and profile definitions; everything else is shared.
    pub(super) fn copied(&self, id: EntityId) -> bool {
        self.view.get(id).is_some_and(|entity| {
            let t = &entity.type_name;
            self.schema.is_a(t, "IfcRepresentationItem") || self.schema.is_a(t, "IfcProfileDef")
        })
    }

    /// A new entity like `original`, its references to copied entities
    /// replaced by their copies.
    pub(super) fn copy_of(
        &mut self,
        original: EntityId,
        copies: &BTreeMap<EntityId, EntityId>,
    ) -> Result<EntityId, F> {
        let mut entity: Entity = self
            .view
            .get(original)
            .cloned()
            .ok_or(F::MissingEntity(original))?;
        for value in &mut entity.attributes {
            remap(value, copies);
        }
        self.insert(entity)
    }

    /// Refuse `id` unless it is an `IfcShapeModel`.
    fn shape_model(&self, id: EntityId) -> Result<(), F> {
        let entity = self.view.get(id).ok_or(F::MissingEntity(id))?;
        if self.schema.is_a(&entity.type_name, "IfcShapeModel") {
            Ok(())
        } else {
            Err(F::NotA {
                type_name: entity.type_name.to_string(),
                expected: "IfcShapeModel".to_owned(),
            })
        }
    }

    /// Whether `target` is an `IfcTypeProduct` (else an `IfcProduct`).
    fn target_is_type(&self, target: EntityId) -> Result<bool, F> {
        let entity = self.view.get(target).ok_or(F::MissingEntity(target))?;
        let t = &entity.type_name;
        if self.schema.is_a(t, "IfcTypeProduct") {
            Ok(true)
        } else if self.schema.is_a(t, "IfcProduct") {
            Ok(false)
        } else {
            Err(F::NotA {
                type_name: t.to_string(),
                expected: "IfcProduct or IfcTypeProduct".to_owned(),
            })
        }
    }

    /// The product's own `IfcProductDefinitionShape`, if it has one;
    /// refused when other products share it.
    fn product_shape(&self, kinds: &Kinds<'_>, product: EntityId) -> Result<Option<EntityId>, F> {
        let entity = self.view.get(product).ok_or(F::MissingEntity(product))?;
        let Some(shape) = kinds.reference(entity, "Representation") else {
            return Ok(None);
        };
        let found = self.view.get(shape).ok_or(F::MissingEntity(shape))?;
        if !self
            .schema
            .is_a(&found.type_name, "IfcProductDefinitionShape")
        {
            return Err(F::NotA {
                type_name: found.type_name.to_string(),
                expected: "IfcProductDefinitionShape".to_owned(),
            });
        }
        let products = self.shape_products(kinds, shape);
        if products.len() > 1 {
            return Err(F::SharedProductShape { shape, products });
        }
        Ok(Some(shape))
    }

    /// The products whose `Representation` is `shape`.
    fn shape_products(&self, kinds: &Kinds<'_>, shape: EntityId) -> Vec<EntityId> {
        self.view
            .referrers(shape)
            .into_iter()
            .filter(|referrer| {
                self.view.get(*referrer).is_some_and(|entity| {
                    self.schema.is_a(&entity.type_name, "IfcProduct")
                        && kinds.reference(entity, "Representation") == Some(shape)
                })
            })
            .collect()
    }

    /// Where `representation` is used, by `IfcShapeModel.WR11`'s count.
    fn usages(&self, kinds: &Kinds<'_>, representation: EntityId) -> Vec<Usage> {
        let mut found = Vec::new();
        for referrer in self.view.referrers(representation) {
            let Some(entity) = self.view.get(referrer) else {
                continue;
            };
            let t = &entity.type_name;
            let lists = |name: &str| kinds.references(entity, name).contains(&representation);
            if self.schema.is_a(t, "IfcProductRepresentation") && lists("Representations") {
                found.push(Usage::Shape(referrer));
            } else if self.schema.is_a(t, "IfcRepresentationMap")
                && kinds.reference(entity, "MappedRepresentation") == Some(representation)
            {
                found.push(Usage::Map(referrer));
            } else if self.schema.is_a(t, "IfcShapeAspect") && lists("ShapeRepresentations") {
                found.push(Usage::Aspect(referrer));
            }
        }
        found
    }

    /// The ids `entity` lists in attribute `name`.
    pub(super) fn list(&self, kinds: &Kinds<'_>, entity: EntityId, name: &str) -> Vec<EntityId> {
        self.view
            .get(entity)
            .map(|entity| kinds.references(entity, name))
            .unwrap_or_default()
    }

    /// Take `member` out of `entity`'s list `name`; an emptied optional
    /// list is unset.
    fn drop_member(
        &mut self,
        kinds: &Kinds<'_>,
        entity: EntityId,
        name: &str,
        member: EntityId,
        optional: bool,
    ) -> Result<EntityId, F> {
        let mut members = self.list(kinds, entity, name);
        members.retain(|id| *id != member);
        let value = if members.is_empty() && optional {
            Value::Null
        } else {
            refs(&members)
        };
        self.edit(entity, vec![(name.to_owned(), value)])
    }

    /// Write a whole record, checked as an edit checks one.
    pub(super) fn insert(&mut self, entity: Entity) -> Result<EntityId, F> {
        recheck(self.schema, &entity)?;
        relations(self.schema, &self.view, &entity, None)?;
        let id = self.view.allocate();
        self.view.create(id, entity);
        Ok(id)
    }
}

pub(super) fn refs(ids: &[EntityId]) -> Value {
    Value::List(ids.iter().copied().map(Value::Ref).collect())
}

/// `value` with every reference `copies` maps replaced, at any depth.
pub(super) fn remap(value: &mut Value, copies: &BTreeMap<EntityId, EntityId>) {
    match value {
        Value::Ref(id) => {
            if let Some(copy) = copies.get(id) {
                *id = *copy;
            }
        }
        Value::List(items) => items.iter_mut().for_each(|item| remap(item, copies)),
        Value::Typed { value, .. } => remap(value, copies),
        _ => {}
    }
}
