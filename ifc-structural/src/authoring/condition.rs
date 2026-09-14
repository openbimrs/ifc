//! Authoring for boundary conditions.
//!
//! A structural connection without a boundary condition is unsolved: the
//! condition is what states whether a support is pinned, fixed, sprung or
//! free. The crate could read all four families and author none of them.
//!
//! Each family declares its own attribute names -- node conditions use
//! `TranslationalStiffnessX`, edge conditions `...ByLengthX`, face
//! conditions `...ByAreaX` -- so the entity type chosen here decides how
//! the reader resolves every value. Writing node attributes under an edge
//! entity would leave every stiffness unreadable, which is why the kind
//! selects the entity and the slot order in one place.
//!
//! `IfcBoundaryCondition` contributes `Name` as slot 0, so the stiffness
//! values start at slot 1 in every family.

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::condition::{AxisValues, BoundaryConditionKind, StiffnessValue};
use crate::error::StructuralResult;

/// Authored fields for a boundary condition.
#[derive(Debug, Clone, Copy, Default)]
pub struct BoundaryConditionDraft<'a> {
    /// `IfcBoundaryCondition.Name`, if given.
    pub name: Option<&'a str>,
    /// Translational stiffness per axis. `None` leaves the axis unset,
    /// which the reader reports as absent rather than zero -- an unset
    /// support is not a free one.
    pub translational: AxisValues<Option<StiffnessValue>>,
    /// Rotational stiffness per axis. Ignored for face conditions, which
    /// declare no rotational attributes.
    pub rotational: AxisValues<Option<StiffnessValue>>,
    /// Warping stiffness. Only `NodeWarping` declares it.
    pub warping: Option<StiffnessValue>,
}

/// Stage a boundary condition of the given family.
///
/// The kind selects the entity, and with it the attribute names the reader
/// will look for. Face conditions declare no rotational or warping values
/// and warping is exclusive to `NodeWarping`, so supplying either where the
/// family does not declare it is refused rather than silently dropped: a
/// caller who sets a rotational spring on a face has a modelling error, not
/// a formatting one.
pub fn stage_boundary_condition(
    tx: &mut Transaction,
    kind: BoundaryConditionKind,
    draft: BoundaryConditionDraft<'_>,
) -> StructuralResult<EntityId> {
    let entity_type = entity_name(kind);
    if kind == BoundaryConditionKind::Face && has_any(draft.rotational) {
        return Err(crate::error::StructuralError::InvalidDraftValue {
            entity_type,
            attribute: "RotationalStiffness",
            expected: "no rotational stiffness: face conditions do not declare one",
        });
    }
    if draft.warping.is_some() && kind != BoundaryConditionKind::NodeWarping {
        return Err(crate::error::StructuralError::InvalidDraftValue {
            entity_type,
            attribute: "WarpingStiffness",
            expected: "no warping stiffness outside IfcBoundaryNodeConditionWarping",
        });
    }
    for (axis, value) in axes(draft.translational).chain(axes(draft.rotational)) {
        if let Some(StiffnessValue::Measure(measure)) = value {
            if !measure.is_finite() {
                return Err(crate::error::StructuralError::InvalidDraftValue {
                    entity_type,
                    attribute: axis,
                    expected: "a finite stiffness measure",
                });
            }
        }
    }
    let mut attributes = vec![optional_text(draft.name)];
    attributes.extend(stiffness_triple(draft.translational, kind, true));
    if kind != BoundaryConditionKind::Face {
        attributes.extend(stiffness_triple(draft.rotational, kind, false));
    }
    if kind == BoundaryConditionKind::NodeWarping {
        attributes.push(stiffness_value(draft.warping, kind, false));
    }
    Ok(tx.create(Entity::new(entity_type, attributes)))
}

fn entity_name(kind: BoundaryConditionKind) -> &'static str {
    match kind {
        BoundaryConditionKind::Edge => "IFCBOUNDARYEDGECONDITION",
        BoundaryConditionKind::Face => "IFCBOUNDARYFACECONDITION",
        BoundaryConditionKind::Node => "IFCBOUNDARYNODECONDITION",
        BoundaryConditionKind::NodeWarping => "IFCBOUNDARYNODECONDITIONWARPING",
    }
}

/// The measure each family uses. A node condition carries a stiffness, an
/// edge condition stiffness per length, a face condition per area: writing
/// the wrong wrapper states a different physical quantity.
fn measure_name(kind: BoundaryConditionKind, translational: bool) -> &'static str {
    match (kind, translational) {
        (BoundaryConditionKind::Edge, true) => "IFCMODULUSOFTRANSLATIONALSUBGRADEREACTIONMEASURE",
        (BoundaryConditionKind::Edge, false) => "IFCMODULUSOFROTATIONALSUBGRADEREACTIONMEASURE",
        (BoundaryConditionKind::Face, _) => "IFCMODULUSOFSUBGRADEREACTIONMEASURE",
        (_, true) => "IFCLINEARSTIFFNESSMEASURE",
        (_, false) => "IFCROTATIONALSTIFFNESSMEASURE",
    }
}

fn stiffness_value(
    value: Option<StiffnessValue>,
    kind: BoundaryConditionKind,
    translational: bool,
) -> Value {
    match value {
        None => Value::Null,
        Some(StiffnessValue::Boolean(flag)) => Value::Bool(flag),
        Some(StiffnessValue::Measure(measure)) => Value::Typed {
            type_name: measure_name(kind, translational).into(),
            value: Box::new(Value::Real(measure)),
        },
    }
}

fn stiffness_triple(
    values: AxisValues<Option<StiffnessValue>>,
    kind: BoundaryConditionKind,
    translational: bool,
) -> [Value; 3] {
    [
        stiffness_value(values.x, kind, translational),
        stiffness_value(values.y, kind, translational),
        stiffness_value(values.z, kind, translational),
    ]
}

fn has_any(values: AxisValues<Option<StiffnessValue>>) -> bool {
    values.x.is_some() || values.y.is_some() || values.z.is_some()
}

fn axes(
    values: AxisValues<Option<StiffnessValue>>,
) -> impl Iterator<Item = (&'static str, Option<StiffnessValue>)> {
    [("X", values.x), ("Y", values.y), ("Z", values.z)].into_iter()
}

fn optional_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |text| Value::Text(text.into()))
}
