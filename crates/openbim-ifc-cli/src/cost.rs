//! `cost`: every cost schedule with its tree of cost items.
//!
//! Through the facade's `ifc::cost` view: a schedule's items are those
//! assigned to it (`IfcRelAssignsToControl`) that no other of its items
//! nests; each item lists its own cost values (name, category, applied
//! value, operator), its direct total (its own values) and, when it nests
//! items (`IfcRelNests`), the rolled-up total of their values: IFC does
//! not say whether a parent's value summarises its children, so both are
//! shown and neither is called the total. An item nested twice is
//! shown under the first nesting the file states, as the view reads it.
//! `--depth N` expands N levels of nested items and counts the rest.

use std::io::Write;

use ifc::cost::{
    children_of, controlled_by, direct_total, rolled_up_total, CostError, CostItem, CostValue,
    CostView,
};
use ifc::selector::{value_text, SelectedValue};
use ifc::{EntityId, Model};

use crate::cli::{CostArgs, TextFormat};
use crate::error::{CliError, CliResult, Outcome};
use crate::input;
use crate::json::Json;

/// One cost item and what it nests.
struct Item {
    id: u64,
    identification: Option<String>,
    name: Option<String>,
    values: Vec<Value>,
    direct: f64,
    rolled_up: Option<f64>,
    children: Children,
}

enum Children {
    Expanded(Vec<Item>),
    Truncated(usize),
}

struct Value {
    name: Option<String>,
    category: Option<String>,
    applied: Option<String>,
    operator: Option<String>,
}

/// Run `cost`.
pub(crate) fn run(args: &CostArgs, out: &mut impl Write) -> CliResult<Outcome> {
    let loaded = input::read(&args.file, args.input.input_layout)?;
    let model = &loaded.model;
    input::declared_schema(model)?;
    let view = CostView::new(model);
    let items: Vec<CostItem<'_>> = view.items().collect();
    let mut schedules = Vec::new();
    for schedule in view.schedules().map_err(error)? {
        let assigned = controlled_by(model, schedule.id());
        let roots: Vec<EntityId> = assigned
            .iter()
            .copied()
            .filter(|id| {
                !assigned
                    .iter()
                    .any(|parent| children_of(model, *parent).contains(id))
            })
            .collect();
        let mut path = Vec::new();
        let tree = roots
            .into_iter()
            .filter_map(|id| item(model, &view, &items, id, args.depth, &mut path).transpose())
            .collect::<CliResult<Vec<_>>>()?;
        schedules.push((
            schedule.id().0,
            schedule.name().map(str::to_owned),
            schedule.predefined_type().map(str::to_owned),
            tree,
        ));
    }
    let text = match args.format {
        TextFormat::Human => {
            if schedules.is_empty() {
                "no cost schedule\n".to_owned()
            } else {
                let mut text = String::new();
                for (id, name, kind, tree) in &schedules {
                    text.push_str(&format!(
                        "#{id} IfcCostSchedule{}{}\n",
                        name.as_ref()
                            .map_or_else(String::new, |n| format!(" {n:?}")),
                        kind.as_ref()
                            .map_or_else(String::new, |k| format!(" ({k})"))
                    ));
                    for item in tree {
                        human(item, 1, &mut text);
                    }
                }
                text
            }
        }
        TextFormat::Json => Json::Array(
            schedules
                .iter()
                .map(|(id, name, kind, tree)| {
                    Json::object([
                        ("id", Json::uint(*id)),
                        ("name", Json::opt_str(name.clone())),
                        ("predefined_type", Json::opt_str(kind.clone())),
                        ("items", Json::Array(tree.iter().map(item_json).collect())),
                    ])
                })
                .collect(),
        )
        .pretty(),
    };
    out.write_all(text.as_bytes())
        .map_err(|error| CliError::stdout(&error))?;
    Ok(Outcome::Clean)
}

/// Item `id` with its nested items to `depth` levels; `None` for an id that
/// is no cost item.
fn item(
    model: &Model,
    view: &CostView<'_>,
    items: &[CostItem<'_>],
    id: EntityId,
    depth: Option<usize>,
    path: &mut Vec<EntityId>,
) -> CliResult<Option<Item>> {
    let Some(found) = items.iter().find(|item| item.id() == id) else {
        return Ok(None);
    };
    if path.contains(&id) {
        return Err(CliError::InvalidModel(format!(
            "cost item #{} nests itself",
            id.0
        )));
    }
    let values = found
        .value_refs()
        .into_iter()
        .filter_map(|value| model.get(value).map(|entity| CostValue::new(value, entity)))
        .map(|value| Value {
            name: value.name().map(str::to_owned),
            category: value.category().map(str::to_owned),
            applied: value
                .applied_value()
                .map(|applied| value_text(&SelectedValue::from_model(applied), "")),
            operator: value
                .operator()
                .map(|operator| format!("{operator:?}").to_ascii_uppercase()),
        })
        .collect();
    let nested = children_of(model, id);
    let children = match depth {
        Some(0) if !nested.is_empty() => Children::Truncated(nested.len()),
        _ => {
            path.push(id);
            let mut expanded = Vec::new();
            for child in nested {
                if let Some(child) = item(
                    model,
                    view,
                    items,
                    child,
                    depth.map(|d| d.saturating_sub(1)),
                    path,
                )? {
                    expanded.push(child);
                }
            }
            path.pop();
            Children::Expanded(expanded)
        }
    };
    Ok(Some(Item {
        id: id.0,
        identification: found.identification().map(str::to_owned),
        name: found.name().map(str::to_owned),
        values,
        // An empty sum is -0.0; a report says 0.
        direct: direct_total(view, found) + 0.0,
        rolled_up: if children_of(model, id).is_empty() {
            None
        } else {
            rolled_up_total(view, found).ok().map(|total| total + 0.0)
        },
        children,
    }))
}

fn human(item: &Item, depth: usize, text: &mut String) {
    let indent = "  ".repeat(depth);
    let mut line = format!("{indent}#{}", item.id);
    if let Some(identification) = &item.identification {
        line.push_str(&format!(" {identification}"));
    }
    if let Some(name) = &item.name {
        line.push_str(&format!(" {name:?}"));
    }
    line.push_str(&format!(" direct {}", item.direct));
    if let Some(rolled_up) = item.rolled_up {
        line.push_str(&format!(", nested {rolled_up}"));
    }
    text.push_str(&line);
    text.push('\n');
    for value in &item.values {
        let mut parts = Vec::new();
        if let Some(category) = &value.category {
            parts.push(category.clone());
        }
        if let Some(name) = &value.name {
            parts.push(format!("{name:?}"));
        }
        if let Some(applied) = &value.applied {
            parts.push(applied.clone());
        }
        if let Some(operator) = &value.operator {
            parts.push(format!("({operator} of its components)"));
        }
        text.push_str(&format!("{indent}  value {}\n", parts.join(" ")));
    }
    match &item.children {
        Children::Expanded(children) => {
            for child in children {
                human(child, depth + 1, text);
            }
        }
        Children::Truncated(count) => {
            text.push_str(&format!(
                "{indent}  ... {count} nested item(s) not expanded\n"
            ));
        }
    }
}

fn item_json(item: &Item) -> Json {
    let values = item
        .values
        .iter()
        .map(|value| {
            Json::object([
                ("name", Json::opt_str(value.name.clone())),
                ("category", Json::opt_str(value.category.clone())),
                ("applied_value", Json::opt_str(value.applied.clone())),
                ("operator", Json::opt_str(value.operator.clone())),
            ])
        })
        .collect();
    let children = match &item.children {
        Children::Expanded(children) => Json::Array(children.iter().map(item_json).collect()),
        Children::Truncated(count) => Json::object([
            ("truncated", Json::Bool(true)),
            ("count", Json::uint(*count as u64)),
        ]),
    };
    Json::object([
        ("id", Json::uint(item.id)),
        ("identification", Json::opt_str(item.identification.clone())),
        ("name", Json::opt_str(item.name.clone())),
        ("values", Json::Array(values)),
        ("direct_total", Json::Real(item.direct)),
        (
            "rolled_up_total",
            item.rolled_up.map_or(Json::Null, Json::Real),
        ),
        ("items", children),
    ])
}

fn error(error: CostError) -> CliError {
    match error {
        CostError::UnsupportedSchema { .. } | CostError::MultipleSchemas { .. } => {
            CliError::UnsupportedSchema(error.to_string())
        }
        CostError::MissingReference { .. } => CliError::MissingReference(error.to_string()),
        other => CliError::InvalidModel(other.to_string()),
    }
}
