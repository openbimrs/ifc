//! What decides the surface colour of a Body item or a product (#659).
//!
//! [`ColourResolver::item_colours`] and [`ColourResolver::product_colour`]
//! say which colour; this says what decides it, also while it states none:
//! a material that carries no style yet is still the material an editor
//! would style. Both read the same rules ([`ColourResolver::leaf_decider`]
//! is the one place they live), so they never disagree.

use ifc_model::EntityId;

use super::{same_colour, ColourError, ColourResolver, Leaf, Made, SurfaceColour, MAX_DEPTH};

/// What decides the surface colour of a Body item, or of a product whose
/// items agree on it, whether or not it states a colour yet (#659).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ColourDecider {
    /// The item's own style or layer, a boolean's `FirstOperand`'s, or the
    /// enclosing mapped item's: no material decides. Items are the same
    /// decider when they share the `IfcSurfaceStyle`, whichever
    /// `IfcStyledItem` assigns it; for a product the colour's source names
    /// the first item's.
    ItemStyle(SurfaceColour),
    /// One `IfcMaterial`: the product's own or its type's, the single one a
    /// set or list names, or the constituent an aspect names.
    Material {
        /// The `IfcMaterial`.
        material: EntityId,
        /// Its own colour; `None` while it states no style.
        colour: Option<SurfaceColour>,
    },
    /// Several materials decide together: all unstyled (`None`) or all
    /// styled alike (`Some`). Styling one of them alone makes the item
    /// [`ColourError::AmbiguousMaterial`].
    Materials {
        /// The material select (the set, the usage or the list).
        definition: EntityId,
        /// The materials it names, ascending.
        materials: Vec<EntityId>,
        /// The colour they agree on; `None` while none is styled.
        colour: Option<SurfaceColour>,
    },
    /// No Body, no material association, or a select naming no
    /// `IfcMaterial`.
    Nothing,
}

impl ColourDecider {
    /// The colour this decides; `None` while it states none.
    #[must_use]
    pub fn colour(&self) -> Option<SurfaceColour> {
        match self {
            Self::ItemStyle(colour) => Some(colour.clone()),
            Self::Material { colour, .. } | Self::Materials { colour, .. } => colour.clone(),
            Self::Nothing => None,
        }
    }

    /// The materials that decide, ascending where several; empty for
    /// [`Self::ItemStyle`] and [`Self::Nothing`].
    #[must_use]
    pub fn materials(&self) -> &[EntityId] {
        match self {
            Self::Material { material, .. } => std::slice::from_ref(material),
            Self::Materials { materials, .. } => materials,
            Self::ItemStyle(_) | Self::Nothing => &[],
        }
    }

    /// Whether `other` is the same decider, by identity: item styles by
    /// their `IfcSurfaceStyle` alone (items each styled by their own
    /// `IfcStyledItem` of one surface style agree), materials by
    /// `IfcMaterial` (or by set and members).
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::ItemStyle(a), Self::ItemStyle(b)) => a.surface_style == b.surface_style,
            (Self::Material { material: a, .. }, Self::Material { material: b, .. }) => a == b,
            (
                Self::Materials {
                    definition: a,
                    materials: am,
                    ..
                },
                Self::Materials {
                    definition: b,
                    materials: bm,
                    ..
                },
            ) => a == b && am == bm,
            (Self::Nothing, Self::Nothing) => true,
            _ => false,
        }
    }
}

impl ColourResolver<'_> {
    /// What decides the colour of each item of `product`'s Body, in
    /// representation order with mapped representations expanded:
    /// `(item, decider)`. Empty when the product has no Body.
    ///
    /// # Errors
    ///
    /// The same as [`Self::item_colours`], which reads this.
    pub fn item_deciders(
        &self,
        product: EntityId,
    ) -> Result<Vec<(EntityId, ColourDecider)>, ColourError> {
        let mut made = None;
        self.leaves(product)?
            .into_iter()
            .map(|leaf| {
                let decider = self.leaf_decider(product, &leaf, &mut made)?;
                Ok((leaf.item, decider))
            })
            .collect()
    }

    /// What decides the colour of `product`, when all its Body items agree
    /// on one decider (item styles: one `IfcSurfaceStyle`, the first item's
    /// styled item as source); [`ColourDecider::Nothing`] when it has no Body.
    ///
    /// # Errors
    ///
    /// [`ColourError::ItemsDiffer`] exactly when [`Self::product_colour`]
    /// reports it; [`ColourError::DecidersDiffer`] when the items agree on a
    /// colour through different deciders; every error of
    /// [`Self::item_deciders`].
    pub fn product_decider(&self, product: EntityId) -> Result<ColourDecider, ColourError> {
        let deciders = self.item_deciders(product)?;
        let Some((_, first)) = deciders.first() else {
            return Ok(ColourDecider::Nothing);
        };
        if deciders.iter().all(|(_, decider)| decider.same(first)) {
            return Ok(first.clone());
        }
        let first = first.colour();
        let items = deciders.iter().map(|(item, _)| *item).collect();
        if deciders
            .iter()
            .all(|(_, decider)| same_colour(&decider.colour(), &first))
        {
            Err(ColourError::DecidersDiffer { product, items })
        } else {
            Err(ColourError::ItemsDiffer { product, items })
        }
    }

    /// What decides `leaf`'s colour: its own style first, else the
    /// product's material (resolved once into `made`).
    pub(super) fn leaf_decider(
        &self,
        product: EntityId,
        leaf: &Leaf,
        made: &mut Option<Made>,
    ) -> Result<ColourDecider, ColourError> {
        // 1. The item, then a boolean's first operand, then the enclosing
        //    mapped items.
        let mut item = leaf.item;
        for _ in 0..MAX_DEPTH {
            if let Some(colour) = self.styles.item_colour(item)? {
                return Ok(ColourDecider::ItemStyle(colour));
            }
            let entity = self.entity(item, item)?;
            if !self.schema.is_a(&entity.type_name, "IfcBooleanResult") {
                break;
            }
            item = self.reference(item, entity, "FirstOperand")?;
        }
        for &mapped in &leaf.mapped {
            if let Some(colour) = self.styles.item_colour(mapped)? {
                return Ok(ColourDecider::ItemStyle(colour));
            }
        }
        // 2. The material, resolved once per product.
        if made.is_none() {
            *made = Some(self.made_of(product)?);
        }
        match made.as_ref().expect("resolved above") {
            Made::Nothing => Ok(ColourDecider::Nothing),
            Made::One(material) => self.material_decider(*material),
            Made::Several {
                definition,
                materials,
            } => Ok(ColourDecider::Materials {
                definition: *definition,
                materials: materials.clone(),
                colour: self.agreed(leaf.item, *definition, materials)?,
            }),
            Made::Constituents { set, parts } => {
                let materials = self.constituent(leaf, parts)?;
                match materials.as_slice() {
                    [material] => self.material_decider(*material),
                    _ => Ok(ColourDecider::Materials {
                        definition: *set,
                        colour: self.agreed(leaf.item, *set, &materials)?,
                        materials,
                    }),
                }
            }
        }
    }

    fn material_decider(&self, material: EntityId) -> Result<ColourDecider, ColourError> {
        Ok(ColourDecider::Material {
            material,
            colour: self.material_colour(material)?,
        })
    }
}
