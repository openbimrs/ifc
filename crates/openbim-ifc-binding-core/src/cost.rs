//! Cost schedules, items and values (feature `cost`, #123).
//!
//! The facade's cost view reads schedules against the declared release and
//! items, values and their nesting as the file states them. One snapshot
//! carries every schedule with the objects assigned to it, every item with
//! its parent, children, values and quantities, and each value's
//! `AppliedValue` in the tagged encoding (`typed IFCMONETARYMEASURE(real
//! 1500.5)`), its unit basis and its component tree. Nothing is summed: a
//! cost value tree's operators are reported, not evaluated, as the facade
//! leaves evaluation to the caller. A release other than IFC2X3, IFC4 or
//! IFC4X3 is refused with `unsupported-schema`.

use crate::record::{Field, Record, ToRecord};
use crate::value::Tagged;
use crate::{BindingError, IfcModel};

/// Every cost schedule and cost item in a model.
#[derive(Debug, Clone, PartialEq)]
pub struct Cost {
    /// `IfcCostSchedule`s, in file order.
    pub schedules: Vec<CostSchedule>,
    /// `IfcCostItem`s, in file order.
    pub items: Vec<CostItem>,
    /// Second parents the file states for nested items; the first nesting
    /// in file order is the one `parent` and `children` report.
    pub anomalies: Vec<CostAnomaly>,
}

/// An `IfcCostSchedule`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostSchedule {
    /// Its entity id.
    pub id: u64,
    /// `GlobalId`.
    pub global_id: Option<String>,
    /// `Name`.
    pub name: Option<String>,
    /// `Identification` (IFC2X3 `ID`).
    pub identification: Option<String>,
    /// `Status`.
    pub status: Option<String>,
    /// `PredefinedType`, e.g. `BUDGET`.
    pub predefined_type: Option<String>,
    /// Objects assigned to it (`IfcRelAssignsToControl`), usually its cost
    /// items, in file order.
    pub items: Vec<u64>,
}

/// An `IfcCostItem`.
#[derive(Debug, Clone, PartialEq)]
pub struct CostItem {
    /// Its entity id.
    pub id: u64,
    /// `GlobalId`.
    pub global_id: Option<String>,
    /// `Name`.
    pub name: Option<String>,
    /// `Identification`.
    pub identification: Option<String>,
    /// `Description`.
    pub description: Option<String>,
    /// `PredefinedType`.
    pub predefined_type: Option<String>,
    /// The item it is nested in (`IfcRelNests`).
    pub parent: Option<u64>,
    /// Items nested in it, in authored order.
    pub children: Vec<u64>,
    /// `CostValues`, resolved.
    pub values: Vec<CostValue>,
    /// `CostQuantities`: the `IfcPhysicalQuantity` ids it is computed
    /// against.
    pub quantities: Vec<u64>,
    /// Objects it prices (`IfcRelAssignsToControl`), in file order.
    pub objects: Vec<u64>,
}

/// An `IfcCostValue` or `IfcAppliedValue`.
#[derive(Debug, Clone, PartialEq)]
pub struct CostValue {
    /// Its entity id.
    pub id: u64,
    /// `Name`.
    pub name: Option<String>,
    /// `Description`.
    pub description: Option<String>,
    /// `Category`, e.g. `Labour`.
    pub category: Option<String>,
    /// `Condition`.
    pub condition: Option<String>,
    /// `AppliedValue` as authored, typed; `null` for a value that states
    /// only components.
    pub applied_value: Tagged,
    /// `ArithmeticOperator`: `ADD`, `DIVIDE`, `MULTIPLY` or `SUBTRACT`.
    pub operator: Option<String>,
    /// `UnitBasis`: the value is a rate per this measure.
    pub unit_basis: Option<UnitBasis>,
    /// `Components`, resolved, in file order.
    pub components: Vec<CostValue>,
}

/// The `IfcMeasureWithUnit` a rate is stated per.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitBasis {
    /// Its entity id.
    pub id: u64,
    /// `ValueComponent` as authored, typed.
    pub value: Tagged,
    /// `UnitComponent`.
    pub unit: Option<u64>,
}

/// A cost item nested under two parents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostAnomaly {
    /// The item.
    pub item: u64,
    /// The parent kept.
    pub kept: u64,
    /// The parent rejected.
    pub rejected: u64,
    /// The rejected `IfcRelNests`.
    pub relation: u64,
}

impl IfcModel {
    /// Every cost schedule and cost item.
    ///
    /// Refused with `unsupported-schema` for a release other than IFC2X3,
    /// IFC4 or IFC4X3, `missing-reference` for a cost value or component
    /// the file does not contain, `budget-exceeded` for a component tree
    /// that cycles or nests deeper than 64, and `feature-disabled` without
    /// the `cost` feature.
    pub fn cost(&self) -> Result<Cost, BindingError> {
        #[cfg(feature = "cost")]
        {
            read::cost(self)
        }
        #[cfg(not(feature = "cost"))]
        {
            Err(BindingError::FeatureDisabled("cost"))
        }
    }
}

#[cfg(feature = "cost")]
mod read {
    use ifc::cost::{
        children_of, controlled_by, nesting_anomalies, parent_of, CostAnomaly as Anomaly,
        CostError, CostValue as View, CostView, MAX_NESTING_DEPTH,
    };
    use ifc::EntityId;

    use super::{Cost, CostAnomaly, CostItem, CostSchedule, CostValue, UnitBasis};
    use crate::value::Tagged;
    use crate::{BindingError, IfcModel};

    pub(super) fn cost(model: &IfcModel) -> Result<Cost, BindingError> {
        let inner = &model.inner;
        let view = CostView::new(inner);
        let owned = |text: Option<&str>| text.map(str::to_owned);
        let mut schedules = Vec::new();
        for schedule in view.schedules().map_err(error)? {
            let id = schedule.id();
            schedules.push(CostSchedule {
                id: id.0,
                global_id: model.identity(id.0)?.global_id,
                name: owned(schedule.name()),
                identification: owned(schedule.identification()),
                status: owned(schedule.status()),
                predefined_type: owned(schedule.predefined_type()),
                items: ids(controlled_by(inner, id)),
            });
        }
        let mut items = Vec::new();
        for item in view.items() {
            let id = item.id();
            let values = item
                .value_refs()
                .into_iter()
                .map(|value| applied(model, id, value, 0, &mut Vec::new()))
                .collect::<Result<_, _>>()?;
            items.push(CostItem {
                id: id.0,
                global_id: model.identity(id.0)?.global_id,
                name: owned(item.name()),
                identification: owned(item.identification()),
                description: owned(item.description()),
                predefined_type: owned(item.predefined_type()),
                parent: parent_of(inner, id).map(|EntityId(id)| id),
                children: ids(children_of(inner, id)),
                values,
                quantities: ids(item.quantity_refs()),
                objects: ids(controlled_by(inner, id)),
            });
        }
        let anomalies = nesting_anomalies(inner)
            .into_iter()
            .filter_map(|anomaly| match anomaly {
                Anomaly::NestedTwice {
                    item,
                    kept,
                    rejected,
                    relation,
                } => Some(CostAnomaly {
                    item: item.0,
                    kept: kept.0,
                    rejected: rejected.0,
                    relation: relation.0,
                }),
                // An anomaly kind added after this binding.
                _ => None,
            })
            .collect();
        Ok(Cost {
            schedules,
            items,
            anomalies,
        })
    }

    /// Value `id`, referenced from `from`, with its component tree.
    fn applied(
        model: &IfcModel,
        from: EntityId,
        id: EntityId,
        depth: usize,
        path: &mut Vec<EntityId>,
    ) -> Result<CostValue, BindingError> {
        if path.contains(&id) || depth > MAX_NESTING_DEPTH {
            return Err(BindingError::BudgetExceeded(format!(
                "the cost value tree under #{} cycles or nests deeper than {MAX_NESTING_DEPTH}",
                from.0
            )));
        }
        let entity = model.inner.get(id).ok_or_else(|| {
            BindingError::MissingReference(format!("#{} references missing #{}", from.0, id.0))
        })?;
        let value = View::new(id, entity);
        path.push(id);
        let components = value
            .component_refs()
            .into_iter()
            .map(|component| applied(model, id, component, depth + 1, path))
            .collect::<Result<_, _>>()?;
        path.pop();
        let owned = |text: Option<&str>| text.map(str::to_owned);
        let unit_basis = value.unit_basis(&model.inner).map(|basis| UnitBasis {
            id: basis.id.0,
            value: value
                .unit_basis_component(&model.inner)
                .map_or(Tagged::Null, Tagged::from_value),
            unit: basis.unit.map(|EntityId(id)| id),
        });
        Ok(CostValue {
            id: id.0,
            name: owned(value.name()),
            description: owned(value.description()),
            category: owned(value.category()),
            condition: owned(value.condition()),
            applied_value: value
                .applied_value()
                .map_or(Tagged::Null, Tagged::from_value),
            operator: value
                .operator()
                .map(|operator| format!("{operator:?}").to_ascii_uppercase()),
            unit_basis,
            components,
        })
    }

    fn ids(ids: Vec<EntityId>) -> Vec<u64> {
        ids.into_iter().map(|EntityId(id)| id).collect()
    }

    fn error(error: CostError) -> BindingError {
        match error {
            CostError::UnsupportedSchema { schema } => BindingError::UnsupportedSchema(schema),
            CostError::MultipleSchemas { schemas } => {
                BindingError::UnsupportedSchema(format!("{schemas} schemas declared"))
            }
            CostError::MissingReference { .. } => BindingError::MissingReference(error.to_string()),
            other => BindingError::InvalidModel(other.to_string()),
        }
    }
}

impl ToRecord for Cost {
    fn to_record(&self) -> Record {
        Record::new(
            "Cost",
            vec![
                ("schedules", Field::records(&self.schedules)),
                ("items", Field::records(&self.items)),
                ("anomalies", Field::records(&self.anomalies)),
            ],
        )
    }
}

impl ToRecord for CostSchedule {
    fn to_record(&self) -> Record {
        Record::new(
            "CostSchedule",
            vec![
                ("id", Field::Id(self.id)),
                ("global_id", Field::text(self.global_id.clone())),
                ("name", Field::text(self.name.clone())),
                ("identification", Field::text(self.identification.clone())),
                ("status", Field::text(self.status.clone())),
                ("predefined_type", Field::text(self.predefined_type.clone())),
                ("items", Field::ids(self.items.iter().copied())),
            ],
        )
    }
}

impl ToRecord for CostItem {
    fn to_record(&self) -> Record {
        Record::new(
            "CostItem",
            vec![
                ("id", Field::Id(self.id)),
                ("global_id", Field::text(self.global_id.clone())),
                ("name", Field::text(self.name.clone())),
                ("identification", Field::text(self.identification.clone())),
                ("description", Field::text(self.description.clone())),
                ("predefined_type", Field::text(self.predefined_type.clone())),
                ("parent", Field::id(self.parent)),
                ("children", Field::ids(self.children.iter().copied())),
                ("values", Field::records(&self.values)),
                ("quantities", Field::ids(self.quantities.iter().copied())),
                ("objects", Field::ids(self.objects.iter().copied())),
            ],
        )
    }
}

impl ToRecord for CostValue {
    fn to_record(&self) -> Record {
        Record::new(
            "CostValue",
            vec![
                ("id", Field::Id(self.id)),
                ("name", Field::text(self.name.clone())),
                ("description", Field::text(self.description.clone())),
                ("category", Field::text(self.category.clone())),
                ("condition", Field::text(self.condition.clone())),
                ("applied_value", Field::Value(self.applied_value.clone())),
                ("operator", Field::text(self.operator.clone())),
                ("unit_basis", Field::record(self.unit_basis.as_ref())),
                ("components", Field::records(&self.components)),
            ],
        )
    }
}

impl ToRecord for UnitBasis {
    fn to_record(&self) -> Record {
        Record::new(
            "UnitBasis",
            vec![
                ("id", Field::Id(self.id)),
                ("value", Field::Value(self.value.clone())),
                ("unit", Field::id(self.unit)),
            ],
        )
    }
}

impl ToRecord for CostAnomaly {
    fn to_record(&self) -> Record {
        Record::new(
            "CostAnomaly",
            vec![
                ("item", Field::Id(self.item)),
                ("kept", Field::Id(self.kept)),
                ("rejected", Field::Id(self.rejected)),
                ("relation", Field::Id(self.relation)),
            ],
        )
    }
}
