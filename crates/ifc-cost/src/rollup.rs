//! Totalling a cost tree.
//!
//! # Direct versus rolled-up totals
//!
//! A cost item may state its own values AND nest children that state theirs.
//! Adding both double-counts if the parent's value is a summary of the
//! children, and under-counts if it is not -- and IFC does not say which.
//!
//! So both readings are offered explicitly and neither is called "the total":
//!
//! - [`direct_total`]: this item's own values only.
//! - [`rolled_up_total`]: the sum over descendants, ignoring the parent's own
//!   stated value.
//!
//! A caller who knows their authoring convention picks one. A caller who does
//! not can compare them: when a parent's direct total equals its rolled-up
//! total, the file is self-consistent and either reading works. When they
//! differ, that difference is information, and [`consistency`] reports it
//! rather than hiding it behind a chosen default.

use ifc_model::{EntityId, Model};

use crate::item::CostItem;
use crate::relation::{descendants_of, CostRelationError};
use crate::view::CostView;

/// Total the direct cost values of one item.
///
/// Does not recurse. Composed values (those with `Components` and no stated
/// `AppliedValue`) contribute nothing here, because this crate does not fold
/// an arithmetic tree whose bracketing the schema leaves undefined.
#[must_use]
pub fn direct_total(view: &CostView<'_>, item: &CostItem<'_>) -> f64 {
    view.values_of(item).iter().filter_map(|v| v.amount()).sum()
}

/// Total the direct values of every descendant, excluding the item itself.
///
/// # Errors
///
/// Propagates [`CostRelationError`] when the nesting graph cycles or exceeds
/// its depth budget.
pub fn rolled_up_total(view: &CostView<'_>, item: &CostItem<'_>) -> Result<f64, CostRelationError> {
    let model = view.model();
    let mut total = 0.0;
    for id in descendants_of(model, item.id())? {
        if let Some(entity) = model.get(id) {
            total += direct_total(view, &CostItem::new(id, entity));
        }
    }
    Ok(total)
}

/// How an item's own stated total compares with the sum of its children.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Consistency {
    /// The item's own stated values.
    pub direct: f64,
    /// The sum over its descendants.
    pub rolled_up: f64,
    /// Whether the item states any values of its own.
    pub states_own_value: bool,
    /// Whether it has any descendants.
    pub has_children: bool,
}

impl Consistency {
    /// Whether both readings agree within `tolerance`.
    ///
    /// An item with no children, or none of its own values, is trivially
    /// consistent: there is only one reading to take.
    #[must_use]
    pub fn agrees(&self, tolerance: f64) -> bool {
        if !self.has_children || !self.states_own_value {
            return true;
        }
        (self.direct - self.rolled_up).abs() <= tolerance
    }
}

/// Compare an item's direct total against its rolled-up total.
///
/// # Errors
///
/// Propagates [`CostRelationError`] from the nesting walk.
pub fn consistency(
    view: &CostView<'_>,
    item: &CostItem<'_>,
) -> Result<Consistency, CostRelationError> {
    let direct = direct_total(view, item);
    let rolled_up = rolled_up_total(view, item)?;
    Ok(Consistency {
        direct,
        rolled_up,
        states_own_value: !view.values_of(item).is_empty(),
        has_children: !crate::relation::children_of(view.model(), item.id()).is_empty(),
    })
}

/// Total the direct cost values of every item in the model.
///
/// A flat sum over every `IfcCostItem`, which double-counts any file whose
/// parents summarise their children. Useful as a coarse figure and as a
/// regression check, not as an estimate.
#[must_use]
pub fn grand_total(view: &CostView<'_>) -> f64 {
    view.items().map(|item| direct_total(view, &item)).sum()
}

/// Cost items that are not nested under any other item.
///
/// These are the entry points of the breakdown: totalling their rolled-up
/// values counts each leaf exactly once.
#[must_use]
pub fn roots(view: &CostView<'_>) -> Vec<EntityId> {
    let model: &Model = view.model();
    view.items()
        .map(|item| item.id())
        .filter(|id| crate::relation::parent_of(model, *id).is_none())
        .collect()
}
