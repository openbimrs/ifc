use ifc_model::{Entity, EntityId, Model, Transaction, Value};

use crate::ArithmeticOperator;

use super::draft::{CostValueDraft, CostValueKind};
use super::validate::{invalid, reference_type};
use super::CostAuthoringResult;

/// Validate and stage one scalar or explicitly composed IFC4 cost value.
pub fn create_cost_value(
    tx: &mut Transaction,
    model: &Model,
    draft: CostValueDraft<'_>,
) -> CostAuthoringResult<EntityId> {
    let (applied_value, operator, components) = match draft.kind {
        CostValueKind::Monetary(amount) => {
            if !amount.is_finite() {
                return Err(invalid(
                    "IFCCOSTVALUE",
                    "AppliedValue",
                    "expected a finite monetary amount",
                ));
            }
            (
                Value::Typed {
                    type_name: "IFCMONETARYMEASURE".into(),
                    value: Box::new(Value::Real(amount)),
                },
                Value::Null,
                Value::Null,
            )
        }
        CostValueKind::Components {
            operator,
            components,
        } => {
            if components.is_empty() {
                return Err(invalid(
                    "IFCCOSTVALUE",
                    "Components",
                    "expected at least one component",
                ));
            }
            for target in components {
                reference_type(
                    tx,
                    model,
                    "IFCCOSTVALUE",
                    "Components",
                    *target,
                    "IFCCOSTVALUE",
                )?;
            }
            (
                Value::Null,
                Value::Enum(operator_token(operator).into()),
                refs(components),
            )
        }
    };
    Ok(tx.create(Entity::new(
        "IFCCOSTVALUE",
        vec![
            optional_text(draft.name),
            optional_text(draft.description),
            applied_value,
            Value::Null,
            optional_text(draft.applicable_date),
            optional_text(draft.fixed_until_date),
            optional_text(draft.category),
            optional_text(draft.condition),
            operator,
            components,
        ],
    )))
}

fn operator_token(operator: ArithmeticOperator) -> &'static str {
    match operator {
        ArithmeticOperator::Add => "ADD",
        ArithmeticOperator::Divide => "DIVIDE",
        ArithmeticOperator::Multiply => "MULTIPLY",
        ArithmeticOperator::Subtract => "SUBTRACT",
    }
}

pub(crate) fn optional_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |value| Value::Text(value.into()))
}
pub(crate) fn optional_enum(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |value| Value::Enum(value.into()))
}
pub(crate) fn refs(ids: &[EntityId]) -> Value {
    Value::List(ids.iter().copied().map(Value::Ref).collect())
}

/// Stage an `IfcMonetaryUnit`.
///
/// `Currency` is an `IfcLabel` in IFC4, not an enum, so any string
/// parses. The reader resolves a file's currency by matching this
/// text, which is why a blank one is refused rather than written: it
/// would leave every cost value denominated in nothing.
///
/// # Errors
///
/// Refuses a blank currency.
pub fn create_monetary_unit(tx: &mut Transaction, currency: &str) -> CostAuthoringResult<EntityId> {
    if currency.trim().is_empty() {
        return Err(invalid(
            "IFCMONETARYUNIT",
            "Currency",
            "expected a currency label",
        ));
    }
    Ok(tx.create(Entity::new(
        "IFCMONETARYUNIT",
        vec![Value::Text(currency.into())],
    )))
}
