//! The cost view: the entry point for reading cost data out of a model.
//!
//! Holds a borrow and nothing else. Constructing one is free, several can
//! coexist over the same model, and dropping one cannot lose data — which is
//! what makes `ifc-cost` safe to leave uncompiled.

use crate::item::CostItem;
use crate::schedule::CostSchedule;
use crate::value::CostValue;
use ifc_model::Model;

/// A borrowed cost interpretation of a model.
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
    ///
    /// A reference that does not resolve is skipped rather than erroring:
    /// dangling references are common in real files, and one bad link should
    /// not make the whole cost tree unreadable. Use
    /// [`Model::dangling_references`] when you need to know about them.
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

    /// A dangling cost value is skipped, not fatal.
    #[test]
    fn unresolvable_value_references_are_skipped() {
        let mut model = model_with_cost();
        model.insert(
            EntityId(3),
            Entity::new(
                "IFCCOSTITEM",
                vec![
                    Value::Text("broken".into()),
                    Value::Null,
                    Value::Text("Dangling".into()),
                    Value::Null,
                    Value::Null,
                    Value::List(vec![Value::Ref(EntityId(999))]),
                ],
            ),
        );

        let view = CostView::new(&model);
        let item = view
            .items()
            .find(|i| i.name() == Some("Dangling"))
            .expect("item present");
        assert!(view.values_of(&item).is_empty(), "missing target skipped");
    }
}
