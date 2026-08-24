//! Summing a cost tree.
//!
//! Cost items nest through `IfcRelNests`, so a schedule total is the sum over
//! a tree rather than a flat list.

use crate::item::CostItem;
use crate::view::CostView;

/// Total the direct cost values of one item.
///
/// Does **not** recurse: nesting resolution needs the relationship graph, and
/// summing both an item and its children would double-count. Callers that want
/// a tree total should walk the nesting relations explicitly.
pub fn direct_total(view: &CostView<'_>, item: &CostItem<'_>) -> f64 {
    view.values_of(item).iter().filter_map(|v| v.amount()).sum()
}

/// Total the direct cost values of every item in the model.
///
/// Useful as a coarse file-level figure and as a regression check.
pub fn grand_total(view: &CostView<'_>) -> f64 {
    view.items().map(|item| direct_total(view, &item)).sum()
}
