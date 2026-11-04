//! What a product's selected representation is made of, without lowering it.
//!
//! # Why a description and not a mesh
//!
//! Rule checks on structural members and walls ask how a body is modelled:
//! "is this beam an extrusion of an I-section, how deep is the web, which way
//! does it run". A mesh has lost every one of those answers. This module
//! reads them from the representation items directly, in SI units and world
//! coordinates (or, for `local_sweep`, the item's own frame), and links no
//! geometry kernel.
//!
//! # One entry per item, never a merged answer
//!
//! A Body representation may hold several items, and a mapped item may map
//! several more. [`body_description`] reports one [`BodyItem`] per resolved
//! geometric item, in authored order, rather than refusing or picking one:
//! a column with a base plate is two extrusions, and both are facts a rule
//! check may need. [`BodyDescription::sole_item`] is the convenience for the
//! common single-item case.
//!
//! # All or nothing
//!
//! If any item cannot be described exactly (an unsupported family, a dangling
//! reference, a profile that bounds no area), the whole call fails with
//! a typed error naming the entity. A partial list would let a caller treat a
//! body as fully checked when part of it was never read.
//!
//! # Purposes
//!
//! [`body_description`] reads the Body representation. [`body_description_for`]
//! reads another purpose, such as the `Reference` void of an IFC4 Reference
//! View `IfcOpeningElement`, which is not subtracted from its host. A product
//! without that purpose's representation yields `Ok(None)`; no other
//! representation stands in for it.
//!
//! # Frames
//!
//! Items are placed exactly as lowering places them: the representation
//! context's `WorldCoordinateSystem`, then the product's placement chain, then
//! for a mapped item `MappingTarget` and `MappingOrigin`, then the item's own
//! `Position`. Mapped geometry therefore resolves to the same answer as the
//! same geometry authored in place.

mod bounds;
mod kind;
mod local;
mod sweep;

pub use kind::BodyKind;

use ifc_model::{EntityId, Model};

use super::context::product_representation_frame;
use super::profile::ProfileDescription;
use super::representation::{select_product_representation, Representation, RepresentationPurpose};
use crate::error::{GeometryError, GeometryResult};
use crate::resource::mapped::MappingWalker;
use crate::resource::operator::operator_transform;
use crate::resource::placement::axis_placement_transform;
use crate::slots::Slots;
use crate::transform::Transform;
use crate::units::UnitScale;

/// The representation of one product selected for a purpose (the Body by
/// default), one entry per geometric item.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct BodyDescription {
    /// The product described.
    pub product: EntityId,
    /// The representation selected for [`Self::purpose`]
    /// ([`crate::select_product_representation`]; for `Body` that is
    /// [`crate::select_shape_representation`]).
    pub representation: EntityId,
    /// The purpose the representation was selected for.
    pub purpose: RepresentationPurpose,
    /// The frame of the selected representation, in metres: the representation
    /// context's `WorldCoordinateSystem` and the product placement
    /// ([`product_representation_frame`]'s answer for [`Self::purpose`]).
    /// Every [`BodyItem::item_world`] is this frame composed with the item's
    /// mappings, and every [`BodyItemBounds`] is expressed in it.
    pub frame: Transform,
    /// One entry per geometric item, mapped items resolved, in authored order.
    pub items: Vec<BodyItem>,
}

impl BodyDescription {
    /// The only item, when the body has exactly one.
    ///
    /// `None` for an empty body and for a body of several items; a caller
    /// that needs every item reads [`Self::items`].
    pub fn sole_item(&self) -> Option<&BodyItem> {
        match self.items.as_slice() {
            [item] => Some(item),
            _ => None,
        }
    }
}

/// One geometric representation item of a body.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct BodyItem {
    /// The geometric item itself, never an `IfcMappedItem`.
    pub item: EntityId,
    /// Its concrete IFC type in upper case.
    pub type_name: String,
    /// The `IfcMappedItem`s this item was reached through, outermost first.
    /// Empty when the item is authored directly in the body.
    pub mapped_by: Vec<EntityId>,
    /// How the item models its shape.
    pub kind: BodyKind,
    /// Profile and path in world coordinates, for the swept-area families
    /// ([`BodyKind::is_swept_area`]); `None` for every other kind and for a
    /// swept-area item whose [`Self::item_world`] is not rigid (a mapping
    /// scaled or mirrored it). [`Self::local_sweep`] still describes it.
    pub swept: Option<SweptSolid>,
    /// Profile, `Position` and path in the item's own frame, for every
    /// swept-area family ([`BodyKind::is_swept_area`]); `None` for every other
    /// kind. These are the authored parameters whatever the mapping does;
    /// [`Self::item_world`] places them.
    pub local_sweep: Option<LocalSweptSolid>,
    /// The frame the item's own coordinates are placed in, in metres: the
    /// representation context's `WorldCoordinateSystem` and the product
    /// placement, composed with every `MappingTarget o MappingOrigin` in
    /// [`Self::mapped_by`] (#185). The identity composition for an item
    /// authored directly in a body at the origin.
    ///
    /// Unlike [`SweptSolid::placement_world`] it is not required to be rigid:
    /// a mapping may scale or mirror any item kind, and this frame says so.
    /// A swept solid's world placement is this frame composed with the
    /// solid's own `Position` ([`LocalSweptSolid::position`]); it is
    /// `placement_world` when this frame is rigid.
    pub item_world: Transform,
    /// The exact bounds of a tessellated item's referenced vertices, in the
    /// representation frame ([`BodyDescription::frame`]): the item's mappings
    /// are applied, the frame is not. The frame maps the corner points: a
    /// world box needs all eight corners transformed, not just `min` and
    /// `max`. `Some` for
    /// [`BodyKind::Tessellated`], `None` for every other kind.
    pub bounds: Option<BodyItemBounds>,
}

/// An axis-aligned box in the representation frame
/// ([`BodyDescription::frame`]), in metres.
///
/// Exact over the vertices a face set's faces address, never over unreferenced
/// entries of its point list.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct BodyItemBounds {
    /// The smallest coordinate on each axis.
    pub min: [f64; 3],
    /// The largest coordinate on each axis.
    pub max: [f64; 3],
}

impl BodyItem {
    /// Whether the item is placed mirrored: [`Self::item_world`] reverses
    /// handedness, so a left-hand part appears as its right-hand twin.
    ///
    /// `None` when the frame is degenerate (a zero or non-finite
    /// determinant), which no valid placement or mapping produces.
    pub fn is_mirrored(&self) -> Option<bool> {
        let determinant = self.item_world.determinant();
        (determinant.is_finite() && determinant != 0.0).then_some(determinant < 0.0)
    }
}

/// A swept-area solid: its profile, where it sits, and the path it follows.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct SweptSolid {
    /// `SweptArea`, in metres and radians.
    pub profile: ProfileDescription,
    /// `EndSweptArea` of a tapered sweep; `None` otherwise.
    pub end_profile: Option<ProfileDescription>,
    /// The solid's `Position` composed into world coordinates, in metres:
    /// [`BodyItem::item_world`] composed with [`LocalSweptSolid::position`].
    ///
    /// The profile lies in this frame's XY plane. Rigid by construction: a
    /// swept solid whose `item_world` scales or mirrors has no `SweptSolid`
    /// (`BodyItem::swept` is `None`), because its profile parameters would no
    /// longer be the authored ones; [`LocalSweptSolid`] describes it instead.
    pub placement_world: Transform,
    /// The path the profile follows.
    pub path: SweepPath,
}

/// A swept-area solid in its own item frame: the frame
/// [`BodyItem::item_world`] places, before any mapping or product placement.
///
/// Every parameter is the authored one in metres and radians, whatever scale
/// or mirror the item's mappings apply. The world answer for a rigid
/// `item_world` is [`SweptSolid`]; in general it is
/// `item_world.compose(&position)`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct LocalSweptSolid {
    /// `SweptArea`, in metres and radians.
    pub profile: ProfileDescription,
    /// `EndSweptArea` of a tapered sweep; `None` otherwise.
    pub end_profile: Option<ProfileDescription>,
    /// The solid's `Position` in the item frame, in metres; `None` when the
    /// file omits it (IFC4 OPTIONAL), which is the identity. Rigid, being an
    /// `IfcAxis2Placement3D`.
    pub position: Option<Transform>,
    /// The path the profile follows, in the `Position` frame.
    pub path: LocalSweepPath,
}

/// The path of a swept-area solid, in its `Position` frame.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum LocalSweepPath {
    /// A straight extrusion (plain or tapered).
    #[non_exhaustive]
    Extrusion {
        /// `ExtrudedDirection` in the `Position` frame, unit length.
        direction: [f64; 3],
        /// `Depth`, in metres, measured along `direction`.
        depth: f64,
    },
    /// A revolution about an axis (plain or tapered).
    #[non_exhaustive]
    Revolution {
        /// The axis origin in the `Position` frame, in metres.
        axis_origin: [f64; 3],
        /// The axis direction in the `Position` frame, unit length.
        axis_direction: [f64; 3],
        /// `Angle`, in radians.
        angle: f64,
    },
    /// A sweep along a directrix curve, reported by reference.
    ///
    /// The curve and its `StartParam`/`EndParam` live in the directrix's own
    /// parameterisation; reading them is a curve question, not a body one.
    #[non_exhaustive]
    Directrix {
        /// The `Directrix` curve.
        directrix: EntityId,
    },
}

/// The path of a swept-area solid, in world coordinates.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SweepPath {
    /// A straight extrusion (plain or tapered).
    #[non_exhaustive]
    Extrusion {
        /// `ExtrudedDirection` in world coordinates, unit length.
        direction_world: [f64; 3],
        /// `Depth`, in metres, measured along `direction_world`.
        depth: f64,
    },
    /// A revolution about an axis (plain or tapered).
    #[non_exhaustive]
    Revolution {
        /// The axis origin in world coordinates, in metres.
        axis_origin_world: [f64; 3],
        /// The axis direction in world coordinates, unit length.
        axis_direction_world: [f64; 3],
        /// `Angle`, in radians.
        angle: f64,
    },
    /// A sweep along a directrix curve, reported by reference.
    ///
    /// The curve and its `StartParam`/`EndParam` live in the directrix's own
    /// parameterisation; reading them is a curve question, not a body one.
    #[non_exhaustive]
    Directrix {
        /// The `Directrix` curve.
        directrix: EntityId,
    },
}

/// Describe the Body representation of `product`.
///
/// Returns `Ok(None)` when the product has no body representation (only an
/// Axis or FootPrint, or none at all): that is an answer, not a failure. A
/// body whose representation lists no items yields an empty `items`.
/// Equivalent to [`body_description_for`] with [`RepresentationPurpose::Body`].
///
/// Every item is described or the call fails; see the module docs. A Body
/// tessellated item is now read for its bounds, so a malformed face set or a
/// TIN with void or hole flags fails the whole description. Kernel-free:
/// available with `--no-default-features`.
///
/// ```no_run
/// # use ifc_model::{EntityId, Model};
/// # use ifc_geometry::{body_description, units, BodyKind, SweepPath};
/// # fn demo(model: &Model, beam: EntityId) -> ifc_geometry::GeometryResult<()> {
/// let scale = units::resolve(model)?;
/// let body = body_description(model, &scale, beam).unwrap().expect("a body");
/// let item = body.sole_item().expect("one item");
/// if item.kind == BodyKind::Extrusion {
///     let swept = item.swept.as_ref().unwrap();
///     if let SweepPath::Extrusion { direction_world, depth, .. } = swept.path {
///         let _ = (swept.profile.type_name.as_str(), direction_world, depth);
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub fn body_description(
    model: &Model,
    units: &UnitScale,
    product: EntityId,
) -> GeometryResult<Option<BodyDescription>> {
    body_description_for(model, units, product, RepresentationPurpose::Body)
}

/// Describe the representation of `product` selected for `purpose`.
///
/// Like [`body_description`] for any purpose: the items of the selected
/// representation, mapped items resolved. `Ok(None)` when the product has no
/// representation for `purpose`; another purpose's representation is never
/// substituted. A tessellated item also carries its exact
/// [`BodyItem::bounds`], which refuses by name a face set lowering would
/// refuse. Kernel-free: available with `--no-default-features`.
pub fn body_description_for(
    model: &Model,
    units: &UnitScale,
    product: EntityId,
    purpose: RepresentationPurpose,
) -> GeometryResult<Option<BodyDescription>> {
    let Some(representation) = select_product_representation(model, product, purpose)? else {
        return Ok(None);
    };
    // The same frame lowering places the representation's items in.
    let Some(frame) = product_representation_frame(model, units, product, purpose)? else {
        return Ok(None);
    };

    let mut walk = Walk {
        model,
        units,
        walker: MappingWalker::new(),
        mapped_by: Vec::new(),
        items: Vec::new(),
    };
    walk.representation(product, representation, frame, Transform::identity())?;
    Ok(Some(BodyDescription {
        product,
        representation,
        purpose,
        frame,
        items: walk.items,
    }))
}

/// State for one body walk: the mapped-item stack and the collected items.
struct Walk<'m> {
    model: &'m Model,
    units: &'m UnitScale,
    walker: MappingWalker,
    mapped_by: Vec<EntityId>,
    items: Vec<BodyItem>,
}

impl Walk<'_> {
    /// Describe every item of one `IfcRepresentation` under `frame`; `local` is the
    /// mappings alone, without the representation frame.
    fn representation(
        &mut self,
        referrer: EntityId,
        representation: EntityId,
        frame: Transform,
        local: Transform,
    ) -> GeometryResult<()> {
        let entity = self
            .model
            .get(representation)
            .ok_or(GeometryError::MissingEntity {
                referrer,
                missing: representation,
            })?;
        for item in Representation::new(representation, entity).items()? {
            self.item(representation, item, frame, local)?;
        }
        Ok(())
    }

    /// Describe one item, resolving a mapped item to what it maps.
    fn item(
        &mut self,
        referrer: EntityId,
        item: EntityId,
        frame: Transform,
        local: Transform,
    ) -> GeometryResult<()> {
        let entity = self.model.get(item).ok_or(GeometryError::MissingEntity {
            referrer,
            missing: item,
        })?;
        let type_name = entity.type_name.to_ascii_uppercase();
        if type_name == "IFCMAPPEDITEM" {
            return self.mapped(item, frame, local);
        }
        let kind = BodyKind::classify(&type_name).ok_or_else(|| {
            Slots::new(item, entity).unsupported("representation item family is not described")
        })?;
        let (local_sweep, swept) = if kind.is_swept_area() {
            let local = local::describe(self.model, self.units, item, entity)?;
            let swept = sweep::world(&local, item, entity, frame)?;
            (Some(local), swept)
        } else {
            (None, None)
        };
        let bounds = if kind == BodyKind::Tessellated {
            Some(bounds::tessellated(
                self.model, self.units, item, entity, &type_name, local,
            )?)
        } else {
            None
        };
        self.items.push(BodyItem {
            item,
            type_name,
            mapped_by: self.mapped_by.clone(),
            kind,
            swept,
            local_sweep,
            item_world: frame,
            bounds,
        });
        Ok(())
    }

    /// Resolve an `IfcMappedItem`: `frame o MappingTarget o MappingOrigin`.
    ///
    /// The same composition as `lower::mapped`, so a mapped body and the same
    /// body authored in place describe identically.
    fn mapped(&mut self, item: EntityId, frame: Transform, local: Transform) -> GeometryResult<()> {
        self.walker.enter(item)?;
        let result = self.mapped_inner(item, frame, local);
        self.walker.exit();
        result
    }

    fn mapped_inner(
        &mut self,
        item: EntityId,
        frame: Transform,
        local: Transform,
    ) -> GeometryResult<()> {
        let instance = self.walker.resolve(self.model, item)?;
        let target_entity =
            self.model
                .get(instance.mapping_target)
                .ok_or(GeometryError::MissingEntity {
                    referrer: item,
                    missing: instance.mapping_target,
                })?;
        // Both frames carry file-unit coordinates; convert exactly once here.
        let target = operator_transform(self.model, instance.mapping_target, target_entity)?
            .to_metres(self.units);
        let origin_entity =
            self.model
                .get(instance.mapping_origin)
                .ok_or(GeometryError::MissingEntity {
                    referrer: item,
                    missing: instance.mapping_origin,
                })?;
        let origin = axis_placement_transform(self.model, instance.mapping_origin, origin_entity)?
            .to_metres(self.units);
        let inner = frame.compose(&target).compose(&origin);
        let inner_local = local.compose(&target).compose(&origin);

        self.mapped_by.push(item);
        let result = self.representation(item, instance.mapped_representation, inner, inner_local);
        self.mapped_by.pop();
        result
    }
}
