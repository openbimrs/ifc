//! `ifc-cost` — cost semantics as a **view** over the model.
//!
//! # This crate owns no data
//!
//! It borrows a `&Model` and interprets entities that happen to be cost
//! entities. That is the whole design: the model stores structure, this crate
//! supplies meaning, and the two are separable.
//!
//! The consequences are the point:
//!
//! - **Optional.** An application that never touches cost compiles none of
//!   this. It is a cargo feature of the `ifc` facade crate.
//! - **Non-destructive.** Because the model would hold the same entities with
//!   or without this crate, a file with cost data round-trips identically in
//!   either build. Verified in `ifc-step/tests/roundtrip.rs`, which runs with
//!   no domain crate compiled at all.
//! - **Replaceable.** A different interpretation of the same entities is
//!   another crate, not a fork of the model.
//!
//! # Modules
//!
//! | Module | Role |
//! | --- | --- |
//! | [`item`] | `IfcCostItem`: the individual line |
//! | [`schedule`] | `IfcCostSchedule`: the containing document |
//! | [`value`] | `IfcCostValue` and applied monetary values |
//! | [`quantity`] | Quantities a cost is computed against |
//! | [`rollup`] | Summing a cost tree |
//! | [`error`] | Why a cost lookup failed |

pub mod error;
pub mod item;
pub mod quantity;
pub mod rollup;
pub mod schedule;
pub mod value;

pub use error::CostError;
pub use item::CostItem;
pub use schedule::CostSchedule;
pub use value::CostValue;

use ifc_model::Model;

/// Entry point: the cost view over a model.
///
/// Holds a borrow rather than owning anything, so constructing it is free and
/// several views over the same model can coexist.
#[derive(Debug, Clone, Copy)]
pub struct CostView<'m> {
    model: &'m Model,
}

impl<'m> CostView<'m> {
    /// Create a cost view over `model`.
    pub fn new(model: &'m Model) -> Self {
        Self { model }
    }

    /// Every cost schedule in the file.
    pub fn schedules(&self) -> impl Iterator<Item = CostSchedule<'m>> + '_ {
        self.model
            .of_type("IFCCOSTSCHEDULE")
            .map(|(id, entity)| CostSchedule::new(id, entity))
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Every cost item in the file.
    pub fn items(&self) -> impl Iterator<Item = CostItem<'m>> + '_ {
        self.model
            .of_type("IFCCOSTITEM")
            .map(|(id, entity)| CostItem::new(id, entity))
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// The model underneath, for callers that need to resolve references.
    pub fn model(&self) -> &'m Model {
        self.model
    }

    /// Resolve the cost values attached to an item.
    pub fn values_of(&self, item: &CostItem<'m>) -> Vec<CostValue<'m>> {
        item.value_refs()
            .into_iter()
            .filter_map(|id| self.model.get(id).map(|e| CostValue::new(id, e)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::{Entity, EntityId, Value};

    fn model_with_cost() -> Model {
        let mut model = Model::new();
        model.insert(
            EntityId(1),
            Entity::new(
                "IFCCOSTVALUE",
                vec![
                    Value::Text("Estimate".into()),
                    Value::Null,
                    Value::Typed {
                        type_name: "IFCMONETARYMEASURE".into(),
                        value: Box::new(Value::Real(1500.50)),
                    },
                ],
            ),
        );
        model.insert(
            EntityId(2),
            Entity::new(
                "IFCCOSTITEM",
                vec![
                    Value::Text("3vB2Y0dTv1LhX9ZzQqFbcd".into()),
                    Value::Null,
                    Value::Text("Excavation".into()),
                    Value::Null,
                    Value::Null,
                    Value::List(vec![Value::Ref(EntityId(1))]),
                ],
            ),
        );
        model
    }

    #[test]
    fn finds_cost_items_without_the_model_knowing_what_cost_is() {
        let model = model_with_cost();
        let view = CostView::new(&model);
        let items: Vec<_> = view.items().collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name(), Some("Excavation"));
    }

    #[test]
    fn resolves_values_through_references() {
        let model = model_with_cost();
        let view = CostView::new(&model);
        let item = view.items().next().unwrap();
        let values = view.values_of(&item);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].amount(), Some(1500.50));
    }

    /// The view is a lens, not storage: dropping it cannot lose data.
    #[test]
    fn view_owns_nothing_and_model_is_unchanged() {
        let model = model_with_cost();
        let before = model.len();
        {
            let view = CostView::new(&model);
            let _ = view.items().count();
        }
        assert_eq!(model.len(), before);
    }
}
