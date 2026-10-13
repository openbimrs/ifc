//! The surface colour a viewer paints each product's Body with (#449).
//!
//! `ifc-style` answers where a style is assigned
//! ([`StyleIndex`](ifc_style::StyleIndex)): on the representation item, on
//! its presentation layer, or on a material. `ifc-material` answers which
//! material an element is made of. This module joins them per product, for
//! the items of the Body representation a viewer draws
//! ([`select_shape_representation`]):
//!
//! 1. **The item's own style**: its `IfcStyledItem`, then its styled
//!    presentation layer (the standard's precedence, `ifc-style`). A
//!    boolean result with no style of its own takes its `FirstOperand`'s,
//!    and an item inside a mapped representation with no style of its own
//!    takes the enclosing `IfcMappedItem`'s, innermost first.
//! 2. **The material's style**: the `IfcMaterialDefinitionRepresentation`
//!    of the `IfcMaterial` the product is made of, the product's own
//!    `IfcRelAssociatesMaterial` first, else its type's.
//!
//! The order, and why the standard supports it, is documented and cited
//! in [`ifc_style::StyleIndex`] (its Precedence section): item over layer
//! is the standard's (IFC4 ADD2 TC1, `IfcPresentationLayerWithStyle`);
//! material last is the general case `IfcStyledItem` and
//! `IfcMaterialDefinitionRepresentation` describe, and IfcOpenShell's
//! order.
//!
//! # Which material
//!
//! Only an `IfcMaterial` has a representation, so each material select
//! comes down to one:
//!
//! - `IfcMaterial`: itself.
//! - `IfcMaterialLayer`, `IfcMaterialProfile`, `IfcMaterialConstituent`:
//!   its `Material`.
//! - `IfcMaterialLayerSet`, `IfcMaterialProfileSet` and their usages
//!   (`ForLayerSet`, `ForProfileSet`; IFC2X3's `IfcMaterialLayerSetUsage`
//!   too), and IFC2X3's `IfcMaterialList`: the one material their members
//!   name. The standard does not say which layer or profile of a body a
//!   viewer paints, so members naming two materials are
//!   [`ColourError::AmbiguousMaterial`], never the first one.
//! - `IfcMaterialConstituentSet` (IFC4, IFC4X3): by item. IFC4 associates a
//!   constituent with "the part of the shape representation" through an
//!   `IfcShapeAspect` whose `Name` is the constituent's `Name`
//!   (`IfcMaterialConstituent`), so an item an aspect of this product
//!   (or of a representation map it is drawn through) lists takes that
//!   constituent's material. An item no aspect names takes the set's one
//!   material, or is ambiguous.
//!
//! IFC4X1 and IFC4X2 files resolve item and layer styles; the material
//! domain is not verified for those releases, so a material fallback there
//! is refused with its typed error.
//!
//! # One colour per product
//!
//! A mesh is one triangle soup per product (`product_meshes`), so
//! the product's colour is the colour its items agree on.
//! [`ColourResolver::item_colours`] gives every item's; items that differ
//! (one painted, one not, or two colours) are
//! [`ColourError::ItemsDiffer`] for the product.

#![cfg(feature = "colour")]

use std::collections::BTreeMap;
use std::fmt;

use ifc_geometry::{select_shape_representation, GeometryError};
use ifc_material::{
    MaterialConstituent, MaterialConstituentSet, MaterialDefinition, MaterialError,
    MaterialLayerSet, MaterialProfileSet, MaterialUsageDefinition, MaterialView,
    ResolvedMaterialSelect,
};
use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::{Schema, SchemaVersion};
use ifc_style::{StyleError, StyleIndex, StyleView};

pub use ifc_style::{ColourSource, SurfaceColour};

/// How deep mapped items and boolean operands are followed.
const MAX_DEPTH: usize = 64;

/// Why a product's or an item's colour could not be resolved.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ColourError {
    /// The header declares no single release this build bundles.
    UnsupportedSchema {
        /// The header's `FILE_SCHEMA`, comma-separated.
        schema: String,
    },
    /// A style is malformed or ambiguous
    /// ([`StyleError::AmbiguousStyleAssignment`],
    /// [`StyleError::AmbiguousSurfaceColour`]).
    Style(StyleError),
    /// The material association is malformed or ambiguous
    /// ([`MaterialError::AmbiguousAssignment`], say).
    Material(MaterialError),
    /// The Body representation or a mapped item is malformed, cyclic or
    /// too deep.
    Geometry(GeometryError),
    /// The material select names several materials and the standard does
    /// not say which one paints the item: a layer or profile set of two
    /// materials, a list, or a constituent set no shape aspect resolves.
    AmbiguousMaterial {
        /// The representation item.
        item: EntityId,
        /// The material select (the set, the usage or the list).
        definition: EntityId,
        /// The materials it names, ascending.
        materials: Vec<EntityId>,
    },
    /// The product's Body items resolve to different colours, or some to
    /// none; a mesh has one colour per product.
    ItemsDiffer {
        /// The product.
        product: EntityId,
        /// Its Body's items, in representation order.
        items: Vec<EntityId>,
    },
}

impl fmt::Display for ColourError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchema { schema } => {
                write!(f, "schema {schema} is not one bundled release")
            }
            Self::Style(error) => write!(f, "{error}"),
            Self::Material(error) => write!(f, "{error}"),
            Self::Geometry(error) => write!(f, "{error}"),
            Self::AmbiguousMaterial {
                item,
                definition,
                materials,
            } => write!(
                f,
                "item {item}: material {definition} names {} materials and none is defined to \
                 colour it",
                materials.len()
            ),
            Self::ItemsDiffer { product, items } => write!(
                f,
                "the {} Body items of {product} resolve to different colours",
                items.len()
            ),
        }
    }
}

impl std::error::Error for ColourError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Style(error) => Some(error),
            Self::Material(error) => Some(error),
            Self::Geometry(error) => Some(error),
            _ => None,
        }
    }
}

impl From<StyleError> for ColourError {
    fn from(error: StyleError) -> Self {
        Self::Style(error)
    }
}

impl From<MaterialError> for ColourError {
    fn from(error: MaterialError) -> Self {
        Self::Material(error)
    }
}

impl From<GeometryError> for ColourError {
    fn from(error: GeometryError) -> Self {
        Self::Geometry(error)
    }
}

/// The surface colour of each of `products`, or, with `None`, of every
/// product that has a shape ([`ifc_geometry::geometric_products`]), in id
/// order. `Ok(None)`: nothing colours it.
///
/// Builds one [`ColourResolver`]; a model it refuses refuses every product
/// with the same error.
pub fn product_colours(
    model: &Model,
    products: Option<&[EntityId]>,
) -> Vec<(EntityId, Result<Option<SurfaceColour>, ColourError>)> {
    let products = products.map_or_else(
        || ifc_geometry::geometric_products(model),
        <[EntityId]>::to_vec,
    );
    match ColourResolver::new(model) {
        Ok(resolver) => products
            .into_iter()
            .map(|product| (product, resolver.product_colour(product)))
            .collect(),
        Err(error) => products
            .into_iter()
            .map(|product| (product, Err(error.clone())))
            .collect(),
    }
}

/// Every style assignment and shape aspect of a model, indexed once, for
/// colour lookups per product.
#[derive(Debug, Clone)]
pub struct ColourResolver<'m> {
    model: &'m Model,
    schema: &'static Schema,
    styles: StyleIndex<'m, 'static>,
    materials: MaterialView<'m>,
    /// Representation item -> the `IfcShapeAspect`s listing it.
    aspects: BTreeMap<EntityId, Vec<Aspect>>,
}

/// One `IfcShapeAspect`, as the constituent lookup reads it.
#[derive(Debug, Clone)]
struct Aspect {
    /// `PartOfProductDefinitionShape`, when stated.
    owner: Option<EntityId>,
    /// `Name`, which names the constituent.
    name: Option<String>,
}

/// One Body item, and what encloses it.
#[derive(Debug, Clone)]
struct Leaf {
    item: EntityId,
    /// Enclosing `IfcMappedItem`s, innermost first.
    mapped: Vec<EntityId>,
    /// The product's `IfcProductDefinitionShape` and every
    /// `IfcRepresentationMap` the item is drawn through.
    owners: Vec<EntityId>,
}

/// The `IfcMaterial` a product is made of, as far as its select says.
#[derive(Debug, Clone)]
enum Made {
    Nothing,
    One(EntityId),
    /// A constituent set naming several materials: `(Name, Material)`.
    Constituents {
        set: EntityId,
        parts: Vec<(Option<String>, EntityId)>,
    },
    Several {
        definition: EntityId,
        materials: Vec<EntityId>,
    },
}

impl<'m> ColourResolver<'m> {
    /// Index `model`'s styles and shape aspects against the release its
    /// header declares.
    ///
    /// # Errors
    ///
    /// [`ColourError::UnsupportedSchema`] unless the header declares one
    /// release this build bundles, and the style index's errors
    /// ([`StyleIndex::new`]).
    pub fn new(model: &'m Model) -> Result<Self, ColourError> {
        let declared = &model.header().schema;
        let unsupported = || ColourError::UnsupportedSchema {
            schema: declared.join(","),
        };
        let [token] = declared.as_slice() else {
            return Err(unsupported());
        };
        let version = SchemaVersion::from_header_token(token).ok_or_else(unsupported)?;
        let schema = ifc_schema::for_version(version).map_err(|_| unsupported())?;
        let styles = StyleIndex::new(StyleView::new(model, schema))?;
        let mut resolver = Self {
            model,
            schema,
            styles,
            materials: MaterialView::new(model),
            aspects: BTreeMap::new(),
        };
        resolver.index_aspects()?;
        Ok(resolver)
    }

    /// The colour every Body item of `product` agrees on; `Ok(None)` when
    /// nothing colours it (or it has no Body).
    ///
    /// # Errors
    ///
    /// [`ColourError::ItemsDiffer`] when its items disagree, and every
    /// error of [`Self::item_colours`].
    pub fn product_colour(&self, product: EntityId) -> Result<Option<SurfaceColour>, ColourError> {
        let colours = self.item_colours(product)?;
        let Some((_, first)) = colours.first() else {
            return Ok(None);
        };
        let same = |a: &Option<SurfaceColour>, b: &Option<SurfaceColour>| match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => a.rgb == b.rgb && a.transparency == b.transparency,
            _ => false,
        };
        if colours.iter().all(|(_, colour)| same(colour, first)) {
            Ok(*first)
        } else {
            Err(ColourError::ItemsDiffer {
                product,
                items: colours.into_iter().map(|(item, _)| item).collect(),
            })
        }
    }

    /// The colour of each item of `product`'s Body, in representation
    /// order with mapped representations expanded: `(item, colour)`.
    /// Empty when the product has no Body.
    ///
    /// # Errors
    ///
    /// The first item's error, in order: a malformed or ambiguous style
    /// ([`ColourError::Style`]), a malformed or ambiguous material
    /// association ([`ColourError::Material`],
    /// [`ColourError::AmbiguousMaterial`]), and a missing, cyclic or
    /// too-deep representation ([`ColourError::Geometry`]).
    pub fn item_colours(
        &self,
        product: EntityId,
    ) -> Result<Vec<(EntityId, Option<SurfaceColour>)>, ColourError> {
        let mut made = None;
        self.leaves(product)?
            .into_iter()
            .map(|leaf| {
                let colour = self.leaf_colour(product, &leaf, &mut made)?;
                Ok((leaf.item, colour))
            })
            .collect()
    }

    fn leaf_colour(
        &self,
        product: EntityId,
        leaf: &Leaf,
        made: &mut Option<Made>,
    ) -> Result<Option<SurfaceColour>, ColourError> {
        // 1. The item, then a boolean's first operand, then the enclosing
        //    mapped items.
        let mut item = leaf.item;
        for _ in 0..MAX_DEPTH {
            if let Some(colour) = self.styles.item_colour(item)? {
                return Ok(Some(colour));
            }
            let entity = self.entity(item, item)?;
            if !self.schema.is_a(&entity.type_name, "IfcBooleanResult") {
                break;
            }
            item = self.reference(item, entity, "FirstOperand")?;
        }
        for &mapped in &leaf.mapped {
            if let Some(colour) = self.styles.item_colour(mapped)? {
                return Ok(Some(colour));
            }
        }
        // 2. The material, resolved once per product.
        if made.is_none() {
            *made = Some(self.made_of(product)?);
        }
        let material = match made.as_ref().expect("resolved above") {
            Made::Nothing => return Ok(None),
            Made::One(material) => *material,
            Made::Several {
                definition,
                materials,
            } => {
                return Err(ColourError::AmbiguousMaterial {
                    item: leaf.item,
                    definition: *definition,
                    materials: materials.clone(),
                })
            }
            Made::Constituents { set, parts } => self.constituent(leaf, *set, parts)?,
        };
        Ok(self.styles.material_colour(material)?)
    }

    /// The material of the constituent an `IfcShapeAspect` of this
    /// product names for `leaf`.
    fn constituent(
        &self,
        leaf: &Leaf,
        set: EntityId,
        parts: &[(Option<String>, EntityId)],
    ) -> Result<EntityId, ColourError> {
        let mut materials: Vec<EntityId> = self
            .aspects
            .get(&leaf.item)
            .into_iter()
            .flatten()
            .filter(|aspect| {
                aspect
                    .owner
                    .is_none_or(|owner| leaf.owners.contains(&owner))
            })
            .filter_map(|aspect| aspect.name.as_deref())
            .flat_map(|name| {
                parts
                    .iter()
                    .filter(move |(part, _)| part.as_deref() == Some(name))
                    .map(|(_, material)| *material)
            })
            .collect();
        materials.sort_unstable();
        materials.dedup();
        if let [material] = materials.as_slice() {
            return Ok(*material);
        }
        if materials.is_empty() {
            materials = parts.iter().map(|(_, material)| *material).collect();
            materials.sort_unstable();
            materials.dedup();
        }
        Err(ColourError::AmbiguousMaterial {
            item: leaf.item,
            definition: set,
            materials,
        })
    }

    /// What `product` is made of, by its own association or its type's.
    fn made_of(&self, product: EntityId) -> Result<Made, ColourError> {
        use MaterialDefinition as D;
        use MaterialUsageDefinition as U;
        use ResolvedMaterialSelect as S;
        let Some(assigned) = self.materials.assigned_material(product)? else {
            return Ok(Made::Nothing);
        };
        let view = self.materials;
        let (definition, materials) = match assigned.material {
            S::Definition(D::Material(material)) => return Ok(Made::One(material.id())),
            S::Definition(D::Layer(layer)) => return Ok(one(layer.material_id()?)),
            S::Definition(D::LayerWithOffsets(layer)) => return Ok(one(layer.material_id()?)),
            S::Definition(D::Profile(profile)) => return Ok(one(profile.material_id()?)),
            S::Definition(D::ProfileWithOffsets(profile)) => {
                return Ok(one(profile.material_id()?))
            }
            S::Definition(D::Constituent(constituent)) => {
                return Ok(Made::One(constituent.material_id()?))
            }
            S::Definition(D::LayerSet(set)) => (set.id(), self.layer_materials(set)?),
            S::Usage(U::LayerSet(usage)) => {
                let set = MaterialLayerSet::try_from_view(view, usage.layer_set_id()?)?;
                (usage.id(), self.layer_materials(set)?)
            }
            S::Definition(D::ProfileSet(set)) => (set.id(), self.profile_materials(set)?),
            S::Usage(U::ProfileSet(usage)) => {
                let set = MaterialProfileSet::try_from_view(view, usage.profile_set_id()?)?;
                (usage.id(), self.profile_materials(set)?)
            }
            S::Usage(U::ProfileSetTapering(usage)) => {
                let set = MaterialProfileSet::try_from_view(view, usage.profile_set_id()?)?;
                (usage.id(), self.profile_materials(set)?)
            }
            S::List(list) => (list.id(), list.material_ids()?),
            S::Definition(D::ConstituentSet(set)) => {
                return self.constituents(set);
            }
        };
        Ok(distinct(definition, materials))
    }

    fn layer_materials(&self, set: MaterialLayerSet<'m>) -> Result<Vec<EntityId>, ColourError> {
        let mut materials = Vec::new();
        for id in set.layer_ids()? {
            let material = match self.materials.resolve_material_select(id)? {
                ResolvedMaterialSelect::Definition(MaterialDefinition::Layer(layer)) => {
                    layer.material_id()?
                }
                ResolvedMaterialSelect::Definition(MaterialDefinition::LayerWithOffsets(layer)) => {
                    layer.material_id()?
                }
                _ => return Err(self.wrong(id, "IfcMaterialLayer")),
            };
            // A layer with no material is an air gap: it paints nothing.
            materials.extend(material);
        }
        Ok(materials)
    }

    fn profile_materials(&self, set: MaterialProfileSet<'m>) -> Result<Vec<EntityId>, ColourError> {
        let mut materials = Vec::new();
        for id in set.profile_ids()? {
            let material = match self.materials.resolve_material_select(id)? {
                ResolvedMaterialSelect::Definition(MaterialDefinition::Profile(profile)) => {
                    profile.material_id()?
                }
                ResolvedMaterialSelect::Definition(MaterialDefinition::ProfileWithOffsets(
                    profile,
                )) => profile.material_id()?,
                _ => return Err(self.wrong(id, "IfcMaterialProfile")),
            };
            materials.extend(material);
        }
        Ok(materials)
    }

    fn constituents(&self, set: MaterialConstituentSet<'m>) -> Result<Made, ColourError> {
        let mut parts = Vec::new();
        for id in set.constituent_ids()?.unwrap_or_default() {
            let constituent = MaterialConstituent::try_from_view(self.materials, id)?;
            let name = match constituent.name() {
                Ok(name) => name.map(str::to_owned),
                Err(MaterialError::NotInSchema { .. }) => None,
                Err(error) => return Err(error.into()),
            };
            parts.push((name, constituent.material_id()?));
        }
        Ok(
            match distinct(set.id(), parts.iter().map(|(_, m)| *m).collect()) {
                Made::Several { .. } => Made::Constituents {
                    set: set.id(),
                    parts,
                },
                made => made,
            },
        )
    }

    fn wrong(&self, id: EntityId, expected: &'static str) -> ColourError {
        ColourError::Material(MaterialError::WrongEntityType {
            expected,
            actual: self
                .model
                .get(id)
                .map_or_else(String::new, |entity| entity.type_name.to_string()),
        })
    }

    /// The Body's items, with mapped representations expanded.
    fn leaves(&self, product: EntityId) -> Result<Vec<Leaf>, ColourError> {
        let Some(body) = select_shape_representation(self.model, product)? else {
            return Ok(Vec::new());
        };
        let entity = self.entity(product, product)?;
        let shape = self.reference(product, entity, "Representation")?;
        let mut leaves = Vec::new();
        self.walk(body, &[], &[shape], &mut leaves)?;
        Ok(leaves)
    }

    fn walk(
        &self,
        representation: EntityId,
        mapped: &[EntityId],
        owners: &[EntityId],
        leaves: &mut Vec<Leaf>,
    ) -> Result<(), ColourError> {
        let entity = self.entity(representation, representation)?;
        for item in self.references(representation, entity, "Items")? {
            let item_entity = self.entity(representation, item)?;
            if !self.schema.is_a(&item_entity.type_name, "IfcMappedItem") {
                leaves.push(Leaf {
                    item,
                    mapped: mapped.to_vec(),
                    owners: owners.to_vec(),
                });
                continue;
            }
            let map = self.reference(item, item_entity, "MappingSource")?;
            if owners.contains(&map) {
                return Err(GeometryError::CyclicChain {
                    entity: item,
                    kind: "mapped item",
                }
                .into());
            }
            if owners.len() > MAX_DEPTH {
                return Err(GeometryError::ChainTooDeep {
                    entity: item,
                    kind: "mapped item",
                    limit: MAX_DEPTH,
                }
                .into());
            }
            let map_entity = self.entity(item, map)?;
            let inner = self.reference(map, map_entity, "MappedRepresentation")?;
            let mut enclosing = Vec::with_capacity(mapped.len() + 1);
            enclosing.push(item);
            enclosing.extend_from_slice(mapped);
            let mut within = owners.to_vec();
            within.push(map);
            self.walk(inner, &enclosing, &within, leaves)?;
        }
        Ok(())
    }

    /// Index every `IfcShapeAspect` by the items of its representations.
    fn index_aspects(&mut self) -> Result<(), ColourError> {
        if self.schema.entity("IfcShapeAspect").is_none() {
            return Ok(());
        }
        for (id, entity) in self.model.of_type("IFCSHAPEASPECT") {
            let name = self
                .value(id, entity, "Name")?
                .and_then(|value| value.unwrap_typed().as_text())
                .map(str::to_owned);
            let owner = self
                .value(id, entity, "PartOfProductDefinitionShape")?
                .and_then(Value::as_ref_id);
            for representation in self.references(id, entity, "ShapeRepresentations")? {
                let representation_entity = self.entity(id, representation)?;
                for item in self.references(representation, representation_entity, "Items")? {
                    self.aspects.entry(item).or_default().push(Aspect {
                        owner,
                        name: name.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    fn entity(&self, referrer: EntityId, id: EntityId) -> Result<&'m Entity, ColourError> {
        self.model.get(id).ok_or_else(|| {
            GeometryError::MissingEntity {
                referrer,
                missing: id,
            }
            .into()
        })
    }

    /// The value of attribute `name`, by the declared release's slot;
    /// `None` when it is unset.
    fn value(
        &self,
        id: EntityId,
        entity: &'m Entity,
        name: &'static str,
    ) -> Result<Option<&'m Value>, ColourError> {
        let type_name = &entity.type_name;
        let slot = (0..self.schema.attribute_count(type_name))
            .find(|&slot| {
                self.schema
                    .attribute_at(type_name, slot)
                    .is_some_and(|attribute| attribute.name.eq_ignore_ascii_case(name))
            })
            .ok_or_else(|| missing(id, entity, name))?;
        Ok(entity
            .attribute(slot)
            .filter(|value| !matches!(value, Value::Null | Value::Derived)))
    }

    fn reference(
        &self,
        id: EntityId,
        entity: &'m Entity,
        name: &'static str,
    ) -> Result<EntityId, ColourError> {
        let target = self
            .value(id, entity, name)?
            .and_then(Value::as_ref_id)
            .ok_or_else(|| missing(id, entity, name))?;
        self.entity(id, target)?;
        Ok(target)
    }

    fn references(
        &self,
        id: EntityId,
        entity: &'m Entity,
        name: &'static str,
    ) -> Result<Vec<EntityId>, ColourError> {
        let list = self
            .value(id, entity, name)?
            .and_then(Value::as_list)
            .ok_or_else(|| missing(id, entity, name))?;
        list.iter()
            .map(|value| {
                let target = value.as_ref_id().ok_or_else(|| missing(id, entity, name))?;
                self.entity(id, target)?;
                Ok(target)
            })
            .collect()
    }
}

fn missing(id: EntityId, entity: &Entity, attribute: &'static str) -> GeometryError {
    GeometryError::MissingAttribute {
        entity: id,
        type_name: entity.type_name.to_string(),
        attribute,
    }
}

fn one(material: Option<EntityId>) -> Made {
    material.map_or(Made::Nothing, Made::One)
}

/// The one material `materials` name, nothing, or several.
fn distinct(definition: EntityId, mut materials: Vec<EntityId>) -> Made {
    materials.sort_unstable();
    materials.dedup();
    match materials.as_slice() {
        [] => Made::Nothing,
        [material] => Made::One(*material),
        _ => Made::Several {
            definition,
            materials,
        },
    }
}
