//! Representation contexts and the sub-contexts drawings are authored into.
//!
//! # Why this matters beyond geometry
//!
//! Every `IfcRepresentation` names a context. A 3D viewer can ignore it -- all
//! the geometry it wants sits in the one `Model` context. Drawing production
//! cannot: a floor plan is *defined* as the geometry authored into a
//! sub-context whose `TargetView` is `PLAN_VIEW`. Without reading contexts
//! there is no way to tell a plan curve from a centreline.
//!
//! # The `*` trap
//!
//! `IfcGeometricRepresentationSubContext` redeclares six inherited attributes
//! as DERIVED. Real files write them as `*`:
//!
//! ```text
//! IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#1,$,.MODEL_VIEW.,$)
//! ```
//!
//! `*` is not `$`. It means "this value lives on my parent", so a sub-context's
//! precision, coordinate dimension and world coordinate system must be resolved
//! by walking to `ParentContext`. Reading the slot directly yields the marker,
//! and a consumer that treats it as absent silently loses the project's
//! precision and placement.

use ifc_model::{Entity, EntityId, Model, Value};

use crate::slots::Slots;

use super::representation::representation_slot;

/// Absolute slots on `IfcGeometricRepresentationContext`.
///
/// Identical in IFC2x3, IFC4 and IFC4x3; asserted in `tests/context_slots.rs`.
pub mod context_slot {
    /// OPTIONAL identifier, e.g. `Plan`, `Model`.
    pub const CONTEXT_IDENTIFIER: usize = 0;
    /// OPTIONAL type, e.g. `Model`, `Plan`, `NotDefined`.
    pub const CONTEXT_TYPE: usize = 1;
    /// 2 or 3; DERIVED on a sub-context.
    pub const COORDINATE_SPACE_DIMENSION: usize = 2;
    /// OPTIONAL model precision; DERIVED on a sub-context.
    pub const PRECISION: usize = 3;
    /// Placement of the context origin; DERIVED on a sub-context.
    pub const WORLD_COORDINATE_SYSTEM: usize = 4;
    /// OPTIONAL true-north direction; DERIVED on a sub-context.
    pub const TRUE_NORTH: usize = 5;
}

/// Slots added by `IfcGeometricRepresentationSubContext`.
///
/// The subtype inherits all six slots above, so its own attributes start at 6.
/// Getting this wrong reads `TargetScale` as the target view.
pub mod sub_context_slot {
    /// The parent `IfcGeometricRepresentationContext`. Required.
    pub const PARENT_CONTEXT: usize = 6;
    /// OPTIONAL scale, e.g. 0.01 for 1:100.
    pub const TARGET_SCALE: usize = 7;
    /// OPTIONAL `.PLAN_VIEW.`, `.MODEL_VIEW.`, `.ELEVATION_VIEW.`, ...
    pub const TARGET_VIEW: usize = 8;
    /// OPTIONAL free-text view when `TargetView` is `.USERDEFINED.`
    pub const USER_DEFINED_TARGET_VIEW: usize = 9;
}

/// The intended presentation of a sub-context.
///
/// Drawing production selects on this: a floor plan is the geometry authored
/// into a `PlanView` sub-context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetView {
    /// `.PLAN_VIEW.` — the horizontal section a floor plan is drawn from.
    PlanView,
    /// `.MODEL_VIEW.` — the 3D shape a viewer draws.
    ModelView,
    /// `.ELEVATION_VIEW.`
    ElevationView,
    /// `.SECTION_VIEW.`
    SectionView,
    /// `.GRAPH_VIEW.` — schematic lines such as an analytical model.
    GraphView,
    /// `.SKETCH_VIEW.`
    SketchView,
    /// `.REFLECTED_PLAN_VIEW.` — a ceiling plan.
    ReflectedPlanView,
    /// `.USERDEFINED.`, carrying `UserDefinedTargetView` when the author set it.
    UserDefined(Option<String>),
    /// `.NOTDEFINED.`
    NotDefined,
    /// An enumeration value this crate does not know.
    ///
    /// Preserved rather than collapsed into `NotDefined`: a later schema may
    /// add views, and reporting the literal keeps the file readable.
    Other(String),
}

impl TargetView {
    /// Parse a STEP enumeration constant.
    fn parse(literal: &str, user_defined: Option<String>) -> Self {
        match literal.trim_matches('.').to_ascii_uppercase().as_str() {
            "PLAN_VIEW" => Self::PlanView,
            "MODEL_VIEW" => Self::ModelView,
            "ELEVATION_VIEW" => Self::ElevationView,
            "SECTION_VIEW" => Self::SectionView,
            "GRAPH_VIEW" => Self::GraphView,
            "SKETCH_VIEW" => Self::SketchView,
            "REFLECTED_PLAN_VIEW" => Self::ReflectedPlanView,
            "USERDEFINED" => Self::UserDefined(user_defined),
            "NOTDEFINED" => Self::NotDefined,
            other => Self::Other(other.to_string()),
        }
    }

    /// Whether this view is drawn as a 2D plan.
    ///
    /// Reflected plans included: a ceiling plan is still a plan, and a caller
    /// selecting plan geometry wants it.
    #[must_use]
    pub fn is_plan(&self) -> bool {
        matches!(self, Self::PlanView | Self::ReflectedPlanView)
    }
}

/// One `IfcGeometricRepresentationContext` or its sub-context.
#[derive(Debug, Clone, Copy)]
pub struct RepresentationContext<'m> {
    slots: Slots<'m>,
}

impl<'m> RepresentationContext<'m> {
    /// Wrap an entity assumed to be a representation context.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// This context's entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// Whether this is an `IfcGeometricRepresentationSubContext`.
    pub fn is_sub_context(&self) -> bool {
        self.slots
            .type_name()
            .eq_ignore_ascii_case("IFCGEOMETRICREPRESENTATIONSUBCONTEXT")
    }

    /// `ContextIdentifier`, e.g. `Body`, `Axis`, `Plan`.
    pub fn identifier(&self) -> Option<String> {
        self.slots.opt_text(context_slot::CONTEXT_IDENTIFIER)
    }

    /// `ContextType`, e.g. `Model`, `Plan`, `Design`.
    pub fn context_type(&self) -> Option<String> {
        self.slots.opt_text(context_slot::CONTEXT_TYPE)
    }

    /// The parent context of a sub-context, if it declares one.
    pub fn parent(&self) -> Option<EntityId> {
        match self.slots.opt(sub_context_slot::PARENT_CONTEXT)? {
            Value::Ref(id) => Some(*id),
            _ => None,
        }
    }

    /// `TargetScale`, e.g. `0.01` for 1:100. Sub-contexts only.
    pub fn target_scale(&self) -> Option<f64> {
        match self
            .slots
            .opt(sub_context_slot::TARGET_SCALE)?
            .unwrap_typed()
        {
            Value::Real(value) => Some(*value),
            Value::Integer(value) => Some(*value as f64),
            _ => None,
        }
    }

    /// `TargetView`. Sub-contexts only; `None` on a root context.
    pub fn target_view(&self) -> Option<TargetView> {
        let literal = match self
            .slots
            .opt(sub_context_slot::TARGET_VIEW)?
            .unwrap_typed()
        {
            Value::Enum(text) => text.to_string(),
            _ => return None,
        };
        let user_defined = self
            .slots
            .opt_text(sub_context_slot::USER_DEFINED_TARGET_VIEW);
        Some(TargetView::parse(&literal, user_defined))
    }

    /// Whether this context is authored for plan drawing.
    pub fn is_plan_view(&self) -> bool {
        self.target_view().is_some_and(|view| view.is_plan())
    }
}

/// Maximum parent links followed when resolving a DERIVED attribute.
///
/// The schema permits one level of sub-context, so 8 is far past any
/// well-formed file. It is a *termination* bound, not a validation rule: a
/// malformed file that chains or cycles contexts stops here and reports the
/// value as unresolved rather than looping.
///
/// This is deliberately the only termination mechanism. An earlier draft also
/// carried a visited-set; with the bound in place it could never change an
/// outcome, and a second guard that cannot fail is a claim the tests cannot
/// check. One bound, tested at its edge.
const MAX_PARENT_DEPTH: usize = 8;

/// Resolve an inherited slot, following `ParentContext` past every `*`.
///
/// A sub-context redeclares six attributes as DERIVED and writes them as `*`,
/// meaning "read this from my parent". Returns `None` when the chain ends
/// without a concrete value, or when it is still unresolved after
/// [`MAX_PARENT_DEPTH`] links — which is what a cycle looks like from here.
fn resolve_inherited(model: &Model, start: EntityId, slot: usize) -> Option<&Value> {
    let mut current = start;

    for _ in 0..MAX_PARENT_DEPTH {
        let entity = model.get(current)?;
        let view = RepresentationContext::new(current, entity);
        match entity.attribute(slot) {
            // A concrete value stops the walk.
            Some(value) if !matches!(value, Value::Derived | Value::Null) => {
                return Some(value);
            }
            // `*` means the value lives further up.
            Some(Value::Derived) => {
                current = view.parent()?;
            }
            // `$` or an absent slot is genuinely unset.
            _ => return None,
        }
    }
    None
}

impl RepresentationContext<'_> {
    /// `Precision`, resolved through `ParentContext` when written as `*`.
    ///
    /// The value a tolerance-sensitive consumer needs; reading the slot
    /// directly on a sub-context yields the DERIVED marker instead.
    pub fn precision(&self, model: &Model) -> Option<f64> {
        match resolve_inherited(model, self.id(), context_slot::PRECISION)?.unwrap_typed() {
            Value::Real(value) => Some(*value),
            Value::Integer(value) => Some(*value as f64),
            _ => None,
        }
    }

    /// `CoordinateSpaceDimension`, resolved through `ParentContext`.
    pub fn coordinate_space_dimension(&self, model: &Model) -> Option<i64> {
        match resolve_inherited(model, self.id(), context_slot::COORDINATE_SPACE_DIMENSION)?
            .unwrap_typed()
        {
            Value::Integer(value) => Some(*value),
            _ => None,
        }
    }

    /// `WorldCoordinateSystem`, resolved through `ParentContext`.
    ///
    /// Without this a sub-context appears to have no placement and geometry
    /// authored into it lands at the wrong origin.
    pub fn world_coordinate_system(&self, model: &Model) -> Option<EntityId> {
        match resolve_inherited(model, self.id(), context_slot::WORLD_COORDINATE_SYSTEM)? {
            Value::Ref(id) => Some(*id),
            _ => None,
        }
    }

    /// `TrueNorth`, resolved through `ParentContext`.
    pub fn true_north(&self, model: &Model) -> Option<EntityId> {
        match resolve_inherited(model, self.id(), context_slot::TRUE_NORTH)? {
            Value::Ref(id) => Some(*id),
            _ => None,
        }
    }
}

/// Every representation context in the model, in file order.
///
/// Includes both root contexts and sub-contexts; use
/// [`RepresentationContext::is_sub_context`] to tell them apart.
pub fn all_contexts(model: &Model) -> Vec<RepresentationContext<'_>> {
    let mut out = Vec::new();
    for type_name in [
        "IFCGEOMETRICREPRESENTATIONCONTEXT",
        "IFCGEOMETRICREPRESENTATIONSUBCONTEXT",
    ] {
        for &id in model.ids_of_type(type_name) {
            if let Some(entity) = model.get(id) {
                out.push(RepresentationContext::new(id, entity));
            }
        }
    }
    out.sort_by_key(RepresentationContext::id);
    out
}

/// Sub-contexts whose `TargetView` is a plan view.
///
/// The entry point for drawing production: geometry authored into one of these
/// is what a floor plan is made of.
pub fn plan_contexts(model: &Model) -> Vec<RepresentationContext<'_>> {
    all_contexts(model)
        .into_iter()
        .filter(RepresentationContext::is_plan_view)
        .collect()
}

/// The context a representation is authored into.
pub fn context_of(model: &Model, representation: EntityId) -> Option<RepresentationContext<'_>> {
    let entity = model.get(representation)?;
    let context_id =
        match Slots::new(representation, entity).opt(representation_slot::CONTEXT_OF_ITEMS)? {
            Value::Ref(id) => *id,
            _ => return None,
        };
    let context_entity = model.get(context_id)?;
    Some(RepresentationContext::new(context_id, context_entity))
}
