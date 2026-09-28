//! The objectified relationships in the model's declared release, with a
//! caller-supplied `IfcOwnerHistory` (#202).
//!
//! Each writer runs the checks of its plain counterpart in
//! [`super::relationships`], then lays the record out by attribute name
//! from the bound release's table (`release.rs`). The names come from the
//! reader's slot constants (`RelSlots::relating_name`), so a writer and a
//! reader cannot disagree about which end is which.
//!
//! Release differences the table carries, from the EXPRESS sources:
//! IFC2X3 TC1 declares no `IfcRelDeclares`, `IfcRelDefinesByObject`,
//! `IfcRelAssignsToGroupByFactor` or `IfcRelInterferesElements`, and calls
//! `IfcRelCoversSpaces.RelatingSpace` `RelatedSpace`; only IFC4X3 ADD2
//! declares `IfcRelAdheresToElement`, `IfcRelPositions` and
//! `IfcRelAssociatesProfileDef`; IFC4X3 adds `InterferenceSpace` to
//! `IfcRelInterferesElements`, which IFC4 declares with nine attributes.

use ifc_model::{EntityId, Model, Transaction, Value};

use super::connections::{check_realizing, distinct};
use super::relationships::check_factor;
use super::release::stage;
use super::{check_relate, check_relate_one, SpatialAuthoringResult};
use crate::relation::slots::{
    RelSlots, ADHERES_TO_ELEMENT, ASSIGNS_TO_ACTOR, ASSIGNS_TO_GROUP_BY_FACTOR, ASSIGNS_TO_PROCESS,
    ASSIGNS_TO_PRODUCT, ASSIGNS_TO_RESOURCE, ASSOCIATES_PROFILE_DEF, CONNECTS_ELEMENTS,
    CONNECTS_WITH_REALIZING, COVERS_ELEMENTS, COVERS_SPACES, DECLARES, DEFINES_BY_OBJECT,
    FILLS_ELEMENT, FLOW_CONTROL_ELEMENTS, INTERFERES_ELEMENTS, POSITIONS, PROJECTS_ELEMENT,
    SERVICES_BUILDINGS, VOIDS_ELEMENT,
};

pub(super) fn refs(ids: &[EntityId]) -> Value {
    Value::List(ids.iter().copied().map(Value::Ref).collect())
}

/// A set-valued relationship: [`super::relate`] by name.
#[allow(clippy::too_many_arguments)]
pub(super) fn relate_owned(
    tx: &mut Transaction,
    model: &Model,
    rel: RelSlots,
    global_id: &str,
    parent: EntityId,
    children: &[EntityId],
    extra: Vec<(&'static str, Value)>,
    owner_history: EntityId,
) -> SpatialAuthoringResult<EntityId> {
    check_relate(rel, global_id, parent, children)?;
    let mut values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        (rel.relating_name(), Value::Ref(parent)),
        (rel.related_name(), refs(children)),
    ];
    values.extend(extra);
    stage(tx, model, rel.type_name, values, Some(owner_history))
}

/// A single-valued relationship: [`super::relate_one`] by name.
fn relate_one_owned(
    tx: &mut Transaction,
    model: &Model,
    rel: RelSlots,
    global_id: &str,
    relating: EntityId,
    related: EntityId,
    owner_history: EntityId,
) -> SpatialAuthoringResult<EntityId> {
    check_relate_one(rel, global_id, relating, related)?;
    let values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        (rel.relating_name(), Value::Ref(relating)),
        (rel.related_name(), Value::Ref(related)),
    ];
    stage(tx, model, rel.type_name, values, Some(owner_history))
}

/// Two distinct elements: the plain `pair` by name.
#[allow(clippy::too_many_arguments)]
fn pair_owned(
    tx: &mut Transaction,
    model: &Model,
    rel: RelSlots,
    global_id: &str,
    relating: EntityId,
    related: EntityId,
    extra: Vec<(&'static str, Value)>,
    owner_history: EntityId,
) -> SpatialAuthoringResult<EntityId> {
    distinct(rel, global_id, relating, related)?;
    let mut values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        (rel.relating_name(), Value::Ref(relating)),
        (rel.related_name(), Value::Ref(related)),
    ];
    values.extend(extra);
    stage(tx, model, rel.type_name, values, Some(owner_history))
}

macro_rules! set_valued {
    ($(#[$doc:meta])* $name:ident, $plain:ident, $rel:ident, $parent:ident, $children:ident) => {
        $(#[$doc])*
        ///
        /// # Errors
        ///
        #[doc = concat!("Those of [`", stringify!($plain), "`](super::", stringify!($plain), "),")]
        /// and the release and owner-history refusals of
        /// [`aggregate_with_owner_history`](super::aggregate_with_owner_history).
        /// Nothing is staged on an error.
        pub fn $name(
            tx: &mut Transaction,
            model: &Model,
            global_id: &str,
            $parent: EntityId,
            $children: &[EntityId],
            owner_history: EntityId,
        ) -> SpatialAuthoringResult<EntityId> {
            relate_owned(
                tx,
                model,
                $rel,
                global_id,
                $parent,
                $children,
                Vec::new(),
                owner_history,
            )
        }
    };
}

macro_rules! single_valued {
    ($(#[$doc:meta])* $name:ident, $plain:ident, $rel:ident, $relating:ident, $related:ident) => {
        $(#[$doc])*
        ///
        /// # Errors
        ///
        #[doc = concat!("Those of [`", stringify!($plain), "`](super::", stringify!($plain), "),")]
        /// and the release and owner-history refusals of
        /// [`aggregate_with_owner_history`](super::aggregate_with_owner_history).
        /// Nothing is staged on an error.
        pub fn $name(
            tx: &mut Transaction,
            model: &Model,
            global_id: &str,
            $relating: EntityId,
            $related: EntityId,
            owner_history: EntityId,
        ) -> SpatialAuthoringResult<EntityId> {
            relate_one_owned(tx, model, $rel, global_id, $relating, $related, owner_history)
        }
    };
}

set_valued!(
    /// [`cover_elements`](super::cover_elements) in the model's declared
    /// release, with a caller-supplied `IfcOwnerHistory`, which IFC2X3
    /// requires.
    cover_elements_with_owner_history, cover_elements, COVERS_ELEMENTS, element, coverings
);
set_valued!(
    /// [`cover_spaces`](super::cover_spaces) in the model's declared release,
    /// with a caller-supplied `IfcOwnerHistory`, which IFC2X3 requires. In
    /// IFC2X3 the space is written as `RelatedSpace`, IFC4's
    /// `RelatingSpace` under its IFC2X3 name.
    cover_spaces_with_owner_history, cover_spaces, COVERS_SPACES, space, coverings
);
set_valued!(
    /// [`declare`](super::declare) in the model's declared release, with a
    /// caller-supplied `IfcOwnerHistory`. IFC2X3 declares no
    /// `IfcRelDeclares` (`EntityNotInSchema`).
    declare_with_owner_history, declare, DECLARES, context, definitions
);
set_valued!(
    /// [`define_by_object`](super::define_by_object) in the model's declared
    /// release, with a caller-supplied `IfcOwnerHistory`. IFC2X3 declares no
    /// `IfcRelDefinesByObject` (`EntityNotInSchema`).
    define_by_object_with_owner_history, define_by_object, DEFINES_BY_OBJECT, defining, defined
);
set_valued!(
    /// [`serve_buildings`](super::serve_buildings) in the model's declared
    /// release, with a caller-supplied `IfcOwnerHistory`, which IFC2X3
    /// requires.
    serve_buildings_with_owner_history, serve_buildings, SERVICES_BUILDINGS, system, buildings
);
set_valued!(
    /// [`control_flow_element`](super::control_flow_element) in the model's
    /// declared release, with a caller-supplied `IfcOwnerHistory`, which
    /// IFC2X3 requires.
    control_flow_element_with_owner_history,
    control_flow_element,
    FLOW_CONTROL_ELEMENTS,
    flow_element,
    controls
);
set_valued!(
    /// [`assign_to_actor`](super::assign_to_actor) in the model's declared
    /// release, with a caller-supplied `IfcOwnerHistory`, which IFC2X3
    /// requires. The record has the release's eight attributes, `ActingRole`
    /// unset; the plain writer stops at seven.
    assign_to_actor_with_owner_history, assign_to_actor, ASSIGNS_TO_ACTOR, actor, objects
);
set_valued!(
    /// [`assign_to_product`](super::assign_to_product) in the model's
    /// declared release, with a caller-supplied `IfcOwnerHistory`, which
    /// IFC2X3 requires.
    assign_to_product_with_owner_history, assign_to_product, ASSIGNS_TO_PRODUCT, product, objects
);
set_valued!(
    /// [`assign_to_process`](super::assign_to_process) in the model's
    /// declared release, with a caller-supplied `IfcOwnerHistory`, which
    /// IFC2X3 requires. The record has the release's eight attributes,
    /// `QuantityInProcess` unset; the plain writer stops at seven.
    assign_to_process_with_owner_history, assign_to_process, ASSIGNS_TO_PROCESS, process, objects
);
set_valued!(
    /// [`adhere_to_element`](super::adhere_to_element) in the model's
    /// declared release, with a caller-supplied `IfcOwnerHistory`. Only
    /// IFC4X3 declares `IfcRelAdheresToElement` (`EntityNotInSchema`
    /// elsewhere).
    adhere_to_element_with_owner_history, adhere_to_element, ADHERES_TO_ELEMENT, element, features
);
set_valued!(
    /// [`position_products`](super::position_products) in the model's
    /// declared release, with a caller-supplied `IfcOwnerHistory`. Only
    /// IFC4X3 declares `IfcRelPositions` (`EntityNotInSchema` elsewhere).
    position_products_with_owner_history, position_products, POSITIONS, positioning, products
);
set_valued!(
    /// [`assign_to_resource`](super::assign_to_resource) in the model's
    /// declared release, with a caller-supplied `IfcOwnerHistory`, which
    /// IFC2X3 requires.
    assign_to_resource_with_owner_history, assign_to_resource, ASSIGNS_TO_RESOURCE, resource, objects
);
set_valued!(
    /// [`associate_profile_def`](super::associate_profile_def) in the model's
    /// declared release, with a caller-supplied `IfcOwnerHistory`. Only
    /// IFC4X3 declares `IfcRelAssociatesProfileDef` (`EntityNotInSchema`
    /// elsewhere).
    associate_profile_def_with_owner_history,
    associate_profile_def,
    ASSOCIATES_PROFILE_DEF,
    profile,
    objects
);
single_valued!(
    /// [`void_element`](super::void_element) in the model's declared
    /// release, with a caller-supplied `IfcOwnerHistory`, which IFC2X3
    /// requires.
    void_element_with_owner_history, void_element, VOIDS_ELEMENT, element, opening
);
single_valued!(
    /// [`fill_element`](super::fill_element) in the model's declared
    /// release, with a caller-supplied `IfcOwnerHistory`, which IFC2X3
    /// requires.
    fill_element_with_owner_history, fill_element, FILLS_ELEMENT, opening, filling
);
single_valued!(
    /// [`project_element`](super::project_element) in the model's declared
    /// release, with a caller-supplied `IfcOwnerHistory`, which IFC2X3
    /// requires.
    project_element_with_owner_history, project_element, PROJECTS_ELEMENT, element, feature
);

/// [`assign_to_group_by_factor`](super::assign_to_group_by_factor) in the
/// model's declared release, with a caller-supplied `IfcOwnerHistory`.
/// IFC2X3 declares no `IfcRelAssignsToGroupByFactor` (`EntityNotInSchema`).
///
/// # Errors
///
/// Those of [`assign_to_group_by_factor`](super::assign_to_group_by_factor),
/// and the release and owner-history refusals of
/// [`aggregate_with_owner_history`](super::aggregate_with_owner_history).
/// Nothing is staged on an error.
pub fn assign_to_group_by_factor_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    group: EntityId,
    members: &[EntityId],
    factor: f64,
    owner_history: EntityId,
) -> SpatialAuthoringResult<EntityId> {
    check_factor(factor)?;
    relate_owned(
        tx,
        model,
        ASSIGNS_TO_GROUP_BY_FACTOR,
        global_id,
        group,
        members,
        vec![("Factor", Value::Real(factor))],
        owner_history,
    )
}

/// [`connect_elements`](super::connect_elements) in the model's declared
/// release, with a caller-supplied `IfcOwnerHistory`, which IFC2X3
/// requires.
///
/// # Errors
///
/// Those of [`connect_elements`](super::connect_elements), and the release
/// and owner-history refusals of
/// [`aggregate_with_owner_history`](super::aggregate_with_owner_history).
/// Nothing is staged on an error.
pub fn connect_elements_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    relating: EntityId,
    related: EntityId,
    owner_history: EntityId,
) -> SpatialAuthoringResult<EntityId> {
    pair_owned(
        tx,
        model,
        CONNECTS_ELEMENTS,
        global_id,
        relating,
        related,
        Vec::new(),
        owner_history,
    )
}

/// [`connect_with_realizing_elements`](super::connect_with_realizing_elements)
/// in the model's declared release, with a caller-supplied
/// `IfcOwnerHistory`, which IFC2X3 requires. The record has the release's
/// nine attributes, `ConnectionType` unset; the plain writer stops at
/// eight.
///
/// # Errors
///
/// Those of
/// [`connect_with_realizing_elements`](super::connect_with_realizing_elements),
/// and the release and owner-history refusals of
/// [`aggregate_with_owner_history`](super::aggregate_with_owner_history).
/// Nothing is staged on an error.
pub fn connect_with_realizing_elements_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    relating: EntityId,
    related: EntityId,
    realizing: &[EntityId],
    owner_history: EntityId,
) -> SpatialAuthoringResult<EntityId> {
    check_realizing(realizing)?;
    pair_owned(
        tx,
        model,
        CONNECTS_WITH_REALIZING,
        global_id,
        relating,
        related,
        vec![("RealizingElements", refs(realizing))],
        owner_history,
    )
}

/// [`interfere_elements`](super::interfere_elements) in the model's
/// declared release, with a caller-supplied `IfcOwnerHistory`. IFC2X3
/// declares no `IfcRelInterferesElements` (`EntityNotInSchema`).
///
/// The record has the release's own arity: nine attributes in IFC4, ten
/// in IFC4X3 (`InterferenceSpace`, unset). The plain writer writes ten in
/// both.
///
/// # Errors
///
/// Those of [`interfere_elements`](super::interfere_elements), and the
/// release and owner-history refusals of
/// [`aggregate_with_owner_history`](super::aggregate_with_owner_history).
/// Nothing is staged on an error.
pub fn interfere_elements_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    relating: EntityId,
    related: EntityId,
    implied_order: Option<bool>,
    owner_history: EntityId,
) -> SpatialAuthoringResult<EntityId> {
    pair_owned(
        tx,
        model,
        INTERFERES_ELEMENTS,
        global_id,
        relating,
        related,
        vec![(
            "ImpliedOrder",
            implied_order.map_or(Value::LogicalUnknown, Value::Bool),
        )],
        owner_history,
    )
}
